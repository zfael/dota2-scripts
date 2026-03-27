use enigo::{Button, Direction, Enigo, Key, Keyboard, Mouse, Settings};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;
use tracing::warn;

const POST_ACTION_GUARD_DELAY_MS: u64 = 10;

static SYNTHETIC_INPUT_TX: OnceLock<Sender<SyntheticInputJob>> = OnceLock::new();
static METRICS: OnceLock<Mutex<SyntheticInputMetricsState>> = OnceLock::new();

/// Global flag to indicate we're simulating keys - prevents keyboard grab re-interception
pub static SIMULATING_KEYS: AtomicBool = AtomicBool::new(false);

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct SyntheticInputMetricsState {
    queued_total: u64,
    completed_total: u64,
    dropped_total: u64,
    current_depth: usize,
    peak_depth: usize,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SyntheticInputMetricsSnapshot {
    pub queued_total: u64,
    pub completed_total: u64,
    pub dropped_total: u64,
    pub current_depth: usize,
    pub peak_depth: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModifierKey {
    Alt,
    Control,
    Shift,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SyntheticAction {
    KeyClick(char),
    KeyDown(char),
    KeyUp(char),
    RightClick,
    #[allow(dead_code)]
    LeftClick,
    ModifierDown(ModifierKey),
    ModifierUp(ModifierKey),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GuardBehavior {
    None,
    Pulse { delay_ms: u64 },
    HoldUntilAltUp,
    ReleaseHold { delay_ms: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SyntheticInputCommand {
    action: SyntheticAction,
    guard_behavior: GuardBehavior,
}

struct SyntheticInputJob {
    command: SyntheticInputCommand,
    completion_tx: Sender<()>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EnqueueMetricsCheckpoint {
    queued_total: u64,
    current_depth: usize,
    peak_depth: usize,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct WorkerGuardState {
    modifier_guard_held: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GuardExecutionPlan {
    set_simulating_before: Option<bool>,
    post_action_delay_ms: Option<u64>,
    final_simulating_value: Option<bool>,
}

/// Press a single key (sets SIMULATING_KEYS flag to prevent re-interception)
pub fn press_key(key_char: char) {
    enqueue_command_and_wait(press_key_command(key_char));
}

/// Press a key down (hold)
#[allow(dead_code)]
pub fn key_down(key_char: char) {
    enqueue_command_and_wait(key_down_command(key_char));
}

/// Release a key
#[allow(dead_code)]
pub fn key_up(key_char: char) {
    enqueue_command_and_wait(key_up_command(key_char));
}

/// Perform a right mouse click
pub fn mouse_click() {
    enqueue_command_and_wait(mouse_click_command());
}

/// Perform a left mouse click
#[allow(dead_code)]
pub fn left_click() {
    enqueue_command_and_wait(left_click_command());
}

/// Hold ALT key down
pub fn alt_down() {
    modifier_down(ModifierKey::Alt);
}

/// Release ALT key
pub fn alt_up() {
    modifier_up(ModifierKey::Alt);
}

pub fn modifier_down(modifier: ModifierKey) {
    enqueue_command_and_wait(modifier_down_command(modifier));
}

pub fn modifier_up(modifier: ModifierKey) {
    enqueue_command_and_wait(modifier_up_command(modifier));
}

fn press_key_command(key_char: char) -> SyntheticInputCommand {
    SyntheticInputCommand {
        action: SyntheticAction::KeyClick(key_char),
        guard_behavior: GuardBehavior::Pulse {
            delay_ms: POST_ACTION_GUARD_DELAY_MS,
        },
    }
}

fn key_down_command(key_char: char) -> SyntheticInputCommand {
    SyntheticInputCommand {
        action: SyntheticAction::KeyDown(key_char),
        guard_behavior: GuardBehavior::None,
    }
}

fn key_up_command(key_char: char) -> SyntheticInputCommand {
    SyntheticInputCommand {
        action: SyntheticAction::KeyUp(key_char),
        guard_behavior: GuardBehavior::None,
    }
}

fn mouse_click_command() -> SyntheticInputCommand {
    SyntheticInputCommand {
        action: SyntheticAction::RightClick,
        guard_behavior: GuardBehavior::Pulse {
            delay_ms: POST_ACTION_GUARD_DELAY_MS,
        },
    }
}

#[allow(dead_code)]
fn left_click_command() -> SyntheticInputCommand {
    SyntheticInputCommand {
        action: SyntheticAction::LeftClick,
        guard_behavior: GuardBehavior::Pulse {
            delay_ms: POST_ACTION_GUARD_DELAY_MS,
        },
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn alt_down_command() -> SyntheticInputCommand {
    modifier_down_command(ModifierKey::Alt)
}

fn modifier_down_command(modifier: ModifierKey) -> SyntheticInputCommand {
    SyntheticInputCommand {
        action: SyntheticAction::ModifierDown(modifier),
        guard_behavior: GuardBehavior::HoldUntilAltUp,
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn alt_up_command() -> SyntheticInputCommand {
    modifier_up_command(ModifierKey::Alt)
}

fn modifier_up_command(modifier: ModifierKey) -> SyntheticInputCommand {
    SyntheticInputCommand {
        action: SyntheticAction::ModifierUp(modifier),
        guard_behavior: GuardBehavior::ReleaseHold {
            delay_ms: POST_ACTION_GUARD_DELAY_MS,
        },
    }
}

fn enqueue_command_and_wait(command: SyntheticInputCommand) {
    let (completion_tx, completion_rx) = mpsc::channel();
    let action = command.action;
    let job = SyntheticInputJob {
        command,
        completion_tx,
    };

    if !enqueue_with_sender(worker_sender(), job, action) {
        return;
    }

    if completion_rx.recv().is_err() {
        warn!(
            "Synthetic input worker stopped before queued action {:?} completed",
            action
        );
    }
}

fn enqueue_with_sender(
    sender: &Sender<SyntheticInputJob>,
    job: SyntheticInputJob,
    action: SyntheticAction,
) -> bool {
    let mut state = metrics_store().lock().unwrap();
    let checkpoint = EnqueueMetricsCheckpoint {
        queued_total: state.queued_total,
        current_depth: state.current_depth,
        peak_depth: state.peak_depth,
    };
    record_enqueue_success(&mut state);

    if sender.send(job).is_err() {
        warn!(
            "Synthetic input worker is unavailable; dropping queued action {:?}",
            action
        );
        restore_enqueue_success(&mut state, checkpoint);
        record_enqueue_failure(&mut state);
        return false;
    }

    true
}

#[cfg_attr(not(test), allow(dead_code))]
fn record_enqueue_success(state: &mut SyntheticInputMetricsState) {
    state.queued_total += 1;
    state.current_depth += 1;
    state.peak_depth = state.peak_depth.max(state.current_depth);
}

#[cfg_attr(not(test), allow(dead_code))]
fn record_completion(state: &mut SyntheticInputMetricsState) {
    state.completed_total += 1;
    if state.current_depth > 0 {
        state.current_depth -= 1;
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn restore_enqueue_success(
    state: &mut SyntheticInputMetricsState,
    checkpoint: EnqueueMetricsCheckpoint,
) {
    state.queued_total = checkpoint.queued_total;
    state.current_depth = checkpoint.current_depth;
    state.peak_depth = checkpoint.peak_depth;
}

#[cfg_attr(not(test), allow(dead_code))]
fn record_enqueue_failure(state: &mut SyntheticInputMetricsState) {
    state.dropped_total += 1;
}

#[cfg_attr(not(test), allow(dead_code))]
fn metrics_snapshot(state: &SyntheticInputMetricsState) -> SyntheticInputMetricsSnapshot {
    SyntheticInputMetricsSnapshot {
        queued_total: state.queued_total,
        completed_total: state.completed_total,
        dropped_total: state.dropped_total,
        current_depth: state.current_depth,
        peak_depth: state.peak_depth,
    }
}

fn metrics_store() -> &'static Mutex<SyntheticInputMetricsState> {
    METRICS.get_or_init(|| Mutex::new(SyntheticInputMetricsState::default()))
}

/// Public snapshot access to synthetic input backlog metrics
pub fn synthetic_input_metrics() -> SyntheticInputMetricsSnapshot {
    let store = metrics_store();
    let state = store.lock().unwrap();
    metrics_snapshot(&state)
}

fn worker_sender() -> &'static Sender<SyntheticInputJob> {
    SYNTHETIC_INPUT_TX.get_or_init(spawn_worker)
}

fn spawn_worker() -> Sender<SyntheticInputJob> {
    let (tx, rx) = mpsc::channel();
    let enigo = Enigo::new(&Settings::default()).expect("Failed to initialize Enigo");

    thread::Builder::new()
        .name("synthetic-input-worker".to_string())
        .spawn(move || run_worker(rx, enigo))
        .expect("Failed to spawn synthetic input worker");

    tx
}

fn run_worker(rx: Receiver<SyntheticInputJob>, mut enigo: Enigo) {
    let mut guard_state = WorkerGuardState::default();

    while let Ok(job) = rx.recv() {
        execute_command(&mut enigo, job.command, &mut guard_state);
        
        let mut state = metrics_store().lock().unwrap();
        record_completion(&mut state);
        drop(state);
        
        let _ = job.completion_tx.send(());
    }
}

fn execute_command(
    enigo: &mut Enigo,
    command: SyntheticInputCommand,
    guard_state: &mut WorkerGuardState,
) {
    let guard_plan = plan_guard_execution(guard_state, command.guard_behavior);

    if let Some(value) = guard_plan.set_simulating_before {
        SIMULATING_KEYS.store(value, Ordering::SeqCst);
    }

    perform_action(enigo, command.action);

    if let Some(delay_ms) = guard_plan.post_action_delay_ms {
        thread::sleep(Duration::from_millis(delay_ms));
    }

    if let Some(value) = guard_plan.final_simulating_value {
        SIMULATING_KEYS.store(value, Ordering::SeqCst);
    }
}

fn plan_guard_execution(
    guard_state: &mut WorkerGuardState,
    behavior: GuardBehavior,
) -> GuardExecutionPlan {
    match behavior {
        GuardBehavior::None => GuardExecutionPlan {
            set_simulating_before: None,
            post_action_delay_ms: None,
            final_simulating_value: None,
        },
        GuardBehavior::Pulse { delay_ms } => GuardExecutionPlan {
            set_simulating_before: Some(true),
            post_action_delay_ms: Some(delay_ms),
            final_simulating_value: Some(guard_state.modifier_guard_held),
        },
        GuardBehavior::HoldUntilAltUp => {
            guard_state.modifier_guard_held = true;
            GuardExecutionPlan {
                set_simulating_before: Some(true),
                post_action_delay_ms: None,
                final_simulating_value: None,
            }
        }
        GuardBehavior::ReleaseHold { delay_ms } => {
            guard_state.modifier_guard_held = false;
            GuardExecutionPlan {
                set_simulating_before: None,
                post_action_delay_ms: Some(delay_ms),
                final_simulating_value: Some(false),
            }
        }
    }
}

fn perform_action(enigo: &mut Enigo, action: SyntheticAction) {
    match action {
        SyntheticAction::KeyClick(key_char) => {
            if let Err(e) = enigo.key(Key::Unicode(key_char), Direction::Click) {
                warn!("Failed to press key '{}': {}", key_char, e);
            }
        }
        SyntheticAction::KeyDown(key_char) => {
            if let Err(e) = enigo.key(Key::Unicode(key_char), Direction::Press) {
                warn!("Failed to press down key '{}': {}", key_char, e);
            }
        }
        SyntheticAction::KeyUp(key_char) => {
            if let Err(e) = enigo.key(Key::Unicode(key_char), Direction::Release) {
                warn!("Failed to release key '{}': {}", key_char, e);
            }
        }
        SyntheticAction::RightClick => {
            if let Err(e) = enigo.button(Button::Right, Direction::Click) {
                warn!("Failed to perform right click: {}", e);
            }
        }
        SyntheticAction::LeftClick => {
            if let Err(e) = enigo.button(Button::Left, Direction::Click) {
                warn!("Failed to perform left click: {}", e);
            }
        }
        SyntheticAction::ModifierDown(modifier) => {
            if let Err(e) = enigo.key(enigo_modifier_key(modifier), Direction::Press) {
                warn!("Failed to press {:?} down: {}", modifier, e);
            }
        }
        SyntheticAction::ModifierUp(modifier) => {
            if let Err(e) = enigo.key(enigo_modifier_key(modifier), Direction::Release) {
                warn!("Failed to release {:?}: {}", modifier, e);
            }
        }
    }
}

fn enigo_modifier_key(modifier: ModifierKey) -> Key {
    match modifier {
        ModifierKey::Alt => Key::Alt,
        ModifierKey::Control => Key::Control,
        ModifierKey::Shift => Key::Shift,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static METRICS_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn test_job(command: SyntheticInputCommand) -> SyntheticInputJob {
        let (completion_tx, _completion_rx) = mpsc::channel();
        SyntheticInputJob {
            command,
            completion_tx,
        }
    }

    #[test]
    fn commands_with_post_action_delay_are_labeled_correctly() {
        assert_eq!(
            press_key_command('q').guard_behavior,
            GuardBehavior::Pulse {
                delay_ms: POST_ACTION_GUARD_DELAY_MS,
            }
        );
        assert_eq!(
            mouse_click_command().guard_behavior,
            GuardBehavior::Pulse {
                delay_ms: POST_ACTION_GUARD_DELAY_MS,
            }
        );
        assert_eq!(
            left_click_command().guard_behavior,
            GuardBehavior::Pulse {
                delay_ms: POST_ACTION_GUARD_DELAY_MS,
            }
        );
        assert_eq!(
            alt_up_command().guard_behavior,
            GuardBehavior::ReleaseHold {
                delay_ms: POST_ACTION_GUARD_DELAY_MS,
            }
        );
    }

    #[test]
    fn alt_down_keeps_guard_active_until_alt_up_clears_it() {
        let mut guard_state = WorkerGuardState::default();

        let alt_down_plan =
            plan_guard_execution(&mut guard_state, alt_down_command().guard_behavior);
        assert_eq!(
            alt_down_plan,
            GuardExecutionPlan {
                set_simulating_before: Some(true),
                post_action_delay_ms: None,
                final_simulating_value: None,
            }
        );
        assert!(guard_state.modifier_guard_held);

        let click_plan =
            plan_guard_execution(&mut guard_state, mouse_click_command().guard_behavior);
        assert_eq!(
            click_plan,
            GuardExecutionPlan {
                set_simulating_before: Some(true),
                post_action_delay_ms: Some(POST_ACTION_GUARD_DELAY_MS),
                final_simulating_value: Some(true),
            }
        );
        assert!(guard_state.modifier_guard_held);

        let alt_up_plan =
            plan_guard_execution(&mut guard_state, alt_up_command().guard_behavior);
        assert_eq!(
            alt_up_plan,
            GuardExecutionPlan {
                set_simulating_before: None,
                post_action_delay_ms: Some(POST_ACTION_GUARD_DELAY_MS),
                final_simulating_value: Some(false),
            }
        );
        assert!(!guard_state.modifier_guard_held);
    }

    #[test]
    fn queued_commands_preserve_fifo_order() {
        let _guard = METRICS_TEST_LOCK.lock().unwrap();
        let (tx, rx) = mpsc::channel();

        assert!(enqueue_with_sender(
            &tx,
            test_job(press_key_command('q')),
            SyntheticAction::KeyClick('q')
        ));
        assert!(enqueue_with_sender(
            &tx,
            test_job(alt_down_command()),
            SyntheticAction::ModifierDown(ModifierKey::Alt)
        ));
        assert!(enqueue_with_sender(
            &tx,
            test_job(mouse_click_command()),
            SyntheticAction::RightClick
        ));

        let queued_commands: Vec<_> = rx.try_iter().map(|job| job.command).collect();
        assert_eq!(
            queued_commands,
            vec![
                press_key_command('q'),
                alt_down_command(),
                mouse_click_command()
            ]
        );
    }

    #[test]
    fn enqueue_success_updates_depth_and_peak() {
        let mut metrics = SyntheticInputMetricsState::default();

        record_enqueue_success(&mut metrics);

        assert_eq!(
            metrics_snapshot(&metrics),
            SyntheticInputMetricsSnapshot {
                queued_total: 1,
                completed_total: 0,
                dropped_total: 0,
                current_depth: 1,
                peak_depth: 1,
            }
        );

        record_enqueue_success(&mut metrics);

        assert_eq!(
            metrics_snapshot(&metrics),
            SyntheticInputMetricsSnapshot {
                queued_total: 2,
                completed_total: 0,
                dropped_total: 0,
                current_depth: 2,
                peak_depth: 2,
            }
        );
    }

    #[test]
    fn completion_updates_completed_and_reduces_depth() {
        let mut metrics = SyntheticInputMetricsState {
            queued_total: 3,
            completed_total: 0,
            dropped_total: 0,
            current_depth: 2,
            peak_depth: 2,
        };

        record_completion(&mut metrics);

        assert_eq!(
            metrics_snapshot(&metrics),
            SyntheticInputMetricsSnapshot {
                queued_total: 3,
                completed_total: 1,
                dropped_total: 0,
                current_depth: 1,
                peak_depth: 2,
            }
        );

        record_completion(&mut metrics);
        record_completion(&mut metrics);

        assert_eq!(
            metrics_snapshot(&metrics),
            SyntheticInputMetricsSnapshot {
                queued_total: 3,
                completed_total: 3,
                dropped_total: 0,
                current_depth: 0,
                peak_depth: 2,
            }
        );
    }

    #[test]
    fn failed_enqueue_only_updates_dropped_total() {
        let mut metrics = SyntheticInputMetricsState {
            queued_total: 4,
            completed_total: 2,
            dropped_total: 1,
            current_depth: 2,
            peak_depth: 5,
        };

        record_enqueue_failure(&mut metrics);

        assert_eq!(
            metrics_snapshot(&metrics),
            SyntheticInputMetricsSnapshot {
                queued_total: 4,
                completed_total: 2,
                dropped_total: 2,
                current_depth: 2,
                peak_depth: 5,
            }
        );
    }

    #[test]
    fn failed_enqueue_rolls_back_provisional_success_and_records_drop() {
        let _guard = METRICS_TEST_LOCK.lock().unwrap();
        let before = synthetic_input_metrics();
        let (tx, rx) = mpsc::channel();
        drop(rx);

        assert!(!enqueue_with_sender(
            &tx,
            test_job(mouse_click_command()),
            SyntheticAction::RightClick
        ));

        let after = synthetic_input_metrics();

        assert_eq!(after.queued_total, before.queued_total);
        assert_eq!(after.completed_total, before.completed_total);
        assert_eq!(after.current_depth, before.current_depth);
        assert_eq!(after.peak_depth, before.peak_depth);
        assert_eq!(after.dropped_total, before.dropped_total + 1);
    }

    #[test]
    fn snapshot_copies_all_metric_fields() {
        let state = SyntheticInputMetricsState {
            queued_total: 10,
            completed_total: 7,
            dropped_total: 1,
            current_depth: 2,
            peak_depth: 5,
        };

        let snapshot = metrics_snapshot(&state);

        assert_eq!(snapshot.queued_total, state.queued_total);
        assert_eq!(snapshot.completed_total, state.completed_total);
        assert_eq!(snapshot.dropped_total, state.dropped_total);
        assert_eq!(snapshot.current_depth, state.current_depth);
        assert_eq!(snapshot.peak_depth, state.peak_depth);
    }
}
