use crate::actions::common::SurvivabilityActions;
use crate::actions::heroes::HeroScript;
use crate::config::Settings;
use crate::input::simulation::press_key;
use crate::models::{GsiWebhookEvent, Hero};
use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tracing::info;

lazy_static! {
    /// Shared state for Shadow Fiend to allow keyboard.rs to access last GSI event
    pub static ref SF_LAST_EVENT: Arc<Mutex<Option<GsiWebhookEvent>>> = Arc::new(Mutex::new(None));
}

/// Shadow Fiend raze execution helper
pub struct ShadowFiendState;

impl ShadowFiendState {
    /// Execute a raze with ALT hold for direction facing
    /// This spawns a thread to handle the timing-sensitive sequence
    pub fn execute_raze(raze_key: char, settings: &Settings) {
        let delay_ms = settings.heroes.shadow_fiend.raze_delay_ms;
        
        // Spawn raze execution in separate thread
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            
            // Hold ALT (for cl_dota_alt_unit_movetodirection)
            crate::input::simulation::alt_down();
            
            // Right-click to face direction
            crate::input::simulation::mouse_click();
            
            // Small delay then release ALT
            thread::sleep(Duration::from_millis(50));
            crate::input::simulation::alt_up();
            
            // Wait for direction to register
            thread::sleep(Duration::from_millis(delay_ms));
            
            // Press the raze key
            crate::input::simulation::press_key(raze_key);
        });
    }

    /// Execute ultimate with optional BKB and D
    /// Sequence: BKB (if enabled & available) → D (if enabled) → R
    pub fn execute_ultimate_combo(settings: &Settings) {
        let sf_config = &settings.heroes.shadow_fiend;
        
        // If auto_bkb_on_ultimate is disabled, just press R directly
        if !sf_config.auto_bkb_on_ultimate {
            info!("👻 SF Ultimate: auto_bkb disabled, pressing R directly");
            press_key('r');
            return;
        }
        
        // Spawn in thread to handle timing-sensitive sequence
        let auto_bkb = sf_config.auto_bkb_on_ultimate;
        let auto_d = sf_config.auto_d_on_ultimate;
        
        thread::spawn(move || {
            // Get last GSI event to check for BKB
            let event_guard = SF_LAST_EVENT.lock().unwrap();
            
            if auto_bkb {
                if let Some(event) = event_guard.as_ref() {
                    // Need to get settings again inside thread for item lookup
                    // Check for BKB in inventory
                    let bkb_slot = event.items.all_slots().iter()
                        .find(|(_, item)| item.name.contains("black_king_bar") && item.can_cast == Some(true))
                        .map(|(slot, _)| *slot);
                    
                    if let Some(slot) = bkb_slot {
                        // Map slot to key (simplified - use hardcoded mapping based on common keybindings)
                        let key = match slot {
                            "slot0" => Some('z'),
                            "slot1" => Some('x'),
                            "slot2" => Some('c'),
                            "slot3" => Some('v'),
                            "slot4" => Some('b'),
                            "slot5" => Some('n'),
                            _ => None,
                        };
                        
                        if let Some(bkb_key) = key {
                            info!("👻 SF Ultimate: Using BKB ({}) before Requiem", bkb_key);
                            // Double-tap for self-cast
                            press_key(bkb_key);
                            thread::sleep(Duration::from_millis(30));
                            press_key(bkb_key);
                            thread::sleep(Duration::from_millis(50));
                        }
                    } else {
                        info!("👻 SF Ultimate: BKB not found or on cooldown");
                    }
                } else {
                    info!("👻 SF Ultimate: No GSI event available, skipping BKB");
                }
            }
            
            drop(event_guard); // Release lock
            
            // Press D if enabled
            if auto_d {
                info!("👻 SF Ultimate: Using D ability");
                press_key('d');
                thread::sleep(Duration::from_millis(50));
            }
            
            // Press R for ultimate
            info!("👻 SF Ultimate: Casting Requiem of Souls (R)");
            press_key('r');
        });
    }

    /// Execute standalone combo: Blink + Ultimate (with BKB/D if configured)
    /// Only executes if Blink AND Ultimate are available (not on cooldown)
    pub fn execute_standalone_combo(settings: &Settings) {
        let sf_config = &settings.heroes.shadow_fiend;
        let auto_bkb = sf_config.auto_bkb_on_ultimate;
        let auto_d = sf_config.auto_d_on_ultimate;
        
        thread::spawn(move || {
            // Get last GSI event to check for Blink and Ultimate
            let event_guard = SF_LAST_EVENT.lock().unwrap();
            
            if let Some(event) = event_guard.as_ref() {
                // Check if Ultimate (R) is ready - ability5 is usually the ultimate
                // Shadow Fiend's ultimate is "nevermore_requiem"
                let ult_ready = event.abilities.ability5.can_cast;
                if !ult_ready {
                    info!("👻 SF Standalone: Ultimate on cooldown, skipping combo");
                    return;
                }
                
                // Check for Blink Dagger in inventory (must be off cooldown)
                let blink_slot = event.items.all_slots().iter()
                    .find(|(_, item)| item.name.contains("blink") && item.can_cast == Some(true))
                    .map(|(slot, _)| *slot);
                
                if blink_slot.is_none() {
                    info!("👻 SF Standalone: Blink not found or on cooldown, skipping combo");
                    return;
                }
                
                let blink_key = blink_slot.and_then(|slot| match slot {
                    "slot0" => Some('z'),
                    "slot1" => Some('x'),
                    "slot2" => Some('c'),
                    "slot3" => Some('v'),
                    "slot4" => Some('b'),
                    "slot5" => Some('n'),
                    _ => None,
                });
                
                // Check for BKB if enabled
                let bkb_key = if auto_bkb {
                    event.items.all_slots().iter()
                        .find(|(_, item)| item.name.contains("black_king_bar") && item.can_cast == Some(true))
                        .and_then(|(slot, _)| match *slot {
                            "slot0" => Some('z'),
                            "slot1" => Some('x'),
                            "slot2" => Some('c'),
                            "slot3" => Some('v'),
                            "slot4" => Some('b'),
                            "slot5" => Some('n'),
                            _ => None,
                        })
                } else {
                    None
                };
                
                drop(event_guard); // Release lock before sleeping
                
                // Execute combo: Blink → BKB (if available) → D (if enabled) → R
                if let Some(key) = blink_key {
                    info!("👻 SF Standalone: Using Blink ({})", key);
                    press_key(key);
                    thread::sleep(Duration::from_millis(50));
                }
                
                // Use BKB if available
                if let Some(key) = bkb_key {
                    info!("👻 SF Standalone: Using BKB ({})", key);
                    press_key(key);
                    thread::sleep(Duration::from_millis(30));
                    press_key(key); // Double-tap for self-cast
                    thread::sleep(Duration::from_millis(50));
                }
                
                // Press D if enabled
                if auto_d {
                    info!("👻 SF Standalone: Using D ability");
                    press_key('d');
                    thread::sleep(Duration::from_millis(50));
                }
                
                // Press R for ultimate
                info!("👻 SF Standalone: Casting Requiem of Souls (R)");
                press_key('r');
            } else {
                info!("👻 SF Standalone: No GSI event available, cannot check Blink");
            }
        });
    }
}

/// Shadow Fiend script
/// 
/// Raze interception flow:
/// 1. keyboard.rs intercepts Q/W/E when SF is enabled (via app_state.sf_enabled)
/// 2. Calls ShadowFiendState::execute_raze() 
/// 3. execute_raze spawns thread that:
///    - Holds ALT (for cl_dota_alt_unit_movetodirection)
///    - Right-clicks to face direction
///    - Releases ALT, waits for direction to register
///    - Presses the raze key
/// 
/// Auto-BKB on Ultimate flow:
/// 1. keyboard.rs intercepts R when SF is enabled and auto_bkb_on_ultimate is enabled
/// 2. Calls ShadowFiendState::execute_ultimate_combo()
/// 3. execute_ultimate_combo spawns thread that:
///    - Checks for BKB in inventory (from SF_LAST_EVENT)
///    - If BKB available and can_cast: double-tap BKB key
///    - If auto_d_on_ultimate enabled: press D
///    - Press R for Requiem of Souls
pub struct ShadowFiendScript {
    settings: Arc<Mutex<Settings>>,
}

impl ShadowFiendScript {
    pub fn new(settings: Arc<Mutex<Settings>>) -> Self {
        Self { settings }
    }
}

impl HeroScript for ShadowFiendScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        let settings = self.settings.lock().unwrap();
        
        // Store last event for ultimate combo (BKB lookup)
        {
            let mut last_event = SF_LAST_EVENT.lock().unwrap();
            *last_event = Some(event.clone());
        }
        
        // Use common survivability actions (danger detection, healing, defensive items)
        let survivability = SurvivabilityActions::new(self.settings.clone());
        crate::actions::danger_detector::update(event, &settings.danger_detection);
        drop(settings);
        survivability.check_and_use_healing_items(event);
        survivability.use_defensive_items_if_danger(event);
        survivability.use_neutral_item_if_danger(event);
    }

    fn handle_standalone_trigger(&self) {
        info!("👻 Shadow Fiend standalone combo triggered");
        let settings = self.settings.lock().unwrap();
        ShadowFiendState::execute_standalone_combo(&settings);
    }

    fn hero_name(&self) -> &'static str {
        Hero::Nevermore.to_game_name()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
