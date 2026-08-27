//! The cached draft dataset: everything the advisor needs, with no network.
//!
//! Matchup statistics are per-patch aggregates — they do not change during a
//! draft. So the whole thing is fetched once (~132 requests), cached, and
//! every later suggestion is local matrix arithmetic. That keeps the network
//! out of the seconds where the user actually has to decide, and means the
//! feature still works when STRATZ is slow or down.
//!
//! Stored as a compact binary: four `n x n` matrices at 127 heroes is ~258KB
//! packed, and several megabytes as JSON.

use std::io::{Read, Write};
use std::path::Path;

/// Positions in the order STRATZ names them, index 0-4 == position 1-5.
pub const POSITIONS: [&str; 5] = [
    "POSITION_1",
    "POSITION_2",
    "POSITION_3",
    "POSITION_4",
    "POSITION_5",
];

pub const NUM_POSITIONS: usize = 5;

const MAGIC: &[u8; 4] = b"STRZ";
const VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeroEntry {
    /// STRATZ / Dota hero id.
    pub id: i16,
    /// Internal short name (`nevermore`) — matches the draft reader's slugs.
    pub slug: String,
    /// Human-facing name (`Shadow Fiend`).
    pub display_name: String,
}

/// Everything needed to rank a pick, indexed by dense hero index (not id).
#[derive(Debug, Clone, Default)]
pub struct StratzDataset {
    pub heroes: Vec<HeroEntry>,
    /// `advantage[i * n + j]`: hero i's win rate against hero j, minus i's own
    /// baseline. Stored as an offset so "this hero is simply strong" does not
    /// masquerade as "this hero counters everything".
    pub advantage: Vec<f32>,
    /// Sample size behind each `advantage` entry, for reliability shrinkage.
    pub vs_matches: Vec<u32>,
    /// `synergy[i * n + j]`: same idea for heroes played together.
    pub synergy: Vec<f32>,
    pub with_matches: Vec<u32>,
    /// Each hero's overall win rate, the baseline the offsets are measured
    /// against.
    pub base_win_rate: Vec<f32>,
    /// `position_win_rate[i * 5 + p]`, or -1 where there is no data.
    pub position_win_rate: Vec<f32>,
    /// `position_share[i * 5 + p]`: fraction of this hero's games in that
    /// position. Distinguishes "plays this role" from "won once here".
    pub position_share: Vec<f32>,
    /// Unix seconds; drives cache expiry.
    pub built_at: u64,
    /// Which rank bracket the numbers describe, so the UI can say so.
    pub bracket: String,
}

impl StratzDataset {
    pub fn len(&self) -> usize {
        self.heroes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.heroes.is_empty()
    }

    /// Dense index for a draft-reader slug, tolerating the naming variants
    /// STRATZ and Dota disagree on.
    pub fn index_of_slug(&self, slug: &str) -> Option<usize> {
        let needle = normalise_slug(slug);
        self.heroes
            .iter()
            .position(|h| normalise_slug(&h.slug) == needle)
    }

    pub fn hero(&self, index: usize) -> Option<&HeroEntry> {
        self.heroes.get(index)
    }

    pub fn advantage_of(&self, hero: usize, against: usize) -> (f32, u32) {
        let n = self.len();
        if hero >= n || against >= n {
            return (0.0, 0);
        }
        (self.advantage[hero * n + against], self.vs_matches[hero * n + against])
    }

    pub fn synergy_of(&self, hero: usize, with: usize) -> (f32, u32) {
        let n = self.len();
        if hero >= n || with >= n {
            return (0.0, 0);
        }
        (self.synergy[hero * n + with], self.with_matches[hero * n + with])
    }

    /// Win rate for a hero in a position, or `None` where unmeasured.
    pub fn position_win_rate_of(&self, hero: usize, position: usize) -> Option<f32> {
        if hero >= self.len() || position >= NUM_POSITIONS {
            return None;
        }
        let v = self.position_win_rate[hero * NUM_POSITIONS + position];
        (v >= 0.0).then_some(v)
    }

    pub fn position_share_of(&self, hero: usize, position: usize) -> f32 {
        if hero >= self.len() || position >= NUM_POSITIONS {
            return 0.0;
        }
        self.position_share[hero * NUM_POSITIONS + position]
    }

    /// Whether the cache is younger than `ttl_hours`.
    pub fn is_fresh(&self, ttl_hours: u64, now_unix: u64) -> bool {
        if self.built_at == 0 || self.is_empty() {
            return false;
        }
        // A clock that jumped backwards must not make a cache immortal.
        now_unix
            .checked_sub(self.built_at)
            .is_some_and(|age| age < ttl_hours.saturating_mul(3600))
    }

    /// Fail loudly rather than silently ranking against a half-built matrix.
    fn validate(&self) -> Result<(), String> {
        let n = self.len();
        let square = n * n;
        for (name, got) in [
            ("advantage", self.advantage.len()),
            ("vs_matches", self.vs_matches.len()),
            ("synergy", self.synergy.len()),
            ("with_matches", self.with_matches.len()),
        ] {
            if got != square {
                return Err(format!("{name} has {got} entries, expected {square}"));
            }
        }
        if self.base_win_rate.len() != n {
            return Err(format!(
                "base_win_rate has {} entries, expected {n}",
                self.base_win_rate.len()
            ));
        }
        for (name, got) in [
            ("position_win_rate", self.position_win_rate.len()),
            ("position_share", self.position_share.len()),
        ] {
            if got != n * NUM_POSITIONS {
                return Err(format!(
                    "{name} has {got} entries, expected {}",
                    n * NUM_POSITIONS
                ));
            }
        }
        Ok(())
    }
}

/// STRATZ and Dota disagree on separators for a handful of heroes
/// (`vengefulspirit` vs `vengeful_spirit`), so compare without them.
fn normalise_slug(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

pub fn encode(dataset: &StratzDataset) -> Result<Vec<u8>, String> {
    dataset.validate()?;

    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&(dataset.len() as u32).to_le_bytes());
    out.extend_from_slice(&dataset.built_at.to_le_bytes());
    write_string(&mut out, &dataset.bracket);

    for hero in &dataset.heroes {
        out.extend_from_slice(&hero.id.to_le_bytes());
        write_string(&mut out, &hero.slug);
        write_string(&mut out, &hero.display_name);
    }
    for v in &dataset.advantage {
        out.extend_from_slice(&v.to_le_bytes());
    }
    for v in &dataset.vs_matches {
        out.extend_from_slice(&v.to_le_bytes());
    }
    for v in &dataset.synergy {
        out.extend_from_slice(&v.to_le_bytes());
    }
    for v in &dataset.with_matches {
        out.extend_from_slice(&v.to_le_bytes());
    }
    for v in &dataset.base_win_rate {
        out.extend_from_slice(&v.to_le_bytes());
    }
    for v in &dataset.position_win_rate {
        out.extend_from_slice(&v.to_le_bytes());
    }
    for v in &dataset.position_share {
        out.extend_from_slice(&v.to_le_bytes());
    }
    Ok(out)
}

pub fn decode(bytes: &[u8]) -> Result<StratzDataset, String> {
    let mut r = Cursor::new(bytes);

    let magic = r.take(4)?;
    if magic != MAGIC {
        return Err("not a STRATZ dataset (bad magic)".into());
    }
    let version = u16::from_le_bytes(r.take_array::<2>()?);
    if version != VERSION {
        // A stale format is a cache to rebuild, not a crash.
        return Err(format!("unsupported dataset version {version}"));
    }
    let n = u32::from_le_bytes(r.take_array::<4>()?) as usize;
    let built_at = u64::from_le_bytes(r.take_array::<8>()?);
    let bracket = r.take_string()?;

    let mut heroes = Vec::with_capacity(n);
    for _ in 0..n {
        let id = i16::from_le_bytes(r.take_array::<2>()?);
        let slug = r.take_string()?;
        let display_name = r.take_string()?;
        heroes.push(HeroEntry { id, slug, display_name });
    }

    let square = n * n;
    let dataset = StratzDataset {
        heroes,
        advantage: r.take_f32s(square)?,
        vs_matches: r.take_u32s(square)?,
        synergy: r.take_f32s(square)?,
        with_matches: r.take_u32s(square)?,
        base_win_rate: r.take_f32s(n)?,
        position_win_rate: r.take_f32s(n * NUM_POSITIONS)?,
        position_share: r.take_f32s(n * NUM_POSITIONS)?,
        built_at,
        bracket,
    };
    dataset.validate()?;
    Ok(dataset)
}

pub fn save(dataset: &StratzDataset, path: &Path) -> Result<(), String> {
    let bytes = encode(dataset)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    // Write-then-rename, so an interrupted save cannot leave a truncated
    // cache that decodes as a valid but wrong dataset.
    let tmp = path.with_extension("tmp");
    let mut file = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
    file.write_all(&bytes).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

pub fn load(path: &Path) -> Result<StratzDataset, String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    decode(&bytes)
}

fn write_string(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as u16).to_le_bytes());
    out.extend_from_slice(s.as_bytes());
}

/// Minimal bounds-checked reader; every `take` fails rather than panicking on
/// a truncated file.
struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self.pos.checked_add(n).ok_or("length overflow")?;
        if end > self.bytes.len() {
            return Err("dataset is truncated".into());
        }
        let slice = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    fn take_array<const N: usize>(&mut self) -> Result<[u8; N], String> {
        let slice = self.take(N)?;
        let mut buf = [0u8; N];
        buf.copy_from_slice(slice);
        Ok(buf)
    }

    fn take_string(&mut self) -> Result<String, String> {
        let len = u16::from_le_bytes(self.take_array::<2>()?) as usize;
        let bytes = self.take(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|e| e.to_string())
    }

    fn take_f32s(&mut self, count: usize) -> Result<Vec<f32>, String> {
        let mut out = Vec::with_capacity(count);
        for _ in 0..count {
            out.push(f32::from_le_bytes(self.take_array::<4>()?));
        }
        Ok(out)
    }

    fn take_u32s(&mut self, count: usize) -> Result<Vec<u32>, String> {
        let mut out = Vec::with_capacity(count);
        for _ in 0..count {
            out.push(u32::from_le_bytes(self.take_array::<4>()?));
        }
        Ok(out)
    }
}

#[cfg(test)]
pub(crate) fn sample_dataset(slugs: &[&str]) -> StratzDataset {
    let n = slugs.len();
    StratzDataset {
        heroes: slugs
            .iter()
            .enumerate()
            .map(|(i, s)| HeroEntry {
                id: i as i16 + 1,
                slug: (*s).to_string(),
                display_name: s.to_uppercase(),
            })
            .collect(),
        advantage: vec![0.0; n * n],
        vs_matches: vec![0; n * n],
        synergy: vec![0.0; n * n],
        with_matches: vec![0; n * n],
        base_win_rate: vec![0.5; n],
        position_win_rate: vec![-1.0; n * NUM_POSITIONS],
        position_share: vec![0.0; n * NUM_POSITIONS],
        built_at: 1_000,
        bracket: "DIVINE_IMMORTAL".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_through_the_binary_format() {
        let mut d = sample_dataset(&["antimage", "axe", "nevermore"]);
        // hero 0 vs hero 1, and hero 2 with hero 0, in a 3-hero dataset.
        d.advantage[1] = 0.25;
        d.vs_matches[1] = 1234;
        d.synergy[6] = -0.125;
        d.with_matches[6] = 77;
        d.base_win_rate[1] = 0.5234375; // exactly representable in f32
        // hero 1, position 3.
        d.position_win_rate[NUM_POSITIONS + 2] = 0.5;
        d.position_share[NUM_POSITIONS + 2] = 0.75;

        let decoded = decode(&encode(&d).unwrap()).unwrap();

        assert_eq!(decoded.heroes, d.heroes);
        assert_eq!(decoded.bracket, "DIVINE_IMMORTAL");
        assert_eq!(decoded.built_at, 1_000);
        assert_eq!(decoded.advantage_of(0, 1), (0.25, 1234));
        assert_eq!(decoded.synergy_of(2, 0), (-0.125, 77));
        assert_eq!(decoded.base_win_rate[1], 0.5234375);
        assert_eq!(decoded.position_win_rate_of(1, 2), Some(0.5));
        assert_eq!(decoded.position_share_of(1, 2), 0.75);
    }

    #[test]
    fn a_truncated_cache_is_an_error_not_a_panic() {
        let d = sample_dataset(&["antimage", "axe"]);
        let bytes = encode(&d).unwrap();
        for cut in [0, 4, 10, bytes.len() / 2, bytes.len() - 1] {
            assert!(decode(&bytes[..cut]).is_err(), "cut at {cut} should fail");
        }
    }

    #[test]
    fn foreign_bytes_are_rejected() {
        assert!(decode(b"not a dataset at all").is_err());
    }

    #[test]
    fn a_future_version_is_rejected_so_it_can_be_rebuilt() {
        let mut bytes = encode(&sample_dataset(&["axe"])).unwrap();
        bytes[4..6].copy_from_slice(&99u16.to_le_bytes());
        let err = decode(&bytes).unwrap_err();
        assert!(err.contains("version"), "{err}");
    }

    #[test]
    fn a_ragged_dataset_never_encodes() {
        // Guards the failure this format is most likely to hit: a refresh
        // that died partway leaving matrices of mismatched size.
        let mut d = sample_dataset(&["axe", "lina"]);
        d.advantage.pop();
        assert!(encode(&d).is_err());
    }

    #[test]
    fn slugs_resolve_across_separator_differences() {
        let d = sample_dataset(&["vengefulspirit", "skeleton_king", "nevermore"]);
        // The draft reader and STRATZ do not always agree on underscores.
        assert_eq!(d.index_of_slug("vengeful_spirit"), Some(0));
        assert_eq!(d.index_of_slug("vengefulspirit"), Some(0));
        assert_eq!(d.index_of_slug("SkeletonKing"), Some(1));
        assert_eq!(d.index_of_slug("nevermore"), Some(2));
        assert_eq!(d.index_of_slug("not_a_hero"), None);
    }

    #[test]
    fn freshness_expires_and_survives_a_backwards_clock() {
        let d = sample_dataset(&["axe"]); // built_at = 1000
        assert!(d.is_fresh(24, 1_000 + 3600));
        assert!(!d.is_fresh(24, 1_000 + 24 * 3600));
        // Clock jumped backwards: not fresh, and no underflow panic.
        assert!(!d.is_fresh(24, 500));
    }

    #[test]
    fn an_empty_dataset_is_never_fresh() {
        let d = StratzDataset { built_at: u64::MAX / 2, ..Default::default() };
        assert!(!d.is_fresh(24, 1_000));
    }

    #[test]
    fn out_of_range_lookups_return_neutral_values() {
        let d = sample_dataset(&["axe"]);
        assert_eq!(d.advantage_of(9, 0), (0.0, 0));
        assert_eq!(d.synergy_of(0, 9), (0.0, 0));
        assert_eq!(d.position_win_rate_of(0, 9), None);
        assert_eq!(d.position_share_of(9, 0), 0.0);
    }

    #[test]
    fn unmeasured_positions_read_as_none_not_minus_one() {
        // -1.0 is the sentinel in storage; it must never escape as a win rate.
        let d = sample_dataset(&["axe"]);
        assert_eq!(d.position_win_rate_of(0, 0), None);
    }
}
