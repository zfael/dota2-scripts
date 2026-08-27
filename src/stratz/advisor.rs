//! Ranking a pick: counters, synergy, and position fit.
//!
//! Pure functions over a [`StratzDataset`] — no network, no clock, no I/O —
//! so the scoring model is fully testable and a suggestion costs microseconds.
//!
//! Three corrections separate this from naively sorting by win rate, and each
//! one exists because the naive version is actively misleading:
//!
//! 1. **Offsets, not absolutes.** A matchup contributes
//!    `win_rate(hero vs enemy) - win_rate(hero overall)`. Without this, whoever
//!    is strongest this patch appears to counter all ten enemies, and the
//!    recommendation degenerates into a tier list.
//! 2. **Sample shrinkage.** Each matchup is scaled by `matches / (matches + K)`.
//!    A 3-game 100% matchup would otherwise outrank a 4,000-game 54% one.
//! 3. **Position gating.** A hero is only a candidate for the role the user is
//!    actually queuing, judged by how often the hero is played there — not by
//!    a static role tag, which cannot tell a position 4 from a position 5.

use super::dataset::{StratzDataset, NUM_POSITIONS};

/// Shrinkage constant: a matchup with K games is trusted at half weight.
/// 50 is the value the community draft tools converge on, and it keeps
/// three-game samples from carrying meaningful weight.
pub const DEFAULT_SHRINK_K: f32 = 50.0;

/// How much a hero's own strength counts next to matchup effects. Kept low
/// deliberately: the point of the tool is the matchup, not the tier list.
pub const DEFAULT_BASE_WEIGHT: f32 = 0.4;

pub const DEFAULT_SYNERGY_WEIGHT: f32 = 1.0;

/// A hero must see at least this share of its games in a position before it
/// is offered for that role. Below this the position win rate is a handful of
/// off-role games and means nothing.
pub const MIN_POSITION_SHARE: f32 = 0.05;

/// "Meta" means picked at least this multiple of the average hero's rate.
///
/// Relative rather than absolute so the same rule survives a role filter,
/// where every rate is roughly a fifth of the overall one. Measured against
/// the live Divine+Immortal cache (`examples/stratz_meta_probe`), 1.5x keeps
/// 25 of 127 heroes overall and 23-31 per position — the size of a tier
/// list's top tiers. 1.0x keeps 50, which filters nothing a drafter cares
/// about; 2.0x keeps 12, which throws away real picks.
pub const META_PICK_RATE_MULTIPLE: f32 = 1.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdviceWeights {
    pub shrink_k: f32,
    pub base_weight: f32,
    pub synergy_weight: f32,
}

impl Default for AdviceWeights {
    fn default() -> Self {
        Self {
            shrink_k: DEFAULT_SHRINK_K,
            base_weight: DEFAULT_BASE_WEIGHT,
            synergy_weight: DEFAULT_SYNERGY_WEIGHT,
        }
    }
}

/// What the draft looks like right now.
#[derive(Debug, Clone, Default)]
pub struct DraftContext {
    /// Dense indices of heroes already on our side.
    pub allies: Vec<usize>,
    pub enemies: Vec<usize>,
    /// Position 1-5 as 0-4. `None` ranks without any position filter.
    pub position: Option<usize>,
    /// Restrict candidates to commonly picked heroes. Off by default: the
    /// sharpest counter is often an unpopular hero, and hiding it by default
    /// would defeat the point of the tool.
    pub meta_only: bool,
}

/// One hero's relationship with one already-picked hero.
///
/// Carried per suggestion so the UI can show *which* enemies a pick beats
/// rather than a single summed number the user has to take on trust.
#[derive(Debug, Clone)]
pub struct MatchupDetail {
    pub slug: String,
    pub display_name: String,
    /// Win-rate offset against (or alongside) this hero, over the pick's own
    /// baseline. +0.05 is "five points better than this hero's average".
    pub offset: f32,
    /// Games behind the offset.
    pub matches: u32,
    /// What actually entered the score: `offset` after sample shrinkage.
    pub contribution: f32,
}

#[derive(Debug, Clone)]
pub struct Suggestion {
    pub hero_index: usize,
    pub slug: String,
    pub display_name: String,
    /// Combined score; only meaningful relative to other suggestions.
    pub score: f32,
    /// Contribution from countering the enemy lineup.
    pub counter: f32,
    /// Contribution from working with our own picks.
    pub synergy: f32,
    /// Win rate in the requested position, where measured.
    pub position_win_rate: Option<f32>,
    /// Share of matches this hero is picked in — the popularity the meta
    /// filter reads, and worth showing either way. `None` where the refresh
    /// never fetched the hero.
    pub pick_rate: Option<f32>,
    /// The single enemy this pick most counters, for explaining the pick.
    pub best_against: Option<(String, f32)>,
    /// Every enemy, in draft order, so the UI can show the whole picture
    /// instead of only the best case.
    pub vs_enemies: Vec<MatchupDetail>,
    /// Same for our own picks.
    pub with_allies: Vec<MatchupDetail>,
    /// Total matchup sample behind the counter term, so the UI can show how
    /// much the number is worth trusting.
    pub counter_samples: u32,
}

/// Rank the available heroes for the current draft.
///
/// Heroes already picked by either side are excluded. Returns at most `limit`,
/// best first.
pub fn recommend(
    dataset: &StratzDataset,
    context: &DraftContext,
    weights: &AdviceWeights,
    limit: usize,
) -> Vec<Suggestion> {
    let n = dataset.len();
    if n == 0 {
        return Vec::new();
    }

    let taken: Vec<bool> = {
        let mut t = vec![false; n];
        for &i in context.allies.iter().chain(context.enemies.iter()) {
            if i < n {
                t[i] = true;
            }
        }
        t
    };

    // When a position is chosen, "how good is this hero" means "in my role",
    // not overall — a hero can be strong on average and poor as a 4. Fall
    // back to the overall rate where a position has no measurement.
    let strength = |hero: usize| -> f32 {
        context
            .position
            .and_then(|p| dataset.position_win_rate_of(hero, p))
            .unwrap_or_else(|| dataset.base_win_rate.get(hero).copied().unwrap_or(0.5))
    };

    // Popularity, for the meta filter and for display. Computed over the
    // whole roster rather than the survivors, so the bar for "meta" does not
    // move as heroes are picked.
    let pick_rates = dataset.pick_rates(context.position);
    let cut = meta_cut(&pick_rates);
    let is_meta = |hero: usize| -> bool {
        // Unknown popularity means an unfetched hero, not an unpopular one.
        pick_rates.get(hero).copied().flatten().is_none_or(|r| r >= cut)
    };

    let plays_the_role =
        |hero: usize| !taken[hero] && eligible_for_position(dataset, hero, context.position);

    let mut contenders: Vec<usize> = (0..n)
        .filter(|&h| plays_the_role(h) && (!context.meta_only || is_meta(h)))
        .collect();
    if contenders.is_empty() && context.meta_only {
        // A meta filter that leaves nothing has told the user nothing. Fall
        // back to the unfiltered list, the same judgement the position gate
        // makes for heroes with no positional data.
        contenders = (0..n).filter(|&h| plays_the_role(h)).collect();
    }

    // Centre over the heroes actually in contention, so the base term shifts
    // ranking rather than adding a constant to everyone.
    let mean_base = if contenders.is_empty() {
        0.5
    } else {
        contenders.iter().map(|&h| strength(h)).sum::<f32>() / contenders.len() as f32
    };

    let mut out: Vec<Suggestion> = Vec::new();

    for hero in contenders {
        let mut counter = 0.0f32;
        let mut counter_samples = 0u32;
        let mut best_against: Option<(usize, f32)> = None;
        let mut vs_enemies = Vec::with_capacity(context.enemies.len());

        for &enemy in &context.enemies {
            if enemy >= n {
                continue;
            }
            let (offset, matches) = dataset.advantage_of(hero, enemy);
            let contribution = offset * reliability(matches, weights.shrink_k);
            counter += contribution;
            counter_samples = counter_samples.saturating_add(matches);
            if best_against.is_none_or(|(_, best)| contribution > best) {
                best_against = Some((enemy, contribution));
            }
            vs_enemies.push(detail(dataset, enemy, offset, matches, contribution));
        }

        let mut synergy = 0.0f32;
        let mut with_allies = Vec::with_capacity(context.allies.len());
        for &ally in &context.allies {
            if ally >= n {
                continue;
            }
            let (offset, matches) = dataset.synergy_of(hero, ally);
            let contribution = offset * reliability(matches, weights.shrink_k);
            synergy += contribution;
            with_allies.push(detail(dataset, ally, offset, matches, contribution));
        }

        let base = strength(hero) - mean_base;
        let score = weights.base_weight * base + counter + weights.synergy_weight * synergy;

        let entry = dataset.hero(hero);
        out.push(Suggestion {
            hero_index: hero,
            slug: entry.map(|h| h.slug.clone()).unwrap_or_default(),
            display_name: entry.map(|h| h.display_name.clone()).unwrap_or_default(),
            score,
            counter,
            synergy: weights.synergy_weight * synergy,
            position_win_rate: context
                .position
                .and_then(|p| dataset.position_win_rate_of(hero, p)),
            pick_rate: pick_rates.get(hero).copied().flatten(),
            best_against: best_against
                .filter(|(_, c)| *c > 0.0)
                .and_then(|(i, c)| dataset.hero(i).map(|h| (h.display_name.clone(), c))),
            vs_enemies,
            with_allies,
            counter_samples,
        });
    }

    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            // Ties broken by name, so equal-scoring suggestions do not
            // reshuffle between polls and flicker in the UI.
            .then_with(|| a.slug.cmp(&b.slug))
    });
    out.truncate(limit);
    out
}

/// Weight a matchup by how much data stands behind it: `m / (m + K)`.
///
/// Zero games contributes nothing; K games contributes half.
fn reliability(matches: u32, shrink_k: f32) -> f32 {
    if matches == 0 {
        return 0.0;
    }
    let m = matches as f32;
    m / (m + shrink_k.max(0.0))
}

/// The pick rate a hero must reach to count as meta.
///
/// Measured against the mean of the heroes we have data for, so the rule
/// holds whether the rates are overall or restricted to one role.
fn meta_cut(pick_rates: &[Option<f32>]) -> f32 {
    let known: Vec<f32> = pick_rates.iter().flatten().copied().collect();
    if known.is_empty() {
        // No popularity data at all: the filter must not empty the list.
        return 0.0;
    }
    known.iter().sum::<f32>() / known.len() as f32 * META_PICK_RATE_MULTIPLE
}

fn detail(
    dataset: &StratzDataset,
    hero: usize,
    offset: f32,
    matches: u32,
    contribution: f32,
) -> MatchupDetail {
    let entry = dataset.hero(hero);
    MatchupDetail {
        slug: entry.map(|h| h.slug.clone()).unwrap_or_default(),
        display_name: entry.map(|h| h.display_name.clone()).unwrap_or_default(),
        offset,
        matches,
        contribution,
    }
}

/// Whether a hero is genuinely played in the requested position.
fn eligible_for_position(
    dataset: &StratzDataset,
    hero: usize,
    position: Option<usize>,
) -> bool {
    let Some(p) = position else {
        return true;
    };
    if p >= NUM_POSITIONS {
        return true;
    }
    // If we have no positional data at all for this hero, do not silently
    // drop it — an empty list is worse advice than an unfiltered one.
    let has_any = (0..NUM_POSITIONS).any(|i| dataset.position_share_of(hero, i) > 0.0);
    if !has_any {
        return true;
    }
    dataset.position_share_of(hero, p) >= MIN_POSITION_SHARE
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stratz::dataset::sample_dataset;

    /// Record that `hero` plays `position` (0-4) with the given share of its
    /// games and win rate there.
    fn set_position(
        d: &mut StratzDataset,
        hero: usize,
        position: usize,
        share: f32,
        win_rate: f32,
    ) {
        d.position_share[hero * NUM_POSITIONS + position] = share;
        d.position_win_rate[hero * NUM_POSITIONS + position] = win_rate;
    }

    /// Builds a dataset where `hero` beats `against` at `win_rate` over
    /// `matches` games, with every hero's baseline at 50%.
    fn with_matchup(
        slugs: &[&str],
        entries: &[(usize, usize, f32, u32)],
    ) -> StratzDataset {
        let mut d = sample_dataset(slugs);
        let n = slugs.len();
        for &(hero, against, win_rate, matches) in entries {
            d.advantage[hero * n + against] = win_rate - 0.5;
            d.vs_matches[hero * n + against] = matches;
        }
        d
    }

    #[test]
    fn ranks_the_hero_that_counters_the_enemy_lineup() {
        // "counter" beats the enemy heavily; "filler" is neutral.
        let d = with_matchup(
            &["counter", "filler", "enemy"],
            &[(0, 2, 0.60, 5_000)],
        );
        let ctx = DraftContext { enemies: vec![2], ..Default::default() };

        let picks = recommend(&d, &ctx, &AdviceWeights::default(), 10);

        assert_eq!(picks[0].slug, "counter");
        assert!(picks[0].counter > 0.09, "{}", picks[0].counter);
        assert_eq!(picks[0].best_against.as_ref().unwrap().0, "ENEMY");
    }

    #[test]
    fn a_tiny_sample_cannot_outrank_a_large_one() {
        // This is the whole point of shrinkage. Raw win rates would put the
        // 3-game 100% matchup first by a mile.
        let d = with_matchup(
            &["lucky", "proven", "enemy"],
            &[
                (0, 2, 1.00, 3),      // +50% over 3 games
                (1, 2, 0.56, 8_000),  // +6% over 8000 games
            ],
        );
        let ctx = DraftContext { enemies: vec![2], ..Default::default() };

        let picks = recommend(&d, &ctx, &AdviceWeights::default(), 10);

        assert_eq!(
            picks[0].slug, "proven",
            "3-game sample beat an 8000-game one: {:?}",
            picks.iter().map(|p| (&p.slug, p.score)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_generally_strong_hero_does_not_count_as_countering_everything() {
        // Correction 1: without baseline offsets, the strongest hero in the
        // patch appears to counter every enemy on the board.
        let mut d = sample_dataset(&["strong", "specialist", "enemy"]);
        let n = 3;
        d.base_win_rate[0] = 0.56; // strong overall
        d.base_win_rate[1] = 0.50;
        // Both win 56% against the enemy -- but for "strong" that is merely
        // its own average, while for "specialist" it is a real edge.
        d.advantage[2] = 0.56 - 0.56; // "strong" vs enemy: no real edge
        d.vs_matches[2] = 5_000;
        d.advantage[n + 2] = 0.56 - 0.50; // "specialist" vs enemy: +6%
        d.vs_matches[n + 2] = 5_000;

        let ctx = DraftContext { enemies: vec![2], ..Default::default() };
        let picks = recommend(&d, &ctx, &AdviceWeights::default(), 10);

        assert_eq!(picks[0].slug, "specialist");
        assert!(picks.iter().find(|p| p.slug == "strong").unwrap().counter.abs() < 1e-6);
    }

    #[test]
    fn synergy_with_allies_lifts_a_pick() {
        let mut d = sample_dataset(&["combo", "solo", "ally"]);
        // hero 0 ("combo") with hero 2 ("ally").
        d.synergy[2] = 0.08;
        d.with_matches[2] = 4_000;

        let ctx = DraftContext { allies: vec![2], ..Default::default() };
        let picks = recommend(&d, &ctx, &AdviceWeights::default(), 10);

        assert_eq!(picks[0].slug, "combo");
        assert!(picks[0].synergy > 0.0);
    }

    #[test]
    fn already_picked_heroes_are_never_suggested() {
        let d = sample_dataset(&["a", "b", "c", "d"]);
        let ctx = DraftContext {
            allies: vec![0],
            enemies: vec![1, 2],
            position: None,
            meta_only: false,
        };
        let picks = recommend(&d, &ctx, &AdviceWeights::default(), 10);

        assert_eq!(picks.len(), 1);
        assert_eq!(picks[0].slug, "d");
    }

    #[test]
    fn position_filter_excludes_heroes_who_do_not_play_the_role() {
        let mut d = sample_dataset(&["carry", "hard_support"]);
        // carry: 95% of games as position 1. support: 90% as position 5.
        set_position(&mut d, 0, 0, 0.95, 0.53);
        set_position(&mut d, 1, 4, 0.90, 0.52);

        let pos1 = recommend(
            &d,
            &DraftContext { position: Some(0), ..Default::default() },
            &AdviceWeights::default(),
            10,
        );
        assert_eq!(pos1.len(), 1);
        assert_eq!(pos1[0].slug, "carry");
        assert_eq!(pos1[0].position_win_rate, Some(0.53));

        let pos5 = recommend(
            &d,
            &DraftContext { position: Some(4), ..Default::default() },
            &AdviceWeights::default(),
            10,
        );
        assert_eq!(pos5.len(), 1);
        assert_eq!(pos5[0].slug, "hard_support");
    }

    #[test]
    fn strength_is_judged_in_the_chosen_role_not_overall() {
        // The reason the role is asked for at all: a hero can be strong on
        // average and mediocre in the position being queued.
        let mut d = sample_dataset(&["great_overall_poor_4", "made_for_4"]);
        d.base_win_rate[0] = 0.56;
        d.base_win_rate[1] = 0.48;
        // Both are played as position 4, but the second one wins there.
        set_position(&mut d, 0, 3, 0.5, 0.47);
        set_position(&mut d, 1, 3, 0.5, 0.55);

        let as_four = recommend(
            &d,
            &DraftContext { position: Some(3), ..Default::default() },
            &AdviceWeights::default(),
            10,
        );
        assert_eq!(as_four[0].slug, "made_for_4");

        // With no role selected, overall strength decides instead.
        let anywhere = recommend(&d, &DraftContext::default(), &AdviceWeights::default(), 10);
        assert_eq!(anywhere[0].slug, "great_overall_poor_4");
    }

    #[test]
    fn a_hero_with_no_positional_data_is_still_offered() {
        // Better an unfiltered suggestion than an empty list: a brand new
        // hero has no position stats for days after release.
        let d = sample_dataset(&["brand_new"]);
        let picks = recommend(
            &d,
            &DraftContext { position: Some(2), ..Default::default() },
            &AdviceWeights::default(),
            10,
        );
        assert_eq!(picks.len(), 1);
        assert_eq!(picks[0].position_win_rate, None);
    }

    #[test]
    fn an_off_role_sliver_does_not_qualify() {
        let mut d = sample_dataset(&["mostly_mid"]);
        set_position(&mut d, 0, 1, 0.97, 0.52);
        // Position 5: a rounding error's worth of off-role games.
        set_position(&mut d, 0, 4, 0.03, 0.60);
        let picks = recommend(
            &d,
            &DraftContext { position: Some(4), ..Default::default() },
            &AdviceWeights::default(),
            10,
        );
        assert!(picks.is_empty());
    }

    /// Gives `hero` a matchup row heavy enough to imply `games` popularity.
    fn set_popularity(d: &mut StratzDataset, hero: usize, games: u32) {
        let n = d.len();
        for j in 0..n {
            if j != hero {
                d.vs_matches[hero * n + j] = games;
            }
        }
    }

    #[test]
    fn the_meta_filter_keeps_commonly_picked_heroes_only() {
        // Three heroes far above average and two far below: the toggle has to
        // drop the two, and leave the ranking among the rest untouched.
        let mut d = sample_dataset(&["staple_a", "staple_b", "staple_c", "niche_a", "niche_b"]);
        for h in 0..3 {
            set_popularity(&mut d, h, 10_000);
        }
        set_popularity(&mut d, 3, 100);
        set_popularity(&mut d, 4, 100);

        let all = recommend(&d, &DraftContext::default(), &AdviceWeights::default(), 10);
        assert_eq!(all.len(), 5);

        let meta = recommend(
            &d,
            &DraftContext { meta_only: true, ..Default::default() },
            &AdviceWeights::default(),
            10,
        );
        let names: Vec<&str> = meta.iter().map(|s| s.slug.as_str()).collect();
        assert_eq!(names, vec!["staple_a", "staple_b", "staple_c"]);
    }

    #[test]
    fn a_hero_with_no_popularity_data_survives_the_meta_filter() {
        // An unfetched hero is missing information, not an unpopular pick.
        // Dropping it would quietly hide heroes because STRATZ 503'd.
        let mut d = sample_dataset(&["staple", "unfetched", "niche"]);
        set_popularity(&mut d, 0, 10_000);
        set_popularity(&mut d, 2, 50);

        let meta = recommend(
            &d,
            &DraftContext { meta_only: true, ..Default::default() },
            &AdviceWeights::default(),
            10,
        );
        let names: Vec<&str> = meta.iter().map(|s| s.slug.as_str()).collect();
        assert!(names.contains(&"unfetched"), "{names:?}");
        assert!(!names.contains(&"niche"), "{names:?}");
    }

    #[test]
    fn the_meta_filter_is_role_aware() {
        // Equally popular overall, but only one of them is actually played as
        // a position 5 — so "meta support" must mean the support.
        let mut d = sample_dataset(&["carry", "support", "filler_a", "filler_b"]);
        set_popularity(&mut d, 0, 10_000);
        set_popularity(&mut d, 1, 10_000);
        set_popularity(&mut d, 2, 2_000);
        set_popularity(&mut d, 3, 2_000);
        set_position(&mut d, 0, 0, 0.95, 0.52);
        set_position(&mut d, 0, 4, 0.05, 0.50);
        set_position(&mut d, 1, 4, 0.95, 0.52);
        set_position(&mut d, 2, 4, 0.30, 0.50);
        set_position(&mut d, 3, 4, 0.30, 0.50);

        let meta = recommend(
            &d,
            &DraftContext { position: Some(4), meta_only: true, ..Default::default() },
            &AdviceWeights::default(),
            10,
        );
        let names: Vec<&str> = meta.iter().map(|s| s.slug.as_str()).collect();
        assert_eq!(names, vec!["support"]);
    }

    #[test]
    fn a_meta_filter_that_would_leave_nothing_falls_back_to_the_full_list() {
        // Every hero equally popular: nobody clears 1.5x the mean, and an
        // empty advice panel mid-draft is worse than an unfiltered one.
        let mut d = sample_dataset(&["a", "b", "c"]);
        for h in 0..3 {
            set_popularity(&mut d, h, 5_000);
        }
        let meta = recommend(
            &d,
            &DraftContext { meta_only: true, ..Default::default() },
            &AdviceWeights::default(),
            10,
        );
        assert_eq!(meta.len(), 3);
    }

    #[test]
    fn an_empty_meta_rule_never_empties_the_list() {
        // With no matchup data at all every rate is unknown; the toggle must
        // degrade to "no filter" rather than to "no suggestions".
        let d = sample_dataset(&["a", "b", "c"]);
        let meta = recommend(
            &d,
            &DraftContext { meta_only: true, ..Default::default() },
            &AdviceWeights::default(),
            10,
        );
        assert_eq!(meta.len(), 3);
    }

    #[test]
    fn every_enemy_is_reported_not_just_the_best_one() {
        // The UI showed one "best vs" line per row and it named the same
        // enemy for nine of twelve picks. The whole row is carried so the
        // panel can show where a pick is weak as well as where it is strong.
        let d = with_matchup(
            &["pick", "enemy_soft", "enemy_hard"],
            &[(0, 1, 0.60, 5_000), (0, 2, 0.42, 5_000)],
        );
        let ctx = DraftContext { enemies: vec![1, 2], ..Default::default() };
        let picks = recommend(&d, &ctx, &AdviceWeights::default(), 10);
        let top = picks.iter().find(|p| p.slug == "pick").unwrap();

        assert_eq!(top.vs_enemies.len(), 2);
        assert_eq!(top.vs_enemies[0].display_name, "ENEMY_SOFT");
        assert!((top.vs_enemies[0].offset - 0.10).abs() < 1e-6);
        assert_eq!(top.vs_enemies[0].matches, 5_000);
        // The bad matchup is reported too, rather than hidden behind the sum.
        assert!(top.vs_enemies[1].offset < 0.0);
        // Contributions are the shrunk values that actually entered the score.
        let summed: f32 = top.vs_enemies.iter().map(|d| d.contribution).sum();
        assert!((summed - top.counter).abs() < 1e-6);
    }

    #[test]
    fn ally_synergy_is_reported_per_ally() {
        let mut d = sample_dataset(&["pick", "ally"]);
        d.synergy[1] = 0.08;
        d.with_matches[1] = 4_000;
        let ctx = DraftContext { allies: vec![1], ..Default::default() };
        let picks = recommend(&d, &ctx, &AdviceWeights::default(), 10);
        let top = picks.iter().find(|p| p.slug == "pick").unwrap();

        assert_eq!(top.with_allies.len(), 1);
        assert_eq!(top.with_allies[0].slug, "ally");
        assert!((top.with_allies[0].offset - 0.08).abs() < 1e-6);
    }

    #[test]
    fn pick_rate_is_reported_even_with_the_filter_off() {
        // Popularity is worth showing whether or not it is being filtered on:
        // it is the fastest way to tell a comfort pick from a pocket one.
        let mut d = sample_dataset(&["popular", "niche"]);
        set_popularity(&mut d, 0, 9_000);
        set_popularity(&mut d, 1, 1_000);
        let picks = recommend(&d, &DraftContext::default(), &AdviceWeights::default(), 10);
        let popular = picks.iter().find(|p| p.slug == "popular").unwrap();
        let niche = picks.iter().find(|p| p.slug == "niche").unwrap();
        assert!(popular.pick_rate.unwrap() > niche.pick_rate.unwrap());
    }

    #[test]
    fn ordering_is_stable_for_equal_scores() {
        // Equal-scoring picks must not reshuffle between polls, or the list
        // visibly flickers while the user is reading it.
        let d = sample_dataset(&["zeta", "alpha", "mu"]);
        let ctx = DraftContext::default();
        let first = recommend(&d, &ctx, &AdviceWeights::default(), 10);
        let second = recommend(&d, &ctx, &AdviceWeights::default(), 10);
        let names: Vec<&str> = first.iter().map(|p| p.slug.as_str()).collect();
        assert_eq!(names, vec!["alpha", "mu", "zeta"]);
        assert_eq!(names, second.iter().map(|p| p.slug.as_str()).collect::<Vec<_>>());
    }

    #[test]
    fn an_empty_dataset_yields_no_suggestions_rather_than_panicking() {
        let d = StratzDataset::default();
        assert!(recommend(&d, &DraftContext::default(), &AdviceWeights::default(), 10).is_empty());
    }

    #[test]
    fn out_of_range_draft_indices_are_ignored() {
        // The draft reader and the dataset could disagree after a patch adds
        // a hero; a stale index must not panic mid-draft.
        let d = sample_dataset(&["a", "b"]);
        let ctx = DraftContext {
            allies: vec![99],
            enemies: vec![100],
            position: None,
            meta_only: false,
        };
        let picks = recommend(&d, &ctx, &AdviceWeights::default(), 10);
        assert_eq!(picks.len(), 2);
    }

    #[test]
    fn limit_is_respected() {
        let d = sample_dataset(&["a", "b", "c", "d", "e"]);
        let picks = recommend(&d, &DraftContext::default(), &AdviceWeights::default(), 3);
        assert_eq!(picks.len(), 3);
    }

    #[test]
    fn reliability_curve_behaves() {
        assert_eq!(reliability(0, 50.0), 0.0);
        assert!((reliability(50, 50.0) - 0.5).abs() < 1e-6);
        assert!(reliability(10_000, 50.0) > 0.99);
    }
}
