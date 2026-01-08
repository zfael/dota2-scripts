//! Soul Ring automation module
//!
//! Automatically triggers Soul Ring before ability/item usage when:
//! - Soul Ring is in inventory and ready to cast
//! - Hero mana is below configured threshold
//! - Hero health is above safety threshold
//! - Cooldown lockout has elapsed (prevents double-fire)
//! - Item being used costs mana (skip list items like Blink, Phase Boots are excluded)

use crate::config::Settings;
use std::collections::HashMap;
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

    /// Check if a key is an item key that should trigger Soul Ring
    pub fn is_item_key(&self, key_char: char, settings: &Settings) -> bool {
        if !settings.soul_ring.intercept_item_keys {
            return false;
        }

        // Check if the key matches any item slot keybinding
        let item_keys = [
            settings.keybindings.slot0,
            settings.keybindings.slot1,
            settings.keybindings.slot2,
            settings.keybindings.slot3,
            settings.keybindings.slot4,
            settings.keybindings.slot5,
        ];

        let key_lower = key_char.to_ascii_lowercase();
        
        // Don't trigger on the Soul Ring's own key (would cause infinite loop)
        if let Some(sr_key) = self.slot_key {
            if key_lower == sr_key.to_ascii_lowercase() {
                return false;
            }
        }

        // Check if this key corresponds to an item slot
        let is_item_slot = item_keys
            .iter()
            .any(|k| k.to_ascii_lowercase() == key_lower);

        if !is_item_slot {
            return false;
        }

        // Check if the item in this slot should be skipped (no mana cost)
        if let Some(item_name) = self.get_item_for_key(key_char) {
            if self.should_skip_item(item_name) {
                debug!(
                    "💍 Soul Ring: skipping no-mana item '{}' on key '{}'",
                    item_name, key_char
                );
                return false;
            }
        }

        true
    }

    /// Check if a key should trigger Soul Ring (ability or item key)
    pub fn should_intercept_key(&self, key_char: char, settings: &Settings) -> bool {
        self.is_ability_key(key_char, settings) || self.is_item_key(key_char, settings)
    }
}

/// Global Soul Ring state, shared between keyboard listener and GSI handler
pub static SOUL_RING_STATE: std::sync::LazyLock<Arc<Mutex<SoulRingState>>> =
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(SoulRingState::new())));

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
}
