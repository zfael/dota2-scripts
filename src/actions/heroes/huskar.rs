use crate::actions::heroes::traits::HeroScript;
use crate::actions::common::{armlet_toggle, ArmletConfig, SurvivabilityActions};
use crate::config::Settings;
use crate::models::{GsiWebhookEvent, Hero};
use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{debug, info};

lazy_static! {
    static ref BERSERKER_BLOOD_DEBUFF_DETECTED: Mutex<Option<Instant>> = Mutex::new(None);
    /// Guard to prevent multiple armlet toggle threads from running simultaneously
    static ref ARMLET_THREAD_GUARD: Mutex<()> = Mutex::new(());
}

pub struct HuskarScript {
    settings: Arc<Mutex<Settings>>,
}

impl HuskarScript {
    pub fn new(settings: Arc<Mutex<Settings>>) -> Self {
        Self { settings }
    }

    fn berserker_blood_cleanse(&self, event: &GsiWebhookEvent) {
        if !event.hero.is_alive() {
            return;
        }

        // Check if hero has debuff
        if !event.hero.has_debuff {
            // Reset debuff tracker when no debuff
            if let Ok(mut debuff_time) = BERSERKER_BLOOD_DEBUFF_DETECTED.try_lock() {
                if debuff_time.is_some() {
                    debug!("No debuffs detected, resetting berserker blood tracker");
                    *debuff_time = None;
                }
            }
            return;
        }

        // Find berserker blood ability
        let berserker_ability = [
            &event.abilities.ability0,
            &event.abilities.ability1,
            &event.abilities.ability2,
            &event.abilities.ability3,
        ]
        .iter()
        .find(|ability| ability.name == "huskar_berserkers_blood")
        .copied();

        let Some(ability) = berserker_ability else {
            return;
        };

        // Check if ability can be cast (not on cooldown and has levels)
        if !ability.can_cast || ability.level == 0 || ability.cooldown > 0 {
            debug!("Berserker Blood not ready: can_cast={}, level={}, cooldown={}",
                ability.can_cast, ability.level, ability.cooldown);
            return;
        }

        let settings = self.settings.lock().unwrap();
        let delay_ms = settings.heroes.huskar.berserker_blood_delay_ms;
        let key = settings.heroes.huskar.berserker_blood_key;
        drop(settings);

        if let Ok(mut debuff_time) = BERSERKER_BLOOD_DEBUFF_DETECTED.try_lock() {
            match *debuff_time {
                Some(first_debuff_time) => {
                    // Debuff already detected, check if delay has passed
                    if first_debuff_time.elapsed() >= Duration::from_millis(delay_ms) {
                        info!("Activating Berserker Blood to cleanse debuffs ({}ms delay elapsed)", delay_ms);
                        crate::input::press_key(key);
                        
                        // Reset tracker after activation
                        *debuff_time = None;
                    } else {
                        debug!("Waiting for more debuffs... ({}ms elapsed)",
                            first_debuff_time.elapsed().as_millis());
                    }
                }
                None => {
                    // First debuff detected, start tracking
                    info!("Debuff detected, starting {}ms timer for Berserker Blood", delay_ms);
                    *debuff_time = Some(Instant::now());
                }
            }
        }
    }
}

impl HeroScript for HuskarScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        // PRIORITY 1: Armlet toggle in separate thread (critical for Huskar survival)
        // Uses try_lock guard to prevent race conditions - if another toggle is in progress, skip
        let settings_clone = self.settings.clone();
        let event_clone = event.clone();
        std::thread::spawn(move || {
            // Try to acquire the guard - if another thread holds it, skip this iteration
            let Ok(_guard) = ARMLET_THREAD_GUARD.try_lock() else {
                debug!("Armlet toggle already in progress, skipping");
                return;
            };
            
            let settings = settings_clone.lock().unwrap();
            let armlet_config = ArmletConfig {
                toggle_threshold: settings.heroes.huskar.armlet_toggle_threshold,
                predictive_offset: settings.heroes.huskar.armlet_predictive_offset,
                toggle_cooldown_ms: settings.heroes.huskar.armlet_toggle_cooldown_ms,
            };
            armlet_toggle(&event_clone, &settings, &armlet_config);
            // _guard is dropped here, releasing the lock
        });

        // PRIORITY 2: Update danger detection state
        let settings = self.settings.lock().unwrap();
        crate::actions::danger_detector::update(event, &settings.danger_detection);
        drop(settings);

        // PRIORITY 3: Create survivability actions for healing and defensive items
        let survivability = SurvivabilityActions::new(self.settings.clone());
        
        // Check healing items (danger-aware)
        survivability.check_and_use_healing_items(event);

        // Use defensive items if in danger
        survivability.use_defensive_items_if_danger(event);

        // Use neutral items if in danger
        survivability.use_neutral_item_if_danger(event);

        // PRIORITY 4: Huskar-specific berserker blood cleanse
        self.berserker_blood_cleanse(event);
    }

    fn handle_standalone_trigger(&self) {
        info!("Huskar standalone trigger not implemented");
    }

    fn hero_name(&self) -> &'static str {
        Hero::Huskar.to_game_name()
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
