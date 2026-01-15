//! Auto-items on attack trigger
//!
//! Common module for automatically using targeted items when attacking.
//! Enabled per-hero with configurable modifier key and item list.
//!
//! Usage: Hold modifier key (e.g., Space) + Right-click to:
//! 1. Use all configured items that are off cooldown
//! 2. Right-click the target

use crate::config::Settings;
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

/// Execute auto-items sequence: use configured items, optionally ult/Q, then right-click
pub fn execute_auto_items(
    settings: &Settings,
    item_names: &[String],
    use_ult: bool,
    use_q: bool,
    q_hp_threshold: u32,
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
    
    // Try to use each configured item
    for item_name in item_names {
        if let Some(key) = find_item_key(&event, settings, item_name) {
            info!("🎯 Using item '{}' (key: {})", item_name, key);
            press_key(key);
            items_used += 1;
            thread::sleep(Duration::from_millis(30));
        }
    }
    
    // Press Q if enabled and HP below threshold
    if use_q && event.hero.health_percent < q_hp_threshold {
        let q_ready = event.abilities.ability0.can_cast && event.abilities.ability0.cooldown == 0;
        if q_ready {
            info!("🎯 Using Q (HP {}% < {}%)", event.hero.health_percent, q_hp_threshold);
            press_key('q');
            thread::sleep(Duration::from_millis(30));
        }
    }
    
    // Press ultimate (R) if enabled
    if use_ult {
        let ult_ready = event.abilities.ability3.can_cast && event.abilities.ability3.cooldown == 0;
        if ult_ready {
            info!("🎯 Using ultimate (R)");
            press_key('r');
            thread::sleep(Duration::from_millis(30));
        }
    }
    
    // Always right-click at the end (attack the target)
    if items_used > 0 || use_ult {
        info!("🎯 Auto-items complete ({} items), attacking", items_used);
    }
    mouse_click();
}
