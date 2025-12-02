use crate::actions::common::SurvivabilityActions;
use crate::actions::heroes::{HeroScript, HuskarScript, LegionCommanderScript, ShadowFiendScript, TinyScript};
use crate::config::Settings;
use crate::models::GsiWebhookEvent;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;

pub struct ActionDispatcher {
    pub hero_scripts: HashMap<String, Arc<dyn HeroScript>>,
    survivability: SurvivabilityActions,
}

// Ensure ActionDispatcher can be shared across threads
unsafe impl Send for ActionDispatcher {}
unsafe impl Sync for ActionDispatcher {}

impl ActionDispatcher {
    pub fn new(settings: Settings) -> Self {
        let mut hero_scripts: HashMap<String, Arc<dyn HeroScript>> = HashMap::new();

        // Register hero scripts
        let huskar = Arc::new(HuskarScript::new(settings.clone()));
        hero_scripts.insert(huskar.hero_name().to_string(), huskar);

        let legion = Arc::new(LegionCommanderScript::new(settings.clone()));
        hero_scripts.insert(legion.hero_name().to_string(), legion);

        let shadow_fiend = Arc::new(ShadowFiendScript::new(settings.clone()));
        hero_scripts.insert(shadow_fiend.hero_name().to_string(), shadow_fiend);

        let tiny = Arc::new(TinyScript::new(settings.clone()));
        hero_scripts.insert(tiny.hero_name().to_string(), tiny);

        Self {
            hero_scripts,
            survivability: SurvivabilityActions::new(settings),
        }
    }

    pub fn dispatch_gsi_event(&self, event: &GsiWebhookEvent) {
        // Check if hero has a custom handler
        if let Some(hero_script) = self.hero_scripts.get(&event.hero.name) {
            // Hero has custom handler, use it
            debug!("Dispatching GSI event to {}", event.hero.name);
            hero_script.handle_gsi_event(event);
        } else {
            // No custom handler, use default strategy (survivability + armlet)
            debug!("No custom handler for {}, using default strategy", event.hero.name);
            self.survivability.execute_default_strategy(event);
        }
    }

    pub fn dispatch_standalone_trigger(&self, hero_name: &str) {
        if let Some(hero_script) = self.hero_scripts.get(hero_name) {
            debug!("Dispatching standalone trigger to {}", hero_name);
            hero_script.handle_standalone_trigger();
        }
    }
}
