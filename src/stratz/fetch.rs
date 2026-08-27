//! Building a [`StratzDataset`] from the API.
//!
//! Three queries, ~132 requests total:
//!
//! 1. `constants.heroes` — one call, maps our draft slugs to STRATZ ids.
//! 2. `heroStats.winWeek` — five calls, one per position, giving real 1-5
//!    win rate and pick share (not the lane-based approximation).
//! 3. `heroStats.matchUp` — one call per hero, giving `vs` (counters) and
//!    `with` (synergy) against every other hero.
//!
//! At the free tier's 250/minute this takes roughly a minute, which is why it
//! is a cached background refresh and not something a draft ever waits on.
//!
//! The query shapes are pinned by tests against recorded response fixtures.
//! They cannot verify STRATZ's live schema — only a real call can — but they
//! do catch a parser that silently maps everything to zero.

use super::client::{StratzClient, StratzError};
use super::dataset::{HeroEntry, StratzDataset, NUM_POSITIONS, POSITIONS};

const HEROES_QUERY: &str = r#"
query {
  constants {
    heroes {
      id
      name
      shortName
      displayName
    }
  }
}
"#;

/// `winWeek` returns one row per hero **per week** — 127 heroes x 20 weeks =
/// 2540 rows — so `take` must cover all of them or heroes fall off the end
/// entirely. Only the most recent [`POSITION_WEEKS`] are kept, so position
/// data reflects the current patch rather than five months of history.
const WIN_WEEK_QUERY: &str = r#"
query($positionIds:[MatchPlayerPositionType!], $bracketIds:[RankBracket!]) {
  heroStats {
    winWeek(positionIds:$positionIds, bracketIds:$bracketIds, take:5000) {
      week
      heroId
      matchCount
      winCount
    }
  }
}
"#;

/// Weeks of position history to keep. Position splits divide a hero's games
/// five ways, so a single week is thin for the rarer roles; four weeks keeps
/// the sample usable without reaching back through several patches.
const POSITION_WEEKS: usize = 4;

const MATCHUP_QUERY: &str = r#"
query($heroId:Short, $bracketIds:[RankBracketBasicEnum!]) {
  heroStats {
    matchUp(heroId:$heroId, bracketBasicIds:$bracketIds, take:200) {
      vs   { heroId2 winCount matchCount }
      with { heroId2 winCount matchCount }
    }
  }
}
"#;

/// Which rank bracket to pull. `winWeek` and `matchUp` take different enums
/// for the same idea, so a bracket carries both spellings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bracket {
    /// `RankBracketBasicEnum`, for `matchUp`.
    pub basic: String,
    /// `RankBracket` values, for `winWeek`.
    pub detailed: Vec<String>,
}

impl Bracket {
    pub fn divine_immortal() -> Self {
        Self {
            basic: "DIVINE_IMMORTAL".to_string(),
            detailed: vec!["DIVINE".to_string(), "IMMORTAL".to_string()],
        }
    }

    pub fn from_name(name: &str) -> Self {
        match name.trim().to_ascii_uppercase().as_str() {
            "HERALD_GUARDIAN" => Self {
                basic: "HERALD_GUARDIAN".into(),
                detailed: vec!["HERALD".into(), "GUARDIAN".into()],
            },
            "CRUSADER_ARCHON" => Self {
                basic: "CRUSADER_ARCHON".into(),
                detailed: vec!["CRUSADER".into(), "ARCHON".into()],
            },
            "LEGEND_ANCIENT" => Self {
                basic: "LEGEND_ANCIENT".into(),
                detailed: vec!["LEGEND".into(), "ANCIENT".into()],
            },
            _ => Self::divine_immortal(),
        }
    }
}

/// Progress callback: `(completed, total, stage)`.
pub type ProgressFn<'a> = &'a mut dyn FnMut(usize, usize, &str);

/// Fetch everything and assemble the dataset.
pub fn build_dataset(
    client: &mut StratzClient,
    bracket: &Bracket,
    now_unix: u64,
    progress: ProgressFn<'_>,
) -> Result<StratzDataset, StratzError> {
    progress(0, 0, "heroes");
    let heroes = fetch_heroes(client)?;
    let n = heroes.len();
    if n == 0 {
        return Err(StratzError::Decode("STRATZ returned no heroes".into()));
    }

    let id_to_index: std::collections::HashMap<i16, usize> =
        heroes.iter().enumerate().map(|(i, h)| (h.id, i)).collect();

    // --- positions -------------------------------------------------------
    let mut position_games = vec![0f32; n * NUM_POSITIONS];
    let mut position_wins = vec![0f32; n * NUM_POSITIONS];

    for (p, position) in POSITIONS.iter().enumerate() {
        progress(p, NUM_POSITIONS, "positions");
        let data = client.query(
            WIN_WEEK_QUERY,
            serde_json::json!({
                "positionIds": [position],
                "bracketIds": bracket.detailed,
            }),
        )?;
        let all: Vec<&serde_json::Value> = rows(&data["heroStats"]["winWeek"]).collect();
        let keep = recent_weeks(&all, POSITION_WEEKS);
        for row in all {
            let Some(&i) = row_hero_id(row, "heroId").and_then(|id| id_to_index.get(&id)) else {
                continue;
            };
            if !keep.contains(&row_i64(row, "week")) {
                continue;
            }
            position_games[i * NUM_POSITIONS + p] += count(row, "matchCount");
            position_wins[i * NUM_POSITIONS + p] += count(row, "winCount");
        }
    }

    // Baseline win rates come from the matchup data itself (below), not from
    // winWeek. The two cover different windows -- measured on Shadow Fiend,
    // matchUp implied 0.4874 while winWeek gave 0.4825 over one week and
    // 0.4790 over four. Subtracting one population's baseline from another's
    // matchups biases every offset by that gap, which is a sizeable fraction
    // of a real matchup signal (typically +/-0.05).
    let mut base_win_rate = vec![0.5f32; n];

    let mut position_share = vec![0f32; n * NUM_POSITIONS];
    let mut position_win_rate = vec![-1f32; n * NUM_POSITIONS];
    for i in 0..n {
        let total: f32 = (0..NUM_POSITIONS).map(|p| position_games[i * NUM_POSITIONS + p]).sum();
        for p in 0..NUM_POSITIONS {
            let g = position_games[i * NUM_POSITIONS + p];
            if total > 0.0 {
                position_share[i * NUM_POSITIONS + p] = g / total;
            }
            if g > 0.0 {
                position_win_rate[i * NUM_POSITIONS + p] =
                    position_wins[i * NUM_POSITIONS + p] / g;
            }
        }
    }

    // --- matchups --------------------------------------------------------
    let mut advantage = vec![0f32; n * n];
    let mut vs_matches = vec![0u32; n * n];
    let mut synergy = vec![0f32; n * n];
    let mut with_matches = vec![0u32; n * n];

    for (done, hero) in heroes.iter().enumerate() {
        progress(done, n, "matchups");
        let data = client.query(
            MATCHUP_QUERY,
            serde_json::json!({ "heroId": hero.id, "bracketIds": [bracket.basic] }),
        )?;

        // matchUp returns a list with a single entry for the queried hero.
        let entry = match data["heroStats"]["matchUp"].as_array().and_then(|a| a.first()) {
            Some(e) => e.clone(),
            None => continue,
        };
        let i = done;

        // The hero's baseline over exactly the matchup population: every game
        // in the `vs` rows is one this hero played. Self-consistent, so the
        // offsets below measure only the matchup effect.
        base_win_rate[i] = implied_win_rate(&entry["vs"]).unwrap_or(0.5);

        for row in rows(&entry["vs"]) {
            let Some(&j) = row_hero_id(row, "heroId2").and_then(|id| id_to_index.get(&id)) else {
                continue;
            };
            let matches = count(row, "matchCount");
            if matches <= 0.0 || i == j {
                continue;
            }
            advantage[i * n + j] = count(row, "winCount") / matches - base_win_rate[i];
            vs_matches[i * n + j] = matches as u32;
        }
        for row in rows(&entry["with"]) {
            let Some(&j) = row_hero_id(row, "heroId2").and_then(|id| id_to_index.get(&id)) else {
                continue;
            };
            let matches = count(row, "matchCount");
            if matches <= 0.0 || i == j {
                continue;
            }
            synergy[i * n + j] = count(row, "winCount") / matches - base_win_rate[i];
            with_matches[i * n + j] = matches as u32;
        }
    }

    progress(n, n, "done");
    Ok(StratzDataset {
        heroes,
        advantage,
        vs_matches,
        synergy,
        with_matches,
        base_win_rate,
        position_win_rate,
        position_share,
        built_at: now_unix,
        bracket: bracket.basic.clone(),
    })
}

fn fetch_heroes(client: &mut StratzClient) -> Result<Vec<HeroEntry>, StratzError> {
    let data = client.query(HEROES_QUERY, serde_json::Value::Null)?;
    Ok(parse_heroes(&data))
}

/// Split out so the shape is testable without a network call.
fn parse_heroes(data: &serde_json::Value) -> Vec<HeroEntry> {
    let mut out = Vec::new();
    for hero in rows(&data["constants"]["heroes"]) {
        let Some(id) = row_hero_id(hero, "id") else { continue };
        // Prefer shortName; fall back to `name` with the npc prefix stripped,
        // which is the same string for every hero that has both.
        let slug = hero
            .get("shortName")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                hero.get("name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim_start_matches("npc_dota_hero_").to_string())
            })
            .unwrap_or_default();
        if slug.is_empty() {
            continue;
        }
        let display_name = hero
            .get("displayName")
            .and_then(|v| v.as_str())
            .unwrap_or(&slug)
            .to_string();
        out.push(HeroEntry { id, slug, display_name });
    }
    out
}

fn rows(value: &serde_json::Value) -> impl Iterator<Item = &serde_json::Value> {
    value.as_array().map(|a| a.as_slice()).unwrap_or(&[]).iter()
}

/// A hero's overall win rate implied by its own matchup rows.
///
/// Each `vs` row counts games this hero played against one opponent, so the
/// totals are that hero's record over the same window the matchups describe.
fn implied_win_rate(vs: &serde_json::Value) -> Option<f32> {
    let mut wins = 0f64;
    let mut games = 0f64;
    for row in rows(vs) {
        games += count(row, "matchCount") as f64;
        wins += count(row, "winCount") as f64;
    }
    (games > 0.0).then(|| (wins / games) as f32)
}

/// The most recent `n` distinct week stamps present in the rows.
fn recent_weeks(rows: &[&serde_json::Value], n: usize) -> std::collections::HashSet<i64> {
    let mut weeks: Vec<i64> = rows
        .iter()
        .map(|r| row_i64(r, "week"))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    weeks.sort_unstable_by(|a, b| b.cmp(a));
    weeks.into_iter().take(n).collect()
}

/// Hero ids arrive as JSON numbers; be tolerant of a string encoding too,
/// since GraphQL `Short` has been seen serialised both ways.
fn row_hero_id(row: &serde_json::Value, key: &str) -> Option<i16> {
    let v = row.get(key)?;
    v.as_i64()
        .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
        .and_then(|n| i16::try_from(n).ok())
}

fn count(row: &serde_json::Value, key: &str) -> f32 {
    row.get(key)
        .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .unwrap_or(0.0) as f32
}

/// Read an integer field at full width.
///
/// Week stamps are unix seconds (~1.79e9), well past the 2^24 where f32 stops
/// representing consecutive integers, so they must never go through [`count`].
fn row_i64(row: &serde_json::Value, key: &str) -> i64 {
    row.get(key)
        .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_hero_constants_shape() {
        let data = serde_json::json!({
            "constants": { "heroes": [
                { "id": 1, "name": "npc_dota_hero_antimage", "shortName": "antimage",
                  "displayName": "Anti-Mage" },
                { "id": 11, "name": "npc_dota_hero_nevermore", "shortName": "nevermore",
                  "displayName": "Shadow Fiend" },
            ]}
        });
        let heroes = parse_heroes(&data);
        assert_eq!(heroes.len(), 2);
        assert_eq!(heroes[1].id, 11);
        assert_eq!(heroes[1].slug, "nevermore");
        assert_eq!(heroes[1].display_name, "Shadow Fiend");
    }

    #[test]
    fn falls_back_to_the_npc_name_when_short_name_is_absent() {
        let data = serde_json::json!({
            "constants": { "heroes": [
                { "id": 42, "name": "npc_dota_hero_skeleton_king", "displayName": "Wraith King" }
            ]}
        });
        let heroes = parse_heroes(&data);
        assert_eq!(heroes[0].slug, "skeleton_king");
    }

    #[test]
    fn heroes_without_a_usable_name_are_skipped_not_defaulted() {
        // A blank slug would collide with every other blank slug and quietly
        // corrupt the id mapping.
        let data = serde_json::json!({
            "constants": { "heroes": [
                { "id": 1, "displayName": "Mystery" },
                { "id": 2, "shortName": "axe", "displayName": "Axe" }
            ]}
        });
        let heroes = parse_heroes(&data);
        assert_eq!(heroes.len(), 1);
        assert_eq!(heroes[0].slug, "axe");
    }

    #[test]
    fn a_missing_heroes_array_yields_nothing_rather_than_panicking() {
        assert!(parse_heroes(&serde_json::json!({})).is_empty());
        assert!(parse_heroes(&serde_json::json!({ "constants": null })).is_empty());
    }

    #[test]
    fn hero_ids_parse_from_numbers_and_strings() {
        let row = serde_json::json!({ "a": 11, "b": "11", "c": "nope", "d": 99999 });
        assert_eq!(row_hero_id(&row, "a"), Some(11));
        assert_eq!(row_hero_id(&row, "b"), Some(11));
        assert_eq!(row_hero_id(&row, "c"), None);
        // Out of i16 range must not wrap around into a valid hero.
        assert_eq!(row_hero_id(&row, "d"), None);
        assert_eq!(row_hero_id(&row, "missing"), None);
    }

    #[test]
    fn counts_default_to_zero_rather_than_failing() {
        let row = serde_json::json!({ "matchCount": 1234, "winCount": null });
        assert_eq!(count(&row, "matchCount"), 1234.0);
        assert_eq!(count(&row, "winCount"), 0.0);
        assert_eq!(count(&row, "absent"), 0.0);
    }

    #[test]
    fn week_stamps_survive_full_precision() {
        // Unix seconds are past 2^24, where f32 stops representing every
        // integer. Routed through the f32 `count` helper, adjacent weeks can
        // collapse onto the same value and the recency filter silently keeps
        // the wrong set.
        let row = serde_json::json!({ "week": 1787184000i64, "s": "1786579200" });
        assert_eq!(row_i64(&row, "week"), 1787184000);
        assert_eq!(row_i64(&row, "s"), 1786579200);
        assert_eq!(row_i64(&row, "missing"), 0);
    }

    #[test]
    fn recency_filter_keeps_only_the_newest_weeks() {
        // winWeek returns 20 weeks per hero, newest first; keeping all of
        // them would average five months across several patches.
        let raw = vec![
            serde_json::json!({ "week": 1787184000i64 }),
            serde_json::json!({ "week": 1786579200i64 }),
            serde_json::json!({ "week": 1785974400i64 }),
            serde_json::json!({ "week": 1785369600i64 }),
            // Same weeks again, as they arrive for other heroes.
            serde_json::json!({ "week": 1787184000i64 }),
        ];
        let refs: Vec<&serde_json::Value> = raw.iter().collect();

        let keep = recent_weeks(&refs, 2);
        assert_eq!(keep.len(), 2);
        assert!(keep.contains(&1787184000));
        assert!(keep.contains(&1786579200));
        assert!(!keep.contains(&1785974400));

        // Asking for more weeks than exist keeps everything present.
        assert_eq!(recent_weeks(&refs, 99).len(), 4);
    }

    #[test]
    fn baseline_comes_from_the_matchup_rows_themselves() {
        // Measured on Shadow Fiend: matchUp implies 0.4874 while winWeek
        // gives 0.4790-0.4825 depending on window. Mixing the two biases
        // every offset, so the baseline is derived from `vs` directly.
        let vs = serde_json::json!([
            { "heroId2": 1, "winCount": 60, "matchCount": 100 },
            { "heroId2": 2, "winCount": 40, "matchCount": 100 },
        ]);
        assert_eq!(implied_win_rate(&vs), Some(0.5));

        // A hero with no matchup rows has no implied rate to offset against.
        assert_eq!(implied_win_rate(&serde_json::json!([])), None);
        assert_eq!(implied_win_rate(&serde_json::Value::Null), None);
        // Rows with no games must not produce a division by zero.
        let empty = serde_json::json!([{ "heroId2": 1, "winCount": 0, "matchCount": 0 }]);
        assert_eq!(implied_win_rate(&empty), None);
    }

    #[test]
    fn implied_baseline_is_weighted_by_sample_not_averaged() {
        // A rarely-faced opponent must not swing the baseline as hard as a
        // common one.
        let vs = serde_json::json!([
            { "winCount": 5_000, "matchCount": 10_000 }, // 50% over 10k
            { "winCount": 10, "matchCount": 10 },        // 100% over 10
        ]);
        let wr = implied_win_rate(&vs).unwrap();
        assert!((wr - 0.5005).abs() < 1e-4, "{wr}");
    }

    #[test]
    fn bracket_names_map_to_both_enum_spellings() {
        let b = Bracket::divine_immortal();
        assert_eq!(b.basic, "DIVINE_IMMORTAL");
        assert_eq!(b.detailed, vec!["DIVINE", "IMMORTAL"]);
        // Unknown names fall back rather than sending an invalid enum that
        // would fail the whole refresh.
        assert_eq!(Bracket::from_name("nonsense"), Bracket::divine_immortal());
        assert_eq!(Bracket::from_name("herald_guardian").basic, "HERALD_GUARDIAN");
    }
}
