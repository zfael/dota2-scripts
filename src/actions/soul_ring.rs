//! Soul Ring automation module
//!
//! Automatically triggers Soul Ring before ability/item usage when:
//! - Soul Ring is in inventory and ready to cast
//! - Hero mana is below configured threshold
//! - Hero health is above safety threshold
//! - Cooldown lockout has elapsed (prevents double-fire)
//! - The ability or item about to be used actually spends mana
//!
//! That last check is the reason [`crate::actions::mana_costs`] exists. Soul Ring pays
//! 170 HP for its mana, so firing it ahead of something free is a pure loss - and free
//! actives are the common case, not the exception: of the items with an active, only 59
//! of 276 cost mana, and every one of Huskar's abilities is free because he spends HP
//! instead. GSI reports readiness but never a cost, so the cost comes from the generated
//! table.

use crate::actions::activity::{push_activity, ActivityCategory};
use crate::actions::mana_costs::{ability_mana_cost, item_mana_cost};
use crate::config::Settings;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{debug, info};

/// Health Soul Ring's Sacrifice spends to refill mana. Fixed by the item, not scaled.
///
/// Danger detection needs this number so the drop is not read as incoming damage; see
/// [`crate::actions::danger_detector::note_self_damage`].
pub const SOUL_RING_HEALTH_COST: u32 = 170;

/// What a key press is about to spend mana on, as far as the last GSI event knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManaSpend {
    /// Known to cost mana. Soul Ring is worth firing.
    Costs(u32),
    /// Known to cost nothing - a passive, a toggle, or a free active such as Quelling
    /// Blade's chop or any Huskar ability. Never worth 170 HP.
    Free,
    /// Nothing castable is bound here at all: an empty slot, an unlearned ability, or a
    /// key GSI has told us nothing about.
    Nothing,
    /// Present and castable, but absent from the generated table - most likely an item or
    /// ability added after the last regeneration. Treated as [`ManaSpend::Nothing`] so a
    /// post-patch gap costs a missed buff rather than 170 HP.
    Unknown,
}

impl ManaSpend {
    /// Whether Soul Ring should fire ahead of this press.
    pub fn warrants_soul_ring(&self) -> bool {
        matches!(self, ManaSpend::Costs(cost) if *cost > 0)
    }
}

/// One ability slot as of the last GSI event, enough to price a key press.
#[derive(Debug, Clone, Default)]
pub struct AbilitySlot {
    pub name: String,
    /// GSI's 1-based level. `0` means unlearned, so the key does nothing.
    pub level: u32,
    pub passive: bool,
}

/// Shared state for Soul Ring automation, updated by GSI events
#[derive(Debug)]
pub struct SoulRingState {
    /// Whether Soul Ring is currently in inventory
    pub available: bool,
    /// The key to press to use Soul Ring (based on its slot)
    pub slot_key: Option<char>,
    /// Whether Soul Ring can be cast (not on cooldown)
    pub can_cast: bool,
    /// Current hero mana percentage (0-100)
    pub hero_mana_percent: u32,
    /// Current hero health percentage (0-100)
    pub hero_health_percent: u32,
    /// Whether the hero is alive
    pub hero_alive: bool,
    /// Last time Soul Ring was triggered (for cooldown lockout)
    pub last_triggered: Option<Instant>,
    /// Maps slot keys to item names (for mana-cost lookup)
    pub slot_items: HashMap<char, String>,
    /// Ability slots by GSI index (0-5), for mana-cost lookup on ability keys.
    pub ability_slots: Vec<AbilitySlot>,
    /// GSI index of the ability flagged `ultimate`, which is what `R` casts.
    pub ultimate_index: Option<usize>,
    /// Whether the hero is currently silenced, muted, or hexed. All three block casting
    /// outright, so Soul Ring would burn 170 HP for a press the game will drop.
    pub cast_blocked: bool,
}

impl Default for SoulRingState {
    fn default() -> Self {
        Self {
            available: false,
            slot_key: None,
            can_cast: false,
            hero_mana_percent: 100,
            hero_health_percent: 100,
            hero_alive: false,
            last_triggered: None,
            slot_items: HashMap::new(),
            ability_slots: Vec::new(),
            ultimate_index: None,
            cast_blocked: false,
        }
    }
}

impl SoulRingState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if Soul Ring should be triggered before an ability/item key press
    pub fn should_trigger(&self, settings: &Settings) -> bool {
        // Master toggle must be enabled
        if !settings.soul_ring.enabled {
            return false;
        }

        // Soul Ring must be available and ready
        if !self.available || !self.can_cast || self.slot_key.is_none() {
            return false;
        }

        // Hero must be alive
        if !self.hero_alive {
            return false;
        }

        // Mana must be below threshold (100 = always trigger)
        if settings.soul_ring.min_mana_percent < 100
            && self.hero_mana_percent >= settings.soul_ring.min_mana_percent
        {
            debug!(
                "💍 Soul Ring: mana {}% >= threshold {}%, skipping",
                self.hero_mana_percent, settings.soul_ring.min_mana_percent
            );
            return false;
        }

        // Health must be above safety threshold
        if self.hero_health_percent <= settings.soul_ring.min_health_percent {
            debug!(
                "💍 Soul Ring: health {}% <= safety threshold {}%, skipping",
                self.hero_health_percent, settings.soul_ring.min_health_percent
            );
            return false;
        }

        // Check cooldown lockout
        if let Some(last) = self.last_triggered {
            let elapsed = last.elapsed();
            let cooldown = Duration::from_millis(settings.soul_ring.trigger_cooldown_ms);
            if elapsed < cooldown {
                debug!(
                    "💍 Soul Ring: cooldown lockout ({:?} < {:?}), skipping",
                    elapsed, cooldown
                );
                return false;
            }
        }

        true
    }

    /// Mark Soul Ring as triggered (updates cooldown lockout)
    pub fn mark_triggered(&mut self) {
        self.last_triggered = Some(Instant::now());

        // Tell danger detection the HP drop it is about to see is ours. Without this the
        // sacrifice alone clears both danger triggers, and buying mana would fire the
        // whole defensive kit - Blade Mail, Ghost Scepter, Glimmer - at nobody.
        crate::actions::danger_detector::note_self_damage(SOUL_RING_HEALTH_COST, "Soul Ring");

        info!(
            "💍 Soul Ring triggered! mana={}%, health={}%",
            self.hero_mana_percent, self.hero_health_percent
        );
    }

    /// Check if a key is an ability key that should trigger Soul Ring
    pub fn is_ability_key(&self, key_char: char, settings: &Settings) -> bool {
        let key_str = key_char.to_ascii_lowercase().to_string();
        settings
            .soul_ring
            .ability_keys
            .iter()
            .any(|k| k.to_lowercase() == key_str)
    }

    /// Get the item name for a given slot key (if any)
    pub fn get_item_for_key(&self, key_char: char) -> Option<&String> {
        let key_lower = key_char.to_ascii_lowercase();
        self.slot_items.get(&key_lower)
    }

    /// Price the item bound to `key_char`.
    pub fn item_spend_for_key(&self, key_char: char) -> ManaSpend {
        let Some(item_name) = self.get_item_for_key(key_char) else {
            // Empty slot, or a slot GSI has not mapped to a key. Nothing to cast.
            return ManaSpend::Nothing;
        };

        match item_mana_cost(item_name) {
            Some(0) => ManaSpend::Free,
            Some(cost) => ManaSpend::Costs(cost),
            None => ManaSpend::Unknown,
        }
    }

    /// Which GSI ability index a key casts, under Dota's default ability bindings.
    ///
    /// `R` is resolved from the `ultimate` flag rather than assumed to be index 3,
    /// because Aghanim's- and talent-granted abilities shift the tail of the list.
    fn ability_index_for_key(&self, key_char: char) -> Option<usize> {
        match key_char.to_ascii_lowercase() {
            'q' => Some(0),
            'w' => Some(1),
            'e' => Some(2),
            'd' => Some(3),
            'f' => Some(4),
            'r' => self.ultimate_index,
            _ => None,
        }
    }

    /// Price the ability bound to `key_char`.
    ///
    /// Deliberately does not consult `ability.can_cast`: that goes false precisely when
    /// mana is short, which is the case Soul Ring exists to fix.
    pub fn ability_spend_for_key(&self, key_char: char) -> ManaSpend {
        let Some(index) = self.ability_index_for_key(key_char) else {
            return ManaSpend::Nothing;
        };
        let Some(slot) = self.ability_slots.get(index) else {
            return ManaSpend::Nothing;
        };

        // Unlearned, passive, or an empty slot: the press casts nothing.
        if slot.level == 0 || slot.passive || slot.name.is_empty() || slot.name == "empty" {
            return ManaSpend::Nothing;
        }

        match ability_mana_cost(&slot.name, slot.level) {
            Some(0) => ManaSpend::Free,
            Some(cost) => ManaSpend::Costs(cost),
            None => ManaSpend::Unknown,
        }
    }
}

/// Global Soul Ring state, shared between keyboard listener and GSI handler
pub static SOUL_RING_STATE: std::sync::LazyLock<Arc<Mutex<SoulRingState>>> =
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(SoulRingState::new())));

/// Static keyboard configuration snapshot for Soul Ring, derived from `Settings`.
///
/// Separates config-time constants (ability keys, item slot keys, thresholds)
/// from live runtime facts (`SOUL_RING_STATE`) that still come from GSI.
#[derive(Debug, Clone)]
pub struct SoulRingKeyboardConfig {
    pub enabled: bool,
    pub min_mana_percent: u32,
    pub min_health_percent: u32,
    pub delay_before_ability_ms: u64,
    pub trigger_cooldown_ms: u64,
    /// Ability keys that should trigger Soul Ring (stored lowercase).
    pub ability_keys: HashSet<char>,
    pub intercept_item_keys: bool,
    /// Item slot keys from keybindings (stored lowercase).
    pub item_slot_keys: HashSet<char>,
}

impl SoulRingKeyboardConfig {
    /// Build a config snapshot from `Settings`.
    pub fn from_settings(settings: &Settings) -> Self {
        let ability_keys = settings
            .soul_ring
            .ability_keys
            .iter()
            .filter_map(|s| s.chars().next())
            .map(|c| c.to_ascii_lowercase())
            .collect();

        let item_slot_keys = [
            settings.keybindings.slot0,
            settings.keybindings.slot1,
            settings.keybindings.slot2,
            settings.keybindings.slot3,
            settings.keybindings.slot4,
            settings.keybindings.slot5,
        ]
        .iter()
        .map(|c| c.to_ascii_lowercase())
        .collect();

        Self {
            enabled: settings.soul_ring.enabled,
            min_mana_percent: settings.soul_ring.min_mana_percent,
            min_health_percent: settings.soul_ring.min_health_percent,
            delay_before_ability_ms: settings.soul_ring.delay_before_ability_ms,
            trigger_cooldown_ms: settings.soul_ring.trigger_cooldown_ms,
            ability_keys,
            intercept_item_keys: settings.soul_ring.intercept_item_keys,
            item_slot_keys,
        }
    }

    /// Return `true` if `key` is in the ability-keys set (case-insensitive).
    pub fn is_ability_key(&self, key: char) -> bool {
        self.ability_keys.contains(&key.to_ascii_lowercase())
    }

    /// Return `true` if `key` is in the item-slot-keys set (case-insensitive).
    pub fn is_item_slot_key(&self, key: char) -> bool {
        self.item_slot_keys.contains(&key.to_ascii_lowercase())
    }
}

impl SoulRingState {
    /// Config-based variant of [`should_trigger`] that accepts a pre-built
    /// `SoulRingKeyboardConfig` instead of the full `Settings`.
    pub fn should_trigger_with_config(&self, config: &SoulRingKeyboardConfig) -> bool {
        if !config.enabled {
            return false;
        }
        if !self.available || !self.can_cast || self.slot_key.is_none() {
            return false;
        }
        if !self.hero_alive {
            return false;
        }
        if config.min_mana_percent < 100 && self.hero_mana_percent >= config.min_mana_percent {
            return false;
        }
        if self.hero_health_percent <= config.min_health_percent {
            return false;
        }
        if let Some(last) = self.last_triggered {
            let elapsed = last.elapsed();
            let cooldown = Duration::from_millis(config.trigger_cooldown_ms);
            if elapsed < cooldown {
                return false;
            }
        }
        true
    }

    /// What pressing `key_char` is about to spend, given the last GSI event.
    ///
    /// Ability keys and item keys can overlap in a custom layout, so an ability that
    /// costs mana wins over an item slot bound to the same key.
    pub fn spend_for_key(&self, key_char: char, config: &SoulRingKeyboardConfig) -> ManaSpend {
        // A silence, mute, or hex drops the press entirely - nothing to pay for.
        if self.cast_blocked {
            return ManaSpend::Nothing;
        }

        if config.is_ability_key(key_char) {
            let spend = self.ability_spend_for_key(key_char);
            if spend.warrants_soul_ring() {
                return spend;
            }

            // Fall through to the item table only if this key is also an item slot.
            if !config.intercept_item_keys || !config.is_item_slot_key(key_char) {
                return spend;
            }
        }

        if !config.intercept_item_keys || !config.is_item_slot_key(key_char) {
            return ManaSpend::Nothing;
        }

        // Never intercept Soul Ring's own key - that would recurse.
        if let Some(sr_key) = self.slot_key {
            if key_char.to_ascii_lowercase() == sr_key.to_ascii_lowercase() {
                return ManaSpend::Nothing;
            }
        }

        self.item_spend_for_key(key_char)
    }

    /// Config-based keyboard interception helper for the cached snapshot path.
    pub fn should_intercept_key_with_config(
        &self,
        key_char: char,
        config: &SoulRingKeyboardConfig,
    ) -> bool {
        self.spend_for_key(key_char, config).warrants_soul_ring()
    }
}

/// Press an ability key with automatic Soul Ring triggering (for use in combos)
/// This is the programmatic equivalent of the keyboard interception
pub fn press_ability_with_soul_ring(key: char, settings: &Settings) {
    let mut state = SOUL_RING_STATE.lock().unwrap();

    // Same mana-cost gate as the keyboard hook: a combo must not spend 170 HP to cast
    // something free either.
    let config = SoulRingKeyboardConfig::from_settings(settings);
    let spend = state.spend_for_key(key, &config);

    if state.should_trigger(settings)
        && state.is_ability_key(key, settings)
        && spend.warrants_soul_ring()
    {
        if let Some(sr_key) = state.slot_key {
            state.mark_triggered();
            drop(state); // Release lock before sleeping

            info!("💍 Soul Ring before ability '{}'", key);
            push_activity(ActivityCategory::Action, "Soul Ring combo triggered");
            crate::input::simulation::press_key(sr_key);
            std::thread::sleep(std::time::Duration::from_millis(
                settings.soul_ring.delay_before_ability_ms,
            ));
        } else {
            drop(state);
        }
    } else {
        drop(state);
    }

    // Press the ability key
    crate::input::simulation::press_key(key);
}

/// Update Soul Ring state from GSI event
pub fn update_from_gsi(
    items: &crate::models::gsi_event::Items,
    abilities: &crate::models::gsi_event::Abilities,
    hero: &crate::models::gsi_event::Hero,
    settings: &Settings,
) {
    let mut state = SOUL_RING_STATE.lock().unwrap();

    // Update hero stats
    state.hero_mana_percent = hero.mana_percent;
    state.hero_health_percent = hero.health_percent;
    state.hero_alive = hero.alive;
    state.cast_blocked = hero.silenced || hero.muted || hero.hexed;

    // Rebuild the ability slots so ability keys can be priced.
    state.ability_slots = (0..=5)
        .filter_map(|index| abilities.get_by_index(index))
        .map(|ability| AbilitySlot {
            name: ability.name.clone(),
            level: ability.level,
            passive: ability.passive,
        })
        .collect();
    state.ultimate_index = (0..=5)
        .filter_map(|index| abilities.get_by_index(index).map(|a| (index, a)))
        .find(|(_, ability)| ability.ultimate)
        .map(|(index, _)| index as usize);

    // Clear and rebuild slot_items mapping
    state.slot_items.clear();

    // Search for Soul Ring in inventory and build slot->item mapping
    let mut found = false;
    for (slot_name, item) in items.all_slots() {
        // Build slot key -> item name mapping for mana-cost lookup
        if let Some(slot_key) = settings.get_key_for_slot(slot_name) {
            if !item.name.is_empty() && item.name != "empty" {
                state
                    .slot_items
                    .insert(slot_key.to_ascii_lowercase(), item.name.clone());
            }
        }

        // Check for Soul Ring
        if item.name == "item_soul_ring" {
            found = true;
            state.available = true;
            state.can_cast = item.can_cast.unwrap_or(false);
            state.slot_key = settings.get_key_for_slot(slot_name);

            debug!(
                "💍 Soul Ring found in {}: can_cast={}, key={:?}",
                slot_name, state.can_cast, state.slot_key
            );
        }
    }

    // If Soul Ring not found, mark as unavailable
    if !found && state.available {
        info!("💍 Soul Ring no longer in inventory, disabling automation");
        state.available = false;
        state.slot_key = None;
        state.can_cast = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ability(name: &str, level: u32) -> AbilitySlot {
        AbilitySlot {
            name: name.to_string(),
            level,
            passive: false,
        }
    }

    fn passive_ability(name: &str, level: u32) -> AbilitySlot {
        AbilitySlot {
            name: name.to_string(),
            level,
            passive: true,
        }
    }

    /// Soul Ring in `z`, one item key under test in `x`.
    fn item_config() -> SoulRingKeyboardConfig {
        SoulRingKeyboardConfig {
            enabled: true,
            min_mana_percent: 100,
            min_health_percent: 1,
            delay_before_ability_ms: 30,
            trigger_cooldown_ms: 250,
            ability_keys: ['q', 'w', 'e', 'r'].into_iter().collect(),
            intercept_item_keys: true,
            item_slot_keys: ['z', 'x', 'c'].into_iter().collect(),
        }
    }

    fn state_holding(key: char, item: &str) -> SoulRingState {
        let mut state = SoulRingState::default();
        state.slot_key = Some('z');
        state.slot_items.insert('z', "item_soul_ring".to_string());
        state.slot_items.insert(key, item.to_string());
        state
    }

    /// The reported bug: chopping a tree with Quelling Blade spends no mana, so it must
    /// never cost 170 HP.
    #[test]
    fn free_item_actives_do_not_trigger_soul_ring() {
        let config = item_config();

        for free in [
            "item_quelling_blade",
            "item_bfury",
            "item_blink",
            "item_phase_boots",
            "item_hand_of_midas",
            "item_satanic",
        ] {
            let state = state_holding('x', free);
            assert!(
                !state.should_intercept_key_with_config('x', &config),
                "{free} costs no mana and must not trigger Soul Ring"
            );
        }
    }

    #[test]
    fn mana_costing_items_still_trigger_soul_ring() {
        let config = item_config();

        for (item, cost) in [
            ("item_shivas_guard", 75),
            ("item_sheepstick", 250),
            ("item_black_king_bar", 50),
            ("item_manta", 125),
        ] {
            let state = state_holding('x', item);
            assert_eq!(
                state.item_spend_for_key('x'),
                ManaSpend::Costs(cost),
                "{item} should be priced at {cost}"
            );
            assert!(state.should_intercept_key_with_config('x', &config));
        }
    }

    /// Regression: an unmapped key used to fall through to "trigger", so pressing an
    /// empty item slot spent 170 HP on nothing.
    #[test]
    fn empty_item_slot_does_not_trigger_soul_ring() {
        let config = item_config();
        let mut state = SoulRingState::default();
        state.slot_key = Some('z');

        assert_eq!(state.item_spend_for_key('x'), ManaSpend::Nothing);
        assert!(!state.should_intercept_key_with_config('x', &config));
    }

    /// An item added after the last table regeneration must fail safe.
    #[test]
    fn item_missing_from_the_table_does_not_trigger_soul_ring() {
        let config = item_config();
        let state = state_holding('x', "item_some_future_patch_thing");

        assert_eq!(state.item_spend_for_key('x'), ManaSpend::Unknown);
        assert!(!state.should_intercept_key_with_config('x', &config));
    }

    #[test]
    fn soul_rings_own_key_never_triggers_itself() {
        let config = item_config();
        let state = state_holding('z', "item_soul_ring");

        assert!(!state.should_intercept_key_with_config('z', &config));
    }

    /// Huskar pays health, not mana, for every one of his abilities.
    #[test]
    fn free_hero_abilities_do_not_trigger_soul_ring() {
        let config = item_config();
        let mut state = SoulRingState::default();
        state.ability_slots = vec![
            ability("huskar_inner_fire", 4),
            ability("huskar_burning_spear", 4),
            passive_ability("huskar_berserkers_blood", 1),
            ability("huskar_life_break", 1),
        ];
        state.ultimate_index = Some(3);

        for key in ['q', 'w', 'e', 'r'] {
            assert!(
                !state.should_intercept_key_with_config(key, &config),
                "Huskar's '{key}' spends health, not mana"
            );
        }
    }

    #[test]
    fn unlearned_and_passive_abilities_do_not_trigger_soul_ring() {
        let config = item_config();
        let mut state = SoulRingState::default();
        state.ability_slots = vec![
            ability("mirana_starfall", 0),                 // unlearned
            passive_ability("huskar_berserkers_blood", 2), // passive
            ability("", 0),                                // empty slot
        ];

        assert_eq!(state.ability_spend_for_key('q'), ManaSpend::Nothing);
        assert_eq!(state.ability_spend_for_key('w'), ManaSpend::Nothing);
        assert_eq!(state.ability_spend_for_key('e'), ManaSpend::Nothing);
        for key in ['q', 'w', 'e'] {
            assert!(!state.should_intercept_key_with_config(key, &config));
        }
    }

    /// `R` follows the `ultimate` flag rather than assuming ability index 3, because
    /// Aghanim's- and talent-granted abilities shift the tail of the list.
    #[test]
    fn ultimate_key_resolves_through_the_ultimate_flag() {
        let mut state = SoulRingState::default();
        state.ability_slots = vec![
            ability("mirana_starfall", 1),
            ability("mirana_arrow", 1),
            ability("mirana_leap", 1),
            ability("mirana_invis", 1),
        ];
        state.ultimate_index = Some(3);

        // Moonlight Shadow costs 125.
        assert_eq!(state.ability_spend_for_key('r'), ManaSpend::Costs(125));

        // With no ultimate reported, 'r' resolves to nothing rather than guessing.
        state.ultimate_index = None;
        assert_eq!(state.ability_spend_for_key('r'), ManaSpend::Nothing);
    }

    #[test]
    fn ability_cost_tracks_the_learned_level() {
        let mut state = SoulRingState::default();

        // Starstorm scales 80 / 90 / 100 / 110.
        state.ability_slots = vec![ability("mirana_starfall", 1)];
        assert_eq!(state.ability_spend_for_key('q'), ManaSpend::Costs(80));

        state.ability_slots = vec![ability("mirana_starfall", 4)];
        assert_eq!(state.ability_spend_for_key('q'), ManaSpend::Costs(110));
    }

    /// A silence, mute, or hex drops the press, so there is nothing to pay for.
    #[test]
    fn blocked_casting_never_triggers_soul_ring() {
        let config = item_config();
        let mut state = state_holding('x', "item_shivas_guard");
        state.ability_slots = vec![ability("mirana_starfall", 1)];

        assert!(state.should_intercept_key_with_config('x', &config));
        assert!(state.should_intercept_key_with_config('q', &config));

        state.cast_blocked = true;
        assert!(!state.should_intercept_key_with_config('x', &config));
        assert!(!state.should_intercept_key_with_config('q', &config));
    }

    #[test]
    fn test_default_state() {
        let state = SoulRingState::default();
        assert!(!state.available);
        assert!(!state.can_cast);
        assert!(state.slot_key.is_none());
        assert!(!state.hero_alive);
    }

    #[test]
    fn soul_ring_keyboard_config_matches_ability_keys_case_insensitively() {
        let config = SoulRingKeyboardConfig {
            enabled: true,
            min_mana_percent: 100,
            min_health_percent: 1,
            delay_before_ability_ms: 30,
            trigger_cooldown_ms: 250,
            ability_keys: ['q', 'w', 'e'].into_iter().collect(),
            intercept_item_keys: false,
            item_slot_keys: ['z', 'x', 'c', 'v', 'b', 'n'].into_iter().collect(),
        };

        assert!(config.is_ability_key('Q'));
        assert!(config.is_ability_key('w'));
        assert!(!config.is_ability_key('r'));
    }

    #[test]
    fn soul_ring_keyboard_config_matches_item_slot_keys_case_insensitively() {
        let config = SoulRingKeyboardConfig {
            enabled: true,
            min_mana_percent: 100,
            min_health_percent: 1,
            delay_before_ability_ms: 30,
            trigger_cooldown_ms: 250,
            ability_keys: ['q', 'w', 'e'].into_iter().collect(),
            intercept_item_keys: true,
            item_slot_keys: ['z', 'x', 'c', 'v', 'b', 'n'].into_iter().collect(),
        };

        assert!(config.is_item_slot_key('Z'));
        assert!(config.is_item_slot_key('n'));
        assert!(!config.is_item_slot_key('q'));
    }

    #[test]
    fn should_trigger_with_config_respects_enabled_flag() {
        let mut state = SoulRingState::default();
        state.available = true;
        state.can_cast = true;
        state.slot_key = Some('z');
        state.hero_alive = true;
        state.hero_mana_percent = 50;
        state.hero_health_percent = 80;

        let config = SoulRingKeyboardConfig {
            enabled: false,
            min_mana_percent: 100,
            min_health_percent: 1,
            delay_before_ability_ms: 30,
            trigger_cooldown_ms: 250,
            ability_keys: ['q'].into_iter().collect(),
            intercept_item_keys: false,
            item_slot_keys: HashSet::new(),
        };

        assert!(!state.should_trigger_with_config(&config));
    }

    #[test]
    fn should_trigger_with_config_passes_when_all_conditions_met() {
        let mut state = SoulRingState::default();
        state.available = true;
        state.can_cast = true;
        state.slot_key = Some('z');
        state.hero_alive = true;
        state.hero_mana_percent = 50;
        state.hero_health_percent = 80;

        let config = SoulRingKeyboardConfig {
            enabled: true,
            min_mana_percent: 100,
            min_health_percent: 1,
            delay_before_ability_ms: 30,
            trigger_cooldown_ms: 250,
            ability_keys: ['q'].into_iter().collect(),
            intercept_item_keys: false,
            item_slot_keys: HashSet::new(),
        };

        assert!(state.should_trigger_with_config(&config));
    }

    /// An ability key only intercepts once GSI has told us the bound ability costs mana.
    /// Before the first event `ability_slots` is empty, which reads as "nothing to cast".
    #[test]
    fn should_intercept_key_with_config_ability_key_needs_a_mana_costing_ability() {
        let config = SoulRingKeyboardConfig {
            enabled: true,
            min_mana_percent: 100,
            min_health_percent: 1,
            delay_before_ability_ms: 30,
            trigger_cooldown_ms: 250,
            ability_keys: ['q', 'w', 'e'].into_iter().collect(),
            intercept_item_keys: false,
            item_slot_keys: HashSet::new(),
        };

        // No GSI event yet - nothing is known to cost mana.
        let empty = SoulRingState::default();
        assert!(!empty.should_intercept_key_with_config('q', &config));

        // Mirana: Starstorm on Q costs 80, Sacred Arrow on W costs 90.
        let mut state = SoulRingState::default();
        state.ability_slots = vec![
            ability("mirana_starfall", 1),
            ability("mirana_arrow", 1),
            ability("mirana_leap", 1),
        ];

        assert!(state.should_intercept_key_with_config('q', &config));
        assert!(state.should_intercept_key_with_config('W', &config));
        // 'r' is not in ability_keys for this config.
        assert!(!state.should_intercept_key_with_config('r', &config));
    }

    #[test]
    fn should_intercept_key_with_config_skips_item_keys_when_disabled() {
        let state = SoulRingState::default();

        let config = SoulRingKeyboardConfig {
            enabled: true,
            min_mana_percent: 100,
            min_health_percent: 1,
            delay_before_ability_ms: 30,
            trigger_cooldown_ms: 250,
            ability_keys: HashSet::new(),
            intercept_item_keys: false,
            item_slot_keys: ['z', 'x', 'c'].into_iter().collect(),
        };

        assert!(!state.should_intercept_key_with_config('z', &config));
    }
}
