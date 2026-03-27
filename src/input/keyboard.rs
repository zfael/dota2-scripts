use rdev::{grab, simulate, Button, Event, EventType, Key};
use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, LazyLock, RwLock};
use std::thread;
use std::time::Duration;
use tracing::{debug, error, info, warn};

use crate::actions::auto_items::MODIFIER_KEY_HELD;
use crate::actions::heroes::broodmother::BROODMOTHER_ACTIVE;
use crate::actions::heroes::shadow_fiend::ShadowFiendState;
use crate::actions::SOUL_RING_STATE;
use crate::actions::soul_ring::{SoulRingKeyboardConfig, SoulRingState};
use crate::config::{AutoAbilityConfig, Settings};
use crate::input::simulation::SIMULATING_KEYS;
use crate::state::app_state::AppState;

pub enum HotkeyEvent {
    ComboTrigger,
    LargoQ,
    LargoW,
    LargoE,
    LargoR,
    BroodmotherSpiderAttack,
}

pub struct KeyboardListenerConfig {
    pub snapshot: Arc<RwLock<KeyboardSnapshot>>,
}

/// Parse key string to rdev::Key (public version)
pub fn parse_key_string(key_str: &str) -> Option<Key> {
    parse_key(key_str)
}

/// Parse key string to rdev::Key
fn parse_key(key_str: &str) -> Option<Key> {
    match key_str.to_lowercase().as_str() {
        "home" => Some(Key::Home),
        "end" => Some(Key::End),
        "insert" => Some(Key::Insert),
        "delete" => Some(Key::Delete),
        "pageup" => Some(Key::PageUp),
        "pagedown" => Some(Key::PageDown),
        "f1" => Some(Key::F1),
        "f2" => Some(Key::F2),
        "f3" => Some(Key::F3),
        "f4" => Some(Key::F4),
        "f5" => Some(Key::F5),
        "f6" => Some(Key::F6),
        "f7" => Some(Key::F7),
        "f8" => Some(Key::F8),
        "f9" => Some(Key::F9),
        "f10" => Some(Key::F10),
        "f11" => Some(Key::F11),
        "f12" => Some(Key::F12),
        // Single char keys
        s if s.len() == 1 => {
            let ch = s.chars().next().unwrap();
            char_to_key(ch)
        }
        _ => None,
    }
}

/// Convert a char to rdev::Key
pub fn char_to_key(ch: char) -> Option<Key> {
    match ch.to_ascii_lowercase() {
        'a' => Some(Key::KeyA),
        'b' => Some(Key::KeyB),
        'c' => Some(Key::KeyC),
        'd' => Some(Key::KeyD),
        'e' => Some(Key::KeyE),
        'f' => Some(Key::KeyF),
        'g' => Some(Key::KeyG),
        'h' => Some(Key::KeyH),
        'i' => Some(Key::KeyI),
        'j' => Some(Key::KeyJ),
        'k' => Some(Key::KeyK),
        'l' => Some(Key::KeyL),
        'm' => Some(Key::KeyM),
        'n' => Some(Key::KeyN),
        'o' => Some(Key::KeyO),
        'p' => Some(Key::KeyP),
        'q' => Some(Key::KeyQ),
        'r' => Some(Key::KeyR),
        's' => Some(Key::KeyS),
        't' => Some(Key::KeyT),
        'u' => Some(Key::KeyU),
        'v' => Some(Key::KeyV),
        'w' => Some(Key::KeyW),
        'x' => Some(Key::KeyX),
        'y' => Some(Key::KeyY),
        'z' => Some(Key::KeyZ),
        '0' => Some(Key::Num0),
        '1' => Some(Key::Num1),
        '2' => Some(Key::Num2),
        '3' => Some(Key::Num3),
        '4' => Some(Key::Num4),
        '5' => Some(Key::Num5),
        '6' => Some(Key::Num6),
        '7' => Some(Key::Num7),
        '8' => Some(Key::Num8),
        '9' => Some(Key::Num9),
        _ => None,
    }
}

/// Convert rdev::Key to char (for keys we care about)
fn key_to_char(key: Key) -> Option<char> {
    match key {
        Key::KeyA => Some('a'),
        Key::KeyB => Some('b'),
        Key::KeyC => Some('c'),
        Key::KeyD => Some('d'),
        Key::KeyE => Some('e'),
        Key::KeyF => Some('f'),
        Key::KeyG => Some('g'),
        Key::KeyH => Some('h'),
        Key::KeyI => Some('i'),
        Key::KeyJ => Some('j'),
        Key::KeyK => Some('k'),
        Key::KeyL => Some('l'),
        Key::KeyM => Some('m'),
        Key::KeyN => Some('n'),
        Key::KeyO => Some('o'),
        Key::KeyP => Some('p'),
        Key::KeyQ => Some('q'),
        Key::KeyR => Some('r'),
        Key::KeyS => Some('s'),
        Key::KeyT => Some('t'),
        Key::KeyU => Some('u'),
        Key::KeyV => Some('v'),
        Key::KeyW => Some('w'),
        Key::KeyX => Some('x'),
        Key::KeyY => Some('y'),
        Key::KeyZ => Some('z'),
        Key::Num0 => Some('0'),
        Key::Num1 => Some('1'),
        Key::Num2 => Some('2'),
        Key::Num3 => Some('3'),
        Key::Num4 => Some('4'),
        Key::Num5 => Some('5'),
        Key::Num6 => Some('6'),
        Key::Num7 => Some('7'),
        Key::Num8 => Some('8'),
        Key::Num9 => Some('9'),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SoulRingReplayPlan {
    TriggerThenOriginal {
        soul_ring_key: Key,
        delay_ms: u64,
        original_key: Key,
    },
    OriginalOnly {
        original_key: Key,
    },
}

fn plan_soul_ring_replay(
    state: &SoulRingState,
    original_key: Key,
    config: &SoulRingKeyboardConfig,
) -> SoulRingReplayPlan {
    if let Some(original_char) = key_to_char(original_key) {
        if state.should_intercept_key_with_config(original_char, config)
            && state.should_trigger_with_config(config)
        {
            if let Some(sr_key) = state.slot_key.and_then(char_to_key) {
                return SoulRingReplayPlan::TriggerThenOriginal {
                    soul_ring_key: sr_key,
                    delay_ms: config.delay_before_ability_ms,
                    original_key,
                };
            }
        }
    }

    SoulRingReplayPlan::OriginalOnly { original_key }
}

/// Request to replay Soul Ring + original key
#[derive(Debug, Clone)]
struct SoulRingReplayRequest {
    original_key: Key,
    config: SoulRingKeyboardConfig,
}

/// Lazy-initialized Soul Ring replay worker queue.
/// The worker thread starts on the first enqueue.
static SOUL_RING_REPLAY_QUEUE: LazyLock<Sender<SoulRingReplayRequest>> = LazyLock::new(|| {
    let (tx, rx) = mpsc::channel::<SoulRingReplayRequest>();
    
    // Spawn dedicated worker thread
    thread::spawn(move || {
        info!("Soul Ring replay worker started");
        
        while let Ok(request) = rx.recv() {
            // Lock state and compute the replay plan
            let soul_ring_state = SOUL_RING_STATE.lock().unwrap();
            let plan = plan_soul_ring_replay(&soul_ring_state, request.original_key, &request.config);
            // Execute using shared helper to ensure identical behavior between worker and fallback
            execute_soul_ring_plan_with_context(soul_ring_state, plan, "");
        }
        
        info!("Soul Ring replay worker exited");
    });
    
    tx
});

/// Simulate a key press using rdev (must be called from a non-grab thread)
/// Sets SIMULATING_KEYS flag to prevent re-interception
pub fn simulate_key(key: Key) {
    SIMULATING_KEYS.store(true, Ordering::SeqCst);
    
    if let Err(e) = simulate(&EventType::KeyPress(key)) {
        warn!("Failed to simulate key press: {:?}", e);
    }
    thread::sleep(Duration::from_millis(5));
    if let Err(e) = simulate(&EventType::KeyRelease(key)) {
        warn!("Failed to simulate key release: {:?}", e);
    }
    
    thread::sleep(Duration::from_millis(5));
    SIMULATING_KEYS.store(false, Ordering::SeqCst);
}

/// Execute a planned Soul Ring replay using the provided mutex guard.
/// The guard is consumed so the function can mark the state and drop the lock
/// before performing key simulation (identical behaviour for worker and fallback).
fn execute_soul_ring_plan_with_context(
    mut guard: std::sync::MutexGuard<'_, SoulRingState>,
    plan: SoulRingReplayPlan,
    context: &str,
) {
    match plan {
        SoulRingReplayPlan::TriggerThenOriginal { soul_ring_key, delay_ms, original_key } => {
            // Mark as triggered before dropping lock, then execute replay
            guard.mark_triggered();
            drop(guard);

            debug!("💍 Pressing Soul Ring key{}: {:?}", context, soul_ring_key);
            simulate_key(soul_ring_key);

            thread::sleep(Duration::from_millis(delay_ms));

            simulate_key(original_key);
        }
        SoulRingReplayPlan::OriginalOnly { original_key } => {
            // No Soul Ring trigger needed, just replay original
            drop(guard);
            simulate_key(original_key);
        }
    }
}

/// Enqueue Soul Ring trigger + ability key simulation to the dedicated worker thread.
/// This is necessary because grab() callback must return quickly.
/// Falls back to spawning a thread if the queue is unexpectedly closed.
fn spawn_soul_ring_then_key(original_key: Key, sr_config: SoulRingKeyboardConfig) {
    let request = SoulRingReplayRequest {
        original_key,
        config: sr_config.clone(),
    };
    
    if let Err(e) = SOUL_RING_REPLAY_QUEUE.send(request) {
        // Queue is closed unexpectedly - fall back to spawning a thread to avoid dropping input
        warn!("Soul Ring replay queue closed unexpectedly, falling back to thread spawn: {:?}", e);
        
        thread::spawn(move || {
            let soul_ring_state = SOUL_RING_STATE.lock().unwrap();
            let plan = plan_soul_ring_replay(&soul_ring_state, original_key, &sr_config);

            // Use the same execution path as the worker but include a (fallback) suffix in logs
            execute_soul_ring_plan_with_context(soul_ring_state, plan, " (fallback)");
        });
    }
}

/// Start keyboard listener in a separate thread with key interception (grab)
/// This intercepts keys and can block/modify them before they reach the game
pub fn start_keyboard_listener(config: KeyboardListenerConfig) -> Receiver<HotkeyEvent> {
    let (event_tx, event_rx) = mpsc::channel::<HotkeyEvent>();

    thread::spawn(move || {
        info!("Starting keyboard listener with key interception (grab)...");

        let callback = move |event: Event| -> Option<Event> {
            // Pass through all events while we're simulating keys
            // This prevents re-interception of our own simulated keypresses
            if SIMULATING_KEYS.load(Ordering::SeqCst) {
                return Some(event);
            }

            // Track Space key (modifier for auto-items)
            match event.event_type {
                EventType::KeyPress(Key::Space) => {
                    MODIFIER_KEY_HELD.store(true, Ordering::SeqCst);
                }
                EventType::KeyRelease(Key::Space) => {
                    MODIFIER_KEY_HELD.store(false, Ordering::SeqCst);
                }
                _ => {}
            }
            
            // Handle Space + Right-click for Broodmother auto-items
            if let EventType::ButtonPress(Button::Right) = event.event_type {
                if MODIFIER_KEY_HELD.load(Ordering::SeqCst) && BROODMOTHER_ACTIVE.load(Ordering::SeqCst) {
                    let snapshot = config.snapshot.read().unwrap().clone();
                    if snapshot.broodmother.auto_items_enabled {
                        let auto_items = snapshot.broodmother.auto_items.clone();
                        let auto_abilities = snapshot.broodmother.auto_abilities.clone();
                        let abilities_first = snapshot.broodmother.abilities_first;
                        let slot_keys = snapshot.broodmother.slot_keys;
                        debug!("🎯 Space+Right-click - Broodmother auto-items");
                        thread::spawn(move || {
                            crate::actions::auto_items::execute_auto_items(
                                &slot_keys, &auto_items, &auto_abilities, abilities_first,
                            );
                        });
                        return None; // Block original right-click
                    }
                }
            }
            
            // Handle Middle Mouse button for Broodmother spider micro
            if let EventType::ButtonPress(button) = event.event_type {
                let is_spider_micro_button = matches!(button, Button::Middle);
                if is_spider_micro_button && BROODMOTHER_ACTIVE.load(Ordering::SeqCst) {
                    let snapshot = config.snapshot.read().unwrap().clone();
                    if snapshot.broodmother.spider_micro_enabled {
                        info!("🕷️ Mouse5 pressed - Broodmother spider attack-move");
                        let _ = event_tx.send(HotkeyEvent::BroodmotherSpiderAttack);
                        let spider_key = snapshot.broodmother.spider_micro_key;
                        let hero_key = snapshot.broodmother.hero_reselect_key;
                        thread::spawn(move || {
                            crate::actions::heroes::broodmother::BroodmotherScript::execute_spider_attack_move_with_keys(
                                spider_key, hero_key,
                            );
                        });
                        return None; // Block the original mouse button
                    }
                }
            }
            
            if let EventType::KeyPress(key) = event.event_type {
                // Read snapshot once per keyboard event — static config comes from here.
                let snapshot = config.snapshot.read().unwrap().clone();

                // Convert key to char to check if we should intercept
                let key_char = key_to_char(key);
                
                // Single live SOUL_RING_STATE read for all Soul Ring interception decisions
                let should_intercept_for_soul_ring = if let Some(ch) = key_char {
                    let soul_ring_state = SOUL_RING_STATE.lock().unwrap();
                    let should_intercept = soul_ring_state.should_intercept_key_with_config(ch, &snapshot.soul_ring);
                    let should_trigger = soul_ring_state.should_trigger_with_config(&snapshot.soul_ring);
                    debug!(
                        "💍 Key '{}': intercept={}, trigger={}, available={}, can_cast={}, mana={}%, health={}%",
                        ch, should_intercept, should_trigger,
                        soul_ring_state.available, soul_ring_state.can_cast,
                        soul_ring_state.hero_mana_percent, soul_ring_state.hero_health_percent
                    );
                    should_intercept && should_trigger
                } else {
                    false
                };

                // Handle Shadow Fiend Q/W/E keys (when SF is selected AND raze interception is enabled)
                let sf_raze_active = snapshot.sf_enabled && snapshot.shadow_fiend.raze_intercept_enabled;
                if sf_raze_active {
                    match key {
                        Key::KeyQ | Key::KeyW | Key::KeyE => {
                            let raze_key = key_to_char(key).unwrap();
                            info!("{} key pressed - SF raze", raze_key.to_ascii_uppercase());
                            
                            ShadowFiendState::execute_raze(raze_key, snapshot.shadow_fiend.raze_delay_ms);
                            
                            // Block original key
                            return None;
                        }
                        _ => {}
                    }
                }

                // Handle Shadow Fiend R key for auto-BKB on ultimate
                if snapshot.sf_enabled && snapshot.shadow_fiend.auto_bkb_on_ultimate && key == Key::KeyR {
                    info!("R key pressed - SF auto-BKB ultimate combo");
                    
                    ShadowFiendState::execute_ultimate_combo(snapshot.shadow_fiend.auto_d_on_ultimate);
                    
                    // Block original key (will be pressed by execute_ultimate_combo)
                    return None;
                }

                // Handle Largo Q/W/E/R keys and other ability keys with Soul Ring
                match key {
                    Key::KeyQ | Key::KeyW | Key::KeyE | Key::KeyR | Key::KeyD | Key::KeyF => {
                        // Send Largo events for beat timing
                        match key {
                            Key::KeyQ => { let _ = event_tx.send(HotkeyEvent::LargoQ); }
                            Key::KeyW => { let _ = event_tx.send(HotkeyEvent::LargoW); }
                            Key::KeyE => { let _ = event_tx.send(HotkeyEvent::LargoE); }
                            Key::KeyR => { let _ = event_tx.send(HotkeyEvent::LargoR); }
                            _ => {}
                        }
                        
                        // If Soul Ring should trigger, spawn handler and block original
                        if should_intercept_for_soul_ring {
                            spawn_soul_ring_then_key(key, snapshot.soul_ring.clone());
                            return None; // Block original
                        }
                        
                        // Pass through if not intercepting for Soul Ring
                        return Some(event);
                    }
                    _ => {}
                }
                
                // Check for item slot keys (for Soul Ring triggering on non-ability keys)
                if should_intercept_for_soul_ring {
                    spawn_soul_ring_then_key(key, snapshot.soul_ring.clone());
                    return None; // Block original
                }
                
                // Check for combo trigger key
                if let Some(trigger_key) = snapshot.trigger_key {
                    if key == trigger_key {
                        info!("{:?} key pressed - triggering combo", trigger_key);
                        let _ = event_tx.send(HotkeyEvent::ComboTrigger);
                        // Pass through - combo trigger doesn't need to be blocked
                    }
                }
            }
            
            // Pass through all other events (key releases, mouse events, etc.)
            Some(event)
        };

        // Start grabbing - this blocks forever
        if let Err(e) = grab(callback) {
            error!("Error in keyboard grab listener: {:?}", e);
        }
    });

    // Return the event receiver
    event_rx
}

/// Snapshot of the Shadow Fiend keyboard-relevant config.
#[derive(Debug, Clone)]
pub struct ShadowFiendKeyboardSnapshot {
    pub raze_intercept_enabled: bool,
    pub auto_bkb_on_ultimate: bool,
    pub raze_delay_ms: u64,
    pub auto_d_on_ultimate: bool,
}

/// Snapshot of the Broodmother keyboard-relevant config.
#[derive(Debug, Clone)]
pub struct BroodmotherKeyboardSnapshot {
    pub spider_micro_enabled: bool,
    pub auto_items_enabled: bool,
    /// Pre-parsed spider control group key (avoids re-parsing in the callback).
    pub spider_micro_key: Option<Key>,
    /// Pre-parsed hero reselect key (avoids re-parsing in the callback).
    pub hero_reselect_key: Option<Key>,
    /// Item names for Space+Right-click auto-items.
    pub auto_items: Vec<String>,
    /// Ability configs for Space+Right-click auto-items.
    pub auto_abilities: Vec<AutoAbilityConfig>,
    pub abilities_first: bool,
    /// Slot keybindings [slot0..slot5] for item-key lookup.
    pub slot_keys: [char; 6],
}

/// Immutable snapshot of all keyboard-listener configuration, derived from
/// `Settings` and `AppState` at a point in time.
///
/// The callback reads this snapshot once per event instead of locking
/// `Settings`, `sf_enabled`, and `trigger_key` separately.
#[derive(Debug, Clone)]
pub struct KeyboardSnapshot {
    /// The parsed combo-trigger key, or `None` if the configured string is
    /// not a recognised key name.
    pub trigger_key: Option<Key>,
    /// Whether Shadow Fiend raze interception is active.
    pub sf_enabled: bool,
    pub shadow_fiend: ShadowFiendKeyboardSnapshot,
    pub broodmother: BroodmotherKeyboardSnapshot,
    /// Static Soul Ring keyboard config (thresholds, key sets, delays).
    pub soul_ring: SoulRingKeyboardConfig,
}

impl KeyboardSnapshot {
    /// Build a snapshot from the current runtime settings and app state.
    pub fn from_runtime(settings: &Settings, state: &AppState) -> Self {
        let trigger_key_str = state.trigger_key.lock().unwrap().clone();
        let trigger_key = parse_key_string(&trigger_key_str);
        let sf_enabled = *state.sf_enabled.lock().unwrap();

        let sf = &settings.heroes.shadow_fiend;
        let bm = &settings.heroes.broodmother;

        Self {
            trigger_key,
            sf_enabled,
            shadow_fiend: ShadowFiendKeyboardSnapshot {
                raze_intercept_enabled: sf.raze_intercept_enabled,
                auto_bkb_on_ultimate: sf.auto_bkb_on_ultimate,
                raze_delay_ms: sf.raze_delay_ms,
                auto_d_on_ultimate: sf.auto_d_on_ultimate,
            },
            broodmother: BroodmotherKeyboardSnapshot {
                spider_micro_enabled: bm.spider_micro_enabled,
                auto_items_enabled: bm.auto_items_enabled,
                spider_micro_key: parse_key_string(&bm.spider_control_group_key),
                hero_reselect_key: parse_key_string(&bm.reselect_hero_key),
                auto_items: bm.auto_items.clone(),
                auto_abilities: bm.auto_abilities.clone(),
                abilities_first: bm.auto_abilities_first,
                slot_keys: [
                    settings.keybindings.slot0,
                    settings.keybindings.slot1,
                    settings.keybindings.slot2,
                    settings.keybindings.slot3,
                    settings.keybindings.slot4,
                    settings.keybindings.slot5,
                ],
            },
            soul_ring: SoulRingKeyboardConfig::from_settings(settings),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use crate::state::app_state::{HeroType, QueueMetrics, UpdateCheckState};
    use crate::actions::soul_ring::{SoulRingState, SoulRingKeyboardConfig};
    use std::collections::HashSet;

    #[test]
    fn keyboard_snapshot_parses_trigger_key_and_sf_flags() {
        let settings = Settings::default();
        let state = AppState {
            selected_hero: Some(HeroType::ShadowFiend),
            gsi_enabled: true,
            standalone_enabled: true,
            last_event: None,
            metrics: QueueMetrics::default(),
            trigger_key: Arc::new(Mutex::new("Home".to_string())),
            sf_enabled: Arc::new(Mutex::new(true)),
            update_state: Arc::new(Mutex::new(UpdateCheckState::Idle)),
        };

        let snapshot = KeyboardSnapshot::from_runtime(&settings, &state);

        assert_eq!(snapshot.trigger_key, Some(Key::Home));
        assert!(snapshot.sf_enabled);
        assert!(snapshot.shadow_fiend.raze_intercept_enabled);
    }

    #[test]
    fn keyboard_snapshot_handles_invalid_trigger_key() {
        let state = AppState::default();
        *state.trigger_key.lock().unwrap() = "not-a-key".to_string();

        let snapshot = KeyboardSnapshot::from_runtime(&Settings::default(), &state);

        assert_eq!(snapshot.trigger_key, None);
    }

    #[test]
    fn keyboard_snapshot_sf_disabled_by_default() {
        let state = AppState::default();
        let snapshot = KeyboardSnapshot::from_runtime(&Settings::default(), &state);
        assert!(!snapshot.sf_enabled);
    }

    #[test]
    fn keyboard_snapshot_parses_f5_trigger_key() {
        let state = AppState::default();
        *state.trigger_key.lock().unwrap() = "F5".to_string();
        let snapshot = KeyboardSnapshot::from_runtime(&Settings::default(), &state);
        assert_eq!(snapshot.trigger_key, Some(Key::F5));
    }

    // Soul Ring replay-plan tests
    fn soul_ring_test_config() -> SoulRingKeyboardConfig {
        SoulRingKeyboardConfig {
            enabled: true,
            min_mana_percent: 100,
            min_health_percent: 1,
            delay_before_ability_ms: 33,
            trigger_cooldown_ms: 250,
            ability_keys: ['q', 'w', 'e'].into_iter().collect(),
            intercept_item_keys: true,
            item_slot_keys: ['z', 'x', 'c', 'v', 'b', 'n'].into_iter().collect(),
        }
    }

    #[test]
    fn eligible_soul_ring_replay_plan_triggers_before_original_key() {
        let mut state = SoulRingState::default();
        state.available = true;
        state.can_cast = true;
        state.slot_key = Some('z');
        state.hero_alive = true;
        state.hero_mana_percent = 10;
        state.hero_health_percent = 90;

        let config = soul_ring_test_config();
        let plan = crate::input::keyboard::plan_soul_ring_replay(&state, Key::KeyQ, &config);

        match plan {
            crate::input::keyboard::SoulRingReplayPlan::TriggerThenOriginal { soul_ring_key, delay_ms, original_key } => {
                assert_eq!(soul_ring_key, Key::KeyZ);
                assert_eq!(delay_ms, config.delay_before_ability_ms);
                assert_eq!(original_key, Key::KeyQ);
            }
            _ => panic!("Expected TriggerThenOriginal plan"),
        }
    }

    #[test]
    fn ineligible_soul_ring_replay_plan_replays_original_only() {
        let mut state = SoulRingState::default();
        state.available = true;
        state.can_cast = true;
        state.slot_key = Some('z');
        state.hero_alive = true;
        state.hero_mana_percent = 100;
        state.hero_health_percent = 90;

        let mut config = soul_ring_test_config();
        // Make ability set empty so key is not intercepted
        config.ability_keys = HashSet::new();
        config.intercept_item_keys = false;

        let plan = crate::input::keyboard::plan_soul_ring_replay(&state, Key::KeyR, &config);

        match plan {
            crate::input::keyboard::SoulRingReplayPlan::OriginalOnly { original_key } => {
                assert_eq!(original_key, Key::KeyR);
            }
            _ => panic!("Expected OriginalOnly plan"),
        }
    }

    #[test]
    fn invalid_soul_ring_slot_key_replays_original_only() {
        let mut state = SoulRingState::default();
        state.available = true;
        state.can_cast = true;
        state.slot_key = Some('?');
        state.hero_alive = true;
        state.hero_mana_percent = 10;
        state.hero_health_percent = 90;

        let mut config = soul_ring_test_config();
        // Ensure original key is intercepted
        config.ability_keys = ['q'].into_iter().collect();

        let plan = crate::input::keyboard::plan_soul_ring_replay(&state, Key::KeyQ, &config);

        match plan {
            crate::input::keyboard::SoulRingReplayPlan::OriginalOnly { original_key } => {
                assert_eq!(original_key, Key::KeyQ);
            }
            _ => panic!("Expected OriginalOnly plan when slot key invalid"),
        }
    }
}
