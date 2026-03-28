use std::cmp::Ordering;
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
#[cfg(test)]
use std::cell::RefCell;
#[cfg(test)]
use std::sync::atomic::AtomicU64;
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, warn};

type ActionJob = Box<dyn FnOnce() + Send + 'static>;

enum ActionMessage {
    Run { label: &'static str, job: ActionJob },
}

#[allow(dead_code)]
struct ScheduledAction {
    due_at: Instant,
    sequence: u64,
    message: ActionMessage,
}

impl ScheduledAction {
    #[cfg(test)]
    fn for_test(base: Instant, due_in: Duration, sequence: u64, label: &'static str) -> Self {
        Self {
            due_at: base + due_in,
            sequence,
            message: ActionMessage::Run {
                label,
                job: Box::new(|| {}),
            },
        }
    }
}

#[allow(dead_code)]
fn scheduled_action_cmp(left: &ScheduledAction, right: &ScheduledAction) -> Ordering {
    left.due_at
        .cmp(&right.due_at)
        .then_with(|| left.sequence.cmp(&right.sequence))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DispatchMode {
    Immediate,
    Delayed(Duration),
}

fn dispatch_mode_for_delay(delay: Duration) -> DispatchMode {
    if delay.is_zero() {
        DispatchMode::Immediate
    } else {
        DispatchMode::Delayed(delay)
    }
}

pub struct ActionExecutor {
    tx: Sender<ActionMessage>,
    #[cfg(test)]
    sequence: AtomicU64,
}

impl ActionExecutor {
    pub fn new() -> Arc<Self> {
        let (tx, rx) = mpsc::channel::<ActionMessage>();

        thread::spawn(move || {
            while let Ok(message) = rx.recv() {
                match message {
                    ActionMessage::Run { label, job } => {
                        debug!("Running action job: {}", label);
                        if let Err(panic_payload) =
                            std::panic::catch_unwind(std::panic::AssertUnwindSafe(job))
                        {
                            let panic_message = if let Some(message) =
                                panic_payload.downcast_ref::<&str>()
                            {
                                *message
                            } else if let Some(message) = panic_payload.downcast_ref::<String>() {
                                message.as_str()
                            } else {
                                "unknown panic payload"
                            };

                            warn!(
                                "Action job {} panicked; executor will continue running: {}",
                                label, panic_message
                            );
                        }
                    }
                }
            }
        });

        Arc::new(Self {
            tx,
            #[cfg(test)]
            sequence: AtomicU64::new(0),
        })
    }

    pub fn enqueue<F>(&self, label: &'static str, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.enqueue_after(label, Duration::ZERO, job);
    }

    pub fn enqueue_after<F>(&self, label: &'static str, delay: Duration, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let tx = self.tx.clone();
        let job = Box::new(job) as ActionJob;

        match dispatch_mode_for_delay(delay) {
            DispatchMode::Immediate => {
                if let Err(error) = tx.send(ActionMessage::Run { label, job }) {
                    warn!("Failed to enqueue action job {}: {}", label, error);
                }
            }
            DispatchMode::Delayed(d) => {
                #[cfg(test)]
                {
                    let sequence = self
                        .sequence
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                    if test_delay_capture_enabled() {
                        capture_test_delayed_action(ScheduledAction {
                            due_at: test_delayed_due_at(d),
                            sequence,
                            message: ActionMessage::Run { label, job },
                        });
                        return;
                    }

                    thread::spawn(move || {
                        thread::sleep(d);
                        if let Err(error) = tx.send(ActionMessage::Run { label, job }) {
                            warn!("Failed to enqueue delayed action job {}: {}", label, error);
                        }
                    });
                    return;
                }

                #[cfg(not(test))]
                {
                    thread::spawn(move || {
                        thread::sleep(d);
                        if let Err(error) = tx.send(ActionMessage::Run { label, job }) {
                            warn!("Failed to enqueue delayed action job {}: {}", label, error);
                        }
                    });
                }
            }
        }
    }
}

#[cfg(test)]
thread_local! {
    static TEST_DELAY_CAPTURE: RefCell<Option<Vec<ScheduledAction>>> = RefCell::new(None);
    static TEST_BASE_INSTANT: Instant = Instant::now();
}

#[cfg(test)]
fn test_delay_capture_enabled() -> bool {
    TEST_DELAY_CAPTURE.with(|capture| capture.borrow().is_some())
}

#[cfg(test)]
fn test_delayed_due_at(delay: Duration) -> Instant {
    TEST_BASE_INSTANT.with(|base| *base + delay)
}

#[cfg(test)]
fn set_test_delay_capture(enabled: bool) {
    TEST_DELAY_CAPTURE.with(|capture| {
        *capture.borrow_mut() = if enabled { Some(Vec::new()) } else { None };
    });
}

#[cfg(test)]
fn capture_test_delayed_action(action: ScheduledAction) {
    TEST_DELAY_CAPTURE.with(|capture| {
        if let Some(actions) = capture.borrow_mut().as_mut() {
            actions.push(action);
        }
    });
}

#[cfg(test)]
fn flush_test_delayed_actions(executor: &ActionExecutor) {
    let actions = TEST_DELAY_CAPTURE.with(|capture| {
        capture
            .borrow_mut()
            .take()
            .expect("test delayed action capture should be enabled")
    });

    let mut actions = actions;
    actions.sort_by(scheduled_action_cmp);

    for action in actions {
        let ActionMessage::Run { label, job } = action.message;
        if let Err(error) = executor.tx.send(ActionMessage::Run { label, job }) {
            warn!("Failed to flush delayed action job {}: {}", label, error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        dispatch_mode_for_delay, flush_test_delayed_actions, scheduled_action_cmp,
        set_test_delay_capture, ActionExecutor, DispatchMode, ScheduledAction,
    };
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::mpsc;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    struct TestDelayCaptureGuard;

    impl TestDelayCaptureGuard {
        fn enable() -> Self {
            set_test_delay_capture(true);
            Self
        }
    }

    impl Drop for TestDelayCaptureGuard {
        fn drop(&mut self) {
            set_test_delay_capture(false);
        }
    }

    #[test]
    fn zero_delay_dispatch_mode_is_immediate() {
        assert_eq!(
            dispatch_mode_for_delay(Duration::ZERO),
            DispatchMode::Immediate
        );
    }

    #[test]
    fn non_zero_delay_dispatch_mode_is_delayed() {
        let delay = Duration::from_millis(1);

        assert_eq!(dispatch_mode_for_delay(delay), DispatchMode::Delayed(delay));
    }

    #[test]
    fn scheduled_action_orders_earlier_deadline_first() {
        let base = Instant::now();
        let earlier = ScheduledAction::for_test(base, Duration::from_millis(10), 0, "earlier");
        let later = ScheduledAction::for_test(base, Duration::from_millis(30), 1, "later");

        assert_eq!(
            scheduled_action_cmp(&earlier, &later),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    fn equal_deadline_delayed_jobs_run_fifo_relative_to_call_order() {
        let base = Instant::now();
        let first = ScheduledAction::for_test(base, Duration::from_millis(10), 0, "first");
        let second = ScheduledAction::for_test(base, Duration::from_millis(10), 1, "second");

        assert_eq!(
            scheduled_action_cmp(&first, &second),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    fn enqueue_runs_job() {
        let executor = ActionExecutor::new();
        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = flag.clone();

        executor.enqueue("test-immediate", move || {
            flag_clone.store(true, Ordering::SeqCst);
        });

        for _ in 0..50 {
            if flag.load(Ordering::SeqCst) {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        panic!("executor did not run immediate job");
    }

    #[test]
    fn enqueue_after_equal_deadline_delayed_jobs_run_fifo_relative_to_call_order() {
        let executor = ActionExecutor::new();
        let (tx, rx) = mpsc::channel::<&'static str>();

        let _capture_guard = TestDelayCaptureGuard::enable();

        let first_tx = tx.clone();
        executor.enqueue_after(
            "test-first-delayed",
            Duration::from_millis(25),
            move || {
                let _ = first_tx.send("first");
            },
        );

        executor.enqueue_after(
            "test-second-delayed",
            Duration::from_millis(25),
            move || {
                let _ = tx.send("second");
            },
        );

        flush_test_delayed_actions(&executor);

        let first = rx
            .recv_timeout(Duration::from_millis(50))
            .expect("first delayed job should run");
        assert_eq!(first, "first");

        let second = rx
            .recv_timeout(Duration::from_millis(50))
            .expect("second delayed job should run");
        assert_eq!(second, "second");
    }

    #[test]
    fn enqueue_after_waits_before_running_job() {
        let executor = ActionExecutor::new();
        let start = Instant::now();
        let elapsed_ms = Arc::new(AtomicU64::new(0));
        let elapsed_clone = elapsed_ms.clone();

        executor.enqueue_after("test-delayed", Duration::from_millis(40), move || {
            elapsed_clone.store(start.elapsed().as_millis() as u64, Ordering::SeqCst);
        });

        for _ in 0..80 {
            let elapsed = elapsed_ms.load(Ordering::SeqCst);
            if elapsed > 0 {
                assert!(elapsed >= 20, "job ran too early: {elapsed}ms");
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        panic!("executor did not run delayed job");
    }

    #[test]
    fn zero_delay_enqueue_after_uses_immediate_fast_path() {
        let executor = ActionExecutor::new();
        let (tx, rx) = mpsc::channel::<&'static str>();

        let delayed_tx = tx.clone();
        executor.enqueue_after("test-delayed", Duration::from_millis(80), move || {
            let _ = delayed_tx.send("delayed");
        });

        executor.enqueue_after("test-zero-delay", Duration::ZERO, move || {
            let _ = tx.send("zero-delay");
        });

        // This is a behavior-level regression: zero-delay enqueue_after must stay
        // on the immediate path even if delayed scheduling internals change later.
        let first = rx
            .recv_timeout(Duration::from_millis(40))
            .expect("zero-delay enqueue_after should use the immediate fast path");
        assert_eq!(first, "zero-delay");

        let second = rx
            .recv_timeout(Duration::from_millis(120))
            .expect("delayed job should still run after its delay");
        assert_eq!(second, "delayed");
    }

    #[test]
    fn newly_queued_earlier_deadline_preempts_longer_existing_wait() {
        let executor = ActionExecutor::new();
        let (tx, rx) = mpsc::channel::<&'static str>();

        let later_tx = tx.clone();
        executor.enqueue_after("test-later-deadline", Duration::from_millis(80), move || {
            let _ = later_tx.send("later");
        });

        executor.enqueue_after("test-earlier-deadline", Duration::from_millis(20), move || {
            let _ = tx.send("earlier");
        });

        let first = rx
            .recv_timeout(Duration::from_millis(60))
            .expect("earlier deadline should run before later deadline");
        assert_eq!(first, "earlier");

        let second = rx
            .recv_timeout(Duration::from_millis(120))
            .expect("later deadline should still run after the earlier job");
        assert_eq!(second, "later");
    }

    #[test]
    fn delayed_job_does_not_block_immediate_job() {
        let executor = ActionExecutor::new();
        let (tx, rx) = mpsc::channel::<&'static str>();

        let delayed_tx = tx.clone();
        executor.enqueue_after("test-delayed-first", Duration::from_millis(80), move || {
            let _ = delayed_tx.send("delayed");
        });

        executor.enqueue("test-immediate-second", move || {
            let _ = tx.send("immediate");
        });

        let first = rx
            .recv_timeout(Duration::from_millis(40))
            .expect("immediate job should not be blocked behind delayed job");
        assert_eq!(first, "immediate");

        let second = rx
            .recv_timeout(Duration::from_millis(120))
            .expect("delayed job should still run after its delay");
        assert_eq!(second, "delayed");
    }

    #[test]
    fn executor_survives_panicking_job() {
        let executor = ActionExecutor::new();
        let (tx, rx) = mpsc::channel::<&'static str>();

        executor.enqueue("test-panic", move || {
            panic!("boom");
        });

        let success_tx = tx.clone();
        executor.enqueue("test-after-panic", move || {
            let _ = success_tx.send("still-running");
        });

        let message = rx
            .recv_timeout(Duration::from_millis(100))
            .expect("executor should continue after a panicking job");
        assert_eq!(message, "still-running");
    }
}
