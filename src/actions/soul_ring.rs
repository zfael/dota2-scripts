//! Soul Ring automation module
//!
//! Automatically triggers Soul Ring before ability/item usage when:
//! - Soul Ring is in inventory and ready to cast
//! - Hero mana is below configured threshold
//! - Hero health is above safety threshold
//! - Cooldown lockout has elapsed (prevents double-fire)
//! - Item being used costs mana (skip list items like Blink, Phase Boots are excluded)

use crate::config::Settings;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{debug, info};

/// Items that should NOT trigger Soul Ring (no mana cost or special behavior)
/// These items are free to use and don't benefit from the extra mana
pub static SOUL_RING_SKIP_ITEMS: &[&str] = &[
    // Mobility items (no mana cost)
    "item_blink",
    "item_overwhelming_blink",
    "item_swift_blink",
    "item_arcane_blink",
    "item_phase_boots",
    "item_travel_boots",
    "item_travel_boots_2",
    // Consumables
    "item_bottle",
    "item_tpscroll",
    "item_flask",              // Healing Salve
    "item_clarity",
    "item_enchanted_mango",
    "item_faerie_fire",
    "item_tango",
    "item_tango_single",
    "item_smoke_of_deceit",
    "item_dust",
    "item_ward_observer",
    "item_ward_sentry",
    "item_tome_of_knowledge",
    "item_cheese",
    // Toggle/no-mana items
    "item_armlet",
    "item_power_treads",
    "item_soul_ring",          // Soul Ring itself
    // Shadow/invis items (no mana)
    "item_shadow_amulet",
    // Other no-mana actives
    "item_satanic",
    "item_hand_of_midas",
    "item_guardian_greaves",   // Greaves restores mana, would be wasteful
];

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
    /// Maps slot keys to item names (for skip-list checking)
    pub slot_items: HashMap<char, String>,
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

    /// Check if an item should be skipped (no mana cost)
    pub fn should_skip_item(&self, item_name: &str) -> bool {
        SOUL_RING_SKIP_ITEMS.contains(&item_name)
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

    /// Config-based keyboard interception helper for the cached snapshot path.
    pub fn should_intercept_key_with_config(
        &self,
        key_char: char,
        config: &SoulRingKeyboardConfig,
    ) -> bool {
        if config.is_ability_key(key_char) {
            return true;
        }
        if !config.intercept_item_keys {
            return false;
        }
        let key_lower = key_char.to_ascii_lowercase();
        // Don't intercept Soul Ring's own key.
        if let Some(sr_key) = self.slot_key {
            if key_lower == sr_key.to_ascii_lowercase() {
                return false;
            }
        }
        if !config.is_item_slot_key(key_char) {
            return false;
        }
        // Skip items with no mana cost.
        if let Some(item_name) = self.get_item_for_key(key_char) {
            if self.should_skip_item(item_name) {
                return false;
            }
        }
        true
    }
}

/// Press an ability key with automatic Soul Ring triggering (for use in combos)
/// This is the programmatic equivalent of the keyboard interception
pub fn press_ability_with_soul_ring(key: char, settings: &Settings) {
    let mut state = SOUL_RING_STATE.lock().unwrap();
    
    if state.should_trigger(settings) && state.is_ability_key(key, settings) {
        if let Some(sr_key) = state.slot_key {
            state.mark_triggered();
            drop(state); // Release lock before sleeping
            
            info!("💍 Soul Ring before ability '{}'", key);
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
    hero: &crate::models::gsi_event::Hero,
    settings: &Settings,
) {
    let mut state = SOUL_RING_STATE.lock().unwrap();

    // Update hero stats
    state.hero_mana_percent = hero.mana_percent;
    state.hero_health_percent = hero.health_percent;
    state.hero_alive = hero.alive;

    // Clear and rebuild slot_items mapping
    state.slot_items.clear();

    // Search for Soul Ring in inventory and build slot->item mapping
    let mut found = false;
    for (slot_name, item) in items.all_slots() {
        // Build slot key -> item name mapping for skip-list checking
        if let Some(slot_key) = settings.get_key_for_slot(slot_name) {
            if !item.name.is_empty() && item.name != "empty" {
                state.slot_items.insert(slot_key.to_ascii_lowercase(), item.name.clone());
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

    #[test]
    fn should_intercept_key_with_config_ability_key_returns_true() {
        let state = SoulRingState::default();

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

        assert!(state.should_intercept_key_with_config('q', &config));
        assert!(state.should_intercept_key_with_config('W', &config));
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
