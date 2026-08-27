//! Draft-strip vision: locate the ten pick slots and identify their heroes.
//!
//! Promoted from `examples/draft_match_probe.rs` after the approach was proven
//! against labelled captures — 105/106 slots correct single-frame across eight
//! games (the one miss is a mid-reveal frame that frame voting recovers). The
//! probe remains the research harness; this module carries only what the
//! measurements settled. Every constant below is measured, not designed, and
//! the probe's module header holds the full results table.
//!
//! The pipeline, and the finding that forced each stage:
//!
//! 1. **Geometry** ([`resolve_slots`], [`strip_region`]) — the strip is
//!    symmetric about the horizontal centre and scales with client *height*
//!    (Panorama behaviour), so ten slots reduce to five height-relative
//!    numbers.
//! 2. **Occupancy gate** ([`OCCUPIED_MIN_STDDEV`]) — slots caught mid-reveal
//!    score 11–17 luma stddev against 32–62 settled; every one that slipped
//!    through produced a confident wrong answer.
//! 3. **Descriptor** ([`fingerprint`]) — 16×16×3 box-downscale, per-channel
//!    zero-mean/unit-variance, unit length. The normalisation discards the
//!    grading difference between Valve's CDN art and Dota's live strip.
//! 4. **Trimmed similarity** ([`trimmed_similarity`]) — Dota paints badges and
//!    overlays over portraits; dropping the worst-agreeing 15% of cells
//!    recovers the match without cataloguing overlays.
//! 5. **Reference pool** ([`Reference`]) — CDN art bootstraps every hero;
//!    harvested same-domain crops (arcanas, personas) join it with a score
//!    penalty pricing out their domain advantage; negative references
//!    (empty-slot logo, menu badges) let non-hero content lose explicitly.
//! 6. **Vote aggregation** ([`SlotVotes`]) — agreement *rate* across frames
//!    separates settled reads from unmatchable portraits at any draft length;
//!    absolute vote counts do not.
//!
//! What this module deliberately does not know: *when* to look. Pointed at the
//! main menu it will happily rank heroes for menu icons — the gate is GSI's
//! `map.game_state` ([`draft_gate_open`]), owned by the caller.

use image::RgbaImage;

// ---------------------------------------------------------------------------
// Slot geometry — measured at 1920x1080 off logs/draft_capture.
//
// Expressed in units of client height off the centre line: Panorama scales
// with viewport height and centres horizontally, so width-relative fractions
// would drift with aspect ratio.
// ---------------------------------------------------------------------------

const CENTER_GAP: f32 = 138.0 / 1080.0;
const SLOT_WIDTH: f32 = 107.0 / 1080.0;
const SLOT_HEIGHT: f32 = 64.0 / 1080.0;
const PITCH: f32 = 124.0 / 1080.0;

/// Slots per side; the strip is always Radiant-left, Dire-right.
pub const SLOTS_PER_TEAM: usize = 5;
pub const TOTAL_SLOTS: usize = SLOTS_PER_TEAM * 2;

/// Contrast below this means the slot holds no usable portrait.
///
/// Most settled portraits measure 32–62, but plain Shadow Fiend's near-black
/// smoke portrait sits at 23.3 — under the original gate of 25 it was silently
/// dropped in every game it appeared in, four sessions running.
///
/// Lowering to 20 admits a 20–25 band worth 135 of 4230 measured slot-frames,
/// and that band holds two cleanly separable populations: reveal-fade frames,
/// which appear **once** per slot per session, and real dark portraits, which
/// persist for 13–28 frames. Vote aggregation already separates them
/// ([`MIN_OCCUPIED_FRAMES`], [`MIN_AGREEMENT`]), so the gate does not have to.
/// Empty slots idle far below at 11–13.
pub const OCCUPIED_MIN_STDDEV: f32 = 20.0;

/// Rows 0–5 of every slot are the player-colour bar — fully saturated colour
/// fixed per slot *index*, identical across games. Left in, fingerprints match
/// by seat instead of hero.
pub const TOP_INSET: f32 = 6.0 / 1080.0;

/// Descriptor edge; 16 was measured best (12–32 swept).
pub const DESCRIPTOR_DIM: usize = 16;

/// Fraction of worst-agreeing cells dropped when scoring. 0.10–0.30 all reach
/// the same accuracy; 0.15 sits mid-plateau.
pub const TRIM: f32 = 0.15;

/// Minimum winner-over-runner-up margin for a frame's read to count at all.
/// The lowest observed *correct* margin is 0.016.
pub const MARGIN_MIN: f32 = 0.01;

/// Score handicap on same-domain (harvested/healed) references.
///
/// A crop of the strip shares the probe's grading, scale and overlays, which
/// inflates its score against *any* live capture: the Legion Commander arcana
/// crop outscored Pugna's own CDN portrait in Pugna's slot, 0.623 to 0.585.
/// 0.10 clears the worst observed steal (0.038) with room, while staying far
/// below the gap an exemplar enjoys matching its own portrait.
pub const HARVESTED_PENALTY: f32 = 0.10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotRect {
    /// 0-4 left team, 5-9 right team.
    pub index: usize,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Resolve all ten slot rectangles for a client of the given size.
///
/// Slots come back in strip order: left team innermost-last (index 4 adjacent
/// to the centre gap), then right team outward.
pub fn resolve_slots(client_w: u32, client_h: u32) -> Vec<SlotRect> {
    let h = client_h as f32;
    let centre = client_w as f32 / 2.0;
    let gap = CENTER_GAP * h;
    let w = (SLOT_WIDTH * h).round();
    let sh = (SLOT_HEIGHT * h).round();
    let pitch = PITCH * h;
    let inset = (TOP_INSET * h).round().max(0.0);
    let sh = (sh - inset).max(1.0);

    let mut slots = Vec::with_capacity(TOTAL_SLOTS);
    for index in 0..SLOTS_PER_TEAM {
        let from_inner = (SLOTS_PER_TEAM - 1 - index) as f32;
        slots.push(SlotRect {
            index,
            x: (centre - gap - w - pitch * from_inner).round().max(0.0) as u32,
            y: inset as u32,
            w: w as u32,
            h: sh as u32,
        });
    }
    for index in 0..SLOTS_PER_TEAM {
        slots.push(SlotRect {
            index: SLOTS_PER_TEAM + index,
            x: (centre + gap + pitch * index as f32).round().max(0.0) as u32,
            y: inset as u32,
            w: w as u32,
            h: sh as u32,
        });
    }
    slots
}

/// The horizontal band containing all ten slots, as `(x, y, w, h)` in client
/// coordinates. Capturing only this costs ~14ms/144KB against ~60ms/2.4MB for
/// the full frame, which is what makes 1Hz polling free.
pub fn strip_region(client_w: u32, client_h: u32) -> (u32, u32, u32, u32) {
    let h = client_h as f32;
    let centre = client_w as f32 / 2.0;
    let half_span = CENTER_GAP * h + PITCH * h * (SLOTS_PER_TEAM as f32 - 1.0) + SLOT_WIDTH * h;
    let x = (centre - half_span).round().max(0.0);
    let width = (half_span * 2.0).round().min(client_w as f32 - x);
    let height = (SLOT_HEIGHT * h).round().min(client_h as f32);
    (x as u32, 0, width as u32, height as u32)
}

// ---------------------------------------------------------------------------
// Descriptor
// ---------------------------------------------------------------------------

fn box_downscale(pixels: &[u8], width: u32, height: u32, dim: usize) -> Vec<f32> {
    let (w, h) = (width as usize, height as usize);
    let mut out = vec![0f32; dim * dim * 3];

    for ty in 0..dim {
        let y0 = ty * h / dim;
        let y1 = (((ty + 1) * h) / dim).max(y0 + 1).min(h);
        for tx in 0..dim {
            let x0 = tx * w / dim;
            let x1 = (((tx + 1) * w) / dim).max(x0 + 1).min(w);

            let (mut r, mut g, mut b, mut n) = (0f32, 0f32, 0f32, 0f32);
            for y in y0..y1 {
                for x in x0..x1 {
                    let i = (y * w + x) * 4;
                    r += pixels[i] as f32;
                    g += pixels[i + 1] as f32;
                    b += pixels[i + 2] as f32;
                    n += 1.0;
                }
            }

            let idx = (ty * dim + tx) * 3;
            out[idx] = r / n;
            out[idx + 1] = g / n;
            out[idx + 2] = b / n;
        }
    }

    out
}

/// RGBA pixels → unit-length descriptor, or `None` for degenerate input.
pub fn fingerprint(pixels: &[u8], width: u32, height: u32) -> Option<Vec<f32>> {
    if width == 0 || height == 0 || pixels.len() < width as usize * height as usize * 4 {
        return None;
    }

    let dim = DESCRIPTOR_DIM;
    let mut v = box_downscale(pixels, width, height, dim);
    let cells = dim * dim;

    for c in 0..3 {
        let mean = (0..cells).map(|i| v[i * 3 + c]).sum::<f32>() / cells as f32;
        let var = (0..cells).map(|i| (v[i * 3 + c] - mean).powi(2)).sum::<f32>() / cells as f32;
        let sd = var.sqrt().max(1e-6);
        for i in 0..cells {
            v[i * 3 + c] = (v[i * 3 + c] - mean) / sd;
        }
    }

    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-6);
    for x in v.iter_mut() {
        *x /= norm;
    }

    Some(v)
}

/// Cosine similarity ignoring the worst-agreeing [`TRIM`] fraction of cells.
///
/// Overlays (own-hero badge, whatever Valve adds next patch) corrupt a
/// contiguous minority of cells while the rest still agrees; plain cosine
/// spreads that damage over the whole score and can flip a ranking. Kept cells
/// are renormalised so candidates that lost different cells stay comparable.
pub fn trimmed_similarity(a: &[f32], b: &[f32]) -> f32 {
    let cells = a.len() / 3;
    let mut per_cell: Vec<(f32, usize)> = (0..cells)
        .map(|i| {
            let dot = (0..3).map(|c| a[i * 3 + c] * b[i * 3 + c]).sum::<f32>();
            (dot, i)
        })
        .collect();

    per_cell.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap_or(std::cmp::Ordering::Equal));

    let drop = ((cells as f32) * TRIM).round() as usize;
    let kept = &per_cell[drop.min(cells.saturating_sub(1))..];

    let mut dot = 0f32;
    let mut norm_a = 0f32;
    let mut norm_b = 0f32;
    for (cell_dot, i) in kept {
        dot += cell_dot;
        for c in 0..3 {
            norm_a += a[i * 3 + c] * a[i * 3 + c];
            norm_b += b[i * 3 + c] * b[i * 3 + c];
        }
    }

    dot / (norm_a.sqrt() * norm_b.sqrt()).max(1e-6)
}

pub fn luma_std_dev(pixels: &[u8], width: u32, height: u32) -> f32 {
    let count = width as usize * height as usize;
    if count == 0 || pixels.len() < count * 4 {
        return 0.0;
    }
    let mut sum = 0f32;
    let mut sum_sq = 0f32;
    for i in 0..count {
        let p = i * 4;
        let l =
            0.299 * pixels[p] as f32 + 0.587 * pixels[p + 1] as f32 + 0.114 * pixels[p + 2] as f32;
        sum += l;
        sum_sq += l * l;
    }
    let mean = sum / count as f32;
    (sum_sq / count as f32 - mean * mean).max(0.0).sqrt()
}

// ---------------------------------------------------------------------------
// References
// ---------------------------------------------------------------------------

/// One exemplar a slot can match against.
#[derive(Debug, Clone)]
pub struct Reference {
    /// Hero slug (`lina`, `skeleton_king`), or an `_`-prefixed label for a
    /// negative reference.
    pub name: String,
    pub fingerprint: Vec<f32>,
    /// Same-domain crop (harvested or healed) — carries [`HARVESTED_PENALTY`].
    pub harvested: bool,
}

impl Reference {
    /// Negative references are things that occupy a slot without being a hero:
    /// the Dota logo in an unfilled slot, menu badges caught mid-transition.
    /// They compete in the ranking so non-hero content can *win*, and a slot
    /// they win reports empty instead of the nearest hero.
    pub fn is_negative(&self) -> bool {
        self.name.starts_with('_')
    }
}

// --- reference pack ---------------------------------------------------------
//
// The app ships its references pre-computed: descriptors, not images. All 127
// heroes fit in ~400KB, load with zero decoding, and comparing a capture is
// ten crops x ~130 dot products — no network, no cache directory, no PNG
// dependency at runtime. `examples/bake_draft_refs.rs` regenerates the pack
// from `.cache/hero_portraits/` when the art or the descriptor changes.

const PACK_MAGIC: &[u8; 4] = b"DRFT";
const PACK_VERSION: u16 = 1;

/// Serialise references into the binary pack format.
pub fn encode_reference_pack(refs: &[Reference]) -> Vec<u8> {
    let fp_len = DESCRIPTOR_DIM * DESCRIPTOR_DIM * 3;
    let mut out = Vec::with_capacity(refs.len() * (fp_len * 4 + 32));
    out.extend_from_slice(PACK_MAGIC);
    out.extend_from_slice(&PACK_VERSION.to_le_bytes());
    out.extend_from_slice(&(DESCRIPTOR_DIM as u16).to_le_bytes());
    out.extend_from_slice(&(refs.len() as u32).to_le_bytes());
    for r in refs {
        let name = r.name.as_bytes();
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(name);
        out.push(u8::from(r.harvested));
        debug_assert_eq!(r.fingerprint.len(), fp_len);
        for v in &r.fingerprint {
            out.extend_from_slice(&v.to_le_bytes());
        }
    }
    out
}

/// The references baked into the binary at build time.
///
/// Regenerate `assets/draft_reference_pack.bin` with
/// `cargo run --release --example bake_draft_refs` whenever the CDN art, the
/// harvested exemplar set, or the descriptor changes. The pack version check
/// in [`decode_reference_pack`] turns a stale pack into a loud error instead
/// of silent garbage matches.
pub fn builtin_references() -> Result<Vec<Reference>, String> {
    decode_reference_pack(include_bytes!("../../assets/draft_reference_pack.bin"))
}

/// Parse a pack produced by [`encode_reference_pack`].
pub fn decode_reference_pack(bytes: &[u8]) -> Result<Vec<Reference>, String> {
    let mut at = 0usize;
    let take = |at: &mut usize, n: usize| -> Result<&[u8], String> {
        let s = bytes
            .get(*at..*at + n)
            .ok_or_else(|| format!("pack truncated at byte {}", *at))?;
        *at += n;
        Ok(s)
    };

    if take(&mut at, 4)? != PACK_MAGIC {
        return Err("not a draft reference pack (bad magic)".into());
    }
    let version = u16::from_le_bytes(take(&mut at, 2)?.try_into().unwrap());
    if version != PACK_VERSION {
        return Err(format!("pack version {version}, expected {PACK_VERSION}"));
    }
    let dim = u16::from_le_bytes(take(&mut at, 2)?.try_into().unwrap()) as usize;
    if dim != DESCRIPTOR_DIM {
        // A pack baked for a different descriptor is not comparable; failing
        // loudly here beats silently matching garbage.
        return Err(format!("pack descriptor dim {dim}, expected {DESCRIPTOR_DIM}"));
    }
    let count = u32::from_le_bytes(take(&mut at, 4)?.try_into().unwrap()) as usize;
    let fp_len = dim * dim * 3;

    let mut refs = Vec::with_capacity(count);
    for _ in 0..count {
        let name_len = u16::from_le_bytes(take(&mut at, 2)?.try_into().unwrap()) as usize;
        let name = String::from_utf8(take(&mut at, name_len)?.to_vec())
            .map_err(|e| format!("bad reference name: {e}"))?;
        let harvested = take(&mut at, 1)?[0] != 0;
        let raw = take(&mut at, fp_len * 4)?;
        let fingerprint = raw
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
            .collect();
        refs.push(Reference {
            name,
            fingerprint,
            harvested,
        });
    }
    Ok(refs)
}

// ---------------------------------------------------------------------------
// Matching
// ---------------------------------------------------------------------------

/// What one frame concluded about one slot.
#[derive(Debug, Clone)]
pub struct SlotOutcome {
    pub slot: SlotRect,
    pub contrast: f32,
    /// Ranked hero candidates, best first. Empty when the slot held no
    /// portrait, or when a negative reference won.
    pub ranked: Vec<(String, f32)>,
}

impl SlotOutcome {
    /// Best candidate as `(hero, score, margin over runner-up)`.
    pub fn best(&self) -> Option<(&str, f32, f32)> {
        let (hero, score) = self.ranked.first()?;
        let runner_up = self.ranked.get(1).map(|(_, s)| *s).unwrap_or(0.0);
        Some((hero.as_str(), *score, score - runner_up))
    }
}

fn crop(frame: &RgbaImage, r: &SlotRect) -> Option<Vec<u8>> {
    if r.x + r.w > frame.width() || r.y + r.h > frame.height() {
        return None;
    }
    let mut out = Vec::with_capacity((r.w * r.h * 4) as usize);
    for row in 0..r.h {
        for col in 0..r.w {
            out.extend_from_slice(&frame.get_pixel(r.x + col, r.y + row).0);
        }
    }
    Some(out)
}

/// Match every slot in one frame.
///
/// `frame` may be the whole client or a partial capture: slots are resolved in
/// client space from `client_w`/`client_h`, then shifted left by `origin_x`
/// (the capture's x-offset — [`strip_region`]'s x for a strip capture, 0 for a
/// full frame). Resolving geometry from a crop's own dimensions would silently
/// produce nonsense.
pub fn match_frame(
    frame: &RgbaImage,
    client_w: u32,
    client_h: u32,
    origin_x: u32,
    refs: &[Reference],
) -> Vec<SlotOutcome> {
    resolve_slots(client_w, client_h)
        .into_iter()
        .map(|mut slot| {
            slot.x = slot.x.saturating_sub(origin_x);

            let empty = |slot, contrast| SlotOutcome {
                slot,
                contrast,
                ranked: Vec::new(),
            };

            let Some(px) = crop(frame, &slot) else {
                return empty(slot, 0.0);
            };
            let contrast = luma_std_dev(&px, slot.w, slot.h);
            if contrast < OCCUPIED_MIN_STDDEV {
                return empty(slot, contrast);
            }
            let Some(fp) = fingerprint(&px, slot.w, slot.h) else {
                return empty(slot, contrast);
            };

            // Best score per name over its exemplars — per file would let a
            // hero with several exemplars crowd rivals out of the runner-up
            // position and inflate its margin.
            let mut best: std::collections::HashMap<&str, f32> = std::collections::HashMap::new();
            for r in refs {
                // Negative references are exempt from the harvested penalty
                // even though they are same-domain crops: their job is to
                // *win* over hero candidates whenever a slot holds non-hero
                // content, and a handicap would reopen the exact hallucination
                // hole they exist to close.
                let penalty = if r.harvested && !r.is_negative() {
                    HARVESTED_PENALTY
                } else {
                    0.0
                };
                let s = trimmed_similarity(&fp, &r.fingerprint) - penalty;
                let e = best.entry(r.name.as_str()).or_insert(f32::NEG_INFINITY);
                if s > *e {
                    *e = s;
                }
            }
            let mut ranked: Vec<(String, f32)> =
                best.into_iter().map(|(n, s)| (n.to_string(), s)).collect();
            ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            if ranked.first().is_some_and(|(n, _)| n.starts_with('_')) {
                return empty(slot, contrast);
            }
            ranked.retain(|(n, _)| !n.starts_with('_'));
            ranked.truncate(8);

            SlotOutcome {
                slot,
                contrast,
                ranked,
            }
        })
        .collect()
}

/// Extract one slot's raw RGBA crop, for harvesting.
pub fn crop_slot(frame: &RgbaImage, slot: &SlotRect) -> Option<RgbaImage> {
    let px = crop(frame, slot)?;
    RgbaImage::from_raw(slot.w, slot.h, px)
}

// ---------------------------------------------------------------------------
// Vote aggregation
// ---------------------------------------------------------------------------

/// A slot must have looked occupied this many frames before it is judged.
pub const MIN_OCCUPIED_FRAMES: u32 = 2;

/// Share of a slot's occupied frames that must agree on one hero.
///
/// Agreement *rate*, not raw count: both genuine and unmatchable slots gain
/// votes as a draft runs longer, so any absolute threshold that looks right at
/// 7 frames fails at 12 (observed: a Shadow Fiend Arcana slot reached 2 votes
/// and cleared a `>= 2` bar with a wrong hero). As a rate the populations stay
/// apart at any length — genuine slots agree on nearly every occupied frame,
/// unmatchable portraits keep changing their mind.
pub const MIN_AGREEMENT: f32 = 0.6;

/// Accumulated evidence for one slot across the frames of a single draft.
#[derive(Debug, Clone, Default)]
pub struct SlotVotes {
    tally: std::collections::HashMap<String, u32>,
    pub best_score: f32,
    pub frames_seen: u32,
    pub occupied_frames: u32,
    total_votes: u32,
}

impl SlotVotes {
    /// Feed one frame's outcome for this slot.
    pub fn observe(&mut self, outcome: &SlotOutcome) {
        self.frames_seen += 1;
        if outcome.contrast >= OCCUPIED_MIN_STDDEV {
            self.occupied_frames += 1;
        }
        if let Some((hero, score, margin)) = outcome.best() {
            if margin >= MARGIN_MIN {
                *self.tally.entry(hero.to_string()).or_insert(0) += 1;
                self.total_votes += 1;
                self.best_score = self.best_score.max(score);
            }
        }
    }

    pub fn winner(&self) -> Option<(&str, u32)> {
        self.tally
            .iter()
            .max_by_key(|(_, n)| **n)
            .map(|(h, n)| (h.as_str(), *n))
    }

    /// Fraction of occupied frames that agreed on the winning hero.
    pub fn agreement(&self) -> f32 {
        match self.winner() {
            Some((_, n)) => (n as f32) / (self.occupied_frames.max(1) as f32),
            None => 0.0,
        }
    }

    /// Whether the winner is trustworthy. An untrustworthy slot must report
    /// unknown, never its best guess — a draft helper that names the wrong
    /// hero sends the player after a counter-pick that isn't there.
    pub fn confident(&self) -> bool {
        match self.winner() {
            Some((_, n)) => {
                self.occupied_frames >= MIN_OCCUPIED_FRAMES
                    && self.agreement() >= MIN_AGREEMENT
                    && (n as f32) / (self.total_votes.max(1) as f32) >= MIN_AGREEMENT
            }
            None => false,
        }
    }
}

// ---------------------------------------------------------------------------
// Gate and team resolution
// ---------------------------------------------------------------------------

/// States in which the ten-slot strip is on screen. Both confirmed on the wire
/// for a player: HERO_SELECTION is the draft itself, STRATEGY_TIME the
/// strategy screen after it (where bot matches first populate the strip).
pub const DRAFT_STATES: [&str; 2] = [
    "DOTA_GAMERULES_STATE_HERO_SELECTION",
    "DOTA_GAMERULES_STATE_STRATEGY_TIME",
];

pub fn draft_gate_open(game_state: &str) -> bool {
    DRAFT_STATES.contains(&game_state)
}

/// Whether slot `index` (0-9, strip order) belongs to the local player's team.
///
/// The strip is always Radiant-left; geometry alone only knows left and right.
/// Observed directly: the local hero landed on the right-hand side in 3 of 8
/// drafts before `player.team_name` was consulted.
pub fn slot_is_ally(index: usize, team_name: &str) -> bool {
    let left = index < SLOTS_PER_TEAM;
    if team_name.eq_ignore_ascii_case("dire") {
        !left
    } else {
        left
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Paint a deterministic block-structured RGBA image; `seed` varies content.
    ///
    /// Deliberately low-frequency: hero portraits carry structure at the scale
    /// of descriptor cells, and that is what the descriptor measures. Per-pixel
    /// noise would be the degenerate opposite — box-downscale averages it to
    /// near-flat cells whose residue the normalisation then amplifies — and
    /// would test a regime no real capture occupies.
    fn noisy_image(w: u32, h: u32, seed: u32) -> RgbaImage {
        RgbaImage::from_fn(w, h, |x, y| {
            let v = (x / 7)
                .wrapping_mul(31)
                .wrapping_add((y / 4).wrapping_mul(17))
                .wrapping_add(seed.wrapping_mul(97));
            image::Rgba([
                (v.wrapping_mul(13) % 256) as u8,
                (v.wrapping_mul(7) % 256) as u8,
                (v.wrapping_mul(29) % 256) as u8,
                255,
            ])
        })
    }

    fn fp_of(img: &RgbaImage) -> Vec<f32> {
        fingerprint(img.as_raw(), img.width(), img.height()).unwrap()
    }

    // --- geometry, pinned to the measured 1920x1080 values ------------------

    #[test]
    fn slots_at_1080p_match_measured_geometry() {
        let slots = resolve_slots(1920, 1080);
        assert_eq!(slots.len(), 10);
        // Innermost/outermost of each side, exactly as measured off captures.
        assert_eq!((slots[0].x, slots[0].y), (219, 6));
        assert_eq!(slots[4].x, 715);
        assert_eq!(slots[5].x, 1098);
        assert_eq!(slots[9].x, 1594);
        assert_eq!((slots[0].w, slots[0].h), (107, 58));
        // Constant pitch within a side.
        assert_eq!(slots[1].x - slots[0].x, 124);
        assert_eq!(slots[9].x - slots[8].x, 124);
    }

    #[test]
    fn slots_are_symmetric_about_centre() {
        let slots = resolve_slots(1920, 1080);
        for i in 0..SLOTS_PER_TEAM {
            let left = &slots[SLOTS_PER_TEAM - 1 - i];
            let right = &slots[SLOTS_PER_TEAM + i];
            // Mirror: left slot's right edge sits where the right slot's left
            // edge reflects across x = 960.
            assert_eq!(left.x + left.w + right.x, 1920, "pair {i}");
        }
    }

    #[test]
    fn strip_region_at_1080p_is_the_verified_band() {
        assert_eq!(strip_region(1920, 1080), (219, 0, 1482, 64));
    }

    #[test]
    fn geometry_scales_with_height_not_width() {
        // An ultrawide client: slots keep 1080p size, only the centre moves.
        let wide = resolve_slots(2560, 1080);
        let base = resolve_slots(1920, 1080);
        assert_eq!(wide[0].w, base[0].w);
        assert_eq!(wide[0].x, base[0].x + (2560 - 1920) / 2);
    }

    // --- descriptor ---------------------------------------------------------

    #[test]
    fn fingerprint_is_unit_length() {
        let img = noisy_image(107, 58, 1);
        let fp = fp_of(&img);
        assert_eq!(fp.len(), DESCRIPTOR_DIM * DESCRIPTOR_DIM * 3);
        let norm: f32 = fp.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4, "norm {norm}");
    }

    #[test]
    fn identical_images_score_one_and_different_score_less() {
        let a = fp_of(&noisy_image(107, 58, 1));
        let b = fp_of(&noisy_image(107, 58, 2));
        assert!(trimmed_similarity(&a, &a) > 0.999);
        assert!(trimmed_similarity(&a, &b) < 0.9);
    }

    #[test]
    fn trim_recovers_an_occluded_corner() {
        // Same image, with a badge-sized corner painted over — the observed
        // own-hero-badge situation. Trimming must keep the score near 1.
        let img = noisy_image(107, 58, 3);
        let mut occluded = img.clone();
        for y in 40..58 {
            for x in 80..107 {
                occluded.put_pixel(x, y, image::Rgba([255, 215, 0, 255]));
            }
        }
        let s = trimmed_similarity(&fp_of(&img), &fp_of(&occluded));
        assert!(s > 0.9, "occluded similarity {s}");
    }

    #[test]
    fn flat_image_has_zero_contrast() {
        let img = RgbaImage::from_pixel(50, 50, image::Rgba([120, 120, 120, 255]));
        assert_eq!(luma_std_dev(img.as_raw(), 50, 50), 0.0);
    }

    // --- reference pack -----------------------------------------------------

    #[test]
    fn reference_pack_roundtrips() {
        let refs = vec![
            Reference {
                name: "lina".into(),
                fingerprint: fp_of(&noisy_image(107, 58, 4)),
                harvested: false,
            },
            Reference {
                name: "rubick".into(),
                fingerprint: fp_of(&noisy_image(107, 58, 5)),
                harvested: true,
            },
            Reference {
                name: "_empty".into(),
                fingerprint: fp_of(&noisy_image(107, 58, 6)),
                harvested: true,
            },
        ];
        let decoded = decode_reference_pack(&encode_reference_pack(&refs)).unwrap();
        assert_eq!(decoded.len(), 3);
        for (a, b) in refs.iter().zip(&decoded) {
            assert_eq!(a.name, b.name);
            assert_eq!(a.harvested, b.harvested);
            assert_eq!(a.fingerprint, b.fingerprint);
        }
        assert!(decoded[2].is_negative());
        assert!(!decoded[0].is_negative());
    }

    #[test]
    fn builtin_pack_decodes_with_full_hero_coverage() {
        let refs = builtin_references().expect("embedded pack must decode");
        let heroes = refs
            .iter()
            .filter(|r| !r.is_negative())
            .map(|r| r.name.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        // Valve had 127 heroes at bake time; a rebake after a hero release may
        // raise this, never lower it.
        assert!(heroes.len() >= 127, "only {} heroes in pack", heroes.len());
        assert!(heroes.contains("axe") && heroes.contains("zuus"));
        // The negatives the live sessions collected must survive rebakes —
        // they are what keeps an empty slot from reading as a hero.
        assert!(refs.iter().any(|r| r.is_negative()));
        for r in &refs {
            assert_eq!(r.fingerprint.len(), DESCRIPTOR_DIM * DESCRIPTOR_DIM * 3);
        }
    }

    #[test]
    fn reference_pack_rejects_garbage_and_truncation() {
        assert!(decode_reference_pack(b"nope").is_err());
        let good = encode_reference_pack(&[Reference {
            name: "axe".into(),
            fingerprint: fp_of(&noisy_image(107, 58, 7)),
            harvested: false,
        }]);
        assert!(decode_reference_pack(&good[..good.len() - 10]).is_err());
    }

    // --- matching -----------------------------------------------------------

    /// Build a full-client frame with the given per-slot images painted in.
    fn frame_with_slots(imgs: &[(usize, &RgbaImage)]) -> RgbaImage {
        let mut frame = RgbaImage::from_pixel(1920, 1080, image::Rgba([10, 10, 10, 255]));
        let slots = resolve_slots(1920, 1080);
        for (idx, img) in imgs {
            let s = &slots[*idx];
            for y in 0..s.h.min(img.height()) {
                for x in 0..s.w.min(img.width()) {
                    frame.put_pixel(s.x + x, s.y + y, *img.get_pixel(x, y));
                }
            }
        }
        frame
    }

    #[test]
    fn match_frame_finds_the_planted_hero_and_gates_empty_slots() {
        let art = noisy_image(107, 58, 8);
        let refs = vec![
            Reference {
                name: "planted".into(),
                fingerprint: fp_of(&art),
                harvested: false,
            },
            Reference {
                name: "other".into(),
                fingerprint: fp_of(&noisy_image(107, 58, 9)),
                harvested: false,
            },
        ];
        let frame = frame_with_slots(&[(3, &art)]);
        let outcomes = match_frame(&frame, 1920, 1080, 0, &refs);

        let (hero, score, margin) = outcomes[3].best().unwrap();
        assert_eq!(hero, "planted");
        assert!(score > 0.99);
        assert!(margin > 0.1);

        // Every unpainted slot is flat background: below the contrast gate.
        for (i, o) in outcomes.iter().enumerate() {
            if i != 3 {
                assert!(o.ranked.is_empty(), "slot {i} should be empty");
            }
        }
    }

    #[test]
    fn strip_capture_with_origin_matches_full_frame() {
        let art = noisy_image(107, 58, 10);
        let refs = vec![Reference {
            name: "planted".into(),
            fingerprint: fp_of(&art),
            harvested: false,
        }];
        let frame = frame_with_slots(&[(7, &art)]);
        let (sx, sy, sw, sh) = strip_region(1920, 1080);
        let strip = image::imageops::crop_imm(&frame, sx, sy, sw, sh).to_image();

        let full = match_frame(&frame, 1920, 1080, 0, &refs);
        let part = match_frame(&strip, 1920, 1080, sx, &refs);
        assert_eq!(full[7].best().unwrap().0, "planted");
        assert_eq!(part[7].best().unwrap().0, "planted");
    }

    #[test]
    fn winning_negative_reference_empties_the_slot() {
        let logo = noisy_image(107, 58, 11);
        let refs = vec![
            Reference {
                name: "_empty_logo".into(),
                fingerprint: fp_of(&logo),
                harvested: false,
            },
            Reference {
                name: "hero".into(),
                fingerprint: fp_of(&noisy_image(107, 58, 12)),
                harvested: false,
            },
        ];
        let frame = frame_with_slots(&[(0, &logo)]);
        let outcomes = match_frame(&frame, 1920, 1080, 0, &refs);
        assert!(outcomes[0].ranked.is_empty());
        // The slot still registers as occupied so voting counts the frame.
        assert!(outcomes[0].contrast >= OCCUPIED_MIN_STDDEV);
    }

    #[test]
    fn harvested_penalty_prices_out_the_domain_advantage() {
        // The measured steal: a same-domain exemplar of another hero outscoring
        // the true hero's CDN art by less than the penalty. After the penalty
        // the true hero must win.
        let capture = noisy_image(107, 58, 13);
        let fp_capture = fp_of(&capture);

        // A "CDN" ref of the true hero: same content, mildly perturbed.
        let mut cdn = capture.clone();
        for y in 0..58 {
            for x in 0..40 {
                let p = cdn.get_pixel_mut(x, y);
                p.0[0] = p.0[0].saturating_add(14);
                p.0[2] = p.0[2].saturating_sub(9);
            }
        }
        // A harvested ref of a DIFFERENT hero: perturbed slightly less, so its
        // raw score edges out the CDN ref — the observed 0.038 steal.
        let mut thief = capture.clone();
        for y in 0..58 {
            for x in 0..40 {
                let p = thief.get_pixel_mut(x, y);
                p.0[0] = p.0[0].saturating_add(10);
                p.0[2] = p.0[2].saturating_sub(7);
            }
        }

        let s_cdn = trimmed_similarity(&fp_capture, &fp_of(&cdn));
        let s_thief = trimmed_similarity(&fp_capture, &fp_of(&thief));
        assert!(
            s_thief > s_cdn && s_thief - s_cdn < HARVESTED_PENALTY,
            "test premise: raw steal within penalty ({s_thief} vs {s_cdn})"
        );

        let refs = vec![
            Reference {
                name: "true_hero".into(),
                fingerprint: fp_of(&cdn),
                harvested: false,
            },
            Reference {
                name: "thief".into(),
                fingerprint: fp_of(&thief),
                harvested: true,
            },
        ];
        let frame = frame_with_slots(&[(2, &capture)]);
        let outcomes = match_frame(&frame, 1920, 1080, 0, &refs);
        assert_eq!(outcomes[2].best().unwrap().0, "true_hero");
    }

    // --- voting -------------------------------------------------------------

    fn outcome_for(hero: &str, score: f32, margin: f32) -> SlotOutcome {
        outcome_at(hero, score, margin, 40.0)
    }

    fn outcome_at(hero: &str, score: f32, margin: f32, contrast: f32) -> SlotOutcome {
        SlotOutcome {
            slot: resolve_slots(1920, 1080)[0],
            contrast,
            ranked: vec![(hero.into(), score), ("runner".into(), score - margin)],
        }
    }

    #[test]
    fn dark_portraits_pass_the_occupancy_gate() {
        // Contrast measured off real captures. Plain Shadow Fiend's smoke
        // portrait sits at 23.3; the original gate of 25 dropped it in every
        // game it appeared in, across four sessions.
        let occupied: &[f32] = &[23.3, 23.4, 23.1, 32.0, 55.6, 71.1];
        // Empty slots idle at 11-13; the deepest reveal fades reach 18.
        let empty: &[f32] = &[11.2, 12.3, 13.0, 16.9, 18.0];

        for c in occupied {
            assert!(*c >= OCCUPIED_MIN_STDDEV, "{c} should read as occupied");
        }
        for c in empty {
            assert!(*c < OCCUPIED_MIN_STDDEV, "{c} should read as empty");
        }
    }

    #[test]
    fn a_lone_band_frame_loses_to_a_persistent_dark_portrait() {
        // The 20-25 band the lowered gate admits holds both populations.
        // Reveal fades appear once per slot; real dark portraits persist for
        // 13-28 frames. Voting is what separates them, not the gate.
        let mut fade = SlotVotes::default();
        fade.observe(&outcome_at("phantom", 0.58, 0.03, 22.8));
        assert!(!fade.confident(), "a single band frame must not settle");

        let mut dark = SlotVotes::default();
        for _ in 0..13 {
            dark.observe(&outcome_at("nevermore", 0.9, 0.23, 23.3));
        }
        assert!(dark.confident());
        assert_eq!(dark.winner().unwrap().0, "nevermore");
    }

    #[test]
    fn settled_slot_is_confident() {
        let mut v = SlotVotes::default();
        for _ in 0..7 {
            v.observe(&outcome_for("sven", 0.9, 0.1));
        }
        assert!(v.confident());
        assert_eq!(v.winner().unwrap(), ("sven", 7));
        assert!(v.agreement() > 0.99);
    }

    #[test]
    fn scattered_slot_abstains_regardless_of_length() {
        // The unmatchable-portrait signature: occupied every frame, but the
        // best guess keeps changing. Must abstain at short AND long drafts —
        // the absolute-count version of this rule failed at 12 frames.
        for frames in [7u32, 12, 30] {
            let mut v = SlotVotes::default();
            for i in 0..frames {
                v.observe(&outcome_for(&format!("guess{}", i % 5), 0.63, 0.02));
            }
            assert!(!v.confident(), "{frames} frames should not be confident");
        }
    }

    #[test]
    fn single_lucky_frame_is_not_confident() {
        let mut v = SlotVotes::default();
        v.observe(&outcome_for("wisp", 0.68, 0.02));
        assert!(!v.confident());
    }

    #[test]
    fn low_margin_frames_do_not_vote() {
        let mut v = SlotVotes::default();
        for _ in 0..5 {
            v.observe(&outcome_for("axe", 0.7, MARGIN_MIN / 2.0));
        }
        assert_eq!(v.winner(), None);
        assert!(!v.confident());
    }

    // --- gate and team ------------------------------------------------------

    #[test]
    fn gate_opens_only_for_draft_states() {
        assert!(draft_gate_open("DOTA_GAMERULES_STATE_HERO_SELECTION"));
        assert!(draft_gate_open("DOTA_GAMERULES_STATE_STRATEGY_TIME"));
        assert!(!draft_gate_open("DOTA_GAMERULES_STATE_GAME_IN_PROGRESS"));
        assert!(!draft_gate_open(""));
    }

    #[test]
    fn dire_flips_the_side_labels() {
        assert!(slot_is_ally(0, "radiant"));
        assert!(!slot_is_ally(5, "radiant"));
        assert!(!slot_is_ally(0, "dire"));
        assert!(slot_is_ally(5, "dire"));
        // Unknown team: assume radiant rather than crash; the reader records
        // the raw team string in telemetry either way.
        assert!(slot_is_ally(0, ""));
    }
}
