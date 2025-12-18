use crate::config::Settings;
use crate::models::{GsiWebhookEvent, Item};
use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};
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
    settings: Arc<Mutex<Settings>>,
}

// Ensure SurvivabilityActions can be shared across threads
unsafe impl Send for SurvivabilityActions {}
unsafe impl Sync for SurvivabilityActions {}

impl SurvivabilityActions {
    pub fn new(settings: Arc<Mutex<Settings>>) -> Self {
        Self { settings }
    }

    /// Execute default GSI strategy (survivability + armlet + danger detection)
    pub fn execute_default_strategy(&self, event: &GsiWebhookEvent) {
        // PRIORITY 1: Check for armlet and toggle if needed (non-blocking)
        // Clone event for thread safety
        let event_clone = event.clone();
        let settings_clone = self.settings.clone();
        std::thread::spawn(move || {
            let settings = settings_clone.lock().unwrap();
            
            // Check if hero has armlet in inventory
            let has_armlet = event_clone.items.all_slots()
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
            
            armlet_toggle(&event_clone, &settings, &armlet_config);
        });
        
        // PRIORITY 2: Update danger detection state
        {
            let settings = self.settings.lock().unwrap();
            let _in_danger = crate::actions::danger_detector::update(event, &settings.danger_detection);
        }
        
        // PRIORITY 3: Always check survivability first
        self.check_and_use_healing_items(event);
        
        // PRIORITY 4: Use defensive items if in danger
        self.use_defensive_items_if_danger(event);
    }

    /// Check if hero needs healing and use appropriate items
    pub fn check_and_use_healing_items(&self, event: &GsiWebhookEvent) {
        if !event.hero.is_alive() {
            return;
        }

        // Determine threshold based on danger state
        let in_danger = crate::actions::danger_detector::is_in_danger();
        let settings = self.settings.lock().unwrap();
        let threshold = if in_danger && settings.danger_detection.enabled {
            settings.danger_detection.healing_threshold_in_danger
        } else {
            settings.common.survivability_hp_threshold
        };

        // Check if HP is below threshold
        if event.hero.health_percent >= threshold {
            return;
        }

        debug!(
            "HP below threshold: {}% < {}% (in_danger: {})",
            event.hero.health_percent, threshold, in_danger
        );

        // Priority order - high value first when in danger, low value first otherwise
        let healing_items = if in_danger {
            vec![
                ("item_cheese", 2000u32),
                ("item_greater_faerie_fire", 350u32),
                ("item_enchanted_mango", 175u32),
                ("item_magic_wand", 100u32), // Approximate (15 per charge)
                ("item_faerie_fire", 85u32),
            ]
        } else {
            vec![
                ("item_cheese", 2000u32),
                ("item_faerie_fire", 85u32),
                ("item_magic_wand", 100u32),
                ("item_enchanted_mango", 175u32),
                ("item_greater_faerie_fire", 350u32),
            ]
        };

        let max_items = if in_danger && settings.danger_detection.enabled {
            settings.danger_detection.max_healing_items_per_danger
        } else {
            1 // Normal mode: only one item
        };
        drop(settings); // Release lock

        let mut items_used = 0u32;

        // Search for healing items in inventory
        for (item_name, _heal_amount) in healing_items {
            if items_used >= max_items {
                break;
            }

            for (slot, item) in event.items.all_slots() {
                if item.name == item_name {
                    // Check if item can be cast
                    if let Some(can_cast) = item.can_cast {
                        if can_cast {
                            self.use_item(slot, &item.name);
                            items_used += 1;
                            break; // Move to next item type
                        }
                    }
                }
            }
        }
    }

    fn use_item(&self, slot: &str, item_name: &str) {
        let settings = self.settings.lock().unwrap();
        if let Some(key) = settings.get_key_for_slot(slot) {
            info!("Using {} in {} (key: {})", item_name, slot, key);
            
            // Items like Glimmer Cape need double-tap for self-cast
            let needs_double_tap = matches!(item_name, "item_glimmer_cape");
            
            crate::input::press_key(key);
            
            if needs_double_tap {
                // Small delay between presses for self-cast
                std::thread::sleep(std::time::Duration::from_millis(50));
                crate::input::press_key(key);
                info!("Double-tapped {} for self-cast", item_name);
            }
        }
    }

    /// Use defensive items when in danger
    pub fn use_defensive_items_if_danger(&self, event: &GsiWebhookEvent) {
        // Check danger state and gather config - release lock before item usage
        let (enabled, satanic_threshold, defensive_items_config) = {
            let settings = self.settings.lock().unwrap();
            let current_config = &settings.danger_detection;
            
            if !current_config.enabled {
                return;
            }

            if !crate::actions::danger_detector::is_in_danger() {
                return;
            }

            if !event.hero.is_alive() {
                return;
            }

            debug!("In danger - checking defensive items");

            // Gather config before releasing lock
            let defensive_items = vec![
                ("item_black_king_bar", current_config.auto_bkb),
                ("item_satanic", current_config.auto_satanic),
                ("item_blade_mail", current_config.auto_blade_mail),
                ("item_glimmer_cape", current_config.auto_glimmer_cape),
                ("item_ghost", current_config.auto_ghost_scepter),
                ("item_shivas_guard", current_config.auto_shivas_guard),
            ];
            
            (true, current_config.satanic_hp_threshold, defensive_items)
        }; // Lock released here

        // Try to activate all enabled items that are ready
        for (item_name, enabled) in defensive_items_config {
            if !enabled {
                continue;
            }

            // Satanic has its own HP threshold check
            if item_name == "item_satanic" {
                let hp_percent = (event.hero.health * 100) / event.hero.max_health;
                if hp_percent > satanic_threshold {
                    debug!("Satanic not used: HP {}% > threshold {}%", hp_percent, satanic_threshold);
                    continue;
                }
            }

            for (slot, item) in event.items.all_slots() {
                if item.name == item_name {
                    // Check if item can be cast (not on cooldown)
                    if let Some(can_cast) = item.can_cast {
                        if can_cast {
                            debug!("Activating defensive item: {}", item_name);
                            self.use_item(slot, &item.name);
                            break; // Move to next item type
                        }
                    }
                }
            }
        }
    }

    /// Check and toggle armlet with default configuration
    fn check_and_toggle_armlet(&self, event: &GsiWebhookEvent) {
        let settings = self.settings.lock().unwrap();
        
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
        
        armlet_toggle(event, &settings, &armlet_config);
    }
}
