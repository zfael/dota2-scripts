use std::collections::VecDeque;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::time::SystemTime;

/// Maximum entries retained in the buffer before oldest are dropped.
const MAX_BUFFER_SIZE: usize = 200;

/// Category of an activity event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivityCategory {
    Action,
    Danger,
    Warning,
    Error,
    System,
}

impl ActivityCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActivityCategory::Action => "action",
            ActivityCategory::Danger => "danger",
            ActivityCategory::Warning => "warning",
            ActivityCategory::Error => "error",
            ActivityCategory::System => "system",
        }
    }
}

/// A single activity event produced by the backend.
#[derive(Debug, Clone)]
pub struct ActivityEntry {
    pub timestamp: SystemTime,
    pub category: ActivityCategory,
    pub message: String,
    pub details: Option<String>,
}

static ACTIVITY_BUFFER: LazyLock<Mutex<VecDeque<ActivityEntry>>> =
    LazyLock::new(|| Mutex::new(VecDeque::with_capacity(MAX_BUFFER_SIZE)));

/// Push an activity event into the global buffer.
pub fn push_activity(category: ActivityCategory, message: impl Into<String>) {
    if let Ok(mut buf) = ACTIVITY_BUFFER.lock() {
        if buf.len() >= MAX_BUFFER_SIZE {
            buf.pop_front();
        }
        buf.push_back(ActivityEntry {
            timestamp: SystemTime::now(),
            category,
            message: message.into(),
            details: None,
        });
    }
}

/// Push an activity event with optional details.
pub fn push_activity_with_details(
    category: ActivityCategory,
    message: impl Into<String>,
    details: impl Into<String>,
) {
    if let Ok(mut buf) = ACTIVITY_BUFFER.lock() {
        if buf.len() >= MAX_BUFFER_SIZE {
            buf.pop_front();
        }
        buf.push_back(ActivityEntry {
            timestamp: SystemTime::now(),
            category,
            message: message.into(),
            details: Some(details.into()),
        });
    }
}

/// Drain all pending activity entries from the buffer.
/// Returns an empty vec if the buffer is empty or the lock is poisoned.
pub fn drain_activities() -> Vec<ActivityEntry> {
    match ACTIVITY_BUFFER.lock() {
        Ok(mut buf) => buf.drain(..).collect(),
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn shared_test_lock() -> &'static Mutex<()> {
        static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        TEST_LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn test_push_and_drain() {
        let _guard = shared_test_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        // Drain any pre-existing entries (from concurrent tests sharing the global buffer)
        drain_activities();

        push_activity(ActivityCategory::System, "test message");
        push_activity_with_details(
            ActivityCategory::Action,
            "action msg",
            "some details",
        );

        let entries = drain_activities();
        // Filter to only our test entries (other parallel tests may push to the global buffer)
        let entries: Vec<_> = entries
            .into_iter()
            .filter(|e| e.message == "test message" || e.message == "action msg")
            .collect();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].category, ActivityCategory::System);
        assert_eq!(entries[0].message, "test message");
        assert!(entries[0].details.is_none());
        assert_eq!(entries[1].category, ActivityCategory::Action);
        assert_eq!(entries[1].message, "action msg");
        assert_eq!(entries[1].details.as_deref(), Some("some details"));

        // Buffer should be empty after drain
        let entries = drain_activities();
        let entries: Vec<_> = entries
            .into_iter()
            .filter(|e| e.message == "test message" || e.message == "action msg")
            .collect();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_buffer_overflow() {
        let _guard = shared_test_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        drain_activities();

        for i in 0..MAX_BUFFER_SIZE + 10 {
            push_activity(ActivityCategory::System, format!("overflow_msg {}", i));
        }

        let entries = drain_activities();
        // The total buffer (including any concurrent test entries) must not exceed MAX_BUFFER_SIZE
        assert!(entries.len() <= MAX_BUFFER_SIZE);
        // Our overflow entries should be present (oldest ones dropped)
        let our_entries: Vec<_> = entries
            .into_iter()
            .filter(|e| e.message.starts_with("overflow_msg "))
            .collect();
        assert!(!our_entries.is_empty());
        // The entries we do have should be the later ones (oldest dropped first)
        let first_msg = &our_entries[0].message;
        let first_idx: usize = first_msg
            .strip_prefix("overflow_msg ")
            .unwrap()
            .parse()
            .unwrap();
        assert!(first_idx > 0, "oldest entries should have been dropped");
    }
}
