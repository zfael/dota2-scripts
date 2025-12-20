use crate::actions::heroes::traits::HeroScript;
use crate::actions::common::SurvivabilityActions;
use crate::config::Settings;
use crate::models::{GsiWebhookEvent, Hero};
use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::thread;
use tracing::{debug, info};

// Ability names for detecting ultimate state from GSI
// When Largo activates ultimate, Q/W/E abilities change to song variants
const SONG_ABILITY_PREFIX: &str = "largo_song_";
const SONG_FIGHT_SONG: &str = "largo_song_fight_song";           // Q during ult - damage (Bullbelly)
const SONG_DOUBLE_TIME: &str = "largo_song_double_time";         // W during ult - movement (Hotfeet)
const SONG_GOOD_VIBRATIONS: &str = "largo_song_good_vibrations"; // E during ult - healing (Island Elixir)

lazy_static! {
    static ref ULTIMATE_STATE: Mutex<UltimateState> = Mutex::new(UltimateState {
        active: false,
        beat_start_time: None,
        beat_count: 0,
        current_song: None,
        previous_song: None,
        pending_song: None,
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
    beat_start_time: Option<Instant>,  // Anchor time for absolute beat calculation
    beat_count: u32,                    // Number of beats since start
    current_song: Option<Song>,
    previous_song: Option<Song>,
    pending_song: Option<Song>,  // Queued song to switch to on next beat
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
                thread::sleep(Duration::from_millis(5)); // Check every 5ms for tighter timing
                
                let mut state = ULTIMATE_STATE.lock().unwrap();
                
                if !state.active {
                    continue;
                }
                
                // Get beat settings
                let settings_guard = settings.lock().unwrap();
                let beat_interval_ms = settings_guard.heroes.largo.beat_interval_ms as i64;
                let beat_correction_ms = settings_guard.heroes.largo.beat_correction_ms as i64;
                let beat_correction_every_n = settings_guard.heroes.largo.beat_correction_every_n_beats;
                drop(settings_guard);
                
                let now = Instant::now();
                
                // Calculate expected beat time with periodic correction
                let should_cast = match state.beat_start_time {
                    Some(start_time) => {
                        // Base time from interval
                        let mut total_ms = beat_interval_ms * state.beat_count as i64;
                        
                        // Apply correction for completed correction cycles
                        if beat_correction_every_n > 0 {
                            let corrections_applied = state.beat_count / beat_correction_every_n;
                            total_ms += beat_correction_ms * corrections_applied as i64;
                        }
                        
                        // Ensure we don't go negative
                        let total_ms = total_ms.max(0) as u64;
                        let expected_beat_time = start_time + Duration::from_millis(total_ms);
                        now >= expected_beat_time
                    }
                    None => true, // First beat - set anchor
                };
                
                if !should_cast {
                    continue;
                }
                
                // Set anchor on first beat
                if state.beat_start_time.is_none() {
                    state.beat_start_time = Some(now);
                }
                
                // Increment beat count for next cycle
                state.beat_count += 1;
                
                // Process pending song switch on the beat
                if let Some(pending) = state.pending_song.take() {
                    if state.current_song != Some(pending) {
                        info!("🎵 Switching song: {:?} -> {:?}", state.current_song, pending);
                        state.previous_song = state.current_song;
                        state.current_song = Some(pending);
                    }
                }
                
                let current_song = match state.current_song {
                    Some(song) => song,
                    None => continue,
                };
                
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
                    debug!("🍸 Beat #{}: {:?} + {:?} (stack: {}) [Aggs]", state.beat_count, state.previous_song, current_song, state.groovin_stacks);
                } else {
                    debug!("🍸 Beat #{}: {:?} (stack: {})", state.beat_count, current_song, state.groovin_stacks);
                }
                
                // Update groovin stacks
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

    // Public method to immediately stop the beat loop when R is pressed
    // This prevents pressing Q/W/E during the window between R press and GSI confirmation
    pub fn deactivate_ultimate(&self) {
        let mut state = ULTIMATE_STATE.lock().unwrap();
        
        if state.active {
            info!("🎵 Amphibian Rhapsody STOPPED (R key pressed - awaiting GSI confirmation)");
            state.active = false;
            state.beat_start_time = None;
            state.beat_count = 0;
            state.current_song = None;
            state.previous_song = None;
            state.pending_song = None;
            state.groovin_stacks = 0;
        }
    }

    // Public method to manually select a song from keyboard input
    // The song is queued and will be switched on the next beat to maintain rhythm
    pub fn select_song_manually(&self, song: Song) {
        let mut state = ULTIMATE_STATE.lock().unwrap();
        
        // If ultimate is not active, don't allow song selection
        if !state.active {
            debug!("Largo: Cannot select song - ultimate is not active");
            return;
        }
        
        // If no current song yet (first selection), set it immediately and start beat
        if state.current_song.is_none() {
            state.current_song = Some(song);
            state.beat_start_time = None; // Will set anchor on first beat
            state.beat_count = 0;
            info!("🎵 Starting with song: {:?}", song);
            return;
        }
        
        // Queue the song for switch on next beat (maintains rhythm)
        if state.current_song != Some(song) {
            state.pending_song = Some(song);
            info!("🎵 Queued song switch to: {:?} (will apply on next beat)", song);
        }
    }

    // Detect if ultimate is active from GSI ability names
    // When ultimate is active, ability0 changes from "largo_catchy_lick" to a song ability
    fn is_ultimate_active_from_gsi(event: &GsiWebhookEvent) -> bool {
        event.abilities.ability0.name.starts_with(SONG_ABILITY_PREFIX)
    }

    // Detect which song is currently in the Q slot from GSI (optional enhancement)
    #[allow(dead_code)]
    fn detect_current_song_from_gsi(event: &GsiWebhookEvent) -> Option<Song> {
        match event.abilities.ability0.name.as_str() {
            SONG_FIGHT_SONG => Some(Song::Bullbelly),
            SONG_DOUBLE_TIME => Some(Song::Hotfeet),
            SONG_GOOD_VIBRATIONS => Some(Song::IslandElixir),
            _ => None,
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
            state.beat_start_time = None;
            state.beat_count = 0;
            state.current_song = None;
            state.previous_song = None;
            state.pending_song = None;
            state.groovin_stacks = 0;
        } else if !enable && state.active {
            info!("🎵 Disabling Amphibian Rhapsody");
            crate::input::press_key(r_key);
            state.active = false;
            state.beat_start_time = None;
            state.beat_count = 0;
            state.current_song = None;
            state.previous_song = None;
            state.pending_song = None;
        }
    }
}

impl HeroScript for LargoScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        // Update danger detection state
        let settings = self.settings.lock().unwrap();
        crate::actions::danger_detector::update(event, &settings.danger_detection);
        drop(settings);
        
        // Detect ultimate state from GSI ability names
        let is_ult_active = Self::is_ultimate_active_from_gsi(event);
        {
            let mut state = ULTIMATE_STATE.lock().unwrap();
            if is_ult_active != state.active {
                state.active = is_ult_active;
                if is_ult_active {
                    info!("🎵 Amphibian Rhapsody ACTIVATED (detected from GSI)");
                } else {
                    info!("🎵 Amphibian Rhapsody ENDED (detected from GSI)");
                }
                // Always reset state on any transition (start or end)
                state.beat_start_time = None;
                state.beat_count = 0;
                state.current_song = None;
                state.previous_song = None;
                state.pending_song = None;
                state.groovin_stacks = 0;
            }
        }
        
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
