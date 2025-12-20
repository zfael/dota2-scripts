use crate::actions::common::SurvivabilityActions;
use crate::actions::heroes::HeroScript;
use crate::config::Settings;
use crate::models::{GsiWebhookEvent, Hero};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tracing::info;

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
        
        // Use common survivability actions (danger detection, healing, defensive items)
        let survivability = SurvivabilityActions::new(self.settings.clone());
        crate::actions::danger_detector::update(event, &settings.danger_detection);
        drop(settings);
        survivability.check_and_use_healing_items(event);
        survivability.use_defensive_items_if_danger(event);
        survivability.use_neutral_item_if_danger(event);
    }

    fn handle_standalone_trigger(&self) {
        // Standalone trigger not used for SF - uses Q/W/E interception instead
        info!("Shadow Fiend uses Q/W/E interception, not standalone combo trigger");
    }

    fn hero_name(&self) -> &'static str {
        Hero::Nevermore.to_game_name()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
