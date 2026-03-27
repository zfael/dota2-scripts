use enigo::{Button, Direction, Enigo, Key, Keyboard, Mouse, Settings};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;
use tracing::warn;

const POST_ACTION_GUARD_DELAY_MS: u64 = 10;

static SYNTHETIC_INPUT_TX: OnceLock<Sender<SyntheticInputJob>> = OnceLock::new();

/// Global flag to indicate we're simulating keys - prevents keyboard grab re-interception
pub static SIMULATING_KEYS: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SyntheticAction {
    KeyClick(char),
    KeyDown(char),
    KeyUp(char),
    RightClick,
    #[allow(dead_code)]
    LeftClick,
    AltDown,
    AltUp,
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct WorkerGuardState {
    alt_guard_held: bool,
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
    enqueue_command_and_wait(alt_down_command());
}

/// Release ALT key
pub fn alt_up() {
    enqueue_command_and_wait(alt_up_command());
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

fn alt_down_command() -> SyntheticInputCommand {
    SyntheticInputCommand {
        action: SyntheticAction::AltDown,
        guard_behavior: GuardBehavior::HoldUntilAltUp,
    }
}

fn alt_up_command() -> SyntheticInputCommand {
    SyntheticInputCommand {
        action: SyntheticAction::AltUp,
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
    if sender.send(job).is_err() {
        warn!(
            "Synthetic input worker is unavailable; dropping queued action {:?}",
            action
        );
        return false;
    }

    true
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
            final_simulating_value: Some(guard_state.alt_guard_held),
        },
        GuardBehavior::HoldUntilAltUp => {
            guard_state.alt_guard_held = true;
            GuardExecutionPlan {
                set_simulating_before: Some(true),
                post_action_delay_ms: None,
                final_simulating_value: None,
            }
        }
        GuardBehavior::ReleaseHold { delay_ms } => {
            guard_state.alt_guard_held = false;
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
        SyntheticAction::AltDown => {
            if let Err(e) = enigo.key(Key::Alt, Direction::Press) {
                warn!("Failed to press ALT down: {}", e);
            }
        }
        SyntheticAction::AltUp => {
            if let Err(e) = enigo.key(Key::Alt, Direction::Release) {
                warn!("Failed to release ALT: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(guard_state.alt_guard_held);

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
        assert!(guard_state.alt_guard_held);

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
        assert!(!guard_state.alt_guard_held);
    }

    #[test]
    fn queued_commands_preserve_fifo_order() {
        let (tx, rx) = mpsc::channel();

        assert!(enqueue_with_sender(
            &tx,
            test_job(press_key_command('q')),
            SyntheticAction::KeyClick('q')
        ));
        assert!(enqueue_with_sender(
            &tx,
            test_job(alt_down_command()),
            SyntheticAction::AltDown
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
}
