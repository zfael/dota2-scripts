//! Phantom Lancer: the Ctrl latch and the illusion send.
//!
//! Dota's "Unified Orders with Ctrl" sends an order to every unit the player
//! controls while Ctrl is held. Phantom Lancer controls a crowd of illusions,
//! so playing him well means holding Ctrl on nearly every right-click. The
//! latch holds it for the player: a tap of Ctrl turns it on, and from then on
//! every right-click reaches Dota inside an injected Ctrl. Another tap turns it
//! off. Holding Ctrl, or pressing it together with anything else, is the
//! player's own Ctrl and toggles nothing.
//!
//! The illusion send is the other half: one button (middle mouse by default)
//! selects every other unit, right-clicks at the cursor, and gives the hero
//! back — the illusions go, the hero stays.
//!
//! **Why Ctrl is not simply held down.** Ctrl + an ability key levels the
//! ability instead of casting it, and Ctrl + a number key reassigns that
//! control group. So the injected Ctrl only ever spans the replayed click, and
//! any key or click the player makes while it is down waits behind it.
//!
//! ## Evidence
//!
//! - `examples/phantom_lancer_input_probe.rs` measured it in game: 12 clean
//!   taps at 81–121ms, every manual hold read as a chord, and 181 latched
//!   right-clicks at a 15ms Ctrl gap with every illusion following and nothing
//!   levelled. In the app, the illusion send's two selections landed but a
//!   left-click between them ordered nothing — in Dota a left-click only
//!   selects — so the order is a right-click.
//! - `examples/phantom_lancer_control.rs` proves the rules below never leave
//!   Ctrl under a key, and that each safeguard is load-bearing.
//!
//! None of those numbers appears in the logic: they are config.
//!
//! ## Rules
//!
//! 1. A clean Ctrl tap (nothing else pressed while it is down, released within
//!    `latch_tap_ms`) toggles the latch. Ctrl itself is never blocked — a bare
//!    Ctrl does nothing in Dota, and a tap is only known once it is released.
//! 2. While latched, a right-click is swallowed and replayed as
//!    Ctrl down → gap → click → gap → Ctrl up. Its physical release is
//!    swallowed too.
//! 3. While the injector holds Ctrl or has work queued, every physical key and
//!    click is held back and replayed behind it, in order. A key pressed in
//!    that window would otherwise reach Dota under our Ctrl.
//! 4. With Ctrl physically held, clicks pass untouched: the player's own Ctrl
//!    already does the job.
//! 5. A key the injector presses for the illusion send lifts the player's own
//!    Ctrl around itself, or the hero-select key becomes "assign group 1".
//! 6. Death, another hero, the latch disabled, or an illusion send turns the
//!    latch off.
//!
//! ## Telling our input from the player's
//!
//! The app's `SIMULATING_KEYS` guard passes *every* event while it is set,
//! physical ones included — exactly the window rule 3 has to catch. So the
//! injector here does not use it. It records each event it is about to send,
//! and the hook recognises those echoes by matching the front of that queue.

use crate::actions::common::SurvivabilityActions;
use crate::actions::executor::ActionExecutor;
use crate::actions::heroes::HeroScript;
use crate::audio::motif::{Motif, Timbre, Tone};
use crate::config::Settings;
use crate::input::binding::KeyBinding;
use crate::models::{GsiWebhookEvent, Hero};
use rdev::{Button, EventType, Key};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, LazyLock, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// How long a synthetic key or button is held between its press and release.
const PRESS_HOLD_MS: u64 = 5;

/// An echo that has not come back in this long never will; dropping it keeps
/// a lost one from mistaking a later physical event for ours.
const ECHO_TIMEOUT: Duration = Duration::from_millis(500);

/// The latch's keyboard-facing config, pre-read into the keyboard snapshot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LatchConfig {
    pub tap_ms: u64,
    pub ctrl_gap_ms: u64,
    pub cue_enabled: bool,
    pub cue_volume: f32,
}

impl Default for LatchConfig {
    fn default() -> Self {
        Self {
            tap_ms: 200,
            ctrl_gap_ms: 15,
            cue_enabled: true,
            cue_volume: 0.5,
        }
    }
}

/// A key or a mouse button, resolved from a [`KeyBinding`] once per snapshot
/// so the hook compares plain values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressInput {
    Key(Key),
    Button(Button),
}

impl PressInput {
    pub fn from_binding(binding: &KeyBinding) -> Option<Self> {
        binding
            .rdev_key()
            .map(PressInput::Key)
            .or_else(|| binding.rdev_button().map(PressInput::Button))
    }

    fn press(self) -> EventType {
        match self {
            PressInput::Key(key) => EventType::KeyPress(key),
            PressInput::Button(button) => EventType::ButtonPress(button),
        }
    }

    fn release(self) -> EventType {
        match self {
            PressInput::Key(key) => EventType::KeyRelease(key),
            PressInput::Button(button) => EventType::ButtonRelease(button),
        }
    }
}

/// The illusion send's keyboard-facing config. Every input is the player's
/// own Dota bind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IllusionSendConfig {
    pub trigger: PressInput,
    pub select_others: PressInput,
    pub select_hero: PressInput,
    pub step_ms: u64,
}

impl Default for IllusionSendConfig {
    fn default() -> Self {
        Self {
            trigger: PressInput::Button(Button::Middle),
            select_others: PressInput::Key(Key::F3),
            select_hero: PressInput::Key(Key::Num1),
            step_ms: 30,
        }
    }
}

/// Everything the Phantom Lancer hook branch reads. `None` is "turned off".
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct HookConfig {
    pub latch: Option<LatchConfig>,
    pub illusion_send: Option<IllusionSendConfig>,
}

/// One unit of work for the injector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Step {
    /// Our own input: the latch's Ctrl, the illusion send's keys and click.
    Send(EventType),
    /// A physical click the latch swallowed, replayed inside its Ctrl.
    Replay(EventType),
    /// A physical event held back by rule 3.
    Forward(EventType),
    Wait(u64),
}

/// What the hook does with one physical event.
#[derive(Debug, Clone, PartialEq)]
pub enum Plan {
    Pass,
    Swallow,
    Replace(Vec<Step>),
}

fn is_ctrl(key: Key) -> bool {
    matches!(key, Key::ControlLeft | Key::ControlRight)
}

/// The hook's state machine. Mirrors `Hook` in `examples/phantom_lancer_control.rs`.
#[derive(Debug, Default)]
pub struct CtrlLatch {
    ctrl_down_at: Option<Instant>,
    ctrl_chorded: bool,
    latched: bool,
    /// A right-click we swallowed, whose release is ours too.
    wrapped_right: bool,
    /// The illusion-send trigger is down; autorepeat must not fire it again.
    trigger_held: bool,
}

impl CtrlLatch {
    pub fn is_latched(&self) -> bool {
        self.latched
    }

    /// Returns whether it was on.
    pub fn unlatch(&mut self) -> bool {
        std::mem::replace(&mut self.latched, false)
    }

    /// Decide one physical event. `active` is "Phantom Lancer is the hero".
    /// The second value is the new latch state when this event toggled it.
    pub fn on_physical(
        &mut self,
        now: Instant,
        event: &EventType,
        config: &HookConfig,
        active: bool,
    ) -> (Plan, Option<bool>) {
        // A click we swallowed is ours to finish, whatever changed since.
        if *event == EventType::ButtonRelease(Button::Right)
            && std::mem::replace(&mut self.wrapped_right, false)
        {
            return (Plan::Swallow, None);
        }

        if !active {
            self.ctrl_down_at = None;
            self.latched = false;
            self.trigger_held = false;
            return (Plan::Pass, None);
        }
        if config.latch.is_none() {
            self.latched = false;
        }

        if let Some(send) = config.illusion_send {
            if *event == send.trigger.press() {
                if self.ctrl_down_at.is_some() {
                    self.ctrl_chorded = true;
                }
                if std::mem::replace(&mut self.trigger_held, true) {
                    return (Plan::Swallow, None);
                }
                // The illusions have their order; the next right-click is the
                // hero's alone.
                let was_latched = std::mem::replace(&mut self.latched, false);
                return (
                    Plan::Replace(plan_illusion_send(&send)),
                    was_latched.then_some(false),
                );
            }
            if *event == send.trigger.release() && std::mem::take(&mut self.trigger_held) {
                return (Plan::Swallow, None);
            }
        }

        match *event {
            EventType::KeyPress(key) if is_ctrl(key) => {
                // Autorepeat sends more presses while it is held.
                if self.ctrl_down_at.is_none() {
                    self.ctrl_down_at = Some(now);
                    self.ctrl_chorded = false;
                }
                (Plan::Pass, None)
            }
            EventType::KeyRelease(key) if is_ctrl(key) => {
                let Some(latch) = config.latch else {
                    self.ctrl_down_at = None;
                    return (Plan::Pass, None);
                };
                let tapped = self.ctrl_down_at.take().is_some_and(|down_at| {
                    !self.ctrl_chorded
                        && now.saturating_duration_since(down_at)
                            <= Duration::from_millis(latch.tap_ms)
                });
                if tapped {
                    self.latched = !self.latched;
                    return (Plan::Pass, Some(self.latched));
                }
                (Plan::Pass, None)
            }
            EventType::KeyPress(_) => {
                if self.ctrl_down_at.is_some() {
                    self.ctrl_chorded = true;
                }
                (Plan::Pass, None)
            }
            EventType::ButtonPress(button) => {
                // The player's own Ctrl is already doing the job, and our
                // Ctrl-up would cut it short.
                if self.ctrl_down_at.is_some() {
                    self.ctrl_chorded = true;
                    return (Plan::Pass, None);
                }
                let Some(latch) = config.latch else {
                    return (Plan::Pass, None);
                };
                if !self.latched || button != Button::Right {
                    return (Plan::Pass, None);
                }
                self.wrapped_right = true;
                (
                    Plan::Replace(wrapped_click(button, latch.ctrl_gap_ms)),
                    None,
                )
            }
            _ => (Plan::Pass, None),
        }
    }
}

/// Ctrl down → gap → the click → gap → Ctrl up.
pub fn wrapped_click(button: Button, gap_ms: u64) -> Vec<Step> {
    vec![
        Step::Send(EventType::KeyPress(Key::ControlLeft)),
        Step::Wait(gap_ms),
        Step::Replay(EventType::ButtonPress(button)),
        Step::Wait(PRESS_HOLD_MS),
        Step::Replay(EventType::ButtonRelease(button)),
        Step::Wait(gap_ms),
        Step::Send(EventType::KeyRelease(Key::ControlLeft)),
    ]
}

fn tap(input: PressInput) -> [Step; 3] {
    [
        Step::Send(input.press()),
        Step::Wait(PRESS_HOLD_MS),
        Step::Send(input.release()),
    ]
}

/// Select every other unit → right-click at the cursor → select the hero.
///
/// A right-click, not a left: with units selected a left-click only changes
/// the selection, which is how the first version of this shipped doing
/// nothing between its two selections.
pub fn plan_illusion_send(send: &IllusionSendConfig) -> Vec<Step> {
    let mut steps = Vec::with_capacity(12);
    steps.extend(tap(send.select_others));
    steps.push(Step::Wait(send.step_ms));
    steps.extend(tap(PressInput::Button(Button::Right)));
    steps.push(Step::Wait(send.step_ms));
    steps.extend(tap(send.select_hero));
    steps
}

static LATCH: LazyLock<Mutex<CtrlLatch>> = LazyLock::new(|| Mutex::new(CtrlLatch::default()));

/// Whether the latch is on right now, for the on-screen indicator.
pub fn is_latched() -> bool {
    LATCH
        .lock()
        .map(|latch| latch.is_latched())
        .unwrap_or(false)
}

// ───────────────────────────── the injector ─────────────────────────────

/// Jobs sent to the injector and not yet finished.
static PENDING_JOBS: AtomicUsize = AtomicUsize::new(0);

/// Ctrl the injector pressed and has not released yet.
static OWN_CTRL: AtomicBool = AtomicBool::new(false);

/// Which Ctrl the player is physically holding: 0 none, 1 left, 2 right.
/// Windows tracks the two separately, so lifting the wrong one lifts nothing.
static PLAYER_CTRL: AtomicU8 = AtomicU8::new(0);

/// Events the injector is about to send, for the hook to recognise.
static EXPECTED_ECHOES: LazyLock<Mutex<VecDeque<(EventType, Instant)>>> =
    LazyLock::new(|| Mutex::new(VecDeque::new()));
static EXPECTED_COUNT: AtomicUsize = AtomicUsize::new(0);

static INJECTOR: LazyLock<mpsc::Sender<Vec<Step>>> = LazyLock::new(|| {
    let (tx, rx) = mpsc::channel::<Vec<Step>>();
    thread::spawn(move || {
        info!("🔱 Phantom Lancer injector started");
        while let Ok(steps) = rx.recv() {
            run_steps(&steps);
            PENDING_JOBS.fetch_sub(1, Ordering::SeqCst);
        }
        info!("🔱 Phantom Lancer injector exited");
    });
    tx
});

fn player_ctrl() -> Option<Key> {
    match PLAYER_CTRL.load(Ordering::SeqCst) {
        1 => Some(Key::ControlLeft),
        2 => Some(Key::ControlRight),
        _ => None,
    }
}

fn run_steps(steps: &[Step]) {
    // The player's Ctrl lifted for the key in flight (rule 5), to put back.
    let mut lifted: Option<Key> = None;
    for step in steps {
        match *step {
            Step::Wait(ms) => thread::sleep(Duration::from_millis(ms)),
            Step::Send(event @ EventType::KeyPress(key)) if is_ctrl(key) => {
                OWN_CTRL.store(true, Ordering::SeqCst);
                emit(event);
            }
            Step::Send(event @ EventType::KeyRelease(key)) if is_ctrl(key) => {
                emit(event);
                OWN_CTRL.store(false, Ordering::SeqCst);
            }
            Step::Send(event @ EventType::KeyPress(_)) => {
                if !OWN_CTRL.load(Ordering::SeqCst) {
                    if let Some(ctrl) = player_ctrl() {
                        emit(EventType::KeyRelease(ctrl));
                        lifted = Some(ctrl);
                    }
                }
                emit(event);
            }
            Step::Send(event @ EventType::KeyRelease(_)) => {
                emit(event);
                // Only if still held: a Ctrl released meanwhile stays released.
                if let Some(ctrl) = lifted.take().filter(|ctrl| player_ctrl() == Some(*ctrl)) {
                    emit(EventType::KeyPress(ctrl));
                }
            }
            Step::Send(event) | Step::Replay(event) | Step::Forward(event) => emit(event),
        }
    }
}

fn emit(event: EventType) {
    {
        let mut expected = EXPECTED_ECHOES.lock().unwrap();
        expected.push_back((event, Instant::now()));
        EXPECTED_COUNT.store(expected.len(), Ordering::SeqCst);
    }
    if let Err(error) = rdev::simulate(&event) {
        warn!("🔱 Phantom Lancer could not send {:?}: {:?}", event, error);
        let mut expected = EXPECTED_ECHOES.lock().unwrap();
        expected.pop_back();
        EXPECTED_COUNT.store(expected.len(), Ordering::SeqCst);
    }
}

fn enqueue(steps: Vec<Step>) {
    PENDING_JOBS.fetch_add(1, Ordering::SeqCst);
    if let Err(error) = INJECTOR.send(steps) {
        // Never strand rule 3 behind a job that will not run.
        PENDING_JOBS.fetch_sub(1, Ordering::SeqCst);
        warn!("🔱 Phantom Lancer injector unavailable; running inline");
        run_steps(&error.0);
    }
}

fn injector_busy() -> bool {
    PENDING_JOBS.load(Ordering::SeqCst) > 0 || OWN_CTRL.load(Ordering::SeqCst)
}

/// Whether this event is one the injector sent. Called first in the hook, so
/// our own input is never decided twice.
pub fn is_own_echo(event: &EventType) -> bool {
    if EXPECTED_COUNT.load(Ordering::SeqCst) == 0 {
        return false;
    }
    let mut expected = EXPECTED_ECHOES.lock().unwrap();
    let now = Instant::now();
    while expected
        .front()
        .is_some_and(|(_, sent_at)| now.saturating_duration_since(*sent_at) > ECHO_TIMEOUT)
    {
        expected.pop_front();
    }
    let ours = expected.front().is_some_and(|(sent, _)| sent == event);
    if ours {
        expected.pop_front();
    }
    EXPECTED_COUNT.store(expected.len(), Ordering::SeqCst);
    ours
}

// ──────────────────────────────── the hook ────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookVerdict {
    /// The event is handled here; the hook returns `None`.
    Block,
    /// Not ours: the rest of the hook decides.
    Continue,
}

/// The Phantom Lancer branch of the keyboard hook. Runs on every physical key
/// and button event, before any other branch, because rule 3 has to see the
/// events every other branch would otherwise pass straight to Dota.
pub fn handle_physical_event(event: &EventType, active: bool, config: &HookConfig) -> HookVerdict {
    match *event {
        EventType::MouseMove { .. } | EventType::Wheel { .. } => return HookVerdict::Continue,
        // Tracked for every hero: rule 5 reads it, and it is cheap.
        EventType::KeyPress(Key::ControlLeft) => PLAYER_CTRL.store(1, Ordering::SeqCst),
        EventType::KeyPress(Key::ControlRight) => PLAYER_CTRL.store(2, Ordering::SeqCst),
        EventType::KeyRelease(key) if is_ctrl(key) => PLAYER_CTRL.store(0, Ordering::SeqCst),
        _ => {}
    }

    let (plan, toggled) = LATCH
        .lock()
        .unwrap()
        .on_physical(Instant::now(), event, config, active);

    if let (Some(on), Some(latch)) = (toggled, config.latch) {
        info!(
            "🔱 Phantom Lancer Ctrl latch {}",
            if on { "ON" } else { "off" }
        );
        if latch.cue_enabled {
            play_cue(on, latch.cue_volume);
        }
    }

    match plan {
        Plan::Swallow => HookVerdict::Block,
        Plan::Replace(steps) => {
            debug!(
                "🔱 Phantom Lancer: {:?} replaced by {} steps",
                event,
                steps.len()
            );
            enqueue(steps);
            HookVerdict::Block
        }
        Plan::Pass if injector_busy() => {
            enqueue(vec![Step::Forward(*event)]);
            HookVerdict::Block
        }
        Plan::Pass => HookVerdict::Continue,
    }
}

/// Rising for on, falling for off: the latch has no in-game indicator.
fn play_cue(on: bool, volume: f32) {
    let (low, high) = (660.0, 990.0);
    let tones = if on {
        vec![Tone::new(low, 60), Tone::new(high, 60)]
    } else {
        vec![Tone::new(high, 60), Tone::new(low, 60)]
    };
    // Off the hook thread: the first cue opens the audio device.
    thread::spawn(move || {
        crate::audio::player::play_motif(&Motif::new(tones, 25, Timbre::Wood), volume);
    });
}

// ─────────────────────────────── the script ───────────────────────────────

/// Phantom Lancer script.
///
/// Both features live in the keyboard hook; GSI only turns the latch off on
/// death. The rest is the shared default strategy every hero without a script
/// gets.
pub struct PhantomLancerScript {
    survivability: SurvivabilityActions,
}

impl PhantomLancerScript {
    pub fn new(settings: Arc<Mutex<Settings>>, executor: Arc<ActionExecutor>) -> Self {
        Self {
            survivability: SurvivabilityActions::new(settings, executor),
        }
    }
}

impl HeroScript for PhantomLancerScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        if !event.hero.alive && LATCH.lock().unwrap().unlatch() {
            info!("🔱 Phantom Lancer Ctrl latch off: hero died");
        }
        self.survivability.execute_default_strategy(event);
    }

    fn handle_standalone_trigger(&self) {}

    fn hero_name(&self) -> &'static str {
        Hero::PhantomLancer.to_game_name()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(start: Instant, ms: u64) -> Instant {
        start + Duration::from_millis(ms)
    }

    fn press(key: Key) -> EventType {
        EventType::KeyPress(key)
    }

    fn release(key: Key) -> EventType {
        EventType::KeyRelease(key)
    }

    fn both() -> HookConfig {
        HookConfig {
            latch: Some(LatchConfig::default()),
            illusion_send: Some(IllusionSendConfig::default()),
        }
    }

    /// Feed events at offsets and return the plans.
    fn run_with(
        latch: &mut CtrlLatch,
        config: &HookConfig,
        events: &[(u64, EventType)],
        active: bool,
    ) -> Vec<Plan> {
        let start = Instant::now();
        events
            .iter()
            .map(|(ms, event)| latch.on_physical(at(start, *ms), event, config, active).0)
            .collect()
    }

    fn run(latch: &mut CtrlLatch, events: &[(u64, EventType)], active: bool) -> Vec<Plan> {
        run_with(latch, &both(), events, active)
    }

    fn tap_ctrl() -> [(u64, EventType); 2] {
        [
            (0, press(Key::ControlLeft)),
            (90, release(Key::ControlLeft)),
        ]
    }

    #[test]
    fn a_clean_tap_toggles_and_a_second_clears() {
        let mut latch = CtrlLatch::default();
        run(&mut latch, &tap_ctrl(), true);
        assert!(latch.is_latched());
        run(&mut latch, &tap_ctrl(), true);
        assert!(!latch.is_latched());
    }

    #[test]
    fn ctrl_itself_is_never_blocked() {
        let mut latch = CtrlLatch::default();
        assert_eq!(
            run(&mut latch, &tap_ctrl(), true),
            vec![Plan::Pass, Plan::Pass]
        );
    }

    #[test]
    fn a_hold_longer_than_the_tap_threshold_toggles_nothing() {
        let mut latch = CtrlLatch::default();
        run(
            &mut latch,
            &[
                (0, press(Key::ControlLeft)),
                (450, release(Key::ControlLeft)),
            ],
            true,
        );
        assert!(!latch.is_latched());
    }

    /// The probe measured quick Ctrl+key presses at 74–98ms: inside the tap
    /// window, so only the chord rule keeps them from toggling.
    #[test]
    fn a_fast_chord_is_not_a_tap() {
        let mut latch = CtrlLatch::default();
        run(
            &mut latch,
            &[
                (0, press(Key::ControlLeft)),
                (20, press(Key::Num1)),
                (40, release(Key::Num1)),
                (60, release(Key::ControlLeft)),
            ],
            true,
        );
        assert!(!latch.is_latched());
    }

    #[test]
    fn a_click_while_ctrl_is_held_is_a_chord_and_passes() {
        let mut latch = CtrlLatch::default();
        run(&mut latch, &tap_ctrl(), true);
        let plans = run(
            &mut latch,
            &[
                (0, press(Key::ControlLeft)),
                (30, EventType::ButtonPress(Button::Right)),
                (60, EventType::ButtonRelease(Button::Right)),
                (90, release(Key::ControlLeft)),
            ],
            true,
        );
        assert_eq!(plans, vec![Plan::Pass; 4]);
        assert!(latch.is_latched(), "the chord toggled the latch");
    }

    #[test]
    fn autorepeat_does_not_restart_the_tap_clock() {
        let mut latch = CtrlLatch::default();
        run(
            &mut latch,
            &[
                (0, press(Key::ControlLeft)),
                (150, press(Key::ControlLeft)),
                (300, release(Key::ControlLeft)),
            ],
            true,
        );
        assert!(!latch.is_latched(), "a 300ms hold read as a tap");
    }

    #[test]
    fn a_latched_right_click_is_replayed_inside_ctrl_and_its_release_swallowed() {
        let mut latch = CtrlLatch::default();
        run(&mut latch, &tap_ctrl(), true);
        let plans = run(
            &mut latch,
            &[
                (0, EventType::ButtonPress(Button::Right)),
                (40, EventType::ButtonRelease(Button::Right)),
            ],
            true,
        );
        assert_eq!(
            plans,
            vec![
                Plan::Replace(wrapped_click(
                    Button::Right,
                    LatchConfig::default().ctrl_gap_ms
                )),
                Plan::Swallow,
            ]
        );
    }

    #[test]
    fn the_wrap_opens_with_ctrl_and_closes_with_it() {
        let steps = wrapped_click(Button::Right, 15);
        assert_eq!(steps.first(), Some(&Step::Send(press(Key::ControlLeft))));
        assert_eq!(steps.last(), Some(&Step::Send(release(Key::ControlLeft))));
        assert!(steps.contains(&Step::Wait(15)));
    }

    #[test]
    fn left_clicks_pass_while_latched() {
        let mut latch = CtrlLatch::default();
        run(&mut latch, &tap_ctrl(), true);
        let plans = run(
            &mut latch,
            &[(0, EventType::ButtonPress(Button::Left))],
            true,
        );
        assert_eq!(plans, vec![Plan::Pass]);
    }

    #[test]
    fn an_unlatched_right_click_passes() {
        let mut latch = CtrlLatch::default();
        let plans = run(
            &mut latch,
            &[
                (0, EventType::ButtonPress(Button::Right)),
                (40, EventType::ButtonRelease(Button::Right)),
            ],
            true,
        );
        assert_eq!(plans, vec![Plan::Pass, Plan::Pass]);
    }

    #[test]
    fn another_hero_clears_the_latch_and_nothing_is_ours() {
        let mut latch = CtrlLatch::default();
        run(&mut latch, &tap_ctrl(), true);
        run(&mut latch, &tap_ctrl(), false);
        assert!(!latch.is_latched());
        let plans = run(
            &mut latch,
            &[
                (0, EventType::ButtonPress(Button::Right)),
                (10, EventType::ButtonPress(Button::Middle)),
            ],
            false,
        );
        assert_eq!(plans, vec![Plan::Pass, Plan::Pass]);
    }

    /// Turning off mid-click must still swallow the release, or Dota gets a
    /// release for a press it never saw.
    #[test]
    fn a_swallowed_click_keeps_its_release_after_the_latch_goes_off() {
        let mut latch = CtrlLatch::default();
        run(&mut latch, &tap_ctrl(), true);
        run(
            &mut latch,
            &[(0, EventType::ButtonPress(Button::Right))],
            true,
        );
        latch.unlatch();
        let plans = run(
            &mut latch,
            &[(0, EventType::ButtonRelease(Button::Right))],
            false,
        );
        assert_eq!(plans, vec![Plan::Swallow]);
    }

    #[test]
    fn unlatch_reports_whether_it_was_on() {
        let mut latch = CtrlLatch::default();
        assert!(!latch.unlatch());
        run(&mut latch, &tap_ctrl(), true);
        assert!(latch.unlatch());
        assert!(!latch.is_latched());
    }

    #[test]
    fn with_the_latch_disabled_taps_do_nothing_and_clicks_pass() {
        let mut latch = CtrlLatch::default();
        let config = HookConfig {
            latch: None,
            illusion_send: Some(IllusionSendConfig::default()),
        };
        run_with(&mut latch, &config, &tap_ctrl(), true);
        assert!(!latch.is_latched());
        let plans = run_with(
            &mut latch,
            &config,
            &[(0, EventType::ButtonPress(Button::Right))],
            true,
        );
        assert_eq!(plans, vec![Plan::Pass]);
    }

    #[test]
    fn the_send_trigger_runs_the_send_and_swallows_both_halves() {
        let mut latch = CtrlLatch::default();
        let plans = run(
            &mut latch,
            &[
                (0, EventType::ButtonPress(Button::Middle)),
                (40, EventType::ButtonRelease(Button::Middle)),
            ],
            true,
        );
        assert_eq!(
            plans,
            vec![
                Plan::Replace(plan_illusion_send(&IllusionSendConfig::default())),
                Plan::Swallow,
            ]
        );
    }

    #[test]
    fn the_send_selects_others_right_clicks_then_selects_the_hero() {
        let sends: Vec<EventType> = plan_illusion_send(&IllusionSendConfig::default())
            .into_iter()
            .filter_map(|step| match step {
                Step::Send(event) => Some(event),
                _ => None,
            })
            .collect();
        assert_eq!(
            sends,
            vec![
                press(Key::F3),
                release(Key::F3),
                EventType::ButtonPress(Button::Right),
                EventType::ButtonRelease(Button::Right),
                press(Key::Num1),
                release(Key::Num1),
            ]
        );
    }

    /// After a send the illusions have their order: the hero's next
    /// right-click is its own, and the toggle reports off so the cue plays.
    #[test]
    fn the_send_turns_the_latch_off() {
        let mut latch = CtrlLatch::default();
        run(&mut latch, &tap_ctrl(), true);
        let (_, toggled) = latch.on_physical(
            Instant::now(),
            &EventType::ButtonPress(Button::Middle),
            &both(),
            true,
        );
        assert_eq!(toggled, Some(false));
        assert!(!latch.is_latched());
        let plans = run(
            &mut latch,
            &[
                (0, EventType::ButtonRelease(Button::Middle)),
                (100, EventType::ButtonPress(Button::Right)),
            ],
            true,
        );
        assert_eq!(plans, vec![Plan::Swallow, Plan::Pass]);
    }

    #[test]
    fn a_send_while_unlatched_reports_no_toggle() {
        let mut latch = CtrlLatch::default();
        let (_, toggled) = latch.on_physical(
            Instant::now(),
            &EventType::ButtonPress(Button::Middle),
            &both(),
            true,
        );
        assert_eq!(toggled, None);
    }

    /// A keyboard trigger autorepeats while held; one hold is one send.
    #[test]
    fn a_held_keyboard_trigger_sends_once() {
        let mut latch = CtrlLatch::default();
        let config = HookConfig {
            latch: None,
            illusion_send: Some(IllusionSendConfig {
                trigger: PressInput::Key(Key::KeyG),
                ..IllusionSendConfig::default()
            }),
        };
        let plans = run_with(
            &mut latch,
            &config,
            &[
                (0, press(Key::KeyG)),
                (30, press(Key::KeyG)),
                (60, release(Key::KeyG)),
                (90, press(Key::KeyG)),
            ],
            true,
        );
        assert!(matches!(plans[0], Plan::Replace(_)));
        assert_eq!(plans[1], Plan::Swallow);
        assert_eq!(plans[2], Plan::Swallow);
        assert!(matches!(plans[3], Plan::Replace(_)));
    }

    #[test]
    fn ctrl_plus_the_trigger_is_a_chord_not_a_tap() {
        let mut latch = CtrlLatch::default();
        run(
            &mut latch,
            &[
                (0, press(Key::ControlLeft)),
                (20, EventType::ButtonPress(Button::Middle)),
                (40, EventType::ButtonRelease(Button::Middle)),
                (60, release(Key::ControlLeft)),
            ],
            true,
        );
        assert!(!latch.is_latched());
    }

    #[test]
    fn middle_mouse_passes_with_the_send_disabled() {
        let mut latch = CtrlLatch::default();
        let config = HookConfig {
            latch: Some(LatchConfig::default()),
            illusion_send: None,
        };
        let plans = run_with(
            &mut latch,
            &config,
            &[
                (0, EventType::ButtonPress(Button::Middle)),
                (40, EventType::ButtonRelease(Button::Middle)),
            ],
            true,
        );
        assert_eq!(plans, vec![Plan::Pass, Plan::Pass]);
    }

    #[test]
    fn bindings_resolve_to_keys_and_buttons() {
        assert_eq!(
            PressInput::from_binding(&KeyBinding::parse("Mouse3")),
            Some(PressInput::Button(Button::Middle))
        );
        assert_eq!(
            PressInput::from_binding(&KeyBinding::parse("F3")),
            Some(PressInput::Key(Key::F3))
        );
        assert_eq!(
            PressInput::from_binding(&KeyBinding::parse("1")),
            Some(PressInput::Key(Key::Num1))
        );
        assert_eq!(PressInput::from_binding(&KeyBinding::parse("")), None);
    }

    #[test]
    fn mouse_moves_are_never_ours() {
        assert_eq!(
            handle_physical_event(&EventType::MouseMove { x: 1.0, y: 2.0 }, true, &both()),
            HookVerdict::Continue
        );
    }

    #[test]
    fn hero_name_is_the_gsi_internal_name() {
        let script = PhantomLancerScript::new(
            Arc::new(Mutex::new(Settings::default())),
            ActionExecutor::new(),
        );
        assert_eq!(script.hero_name(), "npc_dota_hero_phantom_lancer");
    }
}
