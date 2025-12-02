use crate::actions::heroes::traits::HeroScript;
use crate::actions::common::{armlet_toggle, ArmletConfig};
use crate::config::Settings;
use crate::models::{GsiWebhookEvent, Hero};
use lazy_static::lazy_static;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::{debug, info};

lazy_static! {
    static ref BERSERKER_BLOOD_DEBUFF_DETECTED: Mutex<Option<Instant>> = Mutex::new(None);
}

pub struct HuskarScript {
    settings: Settings,
}

impl HuskarScript {
    pub fn new(settings: Settings) -> Self {
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

        let delay_ms = self.settings.heroes.huskar.berserker_blood_delay_ms;
        let key = self.settings.heroes.huskar.berserker_blood_key;

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
        // Use common armlet toggle function
        let armlet_config = ArmletConfig {
            toggle_threshold: self.settings.heroes.huskar.armlet_toggle_threshold,
            predictive_offset: self.settings.heroes.huskar.armlet_predictive_offset,
            toggle_cooldown_ms: self.settings.heroes.huskar.armlet_toggle_cooldown_ms,
        };
        armlet_toggle(event, &self.settings, &armlet_config);
        
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
