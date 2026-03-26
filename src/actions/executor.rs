use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tracing::{debug, warn};

type ActionJob = Box<dyn FnOnce() + Send + 'static>;

enum ActionMessage {
    Run { label: &'static str, job: ActionJob },
}

pub struct ActionExecutor {
    tx: Sender<ActionMessage>,
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
                            } else if let Some(message) =
                                panic_payload.downcast_ref::<String>()
                            {
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

        Arc::new(Self { tx })
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

        thread::spawn(move || {
            if !delay.is_zero() {
                thread::sleep(delay);
            }

            if let Err(error) = tx.send(ActionMessage::Run { label, job }) {
                warn!("Failed to enqueue delayed action job {}: {}", label, error);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::ActionExecutor;
    use std::sync::mpsc;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

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
