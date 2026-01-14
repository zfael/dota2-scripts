//! Broodmother hero script
//!
//! Features:
//! - Spider micro: Mouse5 triggers select spiders → A-click → reselect hero

use crate::actions::heroes::traits::HeroScript;
use crate::config::Settings;
use crate::input::keyboard::{parse_key_string, simulate_key};
use crate::input::simulation::left_click;
use crate::models::{GsiWebhookEvent, Hero};
use lazy_static::lazy_static;
use rdev::Key;
use std::sync::atomic::{AtomicBool, Ordering};
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
}

impl BroodmotherScript {
    pub fn new(settings: Arc<Mutex<Settings>>) -> Self {
        Self { settings }
    }

    /// Execute spider attack-move macro
    /// Sequence: Select spiders (F2) → Attack command (A) → Left click → Reselect hero (F1)
    pub fn execute_spider_attack_move(settings: &Settings) {
        let config = &settings.heroes.broodmother;
        
        if !config.spider_micro_enabled {
            return;
        }

        info!("🕷️ Broodmother: Executing spider attack-move");

        // Parse control group key (e.g., "F2")
        let spider_key = parse_key_string(&config.spider_control_group_key);
        let hero_key = parse_key_string(&config.reselect_hero_key);
        let attack_key = crate::input::keyboard::char_to_key(config.attack_key);

        // Select spiderlings
        if let Some(key) = spider_key {
            simulate_key(key);
            thread::sleep(Duration::from_millis(30));
        }

        // Press attack command
        if let Some(key) = attack_key {
            simulate_key(key);
            thread::sleep(Duration::from_millis(20));
        }

        // Left click at current mouse position
        left_click();
        thread::sleep(Duration::from_millis(30));

        // Reselect hero
        if let Some(key) = hero_key {
            simulate_key(key);
        }

        info!("🕷️ Broodmother: Spider attack-move complete");
    }
}

impl HeroScript for BroodmotherScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        // Update active state based on hero
        let is_broodmother = event.hero.name == Hero::Broodmother.to_game_name();
        BROODMOTHER_ACTIVE.store(is_broodmother, Ordering::SeqCst);
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
