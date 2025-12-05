use crate::actions::heroes::HeroScript;
use crate::actions::common::{find_item_slot, SurvivabilityActions};
use crate::config::Settings;
use crate::input::simulation::press_key;
use crate::models::{GsiWebhookEvent, Hero, Item};
use std::any::Any;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tracing::info;

pub struct LegionCommanderScript {
    settings: Settings,
    last_event: Arc<Mutex<Option<GsiWebhookEvent>>>,
}

impl LegionCommanderScript {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            last_event: Arc::new(Mutex::new(None)),
        }
    }

    pub fn execute_combo(&self) {
        info!("Executing Legion Commander combo sequence...");
        
        let event = self.last_event.lock().unwrap();
        if event.is_none() {
            info!("No GSI event available, cannot determine item slots");
            return;
        }
        
        let event = event.as_ref().unwrap();
        
        // 1. Soul Ring (always first if present)
        if let Some(key) = find_item_slot(event, &self.settings, Item::SoulRing) {
            info!("Using Soul Ring ({})", key);
            press_key(key);
            thread::sleep(Duration::from_millis(50));
        }
        
        // 2. Press The Attack (W) - double tap
        info!("Using Press The Attack (W)");
        press_key('w');
        thread::sleep(Duration::from_millis(30));
        press_key('w');
        thread::sleep(Duration::from_millis(220));
        
        // 3. Blade Mail (if present) - double tap
        if let Some(key) = find_item_slot(event, &self.settings, Item::BladeMail) {
            info!("Using Blade Mail ({})", key);
            press_key(key);
            thread::sleep(Duration::from_millis(30));
            press_key(key);
            thread::sleep(Duration::from_millis(50));
        }
        
        // 4. Mjollnir (if present) - double tap
        if let Some(key) = find_item_slot(event, &self.settings, Item::Mjollnir) {
            info!("Using Mjollnir ({})", key);
            press_key(key);
            thread::sleep(Duration::from_millis(30));
            press_key(key);
            thread::sleep(Duration::from_millis(50));
        }
        
        // 5. BKB (if present) - double tap
        if let Some(key) = find_item_slot(event, &self.settings, Item::BlackKingBar) {
            info!("Using BKB ({})", key);
            press_key(key);
            thread::sleep(Duration::from_millis(30));
            press_key(key);
            thread::sleep(Duration::from_millis(50));
        }
        
        // 6. Blink (single tap)
        if let Some(key) = find_item_slot(event, &self.settings, Item::Blink) {
            info!("Using Blink ({})", key);
            press_key(key);
            thread::sleep(Duration::from_millis(100));
        }
        
        // 7. Orchid or Bloodthorn (spam 3-4 times to remove linkens)
        if let Some(key) = find_item_slot(event, &self.settings, Item::Orchid)
            .or_else(|| find_item_slot(event, &self.settings, Item::Bloodthorn))
        {
            info!("Using Orchid/Bloodthorn ({}) - spam for linkens", key);
            for _ in 0..10 {
                press_key(key);
                thread::sleep(Duration::from_millis(30));
            }
            thread::sleep(Duration::from_millis(50));
        }
        
        // 8. Duel (R) - spam to ensure cast
        info!("Using Duel (R)");
        for _ in 0..6 {
            press_key('r');
            thread::sleep(Duration::from_millis(50));
        }
        
        // 9. Overwhelming Odds (Q) - spam after duel
        info!("Using Overwhelming Odds (Q)");
        for _ in 0..6 {
            press_key('q');
            thread::sleep(Duration::from_millis(50));
        }
        
        info!("Legion Commander combo complete");
    }
}

impl HeroScript for LegionCommanderScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        // Store the latest event for combo execution
        *self.last_event.lock().unwrap() = Some(event.clone());
        
        // Use common survivability actions (danger detection, healing, defensive items)
        let survivability = SurvivabilityActions::new(self.settings.clone());
        crate::actions::danger_detector::update(event, &self.settings.danger_detection);
        survivability.check_and_use_healing_items(event);
        survivability.use_defensive_items_if_danger(event);
    }

    fn handle_standalone_trigger(&self) {
        self.execute_combo();
    }

    fn hero_name(&self) -> &'static str {
        Hero::LegionCommander.to_game_name()
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
}
