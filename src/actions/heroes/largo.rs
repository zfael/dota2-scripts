use crate::actions::common::SurvivabilityActions;
use crate::actions::executor::ActionExecutor;
use crate::actions::heroes::traits::HeroScript;
use crate::config::Settings;
use crate::models::{GsiWebhookEvent, Hero};
use lazy_static::lazy_static;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, info};

// Ability names for detecting ultimate state from GSI
// When Largo activates ultimate, Q/W/E abilities change to song variants
const SONG_ABILITY_PREFIX: &str = "largo_song_";
const SONG_FIGHT_SONG: &str = "largo_song_fight_song"; // Q during ult - damage (Bullbelly)
const SONG_DOUBLE_TIME: &str = "largo_song_double_time"; // W during ult - movement (Hotfeet)
const SONG_GOOD_VIBRATIONS: &str = "largo_song_good_vibrations"; // E during ult - healing (Island Elixir)

lazy_static! {
    static ref LARGO_WORKER_CONTROL: LargoWorkerControl = LargoWorkerControl::new();
    static ref BEAT_THREAD_STARTED: Mutex<bool> = Mutex::new(false);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Song {
    Bullbelly,    // Q - Damage
    Hotfeet,      // W - Movement
    IslandElixir, // E - Healing
}

#[derive(Debug, Clone)]
struct UltimateState {
    active: bool,
    beat_start_time: Option<Instant>, // Anchor time for absolute beat calculation
    beat_count: u32, // Number of completed beats since start; schedules the next beat
    schedule_generation: u64,
    current_song: Option<Song>,
    previous_song: Option<Song>,
    pending_song: Option<Song>, // Queued song to switch to on next beat
    groovin_stacks: u8,
    has_aggs: bool,
    beat_config: LargoBeatConfigSnapshot,
}

struct LargoWorkerControl {
    state: Mutex<UltimateState>,
    wake_worker: Condvar,
    emission_guard: Mutex<()>,
}

impl LargoWorkerControl {
    fn new() -> Self {
        Self {
            state: Mutex::new(UltimateState {
                active: false,
                beat_start_time: None,
                beat_count: 0,
                schedule_generation: 0,
                current_song: None,
                previous_song: None,
                pending_song: None,
                groovin_stacks: 0,
                has_aggs: false,
                beat_config: LargoBeatConfigSnapshot::from_settings(&Settings::default()),
            }),
            wake_worker: Condvar::new(),
            emission_guard: Mutex::new(()),
        }
    }

    fn notify_worker(&self) {
        self.wake_worker.notify_all();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LargoBeatConfigSnapshot {
    beat_interval_ms: u32,
    beat_correction_ms: i32,
    beat_correction_every_n_beats: u32,
    q_key: char,
    w_key: char,
    e_key: char,
    #[allow(dead_code)]
    r_key: char,
}

impl LargoBeatConfigSnapshot {
    fn from_settings(settings: &Settings) -> Self {
        let largo = &settings.heroes.largo;

        Self {
            beat_interval_ms: largo.beat_interval_ms,
            beat_correction_ms: largo.beat_correction_ms,
            beat_correction_every_n_beats: largo.beat_correction_every_n_beats,
            q_key: largo.q_ability_key,
            w_key: largo.w_ability_key,
            e_key: largo.e_ability_key,
            r_key: largo.r_ability_key,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BeatPlan {
    keys_to_press: Vec<char>,
}

fn beat_offset_ms(config: &LargoBeatConfigSnapshot, beat_count: u32) -> u64 {
    if beat_count == 0 {
        return 0;
    }

    // beat_count represents completed beats, so a full correction cycle is already finished
    // when the scheduler is calculating the offset for the next beat at that boundary.
    let mut total_ms = config.beat_interval_ms as i64 * beat_count as i64;

    if config.beat_correction_every_n_beats > 0 {
        let completed_cycles = beat_count / config.beat_correction_every_n_beats;
        total_ms += config.beat_correction_ms as i64 * completed_cycles as i64;
    }

    total_ms.max(0) as u64
}

fn song_key(song: Song, config: &LargoBeatConfigSnapshot) -> char {
    match song {
        Song::Bullbelly => config.q_key,
        Song::Hotfeet => config.w_key,
        Song::IslandElixir => config.e_key,
    }
}

fn plan_beat(state: &mut UltimateState, config: &LargoBeatConfigSnapshot) -> BeatPlan {
    state.beat_count += 1;

    if let Some(pending) = state.pending_song.take() {
        if state.current_song != Some(pending) {
            state.previous_song = state.current_song;
            state.current_song = Some(pending);
        }
    }

    let current_song = match state.current_song {
        Some(song) => song,
        None => {
            return BeatPlan {
                keys_to_press: Vec::new(),
            };
        }
    };

    let mut keys_to_press = vec![song_key(current_song, config)];

    if state.has_aggs {
        if let Some(previous_song) = state.previous_song {
            keys_to_press.push(song_key(previous_song, config));
        }
    }

    if state.groovin_stacks < 5 {
        state.groovin_stacks += 1;
    }

    BeatPlan { keys_to_press }
}

fn next_wait_duration(
    beat_start_time: Option<Instant>,
    beat_count: u32,
    config: &LargoBeatConfigSnapshot,
    now: Instant,
) -> Duration {
    let Some(start_time) = beat_start_time else {
        return Duration::ZERO;
    };

    let next_deadline = start_time + Duration::from_millis(beat_offset_ms(config, beat_count));
    next_deadline
        .checked_duration_since(now)
        .unwrap_or(Duration::ZERO)
}

fn should_emit_planned_beat(
    captured_generation: u64,
    live_generation: u64,
    still_active: bool,
) -> bool {
    still_active && captured_generation == live_generation
}

fn has_dual_song_scepter(event: &GsiWebhookEvent) -> bool {
    event.hero.aghanims_scepter
        || event
            .items
            .all_slots()
            .iter()
            .any(|(_, item)| item.name == "item_ultimate_scepter")
}

fn cancel_planned_beats(state: &mut UltimateState) {
    state.schedule_generation = state.schedule_generation.wrapping_add(1);
}

fn reset_song_schedule(state: &mut UltimateState) {
    state.beat_start_time = None;
    state.beat_count = 0;
    state.current_song = None;
    state.previous_song = None;
    state.pending_song = None;
    state.groovin_stacks = 0;
}

pub struct LargoScript {
    settings: Arc<Mutex<Settings>>,
    executor: Arc<ActionExecutor>,
}

impl LargoScript {
    pub fn new(settings: Arc<Mutex<Settings>>, executor: Arc<ActionExecutor>) -> Self {
        let config_snapshot = {
            let settings_guard = settings.lock().unwrap();
            LargoBeatConfigSnapshot::from_settings(&settings_guard)
        };
        {
            let mut state = LARGO_WORKER_CONTROL.state.lock().unwrap();
            state.beat_config = config_snapshot;
        }

        // Start the beat monitoring thread once
        let mut started = BEAT_THREAD_STARTED.lock().unwrap();
        if !*started {
            *started = true;
            drop(started);
            Self::start_beat_thread();
        }

        Self { settings, executor }
    }

    fn start_beat_thread() {
        thread::Builder::new()
            .name("largo-beat-worker".to_string())
            .spawn(move || {
                info!("🎵 Largo beat worker thread started");

                loop {
                    let mut state = LARGO_WORKER_CONTROL.state.lock().unwrap();

                    while !state.active || state.current_song.is_none() {
                        state = LARGO_WORKER_CONTROL.wake_worker.wait(state).unwrap();
                    }

                    let config = state.beat_config;
                    let wait_duration = next_wait_duration(
                        state.beat_start_time,
                        state.beat_count,
                        &config,
                        Instant::now(),
                    );

                    if !wait_duration.is_zero() {
                        let (guard, timeout_result) = LARGO_WORKER_CONTROL
                            .wake_worker
                            .wait_timeout(state, wait_duration)
                            .unwrap();
                        state = guard;

                        if !timeout_result.timed_out() {
                            continue;
                        }
                    }

                    if !state.active || state.current_song.is_none() {
                        continue;
                    }

                    let config = state.beat_config;
                    if !next_wait_duration(
                        state.beat_start_time,
                        state.beat_count,
                        &config,
                        Instant::now(),
                    )
                    .is_zero()
                    {
                        continue;
                    }

                    let now = Instant::now();
                    if state.beat_start_time.is_none() {
                        state.beat_start_time = Some(now);
                    }

                    let previous_current_song = state.current_song;
                    let groovin_stacks_before = state.groovin_stacks;
                    let had_pending_song = state.pending_song.is_some();
                    let plan = plan_beat(&mut state, &config);
                    let captured_generation = state.schedule_generation;
                    let current_song = state.current_song;
                    let previous_song = state.previous_song;
                    let beat_count = state.beat_count;
                    let switched_song =
                        had_pending_song && state.current_song != previous_current_song;
                    drop(state);

                    if switched_song {
                        info!(
                            "🎵 Switching song: {:?} -> {:?}",
                            previous_current_song, current_song
                        );
                    }

                    if let Some(current_song) = current_song {
                        if plan.keys_to_press.len() > 1 {
                            debug!(
                                "🍸 Beat #{}: {:?} + {:?} (stack: {}) [Aggs]",
                                beat_count, previous_song, current_song, groovin_stacks_before
                            );
                        } else {
                            debug!(
                                "🍸 Beat #{}: {:?} (stack: {})",
                                beat_count, current_song, groovin_stacks_before
                            );
                        }
                    }

                    for key in plan.keys_to_press {
                        let emission_guard = LARGO_WORKER_CONTROL.emission_guard.lock().unwrap();
                        let should_emit = {
                            let state = LARGO_WORKER_CONTROL.state.lock().unwrap();
                            should_emit_planned_beat(
                                captured_generation,
                                state.schedule_generation,
                                state.active,
                            )
                        };

                        if !should_emit {
                            break;
                        }

                        crate::input::press_key(key);
                        drop(emission_guard);
                    }
                }
            })
            .expect("failed to start Largo beat worker");
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
            info!(
                "⚡ Disabling Amphibian Rhapsody - low mana ({}%)",
                event.hero.mana_percent
            );
            return true;
        }

        false
    }

    // Public method to immediately stop the beat loop when R is pressed
    // This prevents pressing Q/W/E during the window between R press and GSI confirmation
    pub fn deactivate_ultimate(&self) {
        let emission_guard = LARGO_WORKER_CONTROL.emission_guard.lock().unwrap();
        let mut state = LARGO_WORKER_CONTROL.state.lock().unwrap();

        if state.active {
            info!("🎵 Amphibian Rhapsody STOPPED (R key pressed - awaiting GSI confirmation)");
            state.active = false;
            cancel_planned_beats(&mut state);
            reset_song_schedule(&mut state);
        }
        drop(state);
        drop(emission_guard);
        LARGO_WORKER_CONTROL.notify_worker();
    }

    // Public method to manually select a song from keyboard input
    // The song is queued and will be switched on the next beat to maintain rhythm
    pub fn select_song_manually(&self, song: Song) {
        let mut should_wake_worker = false;
        let emission_guard = LARGO_WORKER_CONTROL.emission_guard.lock().unwrap();
        let mut state = LARGO_WORKER_CONTROL.state.lock().unwrap();

        // If ultimate is not active, don't allow song selection
        if !state.active {
            debug!("Largo: Cannot select song - ultimate is not active");
            drop(state);
            drop(emission_guard);
            return;
        }

        // If no current song yet (first selection), set it immediately and start beat
        if state.current_song.is_none() {
            cancel_planned_beats(&mut state);
            state.current_song = Some(song);
            state.previous_song = None;
            state.pending_song = None;
            state.beat_start_time = None; // Will set anchor on first beat
            state.beat_count = 0;
            state.groovin_stacks = 0;
            info!("🎵 Starting with song: {:?}", song);
            should_wake_worker = true;
        } else if state.current_song != Some(song) {
            // Queue the song for switch on next beat (maintains rhythm)
            state.pending_song = Some(song);
            info!(
                "🎵 Queued song switch to: {:?} (will apply on next beat)",
                song
            );
            should_wake_worker = true;
        }

        drop(state);
        drop(emission_guard);
        if should_wake_worker {
            LARGO_WORKER_CONTROL.notify_worker();
        }
    }

    // Detect if ultimate is active from GSI ability names
    // When ultimate is active, ability0 changes from "largo_catchy_lick" to a song ability
    fn is_ultimate_active_from_gsi(event: &GsiWebhookEvent) -> bool {
        event
            .abilities
            .ability0
            .name
            .starts_with(SONG_ABILITY_PREFIX)
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
            debug!(
                "Selecting Island Elixir - HP low ({}%)",
                event.hero.health_percent
            );
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
        let config_snapshot = {
            let settings = self.settings.lock().unwrap();
            LargoBeatConfigSnapshot::from_settings(&settings)
        };

        let emission_guard = LARGO_WORKER_CONTROL.emission_guard.lock().unwrap();
        let mut state = LARGO_WORKER_CONTROL.state.lock().unwrap();
        state.beat_config = config_snapshot;
        let r_key = state.beat_config.r_key;
        let mut should_press_r = false;
        let mut should_wake_worker = false;

        if enable && !state.active {
            info!("🎵 Enabling Amphibian Rhapsody - waiting for manual song selection (Q/W/E)");
            should_press_r = true;
            state.active = true;
            cancel_planned_beats(&mut state);
            reset_song_schedule(&mut state);
            should_wake_worker = true;
        } else if !enable && state.active {
            info!("🎵 Disabling Amphibian Rhapsody");
            should_press_r = true;
            state.active = false;
            cancel_planned_beats(&mut state);
            reset_song_schedule(&mut state);
            should_wake_worker = true;
        }
        drop(state);
        drop(emission_guard);

        if should_wake_worker {
            LARGO_WORKER_CONTROL.notify_worker();
        }
        if should_press_r {
            crate::input::press_key(r_key);
        }
    }
}

impl HeroScript for LargoScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        // Update danger detection state
        let settings = self.settings.lock().unwrap();
        let in_danger = crate::actions::danger_detector::update(event, &settings.danger_detection);
        let config_snapshot = LargoBeatConfigSnapshot::from_settings(&settings);
        drop(settings);

        // Detect ultimate state from GSI ability names
        let is_ult_active = Self::is_ultimate_active_from_gsi(event);
        let has_aggs = has_dual_song_scepter(event);

        let emission_guard = LARGO_WORKER_CONTROL.emission_guard.lock().unwrap();
        let mut state = LARGO_WORKER_CONTROL.state.lock().unwrap();
        let mut should_wake_worker = false;

        if state.beat_config != config_snapshot {
            state.beat_config = config_snapshot;
            should_wake_worker = true;
        }
        if is_ult_active != state.active {
            state.active = is_ult_active;
            if is_ult_active {
                info!("🎵 Amphibian Rhapsody ACTIVATED (detected from GSI)");
            } else {
                info!("🎵 Amphibian Rhapsody ENDED (detected from GSI)");
            }
            // Always reset state on any transition (start or end)
            cancel_planned_beats(&mut state);
            reset_song_schedule(&mut state);
            should_wake_worker = true;
        }
        if state.has_aggs != has_aggs {
            state.has_aggs = has_aggs;
            if has_aggs {
                info!("🎵 Aghanim's detected - dual song mode enabled!");
            } else {
                debug!("🎵 Aghanim's not present - single song mode");
            }
        }
        drop(state);
        drop(emission_guard);

        if should_wake_worker {
            LARGO_WORKER_CONTROL.notify_worker();
        }

        // Use common survivability actions (healing, defensive items, neutral items)
        let survivability = SurvivabilityActions::new(self.settings.clone(), self.executor.clone());
        survivability.check_and_use_healing_items_with_danger(event, in_danger);
        survivability.use_defensive_items_if_danger_with_snapshot(event, in_danger);
        survivability.use_neutral_item_if_danger_with_snapshot(event, in_danger);
    }

    fn handle_standalone_trigger(&self) {
        info!("🎸 Manual Amphibian Rhapsody toggle");

        // Toggle ultimate state
        let state = LARGO_WORKER_CONTROL.state.lock().unwrap();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::gsi_event::{
        Abilities, Ability, Hero as GsiHero, Item as GsiItem, Items, Map,
    };

    fn test_beat_config() -> LargoBeatConfigSnapshot {
        LargoBeatConfigSnapshot {
            beat_interval_ms: 995,
            beat_correction_ms: 30,
            beat_correction_every_n_beats: 5,
            q_key: 'q',
            w_key: 'w',
            e_key: 'e',
            r_key: 'r',
        }
    }

    fn test_ultimate_state() -> UltimateState {
        UltimateState {
            active: true,
            beat_start_time: None,
            beat_count: 0,
            schedule_generation: 0,
            current_song: Some(Song::Bullbelly),
            previous_song: None,
            pending_song: None,
            groovin_stacks: 0,
            has_aggs: false,
            beat_config: test_beat_config(),
        }
    }

    fn test_ability(name: &str) -> Ability {
        Ability {
            ability_active: true,
            can_cast: true,
            cooldown: 0,
            level: 1,
            name: name.to_string(),
            passive: false,
            ultimate: false,
        }
    }

    fn test_event() -> GsiWebhookEvent {
        GsiWebhookEvent {
            hero: GsiHero {
                aghanims_scepter: false,
                aghanims_shard: false,
                alive: true,
                attributes_level: 0,
                is_break: false,
                buyback_cooldown: 0,
                buyback_cost: 0,
                disarmed: false,
                facet: 0,
                has_debuff: false,
                health: 1000,
                health_percent: 100,
                hexed: false,
                id: 1,
                level: 1,
                magicimmune: false,
                mana: 500,
                mana_percent: 100,
                max_health: 1000,
                max_mana: 500,
                muted: false,
                name: Hero::Largo.to_game_name().to_string(),
                respawn_seconds: 0,
                silenced: false,
                smoked: false,
                stunned: false,
                talent_1: false,
                talent_2: false,
                talent_3: false,
                talent_4: false,
                talent_5: false,
                talent_6: false,
                talent_7: false,
                talent_8: false,
                xp: 0,
                xpos: 0,
                ypos: 0,
            },
            abilities: Abilities {
                ability0: test_ability(SONG_FIGHT_SONG),
                ability1: test_ability("unused_1"),
                ability2: test_ability("unused_2"),
                ability3: test_ability("unused_3"),
                ability4: test_ability("unused_4"),
                ability5: test_ability("unused_5"),
            },
            items: Items {
                neutral0: GsiItem::default(),
                slot0: GsiItem::default(),
                slot1: GsiItem::default(),
                slot2: GsiItem::default(),
                slot3: GsiItem::default(),
                slot4: GsiItem::default(),
                slot5: GsiItem::default(),
                slot6: GsiItem::default(),
                slot7: GsiItem::default(),
                slot8: GsiItem::default(),
                stash0: GsiItem::default(),
                stash1: GsiItem::default(),
                stash2: GsiItem::default(),
                stash3: GsiItem::default(),
                stash4: GsiItem::default(),
                stash5: GsiItem::default(),
                teleport0: GsiItem::default(),
            },
            map: Map { clock_time: 0 },
            player: None,
        }
    }

    #[test]
    fn largo_config_snapshot_copies_runtime_timing_and_keys() {
        let mut settings = Settings::default();
        settings.heroes.largo.beat_interval_ms = 1234;
        settings.heroes.largo.beat_correction_ms = -17;
        settings.heroes.largo.beat_correction_every_n_beats = 9;
        settings.heroes.largo.q_ability_key = '1';
        settings.heroes.largo.w_ability_key = '2';
        settings.heroes.largo.e_ability_key = '3';
        settings.heroes.largo.r_ability_key = '4';

        let snapshot = LargoBeatConfigSnapshot::from_settings(&settings);

        assert_eq!(snapshot.beat_interval_ms, 1234);
        assert_eq!(snapshot.beat_correction_ms, -17);
        assert_eq!(snapshot.beat_correction_every_n_beats, 9);
        assert_eq!(snapshot.q_key, '1');
        assert_eq!(snapshot.w_key, '2');
        assert_eq!(snapshot.e_key, '3');
        assert_eq!(snapshot.r_key, '4');
    }

    #[test]
    fn beat_deadline_applies_periodic_correction() {
        let config = test_beat_config();

        assert_eq!(beat_offset_ms(&config, 0), 0);
        assert_eq!(beat_offset_ms(&config, 4), 3980);
        assert_eq!(beat_offset_ms(&config, 5), 5005);
    }

    #[test]
    fn plan_beat_switches_pending_song_on_the_beat() {
        let config = test_beat_config();
        let mut state = test_ultimate_state();
        state.current_song = Some(Song::Bullbelly);
        state.pending_song = Some(Song::Hotfeet);

        let plan = plan_beat(&mut state, &config);

        assert_eq!(state.beat_count, 1);
        assert_eq!(state.current_song, Some(Song::Hotfeet));
        assert_eq!(state.previous_song, Some(Song::Bullbelly));
        assert_eq!(state.pending_song, None);
        assert_eq!(state.groovin_stacks, 1);
        assert_eq!(plan.keys_to_press, vec!['w']);
    }

    #[test]
    fn aghs_plan_emits_previous_song_after_current_song() {
        let config = test_beat_config();
        let mut state = test_ultimate_state();
        state.current_song = Some(Song::Hotfeet);
        state.previous_song = Some(Song::Bullbelly);
        state.has_aggs = true;

        let plan = plan_beat(&mut state, &config);

        assert_eq!(state.beat_count, 1);
        assert_eq!(state.groovin_stacks, 1);
        assert_eq!(plan.keys_to_press, vec!['w', 'q']);
    }

    #[test]
    fn dual_song_mode_requires_scepter_not_shard() {
        let mut event = test_event();
        event.hero.aghanims_shard = true;
        event.items.slot0.name = "item_aghanims_shard".to_string();

        assert!(!has_dual_song_scepter(&event));

        event.hero.aghanims_scepter = true;
        assert!(has_dual_song_scepter(&event));

        event.hero.aghanims_scepter = false;
        event.items.slot0.name = "item_ultimate_scepter".to_string();
        assert!(has_dual_song_scepter(&event));
    }

    #[test]
    fn first_beat_increments_count_before_next_interval_is_scheduled() {
        let config = test_beat_config();
        let mut state = test_ultimate_state();

        let plan = plan_beat(&mut state, &config);

        assert_eq!(plan.keys_to_press, vec!['q']);
        assert_eq!(state.beat_count, 1);
        assert_eq!(beat_offset_ms(&config, state.beat_count), 995);
    }

    #[test]
    fn next_wait_duration_is_zero_when_first_beat_is_ready() {
        let config = test_beat_config();

        assert_eq!(
            next_wait_duration(None, 0, &config, Instant::now()),
            Duration::ZERO
        );
    }

    #[test]
    fn next_wait_duration_after_first_beat_uses_incremented_count() {
        let config = test_beat_config();
        let anchor = Instant::now();

        assert_eq!(
            next_wait_duration(Some(anchor), 1, &config, anchor),
            Duration::from_millis(995)
        );
    }

    #[test]
    fn cancelled_generation_prevents_emitting_a_due_beat() {
        assert!(!should_emit_planned_beat(2, 3, true));
        assert!(!should_emit_planned_beat(2, 2, false));
        assert!(should_emit_planned_beat(2, 2, true));
    }

    #[test]
    fn select_song_manually_releases_locks_when_ultimate_is_inactive() {
        let script = LargoScript {
            settings: Arc::new(Mutex::new(Settings::default())),
            executor: ActionExecutor::new(),
        };

        {
            let emission_guard = LARGO_WORKER_CONTROL.emission_guard.lock().unwrap();
            let mut state = LARGO_WORKER_CONTROL.state.lock().unwrap();
            state.active = false;
            cancel_planned_beats(&mut state);
            reset_song_schedule(&mut state);
            drop(state);
            drop(emission_guard);
        }

        script.select_song_manually(Song::Bullbelly);

        let emission_guard = LARGO_WORKER_CONTROL
            .emission_guard
            .try_lock()
            .expect("emission guard should be released on inactive early return");
        drop(emission_guard);

        let state = LARGO_WORKER_CONTROL
            .state
            .try_lock()
            .expect("state lock should be released on inactive early return");
        assert!(!state.active);
        assert_eq!(state.current_song, None);
        assert_eq!(state.pending_song, None);
    }
}
