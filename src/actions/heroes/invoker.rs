use crate::actions::common::SurvivabilityActions;
use crate::actions::executor::ActionExecutor;
use crate::actions::heroes::traits::HeroScript;
use crate::config::Settings;
use crate::models::{GsiWebhookEvent, Hero};
use std::sync::{Arc, Mutex};
use tracing::info;

pub struct InvokerScript {
    settings: Arc<Mutex<Settings>>,
    executor: Arc<ActionExecutor>,
}

impl InvokerScript {
    pub fn new(settings: Arc<Mutex<Settings>>, executor: Arc<ActionExecutor>) -> Self {
        Self { settings, executor }
    }
}

impl HeroScript for InvokerScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        let survivability = SurvivabilityActions::new(self.settings.clone(), self.executor.clone());
        let settings = self.settings.lock().unwrap();
        let in_danger = crate::actions::danger_detector::update(event, &settings.danger_detection);
        drop(settings);
        survivability.check_and_use_healing_items_with_danger(event, in_danger);
        survivability.use_defensive_items_if_danger_with_snapshot(event, in_danger);
        survivability.use_neutral_item_if_danger_with_snapshot(event, in_danger);
    }

    fn handle_standalone_trigger(&self) {
        info!("Invoker standalone combo requested before combo planner is implemented");
    }

    fn hero_name(&self) -> &'static str {
        Hero::Invoker.to_game_name()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
