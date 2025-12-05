use crate::actions::heroes::HeroScript;
use crate::actions::common::SurvivabilityActions;
use crate::config::Settings;
use crate::input::simulation::{press_key, mouse_click};
use crate::models::{GsiWebhookEvent, Hero};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tracing::info;

pub struct ShadowFiendScript {
    settings: Arc<Mutex<Settings>>,
}

impl ShadowFiendScript {
    pub fn new(settings: Arc<Mutex<Settings>>) -> Self {
        Self { settings }
    }

    /// Execute Q raze: right-click then configured key
    pub fn execute_q_raze(&self) {
        let settings = self.settings.lock().unwrap();
        info!("Executing Q raze (right-click + {})", settings.heroes.shadow_fiend.q_ability_key);
        
        let q_key = settings.heroes.shadow_fiend.q_ability_key;
        let delay_ms = settings.heroes.shadow_fiend.raze_delay_ms;
        drop(settings);
        
        // Right-click to point SF in direction
        mouse_click();
        
        // Wait for direction to register
        thread::sleep(Duration::from_millis(delay_ms));
        
        // Press actual Q ability key
        press_key(q_key);
    }

    /// Execute W raze: right-click then configured key
    pub fn execute_w_raze(&self) {
        let settings = self.settings.lock().unwrap();
        info!("Executing W raze (right-click + {})", settings.heroes.shadow_fiend.w_ability_key);
        
        let w_key = settings.heroes.shadow_fiend.w_ability_key;
        let delay_ms = settings.heroes.shadow_fiend.raze_delay_ms;
        drop(settings);
        
        // Right-click to point SF in direction
        mouse_click();
        
        // Wait for direction to register
        thread::sleep(Duration::from_millis(delay_ms));
        
        // Press actual W ability key
        press_key(w_key);
    }

    /// Execute E raze: right-click then configured key
    pub fn execute_e_raze(&self) {
        let settings = self.settings.lock().unwrap();
        info!("Executing E raze (right-click + {})", settings.heroes.shadow_fiend.e_ability_key);
        
        let e_key = settings.heroes.shadow_fiend.e_ability_key;
        let delay_ms = settings.heroes.shadow_fiend.raze_delay_ms;
        drop(settings);
        
        // Right-click to point SF in direction
        mouse_click();
        
        // Wait for direction to register
        thread::sleep(Duration::from_millis(delay_ms));
        
        // Press actual E ability key
        press_key(e_key);
    }
}

impl HeroScript for ShadowFiendScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        // Use common survivability actions (danger detection, healing, defensive items)
        let survivability = SurvivabilityActions::new(self.settings.clone());
        let settings = self.settings.lock().unwrap();
        crate::actions::danger_detector::update(event, &settings.danger_detection);
        drop(settings);
        survivability.check_and_use_healing_items(event);
        survivability.use_defensive_items_if_danger(event);
    }

    fn handle_standalone_trigger(&self) {
        // Standalone trigger not used for SF
        // Individual Q/W/E keys are intercepted instead
        info!("Shadow Fiend uses Q/W/E interception, not standalone combo trigger");
    }
    
    fn hero_name(&self) -> &'static str {
        Hero::Nevermore.to_game_name()
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
