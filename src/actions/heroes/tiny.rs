use crate::actions::heroes::traits::HeroScript;
use crate::actions::common::{find_item_slot, SurvivabilityActions};
use crate::config::Settings;
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
        info!("Triggering Tiny combo sequence...");

        let settings = self.settings.lock().unwrap();
        
        // Find soul ring (x) dynamically
        if let Some(soul_ring_key) = find_item_slot(event, &settings, Item::SoulRing) {
            crate::input::press_key(soul_ring_key);
        } else {
            warn!("Soul ring not found in inventory");
        }

        // Find blink (z) dynamically
        if let Some(blink_key) = find_item_slot(event, &settings, Item::Blink) {
            crate::input::press_key(blink_key);
        } else {
            warn!("Blink dagger not found in inventory");
        }
        
        drop(settings);
        thread::sleep(Duration::from_millis(200));

        // w (3 times with delays) - Avalanche
        crate::input::press_key('w');
        thread::sleep(Duration::from_millis(30));
        crate::input::press_key('w');
        crate::input::press_key('w');
        thread::sleep(Duration::from_millis(100));

        // q (3 times with delays) - Toss
        crate::input::press_key('q');
        thread::sleep(Duration::from_millis(30));
        crate::input::press_key('q');
        crate::input::press_key('q');
        thread::sleep(Duration::from_millis(1400));

        // d (aghanim's) - Tree Grab
        crate::input::press_key('d');

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
