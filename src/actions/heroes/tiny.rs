use crate::actions::heroes::traits::HeroScript;
use crate::actions::common::{find_item_slot, SurvivabilityActions};
use crate::actions::soul_ring::press_ability_with_soul_ring;
use crate::config::Settings;
use crate::input::simulation::press_key;
use crate::models::{GsiWebhookEvent, Hero, Item};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tracing::{info, warn};

lazy_static::lazy_static! {
    static ref LAST_GSI_EVENT: Mutex<Option<GsiWebhookEvent>> = Mutex::new(None);
}

pub struct TinyScript {
    settings: Arc<Mutex<Settings>>,
}

impl TinyScript {
    pub fn new(settings: Arc<Mutex<Settings>>) -> Self {
        Self { settings }
    }

    pub fn execute_combo(&self, event: &GsiWebhookEvent) {
        info!("Executing Tiny combo sequence...");

        let settings = self.settings.lock().unwrap();
        
        // 1. Blink Dagger
        if let Some(key) = find_item_slot(event, &settings, Item::Blink) {
            info!("Using Blink ({})", key);
            press_key(key);
            thread::sleep(Duration::from_millis(100));
        } else {
            warn!("Blink dagger not found in inventory");
        }
        
        // 2. Avalanche (W) - with Soul Ring on first press, then spam
        info!("Using Avalanche (W)");
        press_ability_with_soul_ring('w', &settings);
        for _ in 0..3 {
            thread::sleep(Duration::from_millis(30));
            press_key('w');
        }
        thread::sleep(Duration::from_millis(50));
        
        drop(settings); // Release settings lock after using it

        // 3. Toss (Q) - spam to ensure cast
        info!("Using Toss (Q)");
        for _ in 0..4 {
            press_key('q');
            thread::sleep(Duration::from_millis(30));
        }
        thread::sleep(Duration::from_millis(1400));

        // 4. Tree Grab (D) - Aghanim's ability
        info!("Using Tree Grab (D)");
        for _ in 0..3 {
            press_key('d');
            thread::sleep(Duration::from_millis(30));
        }

        info!("Tiny combo sequence complete.");
    }
}

impl HeroScript for TinyScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        // Store latest GSI event for standalone trigger
        if let Ok(mut last_event) = LAST_GSI_EVENT.try_lock() {
            *last_event = Some(event.clone());
        }
        
        // Use common survivability actions (danger detection, healing, defensive items)
        let survivability = SurvivabilityActions::new(self.settings.clone());
        let settings = self.settings.lock().unwrap();
        crate::actions::danger_detector::update(event, &settings.danger_detection);
        drop(settings);
        survivability.check_and_use_healing_items(event);
        survivability.use_defensive_items_if_danger(event);
        survivability.use_neutral_item_if_danger(event);
    }

    fn handle_standalone_trigger(&self) {
        if let Ok(last_event) = LAST_GSI_EVENT.try_lock() {
            if let Some(event) = last_event.as_ref() {
                self.execute_combo(event);
            } else {
                warn!("No GSI event received yet - Tiny combo needs item data");
            }
        }
    }

    fn hero_name(&self) -> &'static str {
        Hero::Tiny.to_game_name()
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
