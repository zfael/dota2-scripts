//! Soul Ring automation module
//!
//! Automatically triggers Soul Ring before ability/item usage when:
//! - Soul Ring is in inventory and ready to cast
//! - Hero mana is below configured threshold
//! - Hero health is above safety threshold
//! - Cooldown lockout has elapsed (prevents double-fire)

use crate::config::Settings;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{debug, info};

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

        // Mana must be below threshold
        if self.hero_mana_percent >= settings.soul_ring.min_mana_percent {
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

        item_keys
            .iter()
            .any(|k| k.to_ascii_lowercase() == key_lower)
    }

    /// Check if a key should trigger Soul Ring (ability or item key)
    pub fn should_intercept_key(&self, key_char: char, settings: &Settings) -> bool {
        self.is_ability_key(key_char, settings) || self.is_item_key(key_char, settings)
    }
}

/// Global Soul Ring state, shared between keyboard listener and GSI handler
pub static SOUL_RING_STATE: std::sync::LazyLock<Arc<Mutex<SoulRingState>>> =
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(SoulRingState::new())));

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

    // Search for Soul Ring in inventory
    let mut found = false;
    for (slot_name, item) in items.all_slots() {
        if item.name == "item_soul_ring" {
            found = true;
            state.available = true;
            state.can_cast = item.can_cast.unwrap_or(false);
            state.slot_key = settings.get_key_for_slot(slot_name);

            debug!(
                "💍 Soul Ring found in {}: can_cast={}, key={:?}",
                slot_name, state.can_cast, state.slot_key
            );
            break;
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
