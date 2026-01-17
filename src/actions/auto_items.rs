//! Auto-items on attack trigger
//!
//! Common module for automatically using targeted items and abilities when attacking.
//! Enabled per-hero with configurable modifier key, item list, and ability list.
//!
//! Usage: Hold modifier key (e.g., Space) + Right-click to:
//! 1. Use all configured items that are off cooldown
//! 2. Use all configured abilities (with optional HP threshold)
//! 3. Right-click the target

use crate::config::{AutoAbilityConfig, Settings};
use crate::input::simulation::{mouse_click, press_key};
use crate::models::GsiWebhookEvent;
use lazy_static::lazy_static;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use tracing::{debug, info};

lazy_static! {
    /// Track if the modifier key is currently held
    pub static ref MODIFIER_KEY_HELD: AtomicBool = AtomicBool::new(false);
    
    /// Cache of the latest GSI event for item state
    pub static ref LATEST_GSI_EVENT: Mutex<Option<GsiWebhookEvent>> = Mutex::new(None);
}

/// Update the cached GSI state (called from dispatcher)
pub fn update_gsi_state(event: &GsiWebhookEvent) {
    let mut cached = LATEST_GSI_EVENT.lock().unwrap();
    *cached = Some(event.clone());
}

/// Find item slot key by item name (partial match)
fn find_item_key(event: &GsiWebhookEvent, settings: &Settings, item_name: &str) -> Option<char> {
    let items = &event.items;
    let keybinds = &settings.keybindings;
    
    // Check each slot for the item (partial match, e.g., "orchid" matches "item_orchid")
    let slots = [
        (&items.slot0, keybinds.slot0),
        (&items.slot1, keybinds.slot1),
        (&items.slot2, keybinds.slot2),
        (&items.slot3, keybinds.slot3),
        (&items.slot4, keybinds.slot4),
        (&items.slot5, keybinds.slot5),
    ];
    
    for (item, key) in slots {
        if item.name.contains(item_name) {
            // Check if item is castable (not on cooldown, can be used)
            let can_cast = item.can_cast.unwrap_or(false);
            let cooldown = item.cooldown.unwrap_or(0);
            
            if can_cast && cooldown == 0 {
                debug!("🎯 Found castable item '{}' in slot with key '{}'", item.name, key);
                return Some(key);
            } else {
                debug!(
                    "🎯 Item '{}' found but not castable (can_cast={}, cd={})",
                    item.name, can_cast, cooldown
                );
            }
        }
    }
    
    None
}

/// Execute auto-items sequence: use configured items and abilities, then right-click
///
/// # Arguments
/// * `settings` - Global settings for keybindings
/// * `item_names` - List of item names to try using
/// * `auto_abilities` - List of abilities to auto-cast with optional HP thresholds
/// * `abilities_first` - If true, cast abilities before items; if false, items first
pub fn execute_auto_items(
    settings: &Settings,
    item_names: &[String],
    auto_abilities: &[AutoAbilityConfig],
    abilities_first: bool,
) {
    // Get cached GSI state
    let cached = LATEST_GSI_EVENT.lock().unwrap();
    let event = match cached.as_ref() {
        Some(e) => e.clone(),
        None => {
            debug!("🎯 No GSI state available for auto-items");
            // Still do the right-click even without item info
            mouse_click();
            return;
        }
    };
    drop(cached);
    
    let mut items_used = 0;
    let mut abilities_used = 0;
    
    // Helper closure to use items
    let use_items = |items_used: &mut u32| {
        for item_name in item_names {
            if let Some(key) = find_item_key(&event, settings, item_name) {
                info!("🎯 Using item '{}' (key: {})", item_name, key);
                press_key(key);
                *items_used += 1;
                thread::sleep(Duration::from_millis(30));
            }
        }
    };
    
    // Helper closure to use abilities
    let use_abilities = |abilities_used: &mut u32| {
        for ability_config in auto_abilities {
            // Check HP threshold if configured
            if let Some(threshold) = ability_config.hp_threshold {
                if event.hero.health_percent >= threshold {
                    debug!(
                        "🎯 Skipping ability {} (HP {}% >= {}%)",
                        ability_config.index, event.hero.health_percent, threshold
                    );
                    continue;
                }
            }
            
            // Get ability by index and check if castable
            if let Some(ability) = event.abilities.get_by_index(ability_config.index) {
                if ability.can_cast && ability.cooldown == 0 && ability.level > 0 {
                    if let Some(threshold) = ability_config.hp_threshold {
                        info!(
                            "🎯 Using ability {} key '{}' (HP {}% < {}%)",
                            ability_config.index, ability_config.key, event.hero.health_percent, threshold
                        );
                    } else {
                        info!("🎯 Using ability {} key '{}'", ability_config.index, ability_config.key);
                    }
                    press_key(ability_config.key);
                    *abilities_used += 1;
                    thread::sleep(Duration::from_millis(30));
                } else {
                    debug!(
                        "🎯 Ability {} not castable (can_cast={}, cd={}, level={})",
                        ability_config.index, ability.can_cast, ability.cooldown, ability.level
                    );
                }
            }
        }
    };
    
    // Execute in configured order
    if abilities_first {
        use_abilities(&mut abilities_used);
        use_items(&mut items_used);
    } else {
        use_items(&mut items_used);
        use_abilities(&mut abilities_used);
    }
    
    // Always right-click at the end (attack the target)
    if items_used > 0 || abilities_used > 0 {
        info!("🎯 Auto-combo complete ({} items, {} abilities), attacking", items_used, abilities_used);
    }
    mouse_click();
}
