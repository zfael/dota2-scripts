use crate::actions::common::find_item_slot;
use crate::config::Settings;
use crate::models::{GsiWebhookEvent, Hero, Item};
use lazy_static::lazy_static;
use std::sync::Mutex;

const DIG_ABILITY_NAME: &str = "meepo_petrify";
const MEGAMEEPO_ABILITY_NAME: &str = "meepo_megameepo";

lazy_static! {
    static ref MEEPO_OBSERVED_STATE: Mutex<Option<MeepoObservedState>> = Mutex::new(None);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KnownCloneState {
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeepoObservedState {
    pub hero_name: String,
    pub hero_level: u32,
    pub health_percent: u32,
    pub mana_percent: u32,
    pub in_danger: bool,
    pub alive: bool,
    pub stunned: bool,
    pub silenced: bool,
    pub poof_ready: bool,
    pub dig_ready: bool,
    pub megameepo_ready: bool,
    pub has_shard: bool,
    pub has_scepter: bool,
    pub blink_slot_key: Option<char>,
    pub combo_item_keys: Vec<(String, char)>,
    pub clone_state: KnownCloneState,
}

fn ability_is_ready(event: &GsiWebhookEvent, ability_name: &str) -> bool {
    (0..=5).any(|index| {
        event.abilities.get_by_index(index).is_some_and(|ability| {
            ability.name == ability_name && ability.level > 0 && ability.can_cast
        })
    })
}

fn find_combo_item_key(
    event: &GsiWebhookEvent,
    settings: &Settings,
    partial_item_name: &str,
) -> Option<char> {
    event
        .items
        .all_slots()
        .into_iter()
        .find_map(|(slot, item)| {
            (item.name.contains(partial_item_name) && item.can_cast == Some(true))
                .then(|| settings.get_key_for_slot(slot))
                .flatten()
        })
}

pub fn derive_meepo_observed_state(
    event: &GsiWebhookEvent,
    settings: &Settings,
    in_danger: bool,
) -> Option<MeepoObservedState> {
    (event.hero.name == Hero::Meepo.to_game_name()).then(|| MeepoObservedState {
        hero_name: event.hero.name.clone(),
        hero_level: event.hero.level,
        health_percent: event.hero.health_percent,
        mana_percent: event.hero.mana_percent,
        in_danger,
        alive: event.hero.alive,
        stunned: event.hero.stunned,
        silenced: event.hero.silenced,
        poof_ready: ability_is_ready(event, "meepo_poof"),
        dig_ready: ability_is_ready(event, DIG_ABILITY_NAME),
        megameepo_ready: ability_is_ready(event, MEGAMEEPO_ABILITY_NAME),
        has_shard: event.hero.aghanims_shard,
        has_scepter: event.hero.aghanims_scepter,
        blink_slot_key: find_item_slot(event, settings, Item::Blink),
        combo_item_keys: settings
            .heroes
            .meepo
            .combo_items
            .iter()
            .filter_map(|item_name| {
                find_combo_item_key(event, settings, item_name).map(|key| (item_name.clone(), key))
            })
            .collect(),
        clone_state: KnownCloneState::Unavailable,
    })
}

pub fn refresh_meepo_observed_state(event: &GsiWebhookEvent, settings: &Settings, in_danger: bool) {
    let mut state = MEEPO_OBSERVED_STATE.lock().unwrap();
    *state = derive_meepo_observed_state(event, settings, in_danger);
}

pub fn clear_meepo_observed_state() {
    *MEEPO_OBSERVED_STATE.lock().unwrap() = None;
}

pub fn latest_meepo_observed_state() -> Option<MeepoObservedState> {
    MEEPO_OBSERVED_STATE.lock().unwrap().clone()
}

#[cfg(test)]
mod tests {
    use super::{
        clear_meepo_observed_state, derive_meepo_observed_state, latest_meepo_observed_state,
        refresh_meepo_observed_state, KnownCloneState,
    };
    use crate::actions::heroes::meepo_macro::meepo_macro_test_lock;
    use crate::config::Settings;
    use crate::models::GsiWebhookEvent;

    fn meepo_fixture() -> GsiWebhookEvent {
        serde_json::from_str(include_str!("../../../tests/fixtures/meepo_event.json"))
            .expect("Meepo fixture should deserialize")
    }

    #[test]
    fn derive_meepo_observed_state_collects_phase_two_snapshot() {
        let _guard = meepo_macro_test_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let event = meepo_fixture();
        let settings = Settings::default();

        let observed =
            derive_meepo_observed_state(&event, &settings, true).expect("Meepo state should exist");

        assert_eq!(observed.hero_name, "npc_dota_hero_meepo");
        assert_eq!(observed.hero_level, 18);
        assert_eq!(observed.health_percent, 43);
        assert!(observed.in_danger);
        assert!(observed.poof_ready);
        assert!(observed.dig_ready);
        assert!(observed.megameepo_ready);
        assert!(observed.has_shard);
        assert!(observed.has_scepter);
        assert_eq!(observed.blink_slot_key, settings.get_key_for_slot("slot0"));
        assert_eq!(
            observed.combo_item_keys,
            vec![
                (
                    "sheepstick".to_string(),
                    settings
                        .get_key_for_slot("slot1")
                        .expect("slot1 should have a key"),
                ),
                (
                    "disperser".to_string(),
                    settings
                        .get_key_for_slot("slot2")
                        .expect("slot2 should have a key"),
                ),
            ]
        );
        assert_eq!(observed.clone_state, KnownCloneState::Unavailable);
    }

    #[test]
    fn derive_meepo_observed_state_returns_none_for_other_heroes() {
        let _guard = meepo_macro_test_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut event = meepo_fixture();
        let settings = Settings::default();
        event.hero.name = "npc_dota_hero_huskar".to_string();

        assert!(derive_meepo_observed_state(&event, &settings, false).is_none());
    }

    #[test]
    fn meepo_observed_state_cache_can_refresh_and_clear() {
        let _guard = meepo_macro_test_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let event = meepo_fixture();
        let settings = Settings::default();
        clear_meepo_observed_state();

        refresh_meepo_observed_state(&event, &settings, false);
        assert_eq!(
            latest_meepo_observed_state()
                .expect("state should be cached")
                .health_percent,
            43
        );

        clear_meepo_observed_state();
        assert!(latest_meepo_observed_state().is_none());
    }
}
