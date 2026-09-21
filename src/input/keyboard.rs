use rdev::{grab, simulate, Button, Event, EventType, Key};
use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, LazyLock, RwLock};
use std::thread;
use std::time::Duration;
use tracing::{debug, error, info, warn};

use crate::actions::auto_items::MODIFIER_KEY_HELD;
use crate::actions::heroes::broodmother::BROODMOTHER_ACTIVE;
use crate::actions::heroes::outworld_destroyer::{
    build_keyboard_combo_config, OutworldDestroyerComboConfig, OutworldDestroyerState,
};
use crate::actions::heroes::shadow_fiend::ShadowFiendState;
use crate::actions::heroes::magnus::MagnusState;
use crate::actions::heroes::earth_spirit::EarthSpiritState;
use crate::actions::heroes::ember_spirit::EmberSpiritState;
use crate::actions::heroes::largo::{self, Song};
use crate::actions::heroes::mirana::MiranaState;
use crate::actions::heroes::slark::SlarkState;
use crate::actions::heroes::snapfire::SnapfireState;
use crate::actions::soul_ring::{SoulRingKeyboardConfig, SoulRingState};
use crate::actions::SOUL_RING_STATE;
use crate::config::settings::InvokerProfileMode;
use crate::config::{AutoAbilityConfig, Settings};
use crate::input::binding::KeyBinding;
use crate::input::simulation::SIMULATING_KEYS;
use crate::state::app_state::AppState;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum HotkeyEvent {
    ComboTrigger,
    MeepoFarmToggle,
    ArmletRoshanToggle,
    WaveOverlayToggle,
    /// Record the cursor's current position as the HUD hero portrait.
    CaptureHudPortrait,
    InvokerCycleComboProfile,
    InvokerProfile(String),
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
        "space" => Some(Key::Space),
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

/// Something the grab layer can synthesize: a key, or a mouse button.
///
/// Soul Ring lives in an item slot, and an item slot can be bound to a thumb
/// mouse button. Without this the replay simply skipped the trigger for such a
/// slot — no press, no warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SimulatedInput {
    Key(Key),
    Button(Button),
}

/// What `rdev` needs to press this binding, or `None` if it can press neither.
fn rdev_input_for(binding: &KeyBinding) -> Option<SimulatedInput> {
    binding
        .rdev_key()
        .map(SimulatedInput::Key)
        .or_else(|| binding.rdev_button().map(SimulatedInput::Button))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SoulRingReplayPlan {
    TriggerThenOriginal {
        soul_ring_key: SimulatedInput,
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
    if let Some(original_binding) = KeyBinding::from_event(&EventType::KeyPress(original_key)) {
        if state.should_intercept_key_with_config(&original_binding, config)
            && state.should_trigger_with_config(config)
        {
            if let Some(sr_key) = state.slot_key.as_ref().and_then(rdev_input_for) {
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

/// Request to replay treads presses + Soul Ring + original key
#[derive(Debug, Clone)]
struct SoulRingReplayRequest {
    original_key: Key,
    config: SoulRingKeyboardConfig,
    treads: Option<crate::actions::power_treads::TreadsPrefix>,
}

/// Send the treads switch planned for this key, before Soul Ring and the key
/// itself. Items are not interrupted by a treads press, and orders are
/// processed in sequence, so the switch lands before the cast spends mana.
fn press_treads_prefix(prefix: &Option<crate::actions::power_treads::TreadsPrefix>) {
    let Some(prefix) = prefix else {
        return;
    };
    let Some(input) = rdev_input_for(&prefix.binding) else {
        return;
    };
    for _ in 0..prefix.presses {
        simulate_input(input);
        thread::sleep(Duration::from_millis(prefix.gap_ms));
    }
}

/// Lazy-initialized Soul Ring replay worker queue.
/// The worker thread starts on the first enqueue.
static SOUL_RING_REPLAY_QUEUE: LazyLock<Sender<SoulRingReplayRequest>> = LazyLock::new(|| {
    let (tx, rx) = mpsc::channel::<SoulRingReplayRequest>();

    // Spawn dedicated worker thread
    thread::spawn(move || {
        info!("Soul Ring replay worker started");

        while let Ok(request) = rx.recv() {
            press_treads_prefix(&request.treads);
            // Lock state and compute the replay plan
            let soul_ring_state = SOUL_RING_STATE.lock().unwrap();
            let plan =
                plan_soul_ring_replay(&soul_ring_state, request.original_key, &request.config);
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
    simulate_input(SimulatedInput::Key(key));
}

/// Press and release one input, guarding against our own grab re-intercepting it.
fn simulate_input(input: SimulatedInput) {
    let (press, release) = match input {
        SimulatedInput::Key(key) => (EventType::KeyPress(key), EventType::KeyRelease(key)),
        SimulatedInput::Button(button) => (
            EventType::ButtonPress(button),
            EventType::ButtonRelease(button),
        ),
    };

    SIMULATING_KEYS.store(true, Ordering::SeqCst);

    if let Err(e) = simulate(&press) {
        warn!("Failed to simulate press of {:?}: {:?}", input, e);
    }
    thread::sleep(Duration::from_millis(5));
    if let Err(e) = simulate(&release) {
        warn!("Failed to simulate release of {:?}: {:?}", input, e);
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
        SoulRingReplayPlan::TriggerThenOriginal {
            soul_ring_key,
            delay_ms,
            original_key,
        } => {
            // Mark as triggered before dropping lock, then execute replay
            guard.mark_triggered();
            drop(guard);

            debug!("💍 Pressing Soul Ring key{}: {:?}", context, soul_ring_key);
            simulate_input(soul_ring_key);

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
fn spawn_soul_ring_then_key(
    original_key: Key,
    sr_config: SoulRingKeyboardConfig,
    treads: Option<crate::actions::power_treads::TreadsPrefix>,
) {
    let request = SoulRingReplayRequest {
        original_key,
        config: sr_config.clone(),
        treads: treads.clone(),
    };

    if let Err(e) = SOUL_RING_REPLAY_QUEUE.send(request) {
        // Queue is closed unexpectedly - fall back to spawning a thread to avoid dropping input
        warn!(
            "Soul Ring replay queue closed unexpectedly, falling back to thread spawn: {:?}",
            e
        );

        thread::spawn(move || {
            press_treads_prefix(&treads);
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
        let pressed_hotkeys = RefCell::new(HashSet::new());
        let held_follow_up_keys = RefCell::new(HashSet::new());

        let callback = move |event: Event| -> Option<Event> {
            if let EventType::KeyRelease(key) = &event.event_type {
                handle_global_hotkey_key_release(*key, &mut pressed_hotkeys.borrow_mut());
                held_follow_up_keys.borrow_mut().remove(key);
            }

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

            // Handle Broodmother callback actions without touching snapshot unless needed.
            match event.event_type {
                EventType::ButtonPress(Button::Right) => {
                    let modifier_held = MODIFIER_KEY_HELD.load(Ordering::SeqCst);
                    let broodmother_active = BROODMOTHER_ACTIVE.load(Ordering::SeqCst);

                    if modifier_held && broodmother_active {
                        let snapshot = config.snapshot.read().unwrap().clone();
                        if let Some(action) = plan_broodmother_callback_action(
                            &event.event_type,
                            modifier_held,
                            broodmother_active,
                            &snapshot,
                        ) {
                            debug!("🎯 Space+Right-click - Broodmother auto-items");
                            enqueue_broodmother_callback_action(action);
                            return None; // Block original right-click
                        }
                    }
                }
                EventType::ButtonPress(Button::Middle) => {
                    let broodmother_active = BROODMOTHER_ACTIVE.load(Ordering::SeqCst);

                    if broodmother_active {
                        let snapshot = config.snapshot.read().unwrap().clone();
                        if let Some(action) = plan_broodmother_callback_action(
                            &event.event_type,
                            false,
                            broodmother_active,
                            &snapshot,
                        ) {
                            info!("🕷️ Middle mouse pressed - Broodmother spider attack-move");
                            enqueue_broodmother_callback_action(action);
                            return None; // Block the original mouse button
                        }
                    }
                }
                _ => {}
            }

            if let EventType::KeyPress(key) = event.event_type {
                let snapshot = config.snapshot.read().unwrap().clone();
                // Read snapshot once per keyboard event — static config comes from here.
                // Convert key to char to check if we should intercept
                let key_char = key_to_char(key);

                // Single live SOUL_RING_STATE read for all Soul Ring interception decisions
                let should_intercept_for_soul_ring = if let Some(ch) = key_char {
                    let soul_ring_state = SOUL_RING_STATE.lock().unwrap();
                    let spend = soul_ring_state
                        .spend_for_key(&KeyBinding::from(ch), &snapshot.soul_ring);
                    let should_intercept = spend.warrants_soul_ring();
                    let should_trigger =
                        soul_ring_state.should_trigger_with_config(&snapshot.soul_ring);
                    debug!(
                        "💍 Key '{}': spend={:?}, intercept={}, trigger={}, available={}, can_cast={}, mana={}%, health={}%",
                        ch, spend, should_intercept, should_trigger,
                        soul_ring_state.available, soul_ring_state.can_cast,
                        soul_ring_state.hero_mana_percent, soul_ring_state.hero_health_percent
                    );
                    // An unpriced entry means the generated table is behind a patch. Worth
                    // surfacing, since it silently costs the buff rather than misfiring.
                    if matches!(spend, crate::actions::soul_ring::ManaSpend::Unknown) {
                        debug!(
                            "💍 Soul Ring: '{}' is not in mana_costs.rs — rerun scripts/generate-mana-costs.ps1",
                            ch
                        );
                    }
                    should_intercept && should_trigger
                } else {
                    false
                };

                // Handle Shadow Fiend Q/W/E keys (when SF is selected AND raze interception is enabled)
                let sf_raze_active =
                    snapshot.sf_enabled && snapshot.shadow_fiend.raze_intercept_enabled;
                if sf_raze_active {
                    match key {
                        Key::KeyQ | Key::KeyW | Key::KeyE => {
                            let raze_key = key_to_char(key).unwrap();
                            info!("{} key pressed - SF raze", raze_key.to_ascii_uppercase());

                            ShadowFiendState::execute_raze(
                                raze_key,
                                snapshot.shadow_fiend.raze_delay_ms,
                            );

                            // Block original key
                            return None;
                        }
                        _ => {}
                    }
                }

                // Handle Shadow Fiend R key for auto-BKB on ultimate
                if snapshot.sf_enabled
                    && snapshot.shadow_fiend.auto_bkb_on_ultimate
                    && key == Key::KeyR
                {
                    info!("R key pressed - SF auto-BKB ultimate combo");

                    ShadowFiendState::execute_ultimate_combo(
                        snapshot.shadow_fiend.auto_d_on_ultimate,
                    );

                    // Block original key (will be pressed by execute_ultimate_combo)
                    return None;
                }

                // Handle Snapfire directional Firesnap Cookie combo.
                // Trigger (default Space): ALT + right-click to face cursor, then
                // self-cast Cookie so Snapfire leaps toward the cursor. W stays
                // free for manual ally cookies.
                if snapshot.snapfire_enabled && snapshot.snapfire.enabled {
                    if let Some(trigger) = snapshot.snapfire.trigger_key {
                        if key == trigger {
                            info!("🍪 Snapfire trigger pressed - directional cookie leap");
                            SnapfireState::execute_cookie_leap(
                                snapshot.snapfire.cookie_key,
                                snapshot.snapfire.turn_delay_ms,
                            );
                            // Block original key (combo handles the cookie cast).
                            return None;
                        }
                    }
                }

                // Handle Magnus directional Reverse Polarity.
                // Reverse Polarity drags enemies to the arc in front of Magnus,
                // so face the cursor first and the pull lands where the Skewer
                // that follows is aimed.
                if snapshot.magnus_enabled && snapshot.magnus.enabled {
                    if let Some(ultimate_key) = snapshot.magnus.ultimate_key {
                        if key == ultimate_key {
                            // Falling through leaves the key unblocked, so a
                            // cooldown press never issues the facing right-click
                            // and never walks Magnus toward the cursor.
                            if !snapshot.magnus.require_ability_ready
                                || MagnusState::can_intercept_ultimate()
                            {
                                info!("🦏 Magnus ultimate pressed - directional Reverse Polarity");
                                MagnusState::execute_directional_ultimate(
                                    snapshot.magnus.ultimate_char,
                                    snapshot.magnus.turn_delay_ms,
                                    crate::actions::heroes::magnus::plan_camera_center(
                                        snapshot.magnus.center_camera_on_ultimate,
                                        snapshot.magnus.camera_center_key,
                                        snapshot.magnus.camera_center_delay_ms,
                                    ),
                                );
                                // Block original key (combo re-presses it).
                                return None;
                            }
                        }
                    }
                }

                // Handle Slark directional Pounce.
                // Pounce leaps along Slark's facing at cast time, so face the
                // cursor first and the leap lands where you are pointing.
                if snapshot.slark_enabled && snapshot.slark.enabled {
                    if let Some(pounce_key) = snapshot.slark.pounce_key {
                        if key == pounce_key {
                            // Falling through leaves the key unblocked, so a
                            // cooldown press never issues the facing right-click
                            // and never walks Slark toward the cursor.
                            if !snapshot.slark.require_ability_ready
                                || SlarkState::can_intercept_pounce()
                            {
                                info!("🐟 Slark pounce pressed - directional Pounce");
                                SlarkState::execute_directional_pounce(
                                    snapshot.slark.pounce_char,
                                    snapshot.slark.turn_delay_ms,
                                );
                                // Block original key (combo re-presses it).
                                return None;
                            }
                        }
                    }
                }

                // Handle Mirana directional Leap.
                // Leap jumps along Mirana's facing at cast time, so face the
                // cursor first and she lands where you are pointing.
                if snapshot.mirana_enabled && snapshot.mirana.enabled {
                    if let Some(leap_key) = snapshot.mirana.leap_key {
                        if key == leap_key {
                            // Falling through leaves the key unblocked, so a
                            // cooldown press never issues the facing right-click
                            // and never walks Mirana toward the cursor.
                            if !snapshot.mirana.require_ability_ready
                                || MiranaState::can_intercept_leap()
                            {
                                info!("🌙 Mirana leap pressed - directional Leap");
                                MiranaState::execute_directional_leap(
                                    snapshot.mirana.leap_char,
                                    snapshot.mirana.turn_delay_ms,
                                );
                                // Block original key (combo re-presses it).
                                return None;
                            }
                        }
                    }
                }

                // Handle Earth Spirit's two remnant combos.
                //
                // Both pair a Stone Remnant with one more ability, but in
                // opposite order, because the two abilities offer different
                // aiming windows:
                //   grip key -> remnant, then grip  (the silence; grip resolves
                //               on the press, so the remnant must already exist)
                //   roll key -> roll, then a self-cast remnant (the 1600-unit
                //               roll; the ~0.6s windup is long enough to drop
                //               the stone on Earth Spirit himself, which is
                //               where the roll starts, so it needs no aiming)
                //
                // Neither gate reads Stone Remnant. It is charge-based, and
                // GSI's can_cast is unreliable for charge abilities, so gating
                // on it would kill the intercept while remnants are banked.
                if snapshot.earth_spirit_enabled && snapshot.earth_spirit.enabled {
                    if snapshot.earth_spirit.silence_combo_enabled {
                        if let Some(grip_key) = snapshot.earth_spirit.grip_key {
                            if key == grip_key {
                                // Falling through leaves the key unblocked, so a
                                // cooldown press never spends a remnant charge on
                                // a grip that cannot fire.
                                if !snapshot.earth_spirit.require_grip_ready
                                    || EarthSpiritState::can_intercept_grip()
                                {
                                    info!("🗿 Earth Spirit grip pressed - remnant then grip");
                                    EarthSpiritState::execute_silence_combo(
                                        snapshot.earth_spirit.remnant_char,
                                        snapshot.earth_spirit.grip_char,
                                        snapshot.earth_spirit.silence_remnant_delay_ms,
                                    );
                                    // Block original key (combo re-presses it).
                                    return None;
                                }
                            }
                        }
                    }

                    if snapshot.earth_spirit.roll_combo_enabled {
                        if let Some(roll_key) = snapshot.earth_spirit.roll_key {
                            if key == roll_key {
                                if !snapshot.earth_spirit.require_roll_ready
                                    || EarthSpiritState::can_intercept_roll()
                                {
                                    info!("🗿 Earth Spirit roll pressed - roll then remnant");
                                    EarthSpiritState::execute_roll_combo(
                                        snapshot.earth_spirit.roll_char,
                                        snapshot.earth_spirit.roll_double_tap,
                                        snapshot.earth_spirit.roll_double_tap_delay_ms,
                                        snapshot.earth_spirit.remnant_char,
                                        snapshot.earth_spirit.roll_to_remnant_delay_ms,
                                        snapshot.earth_spirit.roll_remnant_alt,
                                        snapshot.earth_spirit.roll_remnant_double_tap,
                                        snapshot.earth_spirit.roll_remnant_double_tap_delay_ms,
                                    );
                                    // Block original key (combo re-presses it).
                                    return None;
                                }
                            }
                        }
                    }
                }

                // Handle Ember Spirit's Sleight activate: Sleight of Fist, then
                // Activate Fire Remnant. The Sleight press is not blocked — it
                // falls through so Soul Ring can still intercept it — and only
                // the activate is scheduled.
                let ember_activate_scheduled = if let Some(activate_delay_ms) =
                    plan_ember_sleight_activate(
                        key,
                        &snapshot,
                        should_intercept_for_soul_ring,
                        &mut held_follow_up_keys.borrow_mut(),
                    ) {
                    info!(
                        "🔥 Ember Spirit Sleight of Fist pressed - activate in {}ms",
                        activate_delay_ms
                    );
                    EmberSpiritState::execute_sleight_activate(
                        snapshot.ember_spirit.activate_char,
                        activate_delay_ms,
                    );
                    true
                } else {
                    false
                };

                if snapshot.od_enabled {
                    if snapshot.outworld_destroyer.ultimate_intercept_enabled && key == Key::KeyR {
                        if OutworldDestroyerState::can_intercept_ultimate() {
                            info!("R key pressed - OD ultimate combo");
                            OutworldDestroyerState::execute_ultimate_combo(
                                snapshot.outworld_destroyer.combo_config.clone(),
                            );
                            return None;
                        }
                    }

                    if snapshot.outworld_destroyer.astral_self_cast_enabled {
                        if let Some(astral_self_cast_key) =
                            snapshot.outworld_destroyer.astral_self_cast_key
                        {
                            if key == astral_self_cast_key {
                                if OutworldDestroyerState::can_self_cast_astral() {
                                    info!(
                                        "{:?} key pressed - OD self-Astral",
                                        astral_self_cast_key
                                    );
                                    OutworldDestroyerState::execute_self_astral(
                                        snapshot
                                            .outworld_destroyer
                                            .combo_config
                                            .astral_imprisonment_key,
                                    );
                                } else {
                                    info!(
                                        "{:?} key pressed - OD self-Astral unavailable",
                                        astral_self_cast_key
                                    );
                                }

                                return None;
                            }
                        }
                    }
                }

                // Handle Largo's songs while Amphibian Rhapsody is up.
                //
                // The songs are a rhythm, not a queue of casts: a key pressed
                // between beats casts the song off the beat, which is exactly
                // what desyncs it — and with Aghanim's, desyncs it from the
                // other song of the pair. So the press is swallowed and kept as
                // the song the player wants, and the beat worker puts it on the
                // grid. Soul Ring deliberately never sees these presses:
                // replaying the key a few hundred ms later is the same desync
                // by another route.
                if let Some(action) = plan_largo_key_action(
                    key,
                    &snapshot.largo,
                    snapshot.largo_enabled,
                    largo::ULTIMATE_ACTIVE.load(Ordering::SeqCst),
                ) {
                    match action {
                        LargoKeyAction::SelectSong(song) => {
                            debug!("🎵 {:?} captured as Largo song {:?}", key, song);
                            largo::queue_song(song);
                            // Blocked: the beat worker presses it on the beat.
                            return None;
                        }
                        LargoKeyAction::StopUltimate => {
                            // Not blocked — this press is what ends the
                            // ultimate. Stopping the loop here rather than
                            // waiting for GSI is what keeps the beat after it
                            // from pressing a song that no longer exists.
                            largo::stop_ultimate();
                        }
                    }
                }

                // Power Treads. The treads key itself is reported and passes
                // through; a key that spends mana or heals may get a treads
                // switch sent ahead of it. Asked only here, past every hero
                // intercept that blocks the key, so a switch is never planned
                // for a press that will not go out. Ember's Sleight is skipped:
                // its activate is already timed from this press.
                let treads_prefix = if ember_activate_scheduled {
                    None
                } else {
                    KeyBinding::from_event(&event.event_type)
                        .and_then(|binding| crate::actions::power_treads::on_key_press(&binding))
                };

                // Handle ability keys with Soul Ring
                match key {
                    Key::KeyQ | Key::KeyW | Key::KeyE | Key::KeyR | Key::KeyD | Key::KeyF => {
                        // If Soul Ring or treads should go first, spawn handler and block original
                        if should_intercept_for_soul_ring || treads_prefix.is_some() {
                            spawn_soul_ring_then_key(
                                key,
                                snapshot.soul_ring.clone(),
                                treads_prefix,
                            );
                            return None; // Block original
                        }

                        // Pass through if not intercepting for Soul Ring
                        return Some(event);
                    }
                    _ => {}
                }

                // Check for item slot keys (for Soul Ring triggering on non-ability keys)
                if should_intercept_for_soul_ring || treads_prefix.is_some() {
                    spawn_soul_ring_then_key(key, snapshot.soul_ring.clone(), treads_prefix);
                    return None; // Block original
                }

                if let Some(hotkey_event) =
                    plan_global_hotkey_press_event(key, &snapshot, &mut pressed_hotkeys.borrow_mut())
                {
                    match hotkey_event {
                        HotkeyEvent::ArmletRoshanToggle => {
                            info!(
                                "{:?} key pressed - toggling Armlet Roshan mode",
                                snapshot.armlet_roshan_toggle_key
                            );
                            let _ = event_tx.send(hotkey_event);
                            return None;
                        }
                        HotkeyEvent::WaveOverlayToggle => {
                            info!(
                                "{:?} key pressed - toggling wave overlay",
                                snapshot.wave_overlay_toggle_key
                            );
                            // Blocked: the overlay hotkey is ours, and letting it
                            // through would also fire whatever Dota binds to it.
                            let _ = event_tx.send(hotkey_event);
                            return None;
                        }
                        HotkeyEvent::CaptureHudPortrait => {
                            info!(
                                "{:?} key pressed - capturing the HUD portrait anchor",
                                snapshot.hud_capture_portrait_key
                            );
                            // Blocked for the same reason as the overlay toggle:
                            // the key is ours while calibrating.
                            let _ = event_tx.send(hotkey_event);
                            return None;
                        }
                        HotkeyEvent::ComboTrigger => {
                            info!("{:?} key pressed - triggering combo", snapshot.trigger_key);
                        }
                        HotkeyEvent::MeepoFarmToggle => {
                            info!(
                                "{:?} key pressed - toggling Meepo farm assist",
                                snapshot.meepo_farm_toggle_key
                            );
                        }
                        HotkeyEvent::InvokerCycleComboProfile => {
                            info!(
                                "{:?} key pressed - cycling Invoker combo profile",
                                snapshot.invoker_cycle_hotkey
                            );
                        }
                        _ => {}
                    }

                    let _ = event_tx.send(hotkey_event);
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

/// Snapshot of the Snapfire keyboard-relevant config.
#[derive(Debug, Clone)]
pub struct SnapfireKeyboardSnapshot {
    pub enabled: bool,
    /// Pre-parsed trigger key (default Space).
    pub trigger_key: Option<Key>,
    pub cookie_key: char,
    pub turn_delay_ms: u64,
}

/// Snapshot of the Slark keyboard-relevant config.
#[derive(Debug, Clone)]
pub struct SlarkKeyboardSnapshot {
    pub enabled: bool,
    /// Pre-parsed Pounce key, used to match the intercepted press.
    pub pounce_key: Option<Key>,
    /// The same key as a char, used to re-press it after the turn.
    pub pounce_char: char,
    pub turn_delay_ms: u64,
    pub require_ability_ready: bool,
}

/// Snapshot of the Mirana keyboard-relevant config.
#[derive(Debug, Clone)]
pub struct MiranaKeyboardSnapshot {
    pub enabled: bool,
    /// Pre-parsed Leap key, used to match the intercepted press.
    pub leap_key: Option<Key>,
    /// The same key as a char, used to re-press it after the turn.
    pub leap_char: char,
    pub turn_delay_ms: u64,
    pub require_ability_ready: bool,
}

/// Snapshot of the Earth Spirit keyboard-relevant config.
///
/// Both combos open with the same Stone Remnant press, so `remnant_char` is
/// shared; everything downstream of it is per-combo.
#[derive(Debug, Clone)]
pub struct EarthSpiritKeyboardSnapshot {
    pub enabled: bool,
    /// Stone Remnant key, pressed first by both combos.
    pub remnant_char: char,
    pub silence_combo_enabled: bool,
    /// Pre-parsed Geomagnetic Grip key, used to match the intercepted press.
    pub grip_key: Option<Key>,
    /// The same key as a char, used to press it after the remnant.
    pub grip_char: char,
    pub silence_remnant_delay_ms: u64,
    pub require_grip_ready: bool,
    pub roll_combo_enabled: bool,
    /// Pre-parsed Rolling Boulder key, used to match the intercepted press.
    pub roll_key: Option<Key>,
    /// The same key as a char, used to press it after the remnant.
    pub roll_char: char,
    pub roll_double_tap: bool,
    pub roll_double_tap_delay_ms: u64,
    /// Gap between the roll firing and the remnant press.
    pub roll_to_remnant_delay_ms: u64,
    /// Hold ALT across the remnant press (Dota's self-cast modifier).
    pub roll_remnant_alt: bool,
    /// Press the remnant key twice (Dota's default self-cast binding).
    pub roll_remnant_double_tap: bool,
    pub roll_remnant_double_tap_delay_ms: u64,
    pub require_roll_ready: bool,
}

/// Snapshot of the Ember Spirit keyboard-relevant config.
///
/// Only the Sleight activate lives here; the remnant chase is fired by the
/// global combo trigger and never intercepts a key.
#[derive(Debug, Clone)]
pub struct EmberSpiritKeyboardSnapshot {
    pub sleight_activate_enabled: bool,
    /// Pre-parsed Sleight of Fist key, used to match the watched press.
    pub sleight_key: Option<Key>,
    /// Activate Fire Remnant key, pressed after the Sleight.
    pub activate_char: char,
    pub sleight_activate_delay_ms: u64,
}

/// Snapshot of the Largo keyboard-relevant config.
///
/// The song keys are the ability keys the player actually bound, pre-parsed
/// once per snapshot so the hook compares `Key`s rather than re-parsing config
/// on every keystroke.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LargoKeyboardSnapshot {
    pub bullbelly_key: Option<Key>,
    pub hotfeet_key: Option<Key>,
    pub island_elixir_key: Option<Key>,
    /// Amphibian Rhapsody. Pressed while the ultimate is up, it ends it.
    pub ultimate_key: Option<Key>,
}

/// What the hook does with a Largo key while the ultimate is up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LargoKeyAction {
    /// Swallow the press; the beat worker emits this song on the beat.
    SelectSong(Song),
    /// Let the press through — it is what ends the ultimate — but stop the
    /// beat loop now rather than waiting a GSI tick for confirmation.
    StopUltimate,
}

/// Decide what a keystroke means to Largo.
///
/// Returns `None` whenever the press is not ours, which is every keystroke in
/// the game except a song or ultimate key during Amphibian Rhapsody. Outside
/// the ultimate the song keys are ordinary abilities and must reach Dota
/// untouched — including in the window after the ultimate key is pressed but
/// before GSI confirms the cast, where the safe answer is to pass through.
fn plan_largo_key_action(
    key: Key,
    snapshot: &LargoKeyboardSnapshot,
    largo_enabled: bool,
    ultimate_active: bool,
) -> Option<LargoKeyAction> {
    if !largo_enabled || !ultimate_active {
        return None;
    }

    // Checked first so binding a song to the ultimate key cannot make the
    // ultimate unstoppable.
    if snapshot.ultimate_key == Some(key) {
        return Some(LargoKeyAction::StopUltimate);
    }

    if snapshot.bullbelly_key == Some(key) {
        return Some(LargoKeyAction::SelectSong(Song::Bullbelly));
    }
    if snapshot.hotfeet_key == Some(key) {
        return Some(LargoKeyAction::SelectSong(Song::Hotfeet));
    }
    if snapshot.island_elixir_key == Some(key) {
        return Some(LargoKeyAction::SelectSong(Song::IslandElixir));
    }

    None
}

/// Snapshot of the Magnus keyboard-relevant config.
#[derive(Debug, Clone)]
pub struct MagnusKeyboardSnapshot {
    pub enabled: bool,
    /// Pre-parsed Reverse Polarity key, used to match the intercepted press.
    pub ultimate_key: Option<Key>,
    /// The same key as a char, used to re-press it after the turn.
    pub ultimate_char: char,
    pub turn_delay_ms: u64,
    pub require_ability_ready: bool,
    pub center_camera_on_ultimate: bool,
    /// Pre-parsed hero-select key double-tapped to recentre the camera.
    pub camera_center_key: Option<Key>,
    pub camera_center_delay_ms: u64,
}

#[derive(Debug, Clone)]
pub struct OutworldDestroyerKeyboardSnapshot {
    pub ultimate_intercept_enabled: bool,
    pub astral_self_cast_enabled: bool,
    pub astral_self_cast_key: Option<Key>,
    pub combo_config: OutworldDestroyerComboConfig,
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
    pub slot_keys: [KeyBinding; 6],
}

/// Immutable snapshot of all keyboard-listener configuration, derived from
/// `Settings` and `AppState` at a point in time.
///
/// The callback reads this snapshot once per event instead of locking
/// `Settings`, `sf_enabled`, and `trigger_key` separately.
#[derive(Debug, Clone)]
pub struct InvokerHotkeyProfileSnapshot {
    pub id: String,
    pub hotkey: Option<Key>,
    pub mode: InvokerProfileMode,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct KeyboardSnapshot {
    pub selected_hero: Option<crate::state::HeroType>,
    /// The parsed combo-trigger key, or `None` if the configured string is
    /// not a recognised key name.
    pub trigger_key: Option<Key>,
    /// Parsed Meepo farm-assist toggle key for the current hero, if active.
    pub meepo_farm_toggle_key: Option<Key>,
    /// Parsed Armlet Roshan mode toggle key, if enabled.
    pub armlet_roshan_toggle_key: Option<Key>,
    /// Parsed wave-overlay toggle key, if the overlay is enabled.
    pub wave_overlay_toggle_key: Option<Key>,
    /// Parsed hotkey that captures the HUD hero portrait anchor.
    pub hud_capture_portrait_key: Option<Key>,
    /// Hardcoded Invoker combo-cycle key for v1.
    pub invoker_cycle_hotkey: Option<Key>,
    /// Whether Shadow Fiend raze interception is active.
    pub sf_enabled: bool,
    pub od_enabled: bool,
    pub shadow_fiend: ShadowFiendKeyboardSnapshot,
    pub outworld_destroyer: OutworldDestroyerKeyboardSnapshot,
    pub broodmother: BroodmotherKeyboardSnapshot,
    /// Whether Snapfire is the active hero (drives the cookie intercept).
    pub snapfire_enabled: bool,
    pub snapfire: SnapfireKeyboardSnapshot,
    /// Whether Magnus is the active hero (drives the ultimate intercept).
    pub magnus_enabled: bool,
    pub magnus: MagnusKeyboardSnapshot,
    /// Whether Slark is the active hero (drives the Pounce intercept).
    pub slark_enabled: bool,
    pub slark: SlarkKeyboardSnapshot,
    /// Whether Mirana is the active hero (drives the Leap intercept).
    pub mirana_enabled: bool,
    pub mirana: MiranaKeyboardSnapshot,
    /// Whether Earth Spirit is the active hero (drives both remnant combos).
    pub earth_spirit_enabled: bool,
    pub earth_spirit: EarthSpiritKeyboardSnapshot,
    /// Whether Ember Spirit is the active hero (drives the Sleight activate).
    pub ember_spirit_enabled: bool,
    pub ember_spirit: EmberSpiritKeyboardSnapshot,
    /// Whether Largo is the active hero (drives the song-key intercept).
    pub largo_enabled: bool,
    pub largo: LargoKeyboardSnapshot,
    /// Static Soul Ring keyboard config (thresholds, key sets, delays).
    pub soul_ring: SoulRingKeyboardConfig,
    pub invoker_profiles: Vec<InvokerHotkeyProfileSnapshot>,
}

#[derive(Debug, Clone)]
enum BroodmotherCallbackAction {
    AutoItems {
        slot_keys: [KeyBinding; 6],
        auto_items: Vec<String>,
        auto_abilities: Vec<AutoAbilityConfig>,
        abilities_first: bool,
    },
    SpiderMicro {
        spider_key: Option<Key>,
        hero_key: Option<Key>,
    },
}

/// Request to execute a Broodmother callback action via the dedicated worker.
#[derive(Debug, Clone)]
struct BroodmotherCallbackRequest {
    action: BroodmotherCallbackAction,
}

/// Single-worker queue for Broodmother callback actions (auto-items and spider micro).
/// Uses LazyLock to initialize once on first access.
static BROODMOTHER_CALLBACK_QUEUE: LazyLock<Sender<BroodmotherCallbackRequest>> =
    LazyLock::new(|| {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || broodmother_callback_worker(rx));
        tx
    });

/// Worker thread that executes Broodmother callback actions sequentially.
fn broodmother_callback_worker(rx: Receiver<BroodmotherCallbackRequest>) {
    info!("Broodmother callback worker started");

    while let Ok(request) = rx.recv() {
        execute_broodmother_callback_action_with_guard("", || {
            execute_broodmother_callback_action(request.action, "");
        });
    }

    info!("Broodmother callback worker exited");
}

/// Execute a Broodmother callback action and keep the worker alive if it panics.
fn execute_broodmother_callback_action_with_guard<F>(context: &str, action: F)
where
    F: FnOnce(),
{
    if let Err(panic_payload) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(action)) {
        let panic_message = if let Some(message) = panic_payload.downcast_ref::<&str>() {
            *message
        } else if let Some(message) = panic_payload.downcast_ref::<String>() {
            message.as_str()
        } else {
            "unknown panic payload"
        };

        warn!(
            "Broodmother callback action panicked{}; worker will continue running: {}",
            context, panic_message
        );
    }
}

/// Shared execution helper for Broodmother callback actions.
/// Used by both the worker and fallback thread to ensure identical behavior.
fn execute_broodmother_callback_action(action: BroodmotherCallbackAction, context: &str) {
    match action {
        BroodmotherCallbackAction::AutoItems {
            slot_keys,
            auto_items,
            auto_abilities,
            abilities_first,
        } => {
            debug!("🕷️ Executing Broodmother auto-items{}", context);
            crate::actions::auto_items::execute_auto_items(
                &slot_keys,
                &auto_items,
                &auto_abilities,
                abilities_first,
            );
        }
        BroodmotherCallbackAction::SpiderMicro {
            spider_key,
            hero_key,
        } => {
            debug!("🕷️ Executing Broodmother spider micro{}", context);
            crate::actions::heroes::broodmother::BroodmotherScript::execute_spider_attack_move_with_keys(
                spider_key,
                hero_key,
            );
        }
    }
}

/// Enqueue Broodmother callback action to the dedicated worker thread.
/// Falls back to spawning a thread if the queue is unexpectedly closed.
fn enqueue_broodmother_callback_action(action: BroodmotherCallbackAction) {
    let request = BroodmotherCallbackRequest {
        action: action.clone(),
    };

    if let Err(e) = BROODMOTHER_CALLBACK_QUEUE.send(request) {
        warn!(
            "Broodmother callback queue closed unexpectedly, falling back to thread spawn: {:?}",
            e
        );

        thread::spawn(move || {
            execute_broodmother_callback_action_with_guard(" (fallback)", || {
                execute_broodmother_callback_action(action, " (fallback)");
            });
        });
    }
}

impl KeyboardSnapshot {
    /// Build a snapshot from the current runtime settings and app state.
    pub fn from_runtime(settings: &Settings, state: &AppState) -> Self {
        let trigger_key_str = state.trigger_key.lock().unwrap().clone();
        let trigger_key = parse_key_string(&trigger_key_str);
        let sf_enabled = *state.sf_enabled.lock().unwrap();
        let od_enabled = *state.od_enabled.lock().unwrap();

        let sf = &settings.heroes.shadow_fiend;
        let od = &settings.heroes.outworld_destroyer;
        let bm = &settings.heroes.broodmother;
        let sp = &settings.heroes.snapfire;
        let mg = &settings.heroes.magnus;
        let sk = &settings.heroes.slark;
        let mi = &settings.heroes.mirana;
        let es = &settings.heroes.earth_spirit;
        let em = &settings.heroes.ember_spirit;
        let lg = &settings.heroes.largo;

        Self {
            selected_hero: state.selected_hero.clone(),
            trigger_key,
            meepo_farm_toggle_key: if state.selected_hero == Some(crate::state::HeroType::Meepo)
                && state.standalone_enabled
                && settings.heroes.meepo.farm_assist.enabled
            {
                parse_key_string(&settings.heroes.meepo.farm_assist.toggle_key)
            } else {
                None
            },
            armlet_roshan_toggle_key: if settings.armlet.roshan.enabled {
                parse_key_string(&settings.armlet.roshan.toggle_key)
            } else {
                None
            },
            wave_overlay_toggle_key: if settings.wave_overlay.enabled {
                parse_key_string(&settings.wave_overlay.toggle_key)
            } else {
                None
            },
            hud_capture_portrait_key: parse_key_string(&settings.hud.capture_portrait_key),
            invoker_cycle_hotkey: parse_key_string(&settings.heroes.invoker.cycle_combo_profiles_hotkey),
            sf_enabled,
            od_enabled,
            shadow_fiend: ShadowFiendKeyboardSnapshot {
                raze_intercept_enabled: sf.raze_intercept_enabled,
                auto_bkb_on_ultimate: sf.auto_bkb_on_ultimate,
                raze_delay_ms: sf.raze_delay_ms,
                auto_d_on_ultimate: sf.auto_d_on_ultimate,
            },
            outworld_destroyer: OutworldDestroyerKeyboardSnapshot {
                ultimate_intercept_enabled: od.ultimate_intercept_enabled,
                astral_self_cast_enabled: od.astral_self_cast_enabled,
                astral_self_cast_key: parse_key_string(&od.astral_self_cast_key),
                combo_config: build_keyboard_combo_config(settings),
            },
            broodmother: BroodmotherKeyboardSnapshot {
                spider_micro_enabled: bm.spider_micro_enabled,
                auto_items_enabled: bm.auto_items_enabled,
                spider_micro_key: parse_key_string(&bm.spider_control_group_key),
                hero_reselect_key: parse_key_string(&bm.reselect_hero_key),
                auto_items: bm.auto_items.clone(),
                auto_abilities: bm.auto_abilities.clone(),
                abilities_first: bm.auto_abilities_first,
                slot_keys: settings.keybindings.item_slots().map(Clone::clone),
            },
            snapfire_enabled: state.selected_hero == Some(crate::state::HeroType::Snapfire),
            snapfire: SnapfireKeyboardSnapshot {
                enabled: sp.enabled,
                trigger_key: parse_key_string(&sp.trigger_key),
                cookie_key: sp.cookie_key,
                turn_delay_ms: sp.turn_delay_ms,
            },
            magnus_enabled: state.selected_hero == Some(crate::state::HeroType::Magnus),
            magnus: MagnusKeyboardSnapshot {
                enabled: mg.enabled,
                ultimate_key: char_to_key(mg.ultimate_key),
                ultimate_char: mg.ultimate_key,
                turn_delay_ms: mg.turn_delay_ms,
                require_ability_ready: mg.require_ability_ready,
                center_camera_on_ultimate: mg.center_camera_on_ultimate,
                camera_center_key: parse_key_string(&mg.camera_center_key),
                camera_center_delay_ms: mg.camera_center_delay_ms,
            },
            slark_enabled: state.selected_hero == Some(crate::state::HeroType::Slark),
            slark: SlarkKeyboardSnapshot {
                enabled: sk.enabled,
                pounce_key: char_to_key(sk.pounce_key),
                pounce_char: sk.pounce_key,
                turn_delay_ms: sk.turn_delay_ms,
                require_ability_ready: sk.require_ability_ready,
            },
            mirana_enabled: state.selected_hero == Some(crate::state::HeroType::Mirana),
            mirana: MiranaKeyboardSnapshot {
                enabled: mi.enabled,
                leap_key: char_to_key(mi.leap_key),
                leap_char: mi.leap_key,
                turn_delay_ms: mi.turn_delay_ms,
                require_ability_ready: mi.require_ability_ready,
            },
            earth_spirit_enabled: state.selected_hero == Some(crate::state::HeroType::EarthSpirit),
            earth_spirit: EarthSpiritKeyboardSnapshot {
                enabled: es.enabled,
                remnant_char: es.remnant_key,
                silence_combo_enabled: es.silence_combo_enabled,
                grip_key: char_to_key(es.grip_key),
                grip_char: es.grip_key,
                silence_remnant_delay_ms: es.silence_remnant_delay_ms,
                require_grip_ready: es.require_grip_ready,
                roll_combo_enabled: es.roll_combo_enabled,
                roll_key: char_to_key(es.roll_key),
                roll_char: es.roll_key,
                roll_double_tap: es.roll_double_tap,
                roll_double_tap_delay_ms: es.roll_double_tap_delay_ms,
                roll_to_remnant_delay_ms: es.roll_to_remnant_delay_ms,
                roll_remnant_alt: es.roll_remnant_alt,
                roll_remnant_double_tap: es.roll_remnant_double_tap,
                roll_remnant_double_tap_delay_ms: es.roll_remnant_double_tap_delay_ms,
                require_roll_ready: es.require_roll_ready,
            },
            ember_spirit_enabled: state.selected_hero == Some(crate::state::HeroType::EmberSpirit),
            ember_spirit: EmberSpiritKeyboardSnapshot {
                sleight_activate_enabled: em.sleight_activate_enabled,
                sleight_key: char_to_key(em.sleight_key),
                activate_char: em.activate_key,
                sleight_activate_delay_ms: em.sleight_activate_delay_ms,
            },
            // Gated the same way the hotkey handler used to be: Largo selected
            // and standalone control on. Swallowing a song key for a hero the
            // operator is not playing would eat an ordinary ability press.
            largo_enabled: state.selected_hero == Some(crate::state::HeroType::Largo)
                && state.standalone_enabled,
            largo: LargoKeyboardSnapshot {
                bullbelly_key: char_to_key(lg.q_ability_key),
                hotfeet_key: char_to_key(lg.w_ability_key),
                island_elixir_key: char_to_key(lg.e_ability_key),
                ultimate_key: char_to_key(lg.r_ability_key),
            },
            soul_ring: SoulRingKeyboardConfig::from_settings(settings),
            invoker_profiles: settings
                .heroes
                .invoker
                .profiles
                .iter()
                .map(|profile| InvokerHotkeyProfileSnapshot {
                    id: profile.id.clone(),
                    hotkey: parse_key(&profile.hotkey),
                    mode: profile.mode.clone(),
                    enabled: profile.enabled,
                })
                .collect(),
        }
    }
}

impl Default for KeyboardSnapshot {
    fn default() -> Self {
        Self {
            selected_hero: None,
            trigger_key: None,
            meepo_farm_toggle_key: None,
            armlet_roshan_toggle_key: None,
            wave_overlay_toggle_key: None,
            hud_capture_portrait_key: None,
            invoker_cycle_hotkey: None,
            sf_enabled: false,
            od_enabled: false,
            shadow_fiend: ShadowFiendKeyboardSnapshot {
                raze_intercept_enabled: false,
                auto_bkb_on_ultimate: false,
                raze_delay_ms: 0,
                auto_d_on_ultimate: false,
            },
            outworld_destroyer: OutworldDestroyerKeyboardSnapshot {
                ultimate_intercept_enabled: false,
                astral_self_cast_enabled: false,
                astral_self_cast_key: None,
                combo_config: build_keyboard_combo_config(&Settings::default()),
            },
            broodmother: BroodmotherKeyboardSnapshot {
                spider_micro_enabled: false,
                auto_items_enabled: false,
                spider_micro_key: None,
                hero_reselect_key: None,
                auto_items: vec![],
                auto_abilities: vec![],
                abilities_first: false,
                slot_keys: ['a', 's', 'd', 'f', 'g', 'h'].map(KeyBinding::Char),
            },
            snapfire_enabled: false,
            snapfire: SnapfireKeyboardSnapshot {
                enabled: false,
                trigger_key: None,
                cookie_key: 'w',
                turn_delay_ms: 0,
            },
            magnus_enabled: false,
            magnus: MagnusKeyboardSnapshot {
                enabled: false,
                ultimate_key: None,
                ultimate_char: 'r',
                turn_delay_ms: 0,
                require_ability_ready: true,
                center_camera_on_ultimate: false,
                camera_center_key: None,
                camera_center_delay_ms: 0,
            },
            slark_enabled: false,
            slark: SlarkKeyboardSnapshot {
                enabled: false,
                pounce_key: None,
                pounce_char: 'w',
                turn_delay_ms: 0,
                require_ability_ready: true,
            },
            mirana_enabled: false,
            mirana: MiranaKeyboardSnapshot {
                enabled: false,
                leap_key: None,
                leap_char: 'e',
                turn_delay_ms: 0,
                require_ability_ready: true,
            },
            earth_spirit_enabled: false,
            earth_spirit: EarthSpiritKeyboardSnapshot {
                enabled: false,
                remnant_char: 'd',
                silence_combo_enabled: false,
                grip_key: None,
                grip_char: 'e',
                silence_remnant_delay_ms: 0,
                require_grip_ready: true,
                roll_combo_enabled: false,
                roll_key: None,
                roll_char: 'w',
                roll_double_tap: true,
                roll_double_tap_delay_ms: 0,
                roll_to_remnant_delay_ms: 0,
                roll_remnant_alt: false,
                roll_remnant_double_tap: true,
                roll_remnant_double_tap_delay_ms: 0,
                require_roll_ready: true,
            },
            ember_spirit_enabled: false,
            ember_spirit: EmberSpiritKeyboardSnapshot {
                sleight_activate_enabled: false,
                sleight_key: None,
                activate_char: 'd',
                sleight_activate_delay_ms: 0,
            },
            largo_enabled: false,
            largo: LargoKeyboardSnapshot::default(),
            soul_ring: SoulRingKeyboardConfig::from_settings(&Settings::default()),
            invoker_profiles: vec![],
        }
    }
}

/// Decide whether a key press should be followed by Activate Fire Remnant, and
/// after how long.
///
/// Returns `None` for anything but a fresh Sleight of Fist press while Ember
/// Spirit is active and the toggle is on. Holding the key sends OS autorepeat
/// presses; `held_keys` swallows those until the release clears it, so one
/// physical press gets one activate.
///
/// When Soul Ring is replaying the Sleight press, the Sleight itself reaches
/// the game `delay_before_ability_ms` late, so the activate waits that much
/// longer to keep its gap behind it.
fn plan_ember_sleight_activate(
    key: Key,
    snapshot: &KeyboardSnapshot,
    soul_ring_replays_key: bool,
    held_keys: &mut HashSet<Key>,
) -> Option<u64> {
    let ember = &snapshot.ember_spirit;
    if !snapshot.ember_spirit_enabled
        || !ember.sleight_activate_enabled
        || ember.sleight_key != Some(key)
    {
        return None;
    }

    if !held_keys.insert(key) {
        return None;
    }

    let soul_ring_delay_ms = if soul_ring_replays_key {
        snapshot.soul_ring.delay_before_ability_ms
    } else {
        0
    };
    Some(ember.sleight_activate_delay_ms + soul_ring_delay_ms)
}

fn plan_broodmother_callback_action(
    event_type: &EventType,
    modifier_held: bool,
    broodmother_active: bool,
    snapshot: &KeyboardSnapshot,
) -> Option<BroodmotherCallbackAction> {
    match event_type {
        EventType::ButtonPress(Button::Right)
            if modifier_held && broodmother_active && snapshot.broodmother.auto_items_enabled =>
        {
            Some(BroodmotherCallbackAction::AutoItems {
                slot_keys: snapshot.broodmother.slot_keys.clone(),
                auto_items: snapshot.broodmother.auto_items.clone(),
                auto_abilities: snapshot.broodmother.auto_abilities.clone(),
                abilities_first: snapshot.broodmother.abilities_first,
            })
        }
        EventType::ButtonPress(Button::Middle)
            if broodmother_active && snapshot.broodmother.spider_micro_enabled =>
        {
            Some(BroodmotherCallbackAction::SpiderMicro {
                spider_key: snapshot.broodmother.spider_micro_key,
                hero_key: snapshot.broodmother.hero_reselect_key,
            })
        }
        _ => None,
    }
}

fn plan_global_hotkey_event(key: Key, snapshot: &KeyboardSnapshot) -> Option<HotkeyEvent> {
    if snapshot
        .meepo_farm_toggle_key
        .is_some_and(|toggle_key| key == toggle_key)
    {
        return Some(HotkeyEvent::MeepoFarmToggle);
    }

    if snapshot
        .armlet_roshan_toggle_key
        .is_some_and(|toggle_key| key == toggle_key)
    {
        return Some(HotkeyEvent::ArmletRoshanToggle);
    }

    if snapshot
        .wave_overlay_toggle_key
        .is_some_and(|toggle_key| key == toggle_key)
    {
        return Some(HotkeyEvent::WaveOverlayToggle);
    }

    if snapshot
        .hud_capture_portrait_key
        .is_some_and(|capture_key| key == capture_key)
    {
        return Some(HotkeyEvent::CaptureHudPortrait);
    }

    if snapshot.selected_hero == Some(crate::state::HeroType::Invoker) {
        if snapshot
            .invoker_cycle_hotkey
            .is_some_and(|cycle_key| key == cycle_key)
            && snapshot
                .invoker_profiles
                .iter()
                .any(|profile| profile.enabled && profile.mode == InvokerProfileMode::Combo)
        {
            return Some(HotkeyEvent::InvokerCycleComboProfile);
        }

        if snapshot
            .trigger_key
            .is_some_and(|trigger_key| key == trigger_key)
        {
            return Some(HotkeyEvent::ComboTrigger);
        }

        if let Some(profile) = snapshot
            .invoker_profiles
            .iter()
            .find(|profile| profile.enabled && profile.hotkey == Some(key))
        {
            return Some(HotkeyEvent::InvokerProfile(profile.id.clone()));
        }
    }

    if snapshot
        .trigger_key
        .is_some_and(|trigger_key| key == trigger_key)
    {
        return Some(HotkeyEvent::ComboTrigger);
    }

    None
}

fn plan_global_hotkey_press_event(
    key: Key,
    snapshot: &KeyboardSnapshot,
    pressed_hotkeys: &mut HashSet<Key>,
) -> Option<HotkeyEvent> {
    if pressed_hotkeys.contains(&key) {
        return None;
    }

    let hotkey_event = plan_global_hotkey_event(key, snapshot);
    if hotkey_event.is_some() {
        pressed_hotkeys.insert(key);
    }

    hotkey_event
}

fn handle_global_hotkey_key_release(key: Key, pressed_hotkeys: &mut HashSet<Key>) {
    pressed_hotkeys.remove(&key);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::soul_ring::{SoulRingKeyboardConfig, SoulRingState};
    use crate::state::app_state::{HeroType, QueueMetrics, UpdateCheckState};
    use std::collections::HashSet;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;

    /// Largo on Dota's default binds.
    fn largo_test_snapshot() -> LargoKeyboardSnapshot {
        LargoKeyboardSnapshot {
            bullbelly_key: Some(Key::KeyQ),
            hotfeet_key: Some(Key::KeyW),
            island_elixir_key: Some(Key::KeyE),
            ultimate_key: Some(Key::KeyR),
        }
    }

    #[test]
    fn largo_song_keys_are_captured_only_while_the_ultimate_is_up() {
        let snapshot = largo_test_snapshot();

        assert_eq!(
            plan_largo_key_action(Key::KeyW, &snapshot, true, true),
            Some(LargoKeyAction::SelectSong(Song::Hotfeet))
        );
        // Outside the ultimate these are ordinary abilities. Swallowing one
        // would stop the hero casting it at all.
        assert_eq!(plan_largo_key_action(Key::KeyW, &snapshot, true, false), None);
        // Another hero is selected, or standalone control is off.
        assert_eq!(plan_largo_key_action(Key::KeyW, &snapshot, false, true), None);
    }

    #[test]
    fn largo_intercept_follows_the_configured_binds_not_qwer() {
        let rebound = LargoKeyboardSnapshot {
            bullbelly_key: Some(Key::KeyZ),
            hotfeet_key: Some(Key::KeyX),
            island_elixir_key: Some(Key::KeyC),
            ultimate_key: Some(Key::KeyV),
        };

        assert_eq!(
            plan_largo_key_action(Key::KeyX, &rebound, true, true),
            Some(LargoKeyAction::SelectSong(Song::Hotfeet))
        );
        assert_eq!(
            plan_largo_key_action(Key::KeyV, &rebound, true, true),
            Some(LargoKeyAction::StopUltimate)
        );
        // The physical W is somebody else's bind now.
        assert_eq!(plan_largo_key_action(Key::KeyW, &rebound, true, true), None);
    }

    #[test]
    fn largo_maps_each_song_key_to_its_own_song() {
        let snapshot = largo_test_snapshot();

        assert_eq!(
            plan_largo_key_action(Key::KeyQ, &snapshot, true, true),
            Some(LargoKeyAction::SelectSong(Song::Bullbelly))
        );
        assert_eq!(
            plan_largo_key_action(Key::KeyE, &snapshot, true, true),
            Some(LargoKeyAction::SelectSong(Song::IslandElixir))
        );
        assert_eq!(
            plan_largo_key_action(Key::KeyR, &snapshot, true, true),
            Some(LargoKeyAction::StopUltimate)
        );
        assert_eq!(plan_largo_key_action(Key::KeyD, &snapshot, true, true), None);
    }

    #[test]
    fn largo_ultimate_key_wins_over_a_song_bound_to_the_same_key() {
        // A config that binds a song to the ultimate key would otherwise make
        // the ultimate impossible to end: the press would be swallowed.
        let clashing = LargoKeyboardSnapshot {
            bullbelly_key: Some(Key::KeyR),
            ..largo_test_snapshot()
        };

        assert_eq!(
            plan_largo_key_action(Key::KeyR, &clashing, true, true),
            Some(LargoKeyAction::StopUltimate)
        );
    }

    #[test]
    fn largo_unparseable_binds_capture_nothing() {
        // `char_to_key` returns None for a key we cannot match, and a None bind
        // must never collide with another None bind.
        let unparsed = LargoKeyboardSnapshot::default();

        assert_eq!(plan_largo_key_action(Key::KeyQ, &unparsed, true, true), None);
        assert_eq!(plan_largo_key_action(Key::KeyR, &unparsed, true, true), None);
    }

    fn broodmother_test_snapshot() -> KeyboardSnapshot {
        KeyboardSnapshot {
            selected_hero: None,
            trigger_key: None,
            meepo_farm_toggle_key: None,
            armlet_roshan_toggle_key: None,
            wave_overlay_toggle_key: None,
            hud_capture_portrait_key: None,
            invoker_cycle_hotkey: None,
            sf_enabled: false,
            od_enabled: false,
            shadow_fiend: ShadowFiendKeyboardSnapshot {
                raze_intercept_enabled: false,
                auto_bkb_on_ultimate: false,
                raze_delay_ms: 0,
                auto_d_on_ultimate: false,
            },
            outworld_destroyer: OutworldDestroyerKeyboardSnapshot {
                ultimate_intercept_enabled: false,
                astral_self_cast_enabled: false,
                astral_self_cast_key: None,
                combo_config: build_keyboard_combo_config(&Settings::default()),
            },
            broodmother: BroodmotherKeyboardSnapshot {
                spider_micro_enabled: true,
                auto_items_enabled: true,
                spider_micro_key: Some(Key::KeyQ),
                hero_reselect_key: Some(Key::KeyH),
                auto_items: vec!["item1".to_string(), "item2".to_string()],
                auto_abilities: vec![],
                abilities_first: true,
                slot_keys: ['a', 's', 'd', 'f', 'g', 'h'].map(KeyBinding::Char),
            },
            snapfire_enabled: false,
            snapfire: SnapfireKeyboardSnapshot {
                enabled: false,
                trigger_key: None,
                cookie_key: 'w',
                turn_delay_ms: 0,
            },
            magnus_enabled: false,
            magnus: MagnusKeyboardSnapshot {
                enabled: false,
                ultimate_key: None,
                ultimate_char: 'r',
                turn_delay_ms: 0,
                require_ability_ready: true,
                center_camera_on_ultimate: false,
                camera_center_key: None,
                camera_center_delay_ms: 0,
            },
            slark_enabled: false,
            slark: SlarkKeyboardSnapshot {
                enabled: false,
                pounce_key: None,
                pounce_char: 'w',
                turn_delay_ms: 0,
                require_ability_ready: true,
            },
            mirana_enabled: false,
            mirana: MiranaKeyboardSnapshot {
                enabled: false,
                leap_key: None,
                leap_char: 'e',
                turn_delay_ms: 0,
                require_ability_ready: true,
            },
            earth_spirit_enabled: false,
            earth_spirit: EarthSpiritKeyboardSnapshot {
                enabled: false,
                remnant_char: 'd',
                silence_combo_enabled: false,
                grip_key: None,
                grip_char: 'e',
                silence_remnant_delay_ms: 0,
                require_grip_ready: true,
                roll_combo_enabled: false,
                roll_key: None,
                roll_char: 'w',
                roll_double_tap: true,
                roll_double_tap_delay_ms: 0,
                roll_to_remnant_delay_ms: 0,
                roll_remnant_alt: false,
                roll_remnant_double_tap: true,
                roll_remnant_double_tap_delay_ms: 0,
                require_roll_ready: true,
            },
            ember_spirit_enabled: false,
            ember_spirit: EmberSpiritKeyboardSnapshot {
                sleight_activate_enabled: false,
                sleight_key: None,
                activate_char: 'd',
                sleight_activate_delay_ms: 0,
            },
            largo_enabled: false,
            largo: LargoKeyboardSnapshot::default(),
            soul_ring: SoulRingKeyboardConfig::from_settings(&Settings::default()),
            invoker_profiles: vec![],
        }
    }

    #[test]
    fn keyboard_snapshot_parses_trigger_key_and_sf_flags() {
        let settings = Settings::default();
        let state = AppState {
            selected_hero: Some(HeroType::ShadowFiend),
            gsi_enabled: true,
            standalone_enabled: true,
            last_event: None,
            last_gsi_activity_at: None,
            metrics: QueueMetrics::default(),
            trigger_key: Arc::new(Mutex::new("Home".to_string())),
            sf_enabled: Arc::new(Mutex::new(true)),
            od_enabled: Arc::new(Mutex::new(false)),
            update_state: Arc::new(Mutex::new(UpdateCheckState::Idle)),
            invoker_active_combo_profile_id: None,
            rune_alerts: None,
            minimap_capture: None,
        };

        let snapshot = KeyboardSnapshot::from_runtime(&settings, &state);

        assert_eq!(snapshot.trigger_key, Some(Key::Home));
        assert_eq!(snapshot.meepo_farm_toggle_key, None);
        assert!(snapshot.sf_enabled);
        assert!(snapshot.shadow_fiend.raze_intercept_enabled);
    }

    #[test]
    fn keyboard_snapshot_parses_od_flags_and_astral_hotkey() {
        let settings = Settings::default();
        let state = AppState {
            selected_hero: Some(HeroType::OutworldDestroyer),
            gsi_enabled: true,
            standalone_enabled: true,
            last_event: None,
            last_gsi_activity_at: None,
            metrics: QueueMetrics::default(),
            trigger_key: Arc::new(Mutex::new("Home".to_string())),
            sf_enabled: Arc::new(Mutex::new(false)),
            od_enabled: Arc::new(Mutex::new(true)),
            update_state: Arc::new(Mutex::new(UpdateCheckState::Idle)),
            invoker_active_combo_profile_id: None,
            rune_alerts: None,
            minimap_capture: None,
        };

        let snapshot = KeyboardSnapshot::from_runtime(&settings, &state);

        assert!(snapshot.od_enabled);
        assert!(snapshot.outworld_destroyer.ultimate_intercept_enabled);
        assert_eq!(
            snapshot.outworld_destroyer.astral_self_cast_key,
            Some(Key::F5)
        );
    }

    #[test]
    fn keyboard_snapshot_handles_invalid_trigger_key() {
        let state = AppState::default();
        *state.trigger_key.lock().unwrap() = "not-a-key".to_string();

        let snapshot = KeyboardSnapshot::from_runtime(&Settings::default(), &state);

        assert_eq!(snapshot.trigger_key, None);
        assert_eq!(snapshot.meepo_farm_toggle_key, None);
    }

    #[test]
    fn keyboard_snapshot_sf_disabled_by_default() {
        let state = AppState::default();
        let snapshot = KeyboardSnapshot::from_runtime(&Settings::default(), &state);
        assert!(!snapshot.sf_enabled);
        assert!(!snapshot.od_enabled);
        assert_eq!(snapshot.meepo_farm_toggle_key, None);
    }

    #[test]
    fn keyboard_snapshot_parses_f5_trigger_key() {
        let state = AppState::default();
        *state.trigger_key.lock().unwrap() = "F5".to_string();
        let snapshot = KeyboardSnapshot::from_runtime(&Settings::default(), &state);
        assert_eq!(snapshot.trigger_key, Some(Key::F5));
    }

    #[test]
    fn parse_key_string_parses_space() {
        assert_eq!(parse_key_string("Space"), Some(Key::Space));
        assert_eq!(parse_key_string("space"), Some(Key::Space));
    }

    #[test]
    fn keyboard_snapshot_populates_snapfire_fields_for_snapfire() {
        let mut state = AppState::default();
        state.selected_hero = Some(HeroType::Snapfire);
        let snapshot = KeyboardSnapshot::from_runtime(&Settings::default(), &state);

        assert!(snapshot.snapfire_enabled);
        assert!(snapshot.snapfire.enabled);
        assert_eq!(snapshot.snapfire.trigger_key, Some(Key::Space));
        assert_eq!(snapshot.snapfire.cookie_key, 'w');

        let other = KeyboardSnapshot::from_runtime(&Settings::default(), &AppState::default());
        assert!(!other.snapfire_enabled);
    }

    #[test]
    fn keyboard_snapshot_populates_magnus_fields_for_magnus() {
        let mut state = AppState::default();
        state.selected_hero = Some(HeroType::Magnus);
        let snapshot = KeyboardSnapshot::from_runtime(&Settings::default(), &state);

        assert!(snapshot.magnus_enabled);
        assert!(snapshot.magnus.enabled);
        assert_eq!(snapshot.magnus.ultimate_key, Some(Key::KeyR));
        assert_eq!(snapshot.magnus.ultimate_char, 'r');
        assert!(snapshot.magnus.require_ability_ready);
        assert!(snapshot.magnus.center_camera_on_ultimate);
        assert_eq!(snapshot.magnus.camera_center_key, Some(Key::Num1));
        assert_eq!(snapshot.magnus.camera_center_delay_ms, 60);

        let other = KeyboardSnapshot::from_runtime(&Settings::default(), &AppState::default());
        assert!(!other.magnus_enabled);
    }

    #[test]
    fn hud_portrait_capture_key_is_parsed_and_plans_its_event() {
        let snapshot = KeyboardSnapshot::from_runtime(&Settings::default(), &AppState::default());
        assert_eq!(snapshot.hud_capture_portrait_key, Some(Key::F9));

        let mut pressed = HashSet::new();
        assert_eq!(
            plan_global_hotkey_press_event(Key::F9, &snapshot, &mut pressed),
            Some(HotkeyEvent::CaptureHudPortrait)
        );
    }

    #[test]
    fn keyboard_snapshot_populates_mirana_fields_for_mirana() {
        let mut state = AppState::default();
        state.selected_hero = Some(HeroType::Mirana);
        let snapshot = KeyboardSnapshot::from_runtime(&Settings::default(), &state);

        assert!(snapshot.mirana_enabled);
        assert!(snapshot.mirana.enabled);
        assert_eq!(snapshot.mirana.leap_key, Some(Key::KeyE));
        assert_eq!(snapshot.mirana.leap_char, 'e');
        assert_eq!(snapshot.mirana.turn_delay_ms, 200);
        assert!(snapshot.mirana.require_ability_ready);

        let other = KeyboardSnapshot::from_runtime(&Settings::default(), &AppState::default());
        assert!(!other.mirana_enabled);
    }

    #[test]
    fn keyboard_snapshot_populates_earth_spirit_fields_for_earth_spirit() {
        let mut state = AppState::default();
        state.selected_hero = Some(HeroType::EarthSpirit);
        let snapshot = KeyboardSnapshot::from_runtime(&Settings::default(), &state);

        assert!(snapshot.earth_spirit_enabled);
        assert!(snapshot.earth_spirit.enabled);
        assert_eq!(snapshot.earth_spirit.remnant_char, 'd');

        assert!(snapshot.earth_spirit.silence_combo_enabled);
        assert_eq!(snapshot.earth_spirit.grip_key, Some(Key::KeyE));
        assert_eq!(snapshot.earth_spirit.grip_char, 'e');
        assert_eq!(snapshot.earth_spirit.silence_remnant_delay_ms, 120);
        assert!(snapshot.earth_spirit.require_grip_ready);

        assert!(snapshot.earth_spirit.roll_combo_enabled);
        assert_eq!(snapshot.earth_spirit.roll_key, Some(Key::KeyW));
        assert_eq!(snapshot.earth_spirit.roll_char, 'w');
        assert!(snapshot.earth_spirit.roll_double_tap);
        assert_eq!(snapshot.earth_spirit.roll_double_tap_delay_ms, 60);
        assert_eq!(snapshot.earth_spirit.roll_to_remnant_delay_ms, 120);
        assert!(snapshot.earth_spirit.roll_remnant_alt);
        assert!(snapshot.earth_spirit.roll_remnant_double_tap);
        assert_eq!(snapshot.earth_spirit.roll_remnant_double_tap_delay_ms, 60);
        assert!(snapshot.earth_spirit.require_roll_ready);

        let other = KeyboardSnapshot::from_runtime(&Settings::default(), &AppState::default());
        assert!(!other.earth_spirit_enabled);
    }

    #[test]
    fn keyboard_snapshot_populates_ember_spirit_fields_for_ember_spirit() {
        let mut settings = Settings::default();
        settings.heroes.ember_spirit.sleight_activate_enabled = true;
        let mut state = AppState::default();
        state.selected_hero = Some(HeroType::EmberSpirit);
        let snapshot = KeyboardSnapshot::from_runtime(&settings, &state);

        assert!(snapshot.ember_spirit_enabled);
        assert!(snapshot.ember_spirit.sleight_activate_enabled);
        assert_eq!(snapshot.ember_spirit.sleight_key, Some(Key::KeyW));
        assert_eq!(snapshot.ember_spirit.activate_char, 'd');
        assert_eq!(snapshot.ember_spirit.sleight_activate_delay_ms, 100);

        let other = KeyboardSnapshot::from_runtime(&settings, &AppState::default());
        assert!(!other.ember_spirit_enabled);
    }

    fn ember_sleight_snapshot() -> KeyboardSnapshot {
        let mut settings = Settings::default();
        settings.heroes.ember_spirit.sleight_activate_enabled = true;
        let mut state = AppState::default();
        state.selected_hero = Some(HeroType::EmberSpirit);
        KeyboardSnapshot::from_runtime(&settings, &state)
    }

    #[test]
    fn ember_sleight_press_schedules_the_activate_after_the_configured_delay() {
        let snapshot = ember_sleight_snapshot();
        let mut held = HashSet::new();

        assert_eq!(
            plan_ember_sleight_activate(Key::KeyW, &snapshot, false, &mut held),
            Some(100)
        );
    }

    #[test]
    fn ember_sleight_activate_ignores_other_keys() {
        let snapshot = ember_sleight_snapshot();
        let mut held = HashSet::new();

        assert_eq!(
            plan_ember_sleight_activate(Key::KeyQ, &snapshot, false, &mut held),
            None
        );
    }

    #[test]
    fn ember_sleight_activate_holds_when_the_toggle_is_off() {
        let mut snapshot = ember_sleight_snapshot();
        snapshot.ember_spirit.sleight_activate_enabled = false;
        let mut held = HashSet::new();

        assert_eq!(
            plan_ember_sleight_activate(Key::KeyW, &snapshot, false, &mut held),
            None
        );
    }

    /// W is Pounce, Cookie and Rolling Boulder on other heroes — the toggle
    /// must not follow those with a D.
    #[test]
    fn ember_sleight_activate_holds_for_other_heroes() {
        let mut snapshot = ember_sleight_snapshot();
        snapshot.ember_spirit_enabled = false;
        let mut held = HashSet::new();

        assert_eq!(
            plan_ember_sleight_activate(Key::KeyW, &snapshot, false, &mut held),
            None
        );
    }

    /// Holding W sends autorepeat presses; one physical press gets one
    /// activate.
    #[test]
    fn ember_sleight_activate_ignores_autorepeat_until_release() {
        let snapshot = ember_sleight_snapshot();
        let mut held = HashSet::new();

        assert!(plan_ember_sleight_activate(Key::KeyW, &snapshot, false, &mut held).is_some());
        assert!(plan_ember_sleight_activate(Key::KeyW, &snapshot, false, &mut held).is_none());

        held.remove(&Key::KeyW);

        assert!(plan_ember_sleight_activate(Key::KeyW, &snapshot, false, &mut held).is_some());
    }

    /// Soul Ring replays the Sleight press late, so the activate waits the
    /// same extra time to stay behind it.
    #[test]
    fn ember_sleight_activate_waits_out_a_soul_ring_replay() {
        let snapshot = ember_sleight_snapshot();
        let mut held = HashSet::new();

        assert_eq!(
            plan_ember_sleight_activate(Key::KeyW, &snapshot, true, &mut held),
            Some(100 + snapshot.soul_ring.delay_before_ability_ms)
        );
    }

    #[test]
    fn keyboard_snapshot_populates_slark_fields_for_slark() {
        let mut state = AppState::default();
        state.selected_hero = Some(HeroType::Slark);
        let snapshot = KeyboardSnapshot::from_runtime(&Settings::default(), &state);

        assert!(snapshot.slark_enabled);
        assert!(snapshot.slark.enabled);
        assert_eq!(snapshot.slark.pounce_key, Some(Key::KeyW));
        assert_eq!(snapshot.slark.pounce_char, 'w');
        assert_eq!(snapshot.slark.turn_delay_ms, 200);
        assert!(snapshot.slark.require_ability_ready);

        let other = KeyboardSnapshot::from_runtime(&Settings::default(), &AppState::default());
        assert!(!other.slark_enabled);
    }

    #[test]
    fn keyboard_snapshot_exposes_meepo_farm_toggle_key_only_for_meepo() {
        let settings = Settings::default();
        let mut meepo_state = AppState::default();
        meepo_state.selected_hero = Some(HeroType::Meepo);

        let meepo_snapshot = KeyboardSnapshot::from_runtime(&settings, &meepo_state);
        assert_eq!(meepo_snapshot.meepo_farm_toggle_key, Some(Key::End));

        let other_snapshot = KeyboardSnapshot::from_runtime(&settings, &AppState::default());
        assert_eq!(other_snapshot.meepo_farm_toggle_key, None);
    }

    #[test]
    fn meepo_farm_toggle_key_plans_hotkey_event() {
        let mut snapshot = broodmother_test_snapshot();
        snapshot.meepo_farm_toggle_key = Some(Key::End);

        assert!(matches!(
            plan_global_hotkey_event(Key::End, &snapshot),
            Some(HotkeyEvent::MeepoFarmToggle)
        ));
    }

    #[test]
    fn keyboard_snapshot_parses_armlet_roshan_toggle_key_when_enabled() {
        let mut settings = Settings::default();
        settings.armlet.roshan.enabled = true;
        settings.armlet.roshan.toggle_key = "Insert".to_string();

        let snapshot = KeyboardSnapshot::from_runtime(&settings, &AppState::default());

        assert_eq!(snapshot.armlet_roshan_toggle_key, Some(Key::Insert));
    }

    #[test]
    fn armlet_roshan_toggle_key_plans_hotkey_event() {
        let mut snapshot = broodmother_test_snapshot();
        snapshot.armlet_roshan_toggle_key = Some(Key::Insert);

        assert!(matches!(
            plan_global_hotkey_event(Key::Insert, &snapshot),
            Some(HotkeyEvent::ArmletRoshanToggle)
        ));
    }

    #[test]
    fn wave_overlay_toggle_key_plans_hotkey_event() {
        let mut snapshot = broodmother_test_snapshot();
        snapshot.wave_overlay_toggle_key = Some(Key::F8);

        assert!(matches!(
            plan_global_hotkey_event(Key::F8, &snapshot),
            Some(HotkeyEvent::WaveOverlayToggle)
        ));
    }

    #[test]
    fn wave_overlay_toggle_key_is_only_parsed_when_the_overlay_is_enabled() {
        let mut settings = Settings::default();
        settings.wave_overlay.toggle_key = "F8".to_string();

        settings.wave_overlay.enabled = false;
        let disabled = KeyboardSnapshot::from_runtime(&settings, &AppState::default());
        assert_eq!(disabled.wave_overlay_toggle_key, None);

        settings.wave_overlay.enabled = true;
        let enabled = KeyboardSnapshot::from_runtime(&settings, &AppState::default());
        assert_eq!(enabled.wave_overlay_toggle_key, Some(Key::F8));
    }

    #[test]
    fn keyboard_snapshot_maps_invoker_profile_modes_and_cycle_hotkey() {
        let mut settings = Settings::default();
        settings.heroes.invoker.cycle_combo_profiles_hotkey = "F10".to_string();
        let snapshot = KeyboardSnapshot::from_runtime(&settings, &AppState::default());

        assert_eq!(snapshot.invoker_cycle_hotkey, Some(Key::F10));
        assert_eq!(
            snapshot
                .invoker_profiles
                .iter()
                .map(|profile| (&profile.id, &profile.mode))
                .collect::<Vec<_>>(),
            settings
                .heroes
                .invoker
                .profiles
                .iter()
                .map(|profile| (&profile.id, &profile.mode))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn keyboard_snapshot_default_keeps_invoker_cycle_hotkey_empty() {
        assert_eq!(KeyboardSnapshot::default().invoker_cycle_hotkey, None);
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
            item_slot_keys: ['z', 'x', 'c', 'v', 'b', 'n'].map(KeyBinding::Char).into_iter().collect(),
        }
    }

    #[test]
    fn eligible_soul_ring_replay_plan_triggers_before_original_key() {
        let mut state = SoulRingState::default();
        state.available = true;
        state.can_cast = true;
        state.slot_key = Some(KeyBinding::Char('z'));
        state.hero_alive = true;
        state.hero_mana_percent = 10;
        state.hero_health_percent = 90;
        // Q must be bound to something that actually spends mana; Starstorm costs 80.
        state.ability_slots = vec![crate::actions::soul_ring::AbilitySlot {
            name: "mirana_starfall".to_string(),
            level: 1,
            passive: false,
        }];

        let config = soul_ring_test_config();
        let plan = crate::input::keyboard::plan_soul_ring_replay(&state, Key::KeyQ, &config);

        match plan {
            crate::input::keyboard::SoulRingReplayPlan::TriggerThenOriginal {
                soul_ring_key,
                delay_ms,
                original_key,
            } => {
                assert_eq!(soul_ring_key, SimulatedInput::Key(Key::KeyZ));
                assert_eq!(delay_ms, config.delay_before_ability_ms);
                assert_eq!(original_key, Key::KeyQ);
            }
            _ => panic!("Expected TriggerThenOriginal plan"),
        }
    }

    /// Soul Ring sits in an item slot, and an item slot can be bound to a thumb
    /// mouse button. The plan has to carry that through, or the trigger is
    /// silently skipped for anyone who binds items to their mouse (issue #20).
    #[test]
    fn soul_ring_replay_plan_handles_a_mouse_bound_soul_ring_slot() {
        let mut state = SoulRingState::default();
        state.available = true;
        state.can_cast = true;
        state.slot_key = Some(KeyBinding::parse("Mouse4"));
        state.hero_alive = true;
        state.hero_mana_percent = 10;
        state.hero_health_percent = 90;
        state.ability_slots = vec![crate::actions::soul_ring::AbilitySlot {
            name: "mirana_starfall".to_string(),
            level: 1,
            passive: false,
        }];

        let config = soul_ring_test_config();
        let plan = crate::input::keyboard::plan_soul_ring_replay(&state, Key::KeyQ, &config);

        match plan {
            crate::input::keyboard::SoulRingReplayPlan::TriggerThenOriginal {
                soul_ring_key,
                original_key,
                ..
            } => {
                assert_eq!(soul_ring_key, SimulatedInput::Button(Button::Unknown(1)));
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
        state.slot_key = Some(KeyBinding::Char('z'));
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
        state.slot_key = Some(KeyBinding::Char('?'));
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

    #[test]
    fn broodmother_space_right_click_plans_auto_items_action() {
        let snapshot = broodmother_test_snapshot();
        let action = plan_broodmother_callback_action(
            &EventType::ButtonPress(Button::Right),
            true,
            true,
            &snapshot,
        );

        match action {
            Some(BroodmotherCallbackAction::AutoItems {
                slot_keys,
                auto_items,
                auto_abilities,
                abilities_first,
            }) => {
                assert_eq!(slot_keys, snapshot.broodmother.slot_keys);
                assert_eq!(auto_items, snapshot.broodmother.auto_items);
                assert_eq!(
                    auto_abilities.len(),
                    snapshot.broodmother.auto_abilities.len()
                );
                assert_eq!(abilities_first, snapshot.broodmother.abilities_first);
            }
            _ => panic!("expected AutoItems action"),
        }
    }

    #[test]
    fn broodmother_middle_mouse_plans_spider_micro_action() {
        let snapshot = broodmother_test_snapshot();
        let action = plan_broodmother_callback_action(
            &EventType::ButtonPress(Button::Middle),
            false,
            true,
            &snapshot,
        );

        match action {
            Some(BroodmotherCallbackAction::SpiderMicro {
                spider_key,
                hero_key,
            }) => {
                assert_eq!(spider_key, snapshot.broodmother.spider_micro_key);
                assert_eq!(hero_key, snapshot.broodmother.hero_reselect_key);
            }
            _ => panic!("expected SpiderMicro action"),
        }
    }

    #[test]
    fn broodmother_callback_action_is_none_when_disabled_or_inactive() {
        let mut auto_items_disabled_snapshot = broodmother_test_snapshot();
        auto_items_disabled_snapshot.broodmother.auto_items_enabled = false;

        match plan_broodmother_callback_action(
            &EventType::ButtonPress(Button::Right),
            true,
            true,
            &auto_items_disabled_snapshot,
        ) {
            None => {}
            Some(action) => panic!("expected None for disabled auto-items, got {:?}", action),
        }

        let mut spider_micro_disabled_snapshot = broodmother_test_snapshot();
        spider_micro_disabled_snapshot
            .broodmother
            .spider_micro_enabled = false;

        match plan_broodmother_callback_action(
            &EventType::ButtonPress(Button::Middle),
            false,
            true,
            &spider_micro_disabled_snapshot,
        ) {
            None => {}
            Some(action) => panic!("expected None for disabled spider micro, got {:?}", action),
        }

        let mut broodmother_inactive_snapshot = broodmother_test_snapshot();
        broodmother_inactive_snapshot
            .broodmother
            .spider_micro_enabled = true;

        match plan_broodmother_callback_action(
            &EventType::ButtonPress(Button::Middle),
            false,
            false,
            &broodmother_inactive_snapshot,
        ) {
            None => {}
            Some(action) => panic!("expected None for inactive Broodmother, got {:?}", action),
        }
    }

    #[test]
    fn broodmother_callback_action_guard_runs_non_panicking_action() {
        let ran = Arc::new(AtomicBool::new(false));
        let ran_clone = ran.clone();

        execute_broodmother_callback_action_with_guard("", move || {
            ran_clone.store(true, Ordering::SeqCst);
        });

        assert!(ran.load(Ordering::SeqCst));
    }

    #[test]
    fn broodmother_callback_action_guard_swallows_panics() {
        let result = std::panic::catch_unwind(|| {
            execute_broodmother_callback_action_with_guard("", || {
                panic!("boom");
            });
        });

        assert!(result.is_ok());
    }

    #[test]
    fn plan_global_hotkey_event_maps_invoker_profile_hotkeys() {
        let mut snapshot = KeyboardSnapshot::default();
        snapshot.selected_hero = Some(HeroType::Invoker);
        snapshot.trigger_key = Some(Key::End);
        snapshot.invoker_profiles = vec![InvokerHotkeyProfileSnapshot {
            id: "qw-pickoff".to_string(),
            hotkey: Some(parse_key_string("Home").unwrap()),
            mode: crate::config::settings::InvokerProfileMode::Combo,
            enabled: true,
        }];

        assert_eq!(
            plan_global_hotkey_event(rdev::Key::Home, &snapshot),
            Some(HotkeyEvent::InvokerProfile("qw-pickoff".to_string()))
        );
    }

    #[test]
    fn plan_global_hotkey_event_prefers_combo_trigger_over_invoker_profile_on_shared_key() {
        let mut snapshot = KeyboardSnapshot::default();
        snapshot.selected_hero = Some(HeroType::Invoker);
        snapshot.trigger_key = Some(Key::Home);
        snapshot.invoker_profiles = vec![InvokerHotkeyProfileSnapshot {
            id: "qw-pickoff".to_string(),
            hotkey: Some(Key::Home),
            mode: crate::config::settings::InvokerProfileMode::Combo,
            enabled: true,
        }];

        assert_eq!(
            plan_global_hotkey_event(Key::Home, &snapshot),
            Some(HotkeyEvent::ComboTrigger)
        );
    }

    #[test]
    fn plan_global_hotkey_event_cycles_invoker_combo_profiles_only_for_invoker() {
        let mut snapshot = KeyboardSnapshot::default();
        snapshot.selected_hero = Some(HeroType::Invoker);
        snapshot.invoker_cycle_hotkey = Some(Key::F10);
        snapshot.invoker_profiles = vec![
            InvokerHotkeyProfileSnapshot {
                id: "prep".to_string(),
                hotkey: Some(Key::Insert),
                mode: crate::config::settings::InvokerProfileMode::Prep,
                enabled: true,
            },
            InvokerHotkeyProfileSnapshot {
                id: "combo".to_string(),
                hotkey: Some(Key::Home),
                mode: crate::config::settings::InvokerProfileMode::Combo,
                enabled: true,
            },
        ];

        assert_eq!(
            plan_global_hotkey_event(Key::F10, &snapshot),
            Some(HotkeyEvent::InvokerCycleComboProfile)
        );
    }

    #[test]
    fn plan_global_hotkey_event_ignores_invoker_cycle_hotkey_for_other_heroes() {
        let mut snapshot = KeyboardSnapshot::default();
        snapshot.selected_hero = Some(HeroType::ShadowFiend);
        snapshot.invoker_cycle_hotkey = Some(Key::F10);
        snapshot.invoker_profiles = vec![InvokerHotkeyProfileSnapshot {
            id: "combo".to_string(),
            hotkey: Some(Key::Home),
            mode: crate::config::settings::InvokerProfileMode::Combo,
            enabled: true,
        }];

        assert_eq!(plan_global_hotkey_event(Key::F10, &snapshot), None);
    }

    #[test]
    fn plan_global_hotkey_event_ignores_invoker_cycle_hotkey_without_enabled_combo_profiles() {
        let mut snapshot = KeyboardSnapshot::default();
        snapshot.selected_hero = Some(HeroType::Invoker);
        snapshot.invoker_cycle_hotkey = Some(Key::F10);
        snapshot.invoker_profiles = vec![InvokerHotkeyProfileSnapshot {
            id: "prep".to_string(),
            hotkey: Some(Key::Insert),
            mode: crate::config::settings::InvokerProfileMode::Prep,
            enabled: true,
        }];

        assert_eq!(plan_global_hotkey_event(Key::F10, &snapshot), None);
    }

    #[test]
    fn plan_global_hotkey_press_event_ignores_repeated_combo_press_until_release() {
        let mut snapshot = KeyboardSnapshot::default();
        snapshot.selected_hero = Some(HeroType::Invoker);
        snapshot.trigger_key = Some(Key::Home);
        let mut pressed_hotkeys = HashSet::new();

        assert_eq!(
            plan_global_hotkey_press_event(Key::Home, &snapshot, &mut pressed_hotkeys),
            Some(HotkeyEvent::ComboTrigger)
        );
        assert_eq!(
            plan_global_hotkey_press_event(Key::Home, &snapshot, &mut pressed_hotkeys),
            None
        );

        handle_global_hotkey_key_release(Key::Home, &mut pressed_hotkeys);

        assert_eq!(
            plan_global_hotkey_press_event(Key::Home, &snapshot, &mut pressed_hotkeys),
            Some(HotkeyEvent::ComboTrigger)
        );
    }

    #[test]
    fn plan_global_hotkey_press_event_ignores_repeated_invoker_profile_press_until_release() {
        let mut snapshot = KeyboardSnapshot::default();
        snapshot.selected_hero = Some(HeroType::Invoker);
        snapshot.invoker_profiles = vec![InvokerHotkeyProfileSnapshot {
            id: "qw-pickoff".to_string(),
            hotkey: Some(Key::End),
            mode: crate::config::settings::InvokerProfileMode::Combo,
            enabled: true,
        }];
        let mut pressed_hotkeys = HashSet::new();

        assert_eq!(
            plan_global_hotkey_press_event(Key::End, &snapshot, &mut pressed_hotkeys),
            Some(HotkeyEvent::InvokerProfile("qw-pickoff".to_string()))
        );
        assert_eq!(
            plan_global_hotkey_press_event(Key::End, &snapshot, &mut pressed_hotkeys),
            None
        );

        handle_global_hotkey_key_release(Key::End, &mut pressed_hotkeys);

        assert_eq!(
            plan_global_hotkey_press_event(Key::End, &snapshot, &mut pressed_hotkeys),
            Some(HotkeyEvent::InvokerProfile("qw-pickoff".to_string()))
        );
    }
}
