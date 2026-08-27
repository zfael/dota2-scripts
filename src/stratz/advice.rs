//! Turning the live draft snapshot into a ranked list of picks.
//!
//! The bridge between the two halves of the feature: the draft reader knows
//! which heroes are on screen and whose side they are on, and the advisor
//! knows what to do about it. Kept here, out of the Tauri layer, so the
//! mapping is unit-testable without a webview.

use super::advisor::{recommend, AdviceWeights, DraftContext, Suggestion};
use super::dataset::StratzDataset;
use crate::config::StratzConfig;
use crate::observability::draft_reader::DraftSlotSnapshot;

/// Advice for the current draft, plus what could not be resolved.
#[derive(Debug, Clone, Default)]
pub struct DraftAdvice {
    pub suggestions: Vec<Suggestion>,
    /// Ally hero slugs the dataset did not recognise, so the UI can say the
    /// advice is based on an incomplete picture rather than pretend.
    pub unresolved: Vec<String>,
    /// Identified heroes that were fed into the ranking.
    pub allies_used: usize,
    pub enemies_used: usize,
}

impl From<&StratzConfig> for AdviceWeights {
    fn from(c: &StratzConfig) -> Self {
        Self {
            shrink_k: c.shrink_k,
            base_weight: c.base_weight,
            synergy_weight: c.synergy_weight,
        }
    }
}

/// Build advice from the draft reader's slots.
///
/// `position` is 1-5 as the user selects it; 0 (or out of range) means no
/// position filter. Slots the matcher has not settled are simply absent —
/// a `?` slot contributes nothing rather than a guess.
pub fn advise(
    dataset: &StratzDataset,
    slots: &[DraftSlotSnapshot],
    config: &StratzConfig,
) -> DraftAdvice {
    let mut allies = Vec::new();
    let mut enemies = Vec::new();
    let mut unresolved = Vec::new();

    for slot in slots {
        let Some(hero) = slot.hero.as_deref() else {
            continue;
        };
        match dataset.index_of_slug(hero) {
            Some(index) => {
                if slot.is_ally {
                    allies.push(index);
                } else {
                    enemies.push(index);
                }
            }
            // A hero the dataset has never heard of means the cache predates
            // a patch. Worth surfacing: the advice is missing a pick.
            None => unresolved.push(hero.to_string()),
        }
    }

    let context = DraftContext {
        position: position_index(config.position),
        allies,
        enemies,
        meta_only: config.meta_only,
    };

    DraftAdvice {
        allies_used: context.allies.len(),
        enemies_used: context.enemies.len(),
        suggestions: recommend(
            dataset,
            &context,
            &AdviceWeights::from(config),
            config.suggestion_count.clamp(1, 50),
        ),
        unresolved,
    }
}

/// User-facing position 1-5 to dense index 0-4; anything else means no filter.
fn position_index(position: u8) -> Option<usize> {
    (1..=5).contains(&position).then(|| position as usize - 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stratz::dataset::{sample_dataset, NUM_POSITIONS};

    fn slot(index: usize, is_ally: bool, hero: Option<&str>) -> DraftSlotSnapshot {
        DraftSlotSnapshot {
            index,
            is_ally,
            hero: hero.map(|h| h.to_string()),
            unknown: hero.is_none(),
            agreement: if hero.is_some() { 1.0 } else { 0.0 },
            best_score: 0.9,
            occupied_frames: 10,
        }
    }

    #[test]
    fn splits_the_lineup_by_side() {
        let d = sample_dataset(&["ally_hero", "enemy_hero", "free"]);
        let slots = vec![
            slot(0, true, Some("ally_hero")),
            slot(5, false, Some("enemy_hero")),
        ];
        let advice = advise(&d, &slots, &StratzConfig::default());

        assert_eq!(advice.allies_used, 1);
        assert_eq!(advice.enemies_used, 1);
        // Both picked heroes are excluded from the suggestions.
        assert_eq!(advice.suggestions.len(), 1);
        assert_eq!(advice.suggestions[0].slug, "free");
    }

    #[test]
    fn unidentified_slots_contribute_nothing() {
        let d = sample_dataset(&["a", "b"]);
        let slots = vec![slot(0, true, None), slot(1, true, None)];
        let advice = advise(&d, &slots, &StratzConfig::default());

        assert_eq!(advice.allies_used, 0);
        assert!(advice.unresolved.is_empty());
        assert_eq!(advice.suggestions.len(), 2);
    }

    #[test]
    fn a_hero_missing_from_the_dataset_is_reported_not_swallowed() {
        // The cache predating a patch is the realistic case; silently
        // dropping the pick would give confidently wrong advice.
        let d = sample_dataset(&["known"]);
        let slots = vec![slot(0, false, Some("brand_new_hero"))];
        let advice = advise(&d, &slots, &StratzConfig::default());

        assert_eq!(advice.unresolved, vec!["brand_new_hero"]);
        assert_eq!(advice.enemies_used, 0);
    }

    #[test]
    fn position_filter_comes_from_config() {
        let mut d = sample_dataset(&["carry", "support"]);
        d.position_share[0] = 1.0; // hero 0 (carry), position 1
        d.position_share[NUM_POSITIONS + 4] = 1.0; // hero 1 (support), position 5

        let mut config = StratzConfig { position: 1, ..Default::default() };
        assert_eq!(advise(&d, &[], &config).suggestions[0].slug, "carry");

        config.position = 5;
        assert_eq!(advise(&d, &[], &config).suggestions[0].slug, "support");

        // 0 means "no filter": both heroes remain candidates.
        config.position = 0;
        assert_eq!(advise(&d, &[], &config).suggestions.len(), 2);
    }

    #[test]
    fn position_maps_from_one_based_to_zero_based() {
        assert_eq!(position_index(1), Some(0));
        assert_eq!(position_index(5), Some(4));
        assert_eq!(position_index(0), None);
        // Out of range must mean "no filter", never an index that panics or
        // silently selects position 1.
        assert_eq!(position_index(6), None);
        assert_eq!(position_index(255), None);
    }

    #[test]
    fn suggestion_count_is_clamped_to_something_sane() {
        let d = sample_dataset(&["a", "b", "c", "d"]);
        let zero = StratzConfig { suggestion_count: 0, ..Default::default() };
        assert_eq!(advise(&d, &[], &zero).suggestions.len(), 1);

        let huge = StratzConfig { suggestion_count: 9_999, ..Default::default() };
        assert_eq!(advise(&d, &[], &huge).suggestions.len(), 4);
    }

    #[test]
    fn the_meta_toggle_is_carried_from_config() {
        // One staple and four niche heroes, by matchup volume.
        let mut d = sample_dataset(&["staple", "niche_a", "niche_b", "niche_c", "niche_d"]);
        let n = 5;
        for (hero, games) in [(0usize, 10_000u32), (1, 250), (2, 250), (3, 250), (4, 250)] {
            for j in 0..n {
                if j != hero {
                    d.vs_matches[hero * n + j] = games;
                }
            }
        }

        let off = StratzConfig::default();
        assert_eq!(advise(&d, &[], &off).suggestions.len(), 5);

        let on = StratzConfig { meta_only: true, ..Default::default() };
        let filtered = advise(&d, &[], &on);
        let slugs: Vec<&str> = filtered.suggestions.iter().map(|s| s.slug.as_str()).collect();
        assert_eq!(slugs, vec!["staple"]);
    }

    #[test]
    fn weights_are_carried_from_config() {
        let config = StratzConfig {
            shrink_k: 12.5,
            base_weight: 0.25,
            synergy_weight: 2.0,
            ..Default::default()
        };
        let w = AdviceWeights::from(&config);
        assert_eq!(w.shrink_k, 12.5);
        assert_eq!(w.base_weight, 0.25);
        assert_eq!(w.synergy_weight, 2.0);
    }
}
