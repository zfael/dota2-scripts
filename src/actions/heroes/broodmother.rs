//! Broodmother hero script
//!
//! Features:
//! - Spider micro: Mouse5 triggers select spiders → A-click → reselect hero
//! - Survivability: Auto-use healing items (Wand, Faerie Fire, Satanic, etc.)
//! - Danger detection: Trigger defensive items when enemy abilities detected

use crate::actions::common::SurvivabilityActions;
use crate::actions::executor::ActionExecutor;
use crate::actions::heroes::traits::HeroScript;
use crate::config::Settings;
use crate::input::keyboard::{parse_key_string, simulate_key};
use crate::models::{GsiWebhookEvent, Hero};
use lazy_static::lazy_static;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tracing::info;

lazy_static! {
    /// Track if Broodmother is the current hero (for Mouse5 interception)
    pub static ref BROODMOTHER_ACTIVE: AtomicBool = AtomicBool::new(false);
}

pub struct BroodmotherScript {
    settings: Arc<Mutex<Settings>>,
    executor: Arc<ActionExecutor>,
}

impl BroodmotherScript {
    pub fn new(settings: Arc<Mutex<Settings>>, executor: Arc<ActionExecutor>) -> Self {
        Self { settings, executor }
    }

    /// Execute spider move macro
    /// Sequence: Select spiders (F2) → Right click → Reselect hero (F1)
    pub fn execute_spider_attack_move(settings: &Settings) {
        let config = &settings.heroes.broodmother;
        
        if !config.spider_micro_enabled {
            return;
        }

        info!("🕷️ Broodmother: Executing spider move");

        // Parse control group key (e.g., "F2")
        let spider_key = parse_key_string(&config.spider_control_group_key);
        let hero_key = parse_key_string(&config.reselect_hero_key);

        // Select spiderlings
        if let Some(key) = spider_key {
            simulate_key(key);
            thread::sleep(Duration::from_millis(30));
        }

        // Right click at current mouse position
        crate::input::simulation::mouse_click();
        thread::sleep(Duration::from_millis(30));

        // Reselect hero
        if let Some(key) = hero_key {
            simulate_key(key);
        }

        info!("🕷️ Broodmother: Spider move complete");
    }
}

impl HeroScript for BroodmotherScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        // BROODMOTHER_ACTIVE is updated by dispatcher for all GSI events
        // This handler is only called when playing Broodmother
        
        // Use common survivability actions (danger detection, healing, defensive items)
        let settings = self.settings.lock().unwrap();
        let survivability = SurvivabilityActions::new(self.settings.clone(), self.executor.clone());
        crate::actions::danger_detector::update(event, &settings.danger_detection);
        drop(settings);
        survivability.check_and_use_healing_items(event);
        survivability.use_defensive_items_if_danger(event);
        survivability.use_neutral_item_if_danger(event);
    }

    fn handle_standalone_trigger(&self) {
        let settings = self.settings.lock().unwrap().clone();
        Self::execute_spider_attack_move(&settings);
    }

    fn hero_name(&self) -> &'static str {
        Hero::Broodmother.to_game_name()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
