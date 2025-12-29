//! Tinker automation script
//!
//! Features:
//! - ⚙️ Smart Combo: Casts available abilities/items in priority order
//! - 🔄 Auto Rearm: Only Rearms when all priority abilities are on cooldown
//! - 🔧 Interrupt Detection: Retries Rearm if interrupted (via cooldown check)
//! - 🛡️ Auto Defense Matrix: Self-cast when in danger
//!
//! This version uses a fast internal loop (~50ms) for responsive combos,
//! with GSI events updating cached state when available.

use crate::actions::common::find_item_slot_by_name;
use crate::actions::danger_detector;
use crate::actions::heroes::traits::HeroScript;
use crate::actions::soul_ring;
use crate::config::Settings;
use crate::input::simulation::{left_click, press_key};
use crate::models::GsiWebhookEvent;
use lazy_static::lazy_static;
use std::any::Any;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Combo loop interval in milliseconds
const COMBO_LOOP_INTERVAL_MS: u64 = 50;

/// Minimum time between casts in milliseconds
const MIN_CAST_INTERVAL_MS: u64 = 80;

lazy_static! {
    /// Global Tinker state for combo tracking
    pub static ref TINKER_STATE: Mutex<TinkerState> = Mutex::new(TinkerState::default());
    
    /// Cached GSI event for fast combo loop
    static ref CACHED_GSI: Mutex<Option<CachedGsiData>> = Mutex::new(None);
}

/// Cached data from GSI events for fast combo execution
#[derive(Debug, Clone)]
pub struct CachedGsiData {
    /// Last update timestamp
    pub last_update: Instant,
    /// Hero is alive
    pub is_alive: bool,
    /// Laser ready state and cooldown
    pub laser_ready: bool,
    pub laser_cooldown: u32,
    /// Warp Flare ready state
    pub warp_flare_ready: bool,
    /// Defense Matrix ready
    pub matrix_ready: bool,
    /// Item slots with ready state (name, slot_key, ready, needs_click)
    pub items: Vec<(String, char, bool, bool)>,
    /// Health percent for auto matrix
    pub health_percent: u32,
    /// Current settings snapshot
    pub settings_snapshot: TinkerSettingsSnapshot,
}

/// Snapshot of Tinker settings to avoid lock contention
#[derive(Debug, Clone)]
pub struct TinkerSettingsSnapshot {
    pub enabled: bool,
    pub combo_priority: Vec<String>,
    pub laser_key: char,
    pub matrix_key: char,
    pub warp_flare_key: char,
    pub rearm_key: char,
    pub auto_matrix_enabled: bool,
    pub auto_matrix_hp_threshold: u32,
    pub rearm_retry_on_interrupt: bool,
}

/// Tinker combo state
#[derive(Debug, Clone)]
pub struct TinkerState {
    /// Whether combo loop is active (toggled by Home key)
    pub combo_active: bool,
    /// Timestamp when Rearm was started
    pub rearm_started: Option<Instant>,
    /// Laser cooldown before Rearm (for interrupt detection)
    pub laser_cd_before_rearm: u32,
    /// Whether we're waiting to verify Rearm success
    pub awaiting_rearm_verify: bool,
    /// Last cast timestamp to prevent spam
    pub last_cast: Option<Instant>,
    /// Whether combo loop thread is running
    pub loop_running: bool,
    /// Timestamp when Rearm completed (wait for GSI update)
    pub rearm_completed_at: Option<Instant>,
}

impl Default for TinkerState {
    fn default() -> Self {
        Self {
            combo_active: false,
            rearm_started: None,
            laser_cd_before_rearm: 0,
            awaiting_rearm_verify: false,
            last_cast: None,
            loop_running: false,
            rearm_completed_at: None,
        }
    }
}

pub struct TinkerScript {
    settings: Arc<Mutex<Settings>>,
}

impl TinkerScript {
    pub fn new(settings: Arc<Mutex<Settings>>) -> Self {
        Self { settings }
    }

    /// Update cached GSI data from event
    fn update_cache(&self, event: &GsiWebhookEvent) {
        let settings = self.settings.lock().unwrap();
        let config = &settings.heroes.tinker;

        // Build item cache
        let mut items = Vec::new();
        for priority_item in &config.combo_priority {
            if priority_item.starts_with("item_") {
                if let Some(slot_key) = find_item_slot_by_name(event, &settings, priority_item) {
                    let ready = self.is_item_ready_static(event, priority_item);
                    let needs_click = priority_item == "item_ethereal_blade";
                    items.push((priority_item.clone(), slot_key, ready, needs_click));
                }
            }
        }

        let cached = CachedGsiData {
            last_update: Instant::now(),
            is_alive: event.hero.is_alive(),
            laser_ready: self.is_ability_ready_static(event, "tinker_laser"),
            laser_cooldown: self.get_laser_cooldown_static(event),
            warp_flare_ready: self.is_ability_ready_static(event, "tinker_warp_flare"),
            matrix_ready: self.is_ability_ready_static(event, "tinker_defense_matrix"),
            items,
            health_percent: event.hero.health_percent,
            settings_snapshot: TinkerSettingsSnapshot {
                enabled: config.enabled,
                combo_priority: config.combo_priority.clone(),
                laser_key: config.laser_key,
                matrix_key: config.matrix_key,
                warp_flare_key: config.warp_flare_key,
                rearm_key: config.rearm_key,
                auto_matrix_enabled: config.auto_matrix_enabled,
                auto_matrix_hp_threshold: config.auto_matrix_hp_threshold,
                rearm_retry_on_interrupt: config.rearm_retry_on_interrupt,
            },
        };

        *CACHED_GSI.lock().unwrap() = Some(cached);
    }

    /// Get Laser cooldown from event (static helper)
    fn get_laser_cooldown_static(&self, event: &GsiWebhookEvent) -> u32 {
        for ability in [
            &event.abilities.ability0,
            &event.abilities.ability1,
            &event.abilities.ability2,
            &event.abilities.ability3,
        ] {
            if ability.name == "tinker_laser" {
                return ability.cooldown;
            }
        }
        0
    }

    /// Check if ability is ready (static helper)
    fn is_ability_ready_static(&self, event: &GsiWebhookEvent, ability_name: &str) -> bool {
        for ability in [
            &event.abilities.ability0,
            &event.abilities.ability1,
            &event.abilities.ability2,
            &event.abilities.ability3,
            &event.abilities.ability4,
            &event.abilities.ability5,
        ] {
            if ability.name == ability_name {
                return ability.can_cast && ability.cooldown == 0 && ability.level > 0;
            }
        }
        false
    }

    /// Check if item is ready (static helper)
    fn is_item_ready_static(&self, event: &GsiWebhookEvent, item_name: &str) -> bool {
        for item in [
            &event.items.slot0,
            &event.items.slot1,
            &event.items.slot2,
            &event.items.slot3,
            &event.items.slot4,
            &event.items.slot5,
        ] {
            if item.name.contains(item_name) {
                return item.cooldown.unwrap_or(0) == 0 && item.can_cast.unwrap_or(false);
            }
        }
        false
    }

    /// Auto Defense Matrix when in danger (called from GSI handler)
    fn auto_matrix(&self, event: &GsiWebhookEvent) {
        let settings = self.settings.lock().unwrap();
        let config = &settings.heroes.tinker;

        if !config.enabled || !config.auto_matrix_enabled {
            return;
        }

        if !event.hero.is_alive() {
            return;
        }

        if !danger_detector::is_in_danger() {
            return;
        }

        if event.hero.health_percent > config.auto_matrix_hp_threshold {
            return;
        }

        if !self.is_ability_ready_static(event, "tinker_defense_matrix") {
            return;
        }

        info!(
            "🛡️ Auto Defense Matrix - HP {}% in danger!",
            event.hero.health_percent
        );

        let key = config.matrix_key;
        drop(settings);
        press_key(key);
        press_key(key); // Double-tap for self-cast
    }

    /// Start the fast combo loop in a background thread
    fn start_combo_loop(&self) {
        let settings = self.settings.clone();

        thread::spawn(move || {
            info!("⚙️ Tinker combo loop STARTED");

            loop {
                // Check if we should stop
                {
                    let state = TINKER_STATE.lock().unwrap();
                    if !state.combo_active {
                        break;
                    }
                }

                // Execute one combo step
                Self::combo_step(&settings);

                // Sleep for loop interval
                thread::sleep(Duration::from_millis(COMBO_LOOP_INTERVAL_MS));
            }

            // Mark loop as stopped
            {
                let mut state = TINKER_STATE.lock().unwrap();
                state.loop_running = false;
            }
            info!("⚙️ Tinker combo loop STOPPED");
        });
    }

    /// Execute one step of the combo (called from fast loop)
    fn combo_step(settings: &Arc<Mutex<Settings>>) {
        // Get cached GSI data
        let cached = {
            let cache_lock = CACHED_GSI.lock().unwrap();
            match &*cache_lock {
                Some(c) => c.clone(),
                None => return, // No GSI data yet
            }
        };

        // Check if cache is too stale (>2 seconds)
        if cached.last_update.elapsed().as_secs() > 2 {
            debug!("⚙️ GSI cache stale, skipping combo step");
            return;
        }

        if !cached.is_alive {
            return;
        }

        let config = &cached.settings_snapshot;
        if !config.enabled {
            return;
        }

        let mut state = TINKER_STATE.lock().unwrap();

        // Rate limit casts
        if let Some(last) = state.last_cast {
            if last.elapsed().as_millis() < MIN_CAST_INTERVAL_MS as u128 {
                return;
            }
        }

        // Wait for GSI cache to update after Rearm completed (500ms buffer)
        if let Some(rearm_done) = state.rearm_completed_at {
            if rearm_done.elapsed().as_millis() < 500 {
                return; // Wait for GSI to update with new cooldowns
            }
            // Clear the flag once we've waited
            state.rearm_completed_at = None;
        }

        // Check if we're awaiting Rearm verification
        if state.awaiting_rearm_verify {
            if let Some(rearm_start) = state.rearm_started {
                // Wait for Rearm channel time + buffer (level 3 = 1.25s + 0.3s buffer)
                if rearm_start.elapsed().as_millis() > 1550 {
                    if cached.laser_cooldown > 0 && state.laser_cd_before_rearm > 0 {
                        // Laser still on CD → Rearm was interrupted
                        warn!(
                            "⚙️ Rearm interrupted! Laser still on CD ({}s). Retrying...",
                            cached.laser_cooldown
                        );

                        if config.rearm_retry_on_interrupt {
                            press_key(config.rearm_key);
                            state.rearm_started = Some(Instant::now());
                            state.last_cast = Some(Instant::now());
                            return;
                        }
                    } else {
                        // Rearm successful - wait for GSI to update before continuing
                        debug!("⚙️ Rearm successful - waiting for GSI update");
                        state.awaiting_rearm_verify = false;
                        state.rearm_started = None;
                        state.rearm_completed_at = Some(Instant::now());
                        return; // Wait for next loop iteration after GSI updates
                    }
                }
                return; // Still waiting for Rearm to complete
            }
        }

        // Find available cast from priority list
        let settings_guard = settings.lock().unwrap();
        
        for priority_item in &config.combo_priority {
            match priority_item.as_str() {
                "laser" => {
                    if cached.laser_ready {
                        debug!("⚙️ Casting Laser");
                        drop(state);
                        soul_ring::press_ability_with_soul_ring(config.laser_key, &settings_guard);
                        drop(settings_guard);
                        TINKER_STATE.lock().unwrap().last_cast = Some(Instant::now());
                        return;
                    }
                }
                "warp_flare" => {
                    if cached.warp_flare_ready {
                        debug!("⚙️ Casting Warp Flare");
                        drop(state);
                        soul_ring::press_ability_with_soul_ring(config.warp_flare_key, &settings_guard);
                        drop(settings_guard);
                        TINKER_STATE.lock().unwrap().last_cast = Some(Instant::now());
                        return;
                    }
                }
                item_name if item_name.starts_with("item_") => {
                    // Find this item in cached items
                    for (name, slot_key, ready, needs_click) in &cached.items {
                        if name == item_name && *ready {
                            debug!("⚙️ Casting item: {}", name);
                            drop(state);
                            drop(settings_guard);
                            press_key(*slot_key);
                            if *needs_click {
                                thread::sleep(Duration::from_millis(30));
                                left_click();
                                // // Press "1" to refocus on hero after clicking target
                                // thread::sleep(Duration::from_millis(20));
                                // press_key('1');
                            }
                            TINKER_STATE.lock().unwrap().last_cast = Some(Instant::now());
                            return;
                        }
                    }
                    // Item not ready or not found - continue to next priority
                }
                _ => {}
            }
        }

        drop(settings_guard);

        // Nothing available to cast - check if all ABILITIES are on CD for Rearm
        if !cached.laser_ready && !cached.warp_flare_ready {
            info!("⚙️ All abilities on cooldown - triggering Rearm");
            state.laser_cd_before_rearm = cached.laser_cooldown;
            press_key(config.rearm_key);
            state.rearm_started = Some(Instant::now());
            state.awaiting_rearm_verify = true;
            state.last_cast = Some(Instant::now());
        }
    }
}

impl HeroScript for TinkerScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        // Update danger detector
        let settings = self.settings.lock().unwrap();
        danger_detector::update(event, &settings.danger_detection);
        drop(settings);

        // Update cached GSI data for fast combo loop
        self.update_cache(event);

        // Auto Defense Matrix (still GSI-driven for safety)
        self.auto_matrix(event);
    }

    fn handle_standalone_trigger(&self) {
        let settings = self.settings.lock().unwrap();
        let config = &settings.heroes.tinker;

        if !config.enabled {
            return;
        }
        drop(settings);

        // Toggle combo state
        let mut state = TINKER_STATE.lock().unwrap();
        state.combo_active = !state.combo_active;

        if state.combo_active {
            info!("⚙️ Tinker combo ENABLED - fast loop active");
            // Reset state
            state.rearm_started = None;
            state.awaiting_rearm_verify = false;
            state.last_cast = None;

            // Start combo loop if not already running
            if !state.loop_running {
                state.loop_running = true;
                drop(state);
                self.start_combo_loop();
            }
        } else {
            info!("⚙️ Tinker combo DISABLED");
            // Loop will stop on next iteration when it sees combo_active = false
        }
    }

    fn hero_name(&self) -> &'static str {
        "npc_dota_hero_tinker"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Check if Tinker combo is currently active
pub fn is_combo_active() -> bool {
    TINKER_STATE
        .lock()
        .map(|s| s.combo_active)
        .unwrap_or(false)
}
