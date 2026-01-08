use crate::actions::common::SurvivabilityActions;
use crate::actions::heroes::{HeroScript, HuskarScript, LargoScript, LegionCommanderScript, ShadowFiendScript, TinyScript};
use crate::actions::soul_ring;
use crate::actions::bottle_optimization;
use crate::config::Settings;
use crate::models::GsiWebhookEvent;
use lazy_static::lazy_static;
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::sync::{Arc, Mutex};
use tracing::{debug, warn};

lazy_static! {
    static ref DISCOVERED_NEUTRAL_ITEMS: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
}

fn log_neutral_item_discovery(event: &GsiWebhookEvent, settings: &Settings) {
    // Skip if logging is disabled
    if !settings.neutral_items.log_discoveries {
        return;
    }

    let neutral_item = &event.items.neutral0;
    
    // Skip empty slots
    if neutral_item.name == "empty" {
        return;
    }

    // Check if we've already logged this item
    let mut discovered = DISCOVERED_NEUTRAL_ITEMS.lock().unwrap();
    if discovered.contains(&neutral_item.name) {
        return;
    }

    // Add to discovered set
    discovered.insert(neutral_item.name.clone());

    // Create logs directory if it doesn't exist
    if let Err(e) = fs::create_dir_all("logs") {
        warn!("Failed to create logs directory: {}", e);
        return;
    }

    // Append to log file
    let log_path = "logs/neutral_items_discovered.txt";
    match OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
    {
        Ok(mut file) => {
            let can_cast = neutral_item.can_cast.unwrap_or(false);
            let passive = neutral_item.passive.unwrap_or(false);
            let cooldown = neutral_item.cooldown.unwrap_or(0);
            
            let log_entry = format!(
                "{} | can_cast: {} | passive: {} | cooldown: {}\n",
                neutral_item.name, can_cast, passive, cooldown
            );
            
            if let Err(e) = file.write_all(log_entry.as_bytes()) {
                warn!("Failed to write to neutral items log: {}", e);
            } else {
                debug!("Discovered neutral item: {}", neutral_item.name);
            }
        }
        Err(e) => {
            warn!("Failed to open neutral items log file: {}", e);
        }
    }
}

pub struct ActionDispatcher {
    pub hero_scripts: HashMap<String, Arc<dyn HeroScript>>,
    survivability: SurvivabilityActions,
}

// Ensure ActionDispatcher can be shared across threads
unsafe impl Send for ActionDispatcher {}
unsafe impl Sync for ActionDispatcher {}

impl ActionDispatcher {
    pub fn new(settings: Arc<Mutex<Settings>>) -> Self {
        let mut hero_scripts: HashMap<String, Arc<dyn HeroScript>> = HashMap::new();

        // Register hero scripts
        let huskar = Arc::new(HuskarScript::new(settings.clone()));
        hero_scripts.insert(huskar.hero_name().to_string(), huskar);

        let largo = Arc::new(LargoScript::new(settings.clone()));
        hero_scripts.insert(largo.hero_name().to_string(), largo);

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
        // Log neutral item discovery
        let settings = self.survivability.settings.lock().unwrap();
        log_neutral_item_discovery(event, &settings);
        
        // Update Soul Ring state from GSI event
        soul_ring::update_from_gsi(&event.items, &event.hero, &settings);
        
        // Update Bottle Optimization state from GSI event
        bottle_optimization::update_from_gsi(&event.items, &event.hero, &event.map, &settings);
        
        drop(settings); // Release lock before further processing
        
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
