use crate::config::Settings;
use crate::models::{GsiWebhookEvent, Item};
use lazy_static::lazy_static;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

lazy_static! {
    static ref ARMLET_LAST_TOGGLE: Mutex<Option<Instant>> = Mutex::new(None);
    static ref ARMLET_CRITICAL_HP: Mutex<Option<u32>> = Mutex::new(None);
}

/// Armlet toggle configuration
pub struct ArmletConfig {
    pub toggle_threshold: u32,
    pub predictive_offset: u32,
    pub toggle_cooldown_ms: u64,
}

/// Handle armlet toggling for any hero
pub fn armlet_toggle(event: &GsiWebhookEvent, settings: &Settings, config: &ArmletConfig) {
    // Find armlet position
    let armlet_slot = event
        .items
        .all_slots()
        .iter()
        .find(|(_, item)| item.name == "item_armlet")
        .map(|(slot, _)| *slot);

    let Some(armlet_slot) = armlet_slot else {
        return;
    };

    let Some(key) = settings.get_key_for_slot(armlet_slot) else {
        return;
    };

    if !event.hero.is_alive() {
        return;
    }

    let health = event.hero.health;
    let threshold = config.toggle_threshold;
    let offset = config.predictive_offset;
    let cooldown_ms = config.toggle_cooldown_ms;
    let trigger_point = threshold + offset;

    // Check for critical HP situation (armlet stayed on, hero not selected)
    if let Ok(mut critical_hp) = ARMLET_CRITICAL_HP.try_lock() {
        if let Some(last_critical) = *critical_hp {
            // If HP is still critically low after toggle attempt, force another toggle
            if health < threshold / 2 && health <= last_critical {
                warn!("Critical HP detected! HP: {} (likely armlet stuck on). Forcing emergency toggle.", health);
                
                // Force toggle regardless of cooldown
                crate::input::press_key(key);
                crate::input::press_key(key);
                
                // Reset critical HP tracker
                *critical_hp = None;
                
                // Update last toggle time
                if let Ok(mut last_toggle) = ARMLET_LAST_TOGGLE.try_lock() {
                    *last_toggle = Some(Instant::now());
                }
                return;
            }
        }
    }

    if health < trigger_point {
        // Check for stun FIRST before any cooldown logic
        if event.hero.is_stunned() {
            debug!("Hero stunned, skipping armlet toggle (HP: {})", health);
            return;
        }

        if let Ok(mut last_toggle) = ARMLET_LAST_TOGGLE.try_lock() {
            // Check if enough time has passed since last toggle
            let can_toggle = match *last_toggle {
                Some(last_time) => last_time.elapsed() >= Duration::from_millis(cooldown_ms),
                None => true,
            };

            if !can_toggle {
                debug!("Armlet toggle on cooldown ({}ms remaining)", 
                    cooldown_ms - last_toggle.unwrap().elapsed().as_millis() as u64);
                return;
            }

            info!("Triggering armlet toggle (HP: {} < trigger: {}, base: {})", health, trigger_point, threshold);

            // Double tap to toggle armlet off then on (no delay needed)
            crate::input::press_key(key);
            crate::input::press_key(key);

            // Update last toggle time
            *last_toggle = Some(Instant::now());
        }
    } else {
        // Reset critical HP tracker when HP is safe
        if let Ok(mut critical_hp) = ARMLET_CRITICAL_HP.try_lock() {
            if critical_hp.is_some() {
                debug!("HP recovered to safe levels, resetting critical HP tracker");
                *critical_hp = None;
            }
        }
    }
}

/// Find the keybinding for a specific item in the hero's inventory
pub fn find_item_slot(event: &GsiWebhookEvent, settings: &Settings, item: Item) -> Option<char> {
    find_item_slot_by_name(event, settings, item.to_game_name())
}

/// Find item slot key by item name string from GSI event (for backward compatibility)
pub fn find_item_slot_by_name(event: &GsiWebhookEvent, settings: &Settings, item_name: &str) -> Option<char> {
    let items = &event.items;
    
    // Check all inventory slots
    if items.slot0.name.contains(item_name) {
        return settings.get_key_for_slot("slot0");
    }
    if items.slot1.name.contains(item_name) {
        return settings.get_key_for_slot("slot1");
    }
    if items.slot2.name.contains(item_name) {
        return settings.get_key_for_slot("slot2");
    }
    if items.slot3.name.contains(item_name) {
        return settings.get_key_for_slot("slot3");
    }
    if items.slot4.name.contains(item_name) {
        return settings.get_key_for_slot("slot4");
    }
    if items.slot5.name.contains(item_name) {
        return settings.get_key_for_slot("slot5");
    }
    if items.neutral0.name.contains(item_name) {
        return settings.get_key_for_slot("neutral0");
    }
    
    None
}

/// Common survivability actions that apply to all heroes
pub struct SurvivabilityActions {
    settings: Settings,
}

// Ensure SurvivabilityActions can be shared across threads
unsafe impl Send for SurvivabilityActions {}
unsafe impl Sync for SurvivabilityActions {}

impl SurvivabilityActions {
    pub fn new(settings: Settings) -> Self {
        Self { settings }
    }

    /// Execute default GSI strategy (survivability + armlet)
    pub fn execute_default_strategy(&self, event: &GsiWebhookEvent) {
        // Always check survivability first
        self.check_and_use_healing_items(event);
        
        // Check for armlet and toggle if needed
        self.check_and_toggle_armlet(event);
    }

    /// Check if hero needs healing and use appropriate items
    pub fn check_and_use_healing_items(&self, event: &GsiWebhookEvent) {
        if !event.hero.is_alive() {
            return;
        }

        // Check if HP is below threshold
        if event.hero.health_percent >= self.settings.common.survivability_hp_threshold {
            return;
        }

        debug!(
            "HP below threshold: {}% < {}%",
            event.hero.health_percent, self.settings.common.survivability_hp_threshold
        );

        // Priority order for healing items
        let healing_items = vec![
            "item_cheese",
            "item_faerie_fire",
            "item_magic_wand",
            "item_enchanted_mango",
            "item_greater_faerie_fire",
        ];

        // Search for healing items in inventory
        for (slot, item) in event.items.all_slots() {
            if healing_items.contains(&item.name.as_str()) {
                // Check if item can be cast
                if let Some(can_cast) = item.can_cast {
                    if can_cast {
                        self.use_item(slot, &item.name);
                        return; // Only use one healing item per check
                    }
                }
            }
        }
    }

    fn use_item(&self, slot: &str, item_name: &str) {
        if let Some(key) = self.settings.get_key_for_slot(slot) {
            info!("Using {} in {} (key: {})", item_name, slot, key);
            // The actual key press will be handled by input simulation
            crate::input::press_key(key);
        }
    }

    /// Check and toggle armlet with default configuration
    fn check_and_toggle_armlet(&self, event: &GsiWebhookEvent) {
        // Check if hero has armlet in inventory
        let has_armlet = event.items.all_slots()
            .iter()
            .any(|(_, item)| item.name == "item_armlet");
        
        if !has_armlet {
            return;
        }
        
        // Use default armlet configuration (suitable for most strength heroes)
        let armlet_config = ArmletConfig {
            toggle_threshold: 320,      // HP threshold
            predictive_offset: 30,       // Predictive offset
            toggle_cooldown_ms: 250,     // Cooldown between toggles
        };
        
        armlet_toggle(event, &self.settings, &armlet_config);
    }
}
