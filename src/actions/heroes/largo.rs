use crate::actions::heroes::traits::HeroScript;
use crate::actions::common::SurvivabilityActions;
use crate::config::Settings;
use crate::models::{GsiWebhookEvent, Hero};
use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::thread;
use tracing::{debug, info};

lazy_static! {
    static ref ULTIMATE_STATE: Mutex<UltimateState> = Mutex::new(UltimateState {
        active: false,
        last_beat_time: None,
        current_song: None,
        previous_song: None,
        groovin_stacks: 0,
        has_aggs: false,
    });
    static ref BEAT_THREAD_STARTED: Mutex<bool> = Mutex::new(false);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Song {
    Bullbelly,   // Q - Damage
    Hotfeet,     // W - Movement
    IslandElixir // E - Healing
}

#[derive(Debug, Clone)]
struct UltimateState {
    active: bool,
    last_beat_time: Option<Instant>,
    current_song: Option<Song>,
    previous_song: Option<Song>,
    groovin_stacks: u8,
    has_aggs: bool,
}

pub struct LargoScript {
    settings: Arc<Mutex<Settings>>,
}

impl LargoScript {
    pub fn new(settings: Arc<Mutex<Settings>>) -> Self {
        // Start the beat monitoring thread once
        let mut started = BEAT_THREAD_STARTED.lock().unwrap();
        if !*started {
            *started = true;
            drop(started);
            Self::start_beat_thread(settings.clone());
        }
        
        Self { settings }
    }

    fn start_beat_thread(settings: Arc<Mutex<Settings>>) {
        thread::spawn(move || {
            info!("🎵 Largo beat monitoring thread started");
            loop {
                thread::sleep(Duration::from_millis(10)); // Check every 10ms
                
                let mut state = ULTIMATE_STATE.lock().unwrap();
                
                if !state.active {
                    continue;
                }
                
                let current_song = match state.current_song {
                    Some(song) => song,
                    None => continue,
                };
                
                let now = Instant::now();
                
                let should_cast = match state.last_beat_time {
                    Some(last_beat) => {
                        let elapsed = now.duration_since(last_beat);
                        elapsed >= Duration::from_millis(990)
                    }
                    None => true, // First beat
                };
                
                if !should_cast {
                    continue;
                }
                
                // Get the keys for current and potentially previous song
                let settings_guard = settings.lock().unwrap();
                let current_key = match current_song {
                    Song::Bullbelly => settings_guard.heroes.largo.q_ability_key,
                    Song::Hotfeet => settings_guard.heroes.largo.w_ability_key,
                    Song::IslandElixir => settings_guard.heroes.largo.e_ability_key,
                };
                
                let previous_key = if state.has_aggs {
                    state.previous_song.map(|song| match song {
                        Song::Bullbelly => settings_guard.heroes.largo.q_ability_key,
                        Song::Hotfeet => settings_guard.heroes.largo.w_ability_key,
                        Song::IslandElixir => settings_guard.heroes.largo.e_ability_key,
                    })
                } else {
                    None
                };
                drop(settings_guard);
                
                if let Some(_prev_key) = previous_key {
                    debug!("🎸 Strumming {:?} + {:?} on beat (stack: {}) [Aggs]", state.previous_song, current_song, state.groovin_stacks);
                } else {
                    debug!("🎸 Strumming {:?} on beat (stack: {})", current_song, state.groovin_stacks);
                }
                
                // Update state before pressing keys
                state.last_beat_time = Some(now);
                if state.groovin_stacks < 5 {
                    state.groovin_stacks += 1;
                }
                drop(state);
                
                // Press the key(s)
                crate::input::press_key(current_key);
                if let Some(prev_key) = previous_key {
                    crate::input::press_key(prev_key);
                }
            }
        });
    }

    #[allow(dead_code)]
    fn should_toggle_ultimate_on(&self, event: &GsiWebhookEvent) -> bool {
        if !event.hero.is_alive() {
            return false;
        }

        let settings = self.settings.lock().unwrap();
        if !settings.heroes.largo.amphibian_rhapsody_enabled {
            return false;
        }

        // Check mana threshold - need enough mana to start
        if event.hero.mana_percent < settings.heroes.largo.mana_threshold_percent {
            return false;
        }

        // Check if auto-toggle on danger is enabled
        if settings.heroes.largo.auto_toggle_on_danger {
            let in_danger = crate::actions::danger_detector::is_in_danger();
            if in_danger {
                return true;
            }
        }

        false
    }

    #[allow(dead_code)]
    fn should_toggle_ultimate_off(&self, event: &GsiWebhookEvent) -> bool {
        let settings = self.settings.lock().unwrap();
        
        // Check mana threshold - disable if too low
        if event.hero.mana_percent <= settings.heroes.largo.mana_threshold_percent {
            info!("⚡ Disabling Amphibian Rhapsody - low mana ({}%)", event.hero.mana_percent);
            return true;
        }

        false
    }

    // Public method to manually select a song from keyboard input
    pub fn select_song_manually(&self, song: Song) {
        let mut state = ULTIMATE_STATE.lock().unwrap();
        
        // If ultimate is not active, don't allow song selection
        if !state.active {
            info!("Largo: Cannot select song - ultimate is not active");
            return;
        }
        
        // Store previous song before updating
        state.previous_song = state.current_song;
        state.current_song = Some(song);
        info!("Largo: Manually selected song: {:?}", song);
        
        // Reset beat timing to start fresh rhythm
        state.last_beat_time = Some(Instant::now());
        state.groovin_stacks = 0;
    }

    // Public method to activate ultimate from R key press
    pub fn activate_ultimate(&self) {
        let mut state = ULTIMATE_STATE.lock().unwrap();
        
        if !state.active {
            info!("🎵 Amphibian Rhapsody ACTIVATED via R key - waiting for song selection (Q/W/E)");
            state.active = true;
            state.last_beat_time = None;
            state.current_song = None;
            state.previous_song = None;
            state.groovin_stacks = 0;
        } else {
            info!("🎵 Amphibian Rhapsody DEACTIVATED via R key");
            state.active = false;
            state.last_beat_time = None;
            state.current_song = None;
            state.previous_song = None;
        }
    }

    #[allow(dead_code)]
    fn select_song(&self, event: &GsiWebhookEvent) -> Song {
        let settings = self.settings.lock().unwrap();
        
        // Priority 1: Healing if HP low
        if event.hero.health_percent < settings.heroes.largo.heal_hp_threshold {
            debug!("Selecting Island Elixir - HP low ({}%)", event.hero.health_percent);
            return Song::IslandElixir;
        }

        // Priority 2: Movement if danger and HP dropping rapidly
        let in_danger = crate::actions::danger_detector::is_in_danger();
        if in_danger && event.hero.health_percent < 70 {
            debug!("Selecting Hotfeet Hustle - in danger and need mobility");
            return Song::Hotfeet;
        }

        // Default: Damage
        Song::Bullbelly
    }

    fn toggle_ultimate(&self, enable: bool) {
        let settings = self.settings.lock().unwrap();
        let r_key = settings.heroes.largo.r_ability_key;
        drop(settings);

        let mut state = ULTIMATE_STATE.lock().unwrap();
        
        if enable && !state.active {
            info!("🎵 Enabling Amphibian Rhapsody - waiting for manual song selection (Q/W/E)");
            crate::input::press_key(r_key);
            state.active = true;
            state.last_beat_time = None;
            state.current_song = None;
            state.previous_song = None;
            state.groovin_stacks = 0;
        } else if !enable && state.active {
            info!("🎵 Disabling Amphibian Rhapsody");
            crate::input::press_key(r_key);
            state.active = false;
            state.last_beat_time = None;
            state.current_song = None;
            state.previous_song = None;
        }
    }
}

impl HeroScript for LargoScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        // Update danger detection state
        let settings = self.settings.lock().unwrap();
        crate::actions::danger_detector::update(event, &settings.danger_detection);
        drop(settings);
        
        // Check for Aghanim's Scepter (either consumed/blessed or in inventory)
        let has_aggs = event.hero.aghanims_scepter || event.hero.aghanims_shard || 
                       event.items.all_slots().iter().any(|(_, item)| {
            item.name == "item_ultimate_scepter" || item.name == "item_aghanims_shard"
        });
        
        let mut state = ULTIMATE_STATE.lock().unwrap();
        if state.has_aggs != has_aggs {
            state.has_aggs = has_aggs;
            if has_aggs {
                info!("🎵 Aghanim's detected - dual song mode enabled!");
            } else {
                debug!("🎵 Aghanim's not present - single song mode");
            }
        }
        drop(state);
        
        // Use common survivability actions (healing, defensive items, neutral items)
        let survivability = SurvivabilityActions::new(self.settings.clone());
        survivability.check_and_use_healing_items(event);
        survivability.use_defensive_items_if_danger(event);
        survivability.use_neutral_item_if_danger(event);
    }

    fn handle_standalone_trigger(&self) {
        info!("🎸 Manual Amphibian Rhapsody toggle");
        
        // Toggle ultimate state
        let state = ULTIMATE_STATE.lock().unwrap();
        let currently_active = state.active;
        drop(state);
        
        self.toggle_ultimate(!currently_active);
    }

    fn hero_name(&self) -> &'static str {
        Hero::Largo.to_game_name()
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
