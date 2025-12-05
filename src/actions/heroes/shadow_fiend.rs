use crate::actions::heroes::HeroScript;
use crate::actions::common::SurvivabilityActions;
use crate::config::Settings;
use crate::input::simulation::{press_key, mouse_click};
use crate::models::{GsiWebhookEvent, Hero};
use std::thread;
use std::time::Duration;
use tracing::info;

pub struct ShadowFiendScript {
    settings: Settings,
}

impl ShadowFiendScript {
    pub fn new(settings: Settings) -> Self {
        Self { settings }
    }

    /// Execute Q raze: right-click then configured key
    pub fn execute_q_raze(&self) {
        info!("Executing Q raze (right-click + {})", self.settings.heroes.shadow_fiend.q_ability_key);
        
        // Right-click to point SF in direction
        mouse_click();
        
        // Wait for direction to register
        thread::sleep(Duration::from_millis(self.settings.heroes.shadow_fiend.raze_delay_ms));
        
        // Press actual Q ability key
        press_key(self.settings.heroes.shadow_fiend.q_ability_key);
    }

    /// Execute W raze: right-click then configured key
    pub fn execute_w_raze(&self) {
        info!("Executing W raze (right-click + {})", self.settings.heroes.shadow_fiend.w_ability_key);
        
        // Right-click to point SF in direction
        mouse_click();
        
        // Wait for direction to register
        thread::sleep(Duration::from_millis(self.settings.heroes.shadow_fiend.raze_delay_ms));
        
        // Press actual W ability key
        press_key(self.settings.heroes.shadow_fiend.w_ability_key);
    }

    /// Execute E raze: right-click then configured key
    pub fn execute_e_raze(&self) {
        info!("Executing E raze (right-click + {})", self.settings.heroes.shadow_fiend.e_ability_key);
        
        // Right-click to point SF in direction
        mouse_click();
        
        // Wait for direction to register
        thread::sleep(Duration::from_millis(self.settings.heroes.shadow_fiend.raze_delay_ms));
        
        // Press actual E ability key
        press_key(self.settings.heroes.shadow_fiend.e_ability_key);
    }
}

impl HeroScript for ShadowFiendScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        // Use common survivability actions (danger detection, healing, defensive items)
        let survivability = SurvivabilityActions::new(self.settings.clone());
        crate::actions::danger_detector::update(event, &self.settings.danger_detection);
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
