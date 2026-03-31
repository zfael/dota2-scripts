# Plan 3: Live Data Wiring & Enhancements

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace all hardcoded/mock data in the Tauri integration with live backend state, add activity logging, executor metrics, config validation, Meepo state panel, and rune alert audio.

**Architecture:** Wire existing library crate globals (`danger_detector::is_in_danger()`, `SOUL_RING_STATE`, `latest_meepo_observed_state()`) into Tauri IPC commands. Create a lightweight activity buffer in the library crate so action code can push events. Add atomic metric counters to `ActionExecutor`. The Tauri event emitter drains the activity buffer and emits events to the frontend. Frontend changes are minimal — activity store wiring, one new Meepo state component, and a Web Audio API rune alert.

**Tech Stack:** Rust (lazy_static/LazyLock globals, AtomicU64 counters, VecDeque buffer), TypeScript/React (Zustand stores, Tauri event listener, Web Audio API)

---

## File Structure

### New Files

| File | Purpose |
|------|---------|
| `src/actions/activity.rs` | Global activity buffer — `push_activity()` + `drain_activities()` |
| `src-tauri/src/commands/meepo.rs` | `get_meepo_state` Tauri command |

### Modified Files — Library Crate

| File | Changes |
|------|---------|
| `src/actions/mod.rs` | Export `activity` module |
| `src/actions/executor.rs` | Add `ExecutorMetrics` struct with atomic counters |
| `src/actions/danger_detector.rs` | Add `push_activity` calls on danger detected/cleared |
| `src/actions/common.rs` | Add `push_activity` calls on defensive item use |
| `src/actions/soul_ring.rs` | Add `push_activity` call on soul ring trigger |
| `src/actions/dispatcher.rs` | Add `push_activity` call on hero action dispatch |
| `src/gsi/handler.rs` | Add `push_activity` call on GSI connect / hero detect |

### Modified Files — Tauri Crate

| File | Changes |
|------|---------|
| `src-tauri/src/lib.rs` | Add `executor_metrics` to `TauriAppState`, register `get_meepo_state` |
| `src-tauri/src/events.rs` | Drain activity buffer, emit `activity_event`; wire `in_danger` |
| `src-tauri/src/commands/game.rs` | Wire `in_danger` to `danger_detector::is_in_danger()` |
| `src-tauri/src/commands/diagnostics.rs` | Wire real soul ring state, blocked keys, executor metrics |
| `src-tauri/src/commands/config.rs` | Add validation before persist |
| `src-tauri/src/commands/mod.rs` | Add `pub mod meepo;` |
| `src-tauri/src/ipc_types.rs` | Add `MeepoStateDto` |

### Modified Files — Frontend

| File | Changes |
|------|---------|
| `src-ui/src/stores/activityStore.ts` | Wire to Tauri `activity_event`, remove mock data |
| `src-ui/src/pages/heroes/configs/MeepoConfig.tsx` | Add observed state panel |
| `src-ui/src/types/game.ts` | Add `MeepoObservedState` type |
| `src-ui/src/components/common/RuneAlert.tsx` | New: rune alert audio hook |
| `src-ui/src/App.tsx` | Add rune alert hook |

---

## Task Dependency Graph

```
Task 1 (danger state)        ─┐
Task 2 (soul ring + keys)    ─┤── independent, parallelizable
Task 3 (executor metrics)    ─┘
Task 4 (activity buffer)     ── foundation for Task 5 & 6
Task 5 (push_activity calls) ── depends on Task 4
Task 6 (activity frontend)   ── depends on Task 4
Task 7 (config validation)   ── independent
Task 8 (meepo state panel)   ── independent
Task 9 (rune alert audio)    ── independent
Task 10 (verification)       ── depends on all above
```

---

### Task 1: Wire danger state to `is_in_danger()`

**Files:**
- Modify: `src-tauri/src/events.rs`
- Modify: `src-tauri/src/commands/game.rs`

- [ ] **Step 1: Update `build_game_state_dto` in events.rs**

Replace the two `in_danger: false` hardcoded values with a call to the danger detector.

In `src-tauri/src/events.rs`, add the import at the top:

```rust
use dota2_scripts::actions::danger_detector;
```

Then replace the function body. The `if let Some(ref event)` branch gets:

```rust
in_danger: danger_detector::is_in_danger(),
```

And the `else` branch (no event) keeps:

```rust
in_danger: false,
```

Full updated function:

```rust
fn build_game_state_dto(state: &dota2_scripts::state::AppState) -> GameStateDto {
    if let Some(ref event) = state.last_event {
        let rune_timer = state
            .rune_alerts
            .as_ref()
            .and_then(|ra| ra.seconds_until_next_rune);

        GameStateDto {
            hero_name: state
                .selected_hero
                .map(|h| h.to_display_name().to_string()),
            hero_level: event.hero.level,
            hp_percent: event.hero.health_percent,
            mana_percent: event.hero.mana_percent,
            in_danger: danger_detector::is_in_danger(),
            connected: true,
            alive: event.hero.alive,
            stunned: event.hero.stunned,
            silenced: event.hero.silenced,
            respawn_timer: if event.hero.respawn_seconds > 0 {
                Some(event.hero.respawn_seconds)
            } else {
                None
            },
            rune_timer,
            game_time: event.map.clock_time,
        }
    } else {
        GameStateDto {
            hero_name: None,
            hero_level: 0,
            hp_percent: 100,
            mana_percent: 100,
            in_danger: false,
            connected: false,
            alive: true,
            stunned: false,
            silenced: false,
            respawn_timer: None,
            rune_timer: None,
            game_time: 0,
        }
    }
}
```

- [ ] **Step 2: Update `get_game_state` command in game.rs**

In `src-tauri/src/commands/game.rs`, add the import:

```rust
use dota2_scripts::actions::danger_detector;
```

Replace the two `in_danger: false` lines. The connected branch:

```rust
in_danger: danger_detector::is_in_danger(),
```

The disconnected `else` branch keeps `in_danger: false`.

- [ ] **Step 3: Run tests**

```bash
cargo test --quiet
```

Expected: all tests pass (no behavior change — `is_in_danger()` returns false when no GSI events have been processed, matching the current hardcoded behavior).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/events.rs src-tauri/src/commands/game.rs
git commit -m "feat: wire danger state to live danger detector"
```

---

### Task 2: Wire soul ring state and blocked keys

**Files:**
- Modify: `src-tauri/src/commands/diagnostics.rs`

- [ ] **Step 1: Wire soul ring state**

In `src-tauri/src/commands/diagnostics.rs`, add imports:

```rust
use dota2_scripts::actions::SOUL_RING_STATE;
```

Replace the `soul_ring_state: "ready".to_string()` line with logic that reads the actual state:

```rust
soul_ring_state: {
    match SOUL_RING_STATE.lock() {
        Ok(state) => {
            if !state.available {
                "unavailable".to_string()
            } else if !state.can_cast {
                "cooldown".to_string()
            } else {
                "ready".to_string()
            }
        }
        Err(_) => "unknown".to_string(),
    }
},
```

- [ ] **Step 2: Wire blocked keys**

Replace the `blocked_keys: vec![]` line with a derived list based on current app state. Still in `diagnostics.rs`, the `get_diagnostics` function already has access to `app` (the locked `AppState`). Read the relevant flags:

```rust
blocked_keys: {
    let mut keys = Vec::new();
    if *app.sf_enabled.lock().unwrap_or(&mut false) {
        keys.extend(["Q", "W", "E"].iter().map(|s| s.to_string()));
    }
    if *app.od_enabled.lock().unwrap_or(&mut false) {
        keys.push("R".to_string());
    }
    if let Ok(sr) = SOUL_RING_STATE.lock() {
        if sr.available && sr.can_cast {
            if let Some(key) = sr.slot_key {
                keys.push(format!("SoulRing({})", key));
            }
        }
    }
    keys
},
```

Actually, `Mutex::lock()` returns `Result<MutexGuard, PoisonError>` — we can't use `unwrap_or` on it. Here's the corrected code:

```rust
blocked_keys: {
    let mut keys = Vec::new();
    if app.sf_enabled.lock().map(|v| *v).unwrap_or(false) {
        keys.extend(["Q", "W", "E"].iter().map(|s| s.to_string()));
    }
    if app.od_enabled.lock().map(|v| *v).unwrap_or(false) {
        keys.push("R".to_string());
    }
    if let Ok(sr) = SOUL_RING_STATE.lock() {
        if sr.available && sr.can_cast {
            if let Some(key) = sr.slot_key {
                keys.push(format!("SoulRing({})", key));
            }
        }
    }
    keys
},
```

Full updated `get_diagnostics` function:

```rust
use crate::ipc_types::{DiagnosticsDto, QueueMetricsDto, SyntheticInputDto};
use crate::TauriAppState;
use dota2_scripts::actions::SOUL_RING_STATE;

#[tauri::command]
pub fn get_diagnostics(state: tauri::State<'_, TauriAppState>) -> Result<DiagnosticsDto, String> {
    let app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;

    Ok(DiagnosticsDto {
        gsi_connected: app.last_event.is_some(),
        keyboard_hook_active: true,
        queue_metrics: QueueMetricsDto {
            events_processed: app.metrics.events_processed,
            events_dropped: app.metrics.events_dropped,
            current_queue_depth: app.metrics.current_queue_depth,
            max_queue_depth: 10,
        },
        synthetic_input: SyntheticInputDto {
            queue_depth: 0,
            total_queued: 0,
            peak_depth: 0,
            completions: 0,
            drops: 0,
        },
        soul_ring_state: {
            match SOUL_RING_STATE.lock() {
                Ok(sr) => {
                    if !sr.available {
                        "unavailable".to_string()
                    } else if !sr.can_cast {
                        "cooldown".to_string()
                    } else {
                        "ready".to_string()
                    }
                }
                Err(_) => "unknown".to_string(),
            }
        },
        blocked_keys: {
            let mut keys = Vec::new();
            if app.sf_enabled.lock().map(|v| *v).unwrap_or(false) {
                keys.extend(["Q", "W", "E"].iter().map(|s| s.to_string()));
            }
            if app.od_enabled.lock().map(|v| *v).unwrap_or(false) {
                keys.push("R".to_string());
            }
            if let Ok(sr) = SOUL_RING_STATE.lock() {
                if sr.available && sr.can_cast {
                    if let Some(key) = sr.slot_key {
                        keys.push(format!("SoulRing({})", key));
                    }
                }
            }
            keys
        },
    })
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test --quiet
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands/diagnostics.rs
git commit -m "feat: wire soul ring state and blocked keys to diagnostics"
```

---

### Task 3: Add executor metrics

**Files:**
- Modify: `src/actions/executor.rs` (library crate)
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/commands/diagnostics.rs`

- [ ] **Step 1: Add `ExecutorMetrics` struct to executor.rs**

In `src/actions/executor.rs`, add a new public struct and a snapshot type near the top (after existing imports):

```rust
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

/// Shared atomic counters for monitoring executor throughput.
#[derive(Debug)]
pub struct ExecutorMetrics {
    pub total_enqueued: AtomicU64,
    pub completions: AtomicU64,
    pub drops: AtomicU64,
}

/// Point-in-time snapshot of executor metrics.
#[derive(Debug, Clone)]
pub struct ExecutorMetricsSnapshot {
    pub total_queued: u64,
    pub completions: u64,
    pub drops: u64,
    pub queue_depth: u64,
}

impl ExecutorMetrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            total_enqueued: AtomicU64::new(0),
            completions: AtomicU64::new(0),
            drops: AtomicU64::new(0),
        })
    }

    pub fn snapshot(&self) -> ExecutorMetricsSnapshot {
        let enqueued = self.total_enqueued.load(AtomicOrdering::Relaxed);
        let completed = self.completions.load(AtomicOrdering::Relaxed);
        let dropped = self.drops.load(AtomicOrdering::Relaxed);
        ExecutorMetricsSnapshot {
            total_queued: enqueued,
            completions: completed,
            drops: dropped,
            queue_depth: enqueued.saturating_sub(completed + dropped),
        }
    }
}
```

Note: `AtomicOrdering` is already imported in executor.rs as `AtomicOrdering` — check the existing import. The file already uses `use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};` or similar. Reuse the existing import alias.

- [ ] **Step 2: Store metrics in `ActionExecutor` and wire counters**

Add `metrics: Arc<ExecutorMetrics>` field to the `ActionExecutor` struct:

```rust
pub struct ActionExecutor {
    ready_tx: Sender<ActionMessage>,
    delayed_tx: Sender<ScheduledAction>,
    sequence: AtomicU64,
    metrics: Arc<ExecutorMetrics>,
}
```

Update `ActionExecutor::new()` to create metrics and pass a clone to the ready worker:

```rust
impl ActionExecutor {
    pub fn new() -> Arc<Self> {
        let (ready_tx, ready_rx) = mpsc::channel::<ActionMessage>();
        let (delayed_tx, delayed_rx) = mpsc::channel::<ScheduledAction>();
        let metrics = ExecutorMetrics::new();

        let worker_metrics = metrics.clone();
        let ready_worker_tx = ready_tx.clone();
        thread::Builder::new()
            .name("action-ready-worker".to_string())
            .spawn(move || run_ready_worker(ready_rx, worker_metrics))
            .expect("failed to spawn ready worker thread");

        thread::Builder::new()
            .name("action-delayed-scheduler".to_string())
            .spawn(move || run_delayed_scheduler(delayed_rx, ready_worker_tx))
            .expect("failed to spawn delayed scheduler thread");

        Arc::new(Self {
            ready_tx,
            delayed_tx,
            sequence: AtomicU64::new(0),
            metrics,
        })
    }

    /// Returns a clone of the metrics handle for external monitoring.
    pub fn metrics(&self) -> Arc<ExecutorMetrics> {
        self.metrics.clone()
    }
}
```

- [ ] **Step 3: Increment counters in `enqueue_after` and `run_ready_worker`**

In `enqueue_after`, increment `total_enqueued` on every successful enqueue, and `drops` on failure:

For the immediate path:
```rust
DispatchMode::Immediate => {
    if let Err(error) = self.ready_tx.send(ActionMessage::Run { label, job }) {
        self.metrics.drops.fetch_add(1, AtomicOrdering::Relaxed);
        warn!("Failed to enqueue action job {}: {}", label, error);
    } else {
        self.metrics.total_enqueued.fetch_add(1, AtomicOrdering::Relaxed);
    }
}
```

For the delayed path (non-test):
```rust
if let Err(error) = self.delayed_tx.send(ScheduledAction {
    due_at: Instant::now() + d,
    sequence,
    message: ActionMessage::Run { label, job },
}) {
    self.metrics.drops.fetch_add(1, AtomicOrdering::Relaxed);
    warn!("Failed to enqueue delayed action job {}: {}", label, error);
} else {
    self.metrics.total_enqueued.fetch_add(1, AtomicOrdering::Relaxed);
}
```

Update `run_ready_worker` signature to accept metrics, and increment `completions` after each job:

```rust
fn run_ready_worker(rx: Receiver<ActionMessage>, metrics: Arc<ExecutorMetrics>) {
    while let Ok(message) = rx.recv() {
        match message {
            ActionMessage::Run { label, job } => {
                debug!("Running action job: {}", label);
                if let Err(panic_payload) =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(job))
                {
                    let panic_message = if let Some(message) = panic_payload.downcast_ref::<&str>()
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
                metrics.completions.fetch_add(1, AtomicOrdering::Relaxed);
            }
        }
    }
}
```

- [ ] **Step 4: Run library tests**

```bash
cargo test --lib --quiet
```

Expected: all tests pass. The existing executor tests use the test-only delayed-capture seam and should still work since we're only adding counters.

- [ ] **Step 5: Store executor metrics in TauriAppState**

In `src-tauri/src/lib.rs`, update `TauriAppState`:

```rust
use dota2_scripts::actions::executor::ExecutorMetrics;

pub struct TauriAppState {
    pub app_state: Arc<Mutex<AppState>>,
    pub settings: Arc<Mutex<Settings>>,
    pub executor_metrics: Arc<ExecutorMetrics>,
}
```

In the `run()` function, after creating the executor, capture its metrics:

```rust
let action_executor = ActionExecutor::new();
let executor_metrics = action_executor.metrics();
let dispatcher = Arc::new(ActionDispatcher::new(settings.clone(), action_executor));
```

Update the `.manage()` call:

```rust
.manage(TauriAppState {
    app_state: app_state.clone(),
    settings: settings.clone(),
    executor_metrics,
})
```

- [ ] **Step 6: Wire executor metrics to diagnostics command**

In `src-tauri/src/commands/diagnostics.rs`, replace the hardcoded `SyntheticInputDto` zeros:

```rust
synthetic_input: {
    let snap = state.executor_metrics.snapshot();
    SyntheticInputDto {
        queue_depth: snap.queue_depth as usize,
        total_queued: snap.total_queued,
        peak_depth: 0, // not tracked
        completions: snap.completions,
        drops: snap.drops,
    }
},
```

- [ ] **Step 7: Run all tests**

```bash
cargo test --quiet
```

Expected: all tests pass.

- [ ] **Step 8: Commit**

```bash
git add src/actions/executor.rs src-tauri/src/lib.rs src-tauri/src/commands/diagnostics.rs
git commit -m "feat: add executor metrics and wire to diagnostics"
```

---

### Task 4: Create activity buffer module

**Files:**
- Create: `src/actions/activity.rs`
- Modify: `src/actions/mod.rs`

- [ ] **Step 1: Create the activity buffer module**

Create `src/actions/activity.rs`:

```rust
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

    #[test]
    fn test_push_and_drain() {
        // Drain any pre-existing entries
        drain_activities();

        push_activity(ActivityCategory::System, "test message");
        push_activity_with_details(
            ActivityCategory::Action,
            "action msg",
            "some details",
        );

        let entries = drain_activities();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].category, ActivityCategory::System);
        assert_eq!(entries[0].message, "test message");
        assert!(entries[0].details.is_none());
        assert_eq!(entries[1].category, ActivityCategory::Action);
        assert_eq!(entries[1].message, "action msg");
        assert_eq!(entries[1].details.as_deref(), Some("some details"));

        // Buffer should be empty after drain
        let entries = drain_activities();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_buffer_overflow() {
        drain_activities();

        for i in 0..MAX_BUFFER_SIZE + 10 {
            push_activity(ActivityCategory::System, format!("msg {}", i));
        }

        let entries = drain_activities();
        assert_eq!(entries.len(), MAX_BUFFER_SIZE);
        // Oldest entries should have been dropped
        assert_eq!(entries[0].message, "msg 10");
    }
}
```

- [ ] **Step 2: Export the activity module**

In `src/actions/mod.rs`, add the module declaration alongside the existing ones:

```rust
pub mod activity;
```

- [ ] **Step 3: Run tests**

```bash
cargo test activity --quiet
```

Expected: 2 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/actions/activity.rs src/actions/mod.rs
git commit -m "feat: add global activity buffer for UI event logging"
```

---

### Task 5: Add push_activity calls at key action points

**Files:**
- Modify: `src/actions/danger_detector.rs`
- Modify: `src/actions/common.rs`
- Modify: `src/actions/soul_ring.rs`
- Modify: `src/gsi/handler.rs`

This task adds `push_activity()` calls at the most important action points. We do NOT add calls everywhere — only at high-signal events the user wants to see in their activity log.

- [ ] **Step 1: Add activity calls to danger_detector.rs**

In `src/actions/danger_detector.rs`, add the import:

```rust
use crate::actions::activity::{push_activity, ActivityCategory};
```

Add a `push_activity` call right after the existing `info!()` on danger detected (inside the `if in_danger && !tracker.danger_detected` block):

```rust
if in_danger && !tracker.danger_detected {
    tracker.danger_detected = true;
    tracker.danger_start_time = Some(now);
    info!(
        "⚠️ DANGER DETECTED! HP: {}/{} ({}%), lost {}HP in {}ms",
        current_hp, max_hp, current_hp_percent, hp_delta, time_delta_ms
    );
    push_activity(
        ActivityCategory::Danger,
        format!("⚠ Danger detected — HP {}%", current_hp_percent),
    );
}
```

Add a `push_activity` call right after the existing `info!()` on danger cleared:

```rust
if danger_start.elapsed().as_secs() >= config.clear_delay_seconds {
    tracker.danger_detected = false;
    tracker.danger_start_time = None;
    info!("✓ Danger cleared - HP stabilized at {}HP ({}%)", current_hp, current_hp_percent);
    push_activity(
        ActivityCategory::Danger,
        format!("✓ Danger cleared — HP {}%", current_hp_percent),
    );
}
```

- [ ] **Step 2: Add activity calls to common.rs for defensive items**

In `src/actions/common.rs`, add the import:

```rust
use crate::actions::activity::{push_activity, ActivityCategory};
```

Find where defensive items are activated (BKB, Satanic, Armlet toggle, etc.) — look for the existing `info!()` or `debug!()` calls near item activation. Add `push_activity` calls next to each one. The exact locations depend on the code structure, but the pattern is:

After any `simulation::press_key` call for a defensive item, add:

```rust
push_activity(ActivityCategory::Action, format!("{} activated (HP {}%)", item_name, hp_percent));
```

Where `item_name` is the item being used (e.g., "BKB", "Satanic", "Armlet").

Note to implementer: Search `common.rs` for all `info!` or `press_key` calls related to item usage and add an activity push after each. Common patterns to look for:
- "bkb" / "black_king_bar" activation → `push_activity(ActivityCategory::Action, "Auto-BKB activated")`
- "satanic" activation → `push_activity(ActivityCategory::Action, format!("Satanic activated (HP {}%)", hp_pct))`
- Healing item usage → `push_activity(ActivityCategory::Action, format!("{} used", item_name))`

If `common.rs` doesn't have obvious logging points for individual items, add activity calls at the entry point where the survivability pipeline runs, with a generic message:

```rust
push_activity(ActivityCategory::Action, "Survivability item activated");
```

- [ ] **Step 3: Add activity call to soul_ring.rs**

In `src/actions/soul_ring.rs`, add the import:

```rust
use crate::actions::activity::{push_activity, ActivityCategory};
```

Find the function that triggers the Soul Ring combo (look for where the actual key press simulation happens — likely `press_ability` or similar). Add after the trigger:

```rust
push_activity(ActivityCategory::Action, format!("Soul Ring → {}", ability_key));
```

Where `ability_key` is the key that was queued after Soul Ring.

- [ ] **Step 4: Add activity calls to handler.rs for system events**

In `src/gsi/handler.rs`, add the import:

```rust
use crate::actions::activity::{push_activity, ActivityCategory};
```

Add a system activity when the first GSI event is received (when `last_event` transitions from `None` to `Some`). Find the spot where `app_state.last_event` is set. Before updating it, check if it was previously `None`:

```rust
let was_disconnected = state.last_event.is_none();
// ... (existing state update) ...
if was_disconnected {
    push_activity(
        ActivityCategory::System,
        format!("GSI connected — hero: {}", event.hero.name),
    );
}
```

Also, when the selected hero changes (if tracked), add:

```rust
push_activity(
    ActivityCategory::System,
    format!("Hero detected: {}", hero_display_name),
);
```

Note to implementer: Only add 1-2 system activities in handler.rs. Don't log every single GSI event — that would flood the buffer.

- [ ] **Step 5: Run tests**

```bash
cargo test --quiet
```

Expected: all tests pass. The push_activity calls are fire-and-forget, so they don't affect existing logic.

- [ ] **Step 6: Commit**

```bash
git add src/actions/danger_detector.rs src/actions/common.rs src/actions/soul_ring.rs src/gsi/handler.rs
git commit -m "feat: add activity logging at key action points"
```

---

### Task 6: Wire activity events to frontend

**Files:**
- Modify: `src-tauri/src/events.rs`
- Modify: `src-tauri/src/ipc_types.rs` (already has `ActivityEntryDto`)
- Modify: `src-ui/src/stores/activityStore.ts`

- [ ] **Step 1: Emit activity events from the Tauri event emitter**

In `src-tauri/src/events.rs`, add imports:

```rust
use crate::ipc_types::ActivityEntryDto;
use dota2_scripts::actions::activity;
use std::time::UNIX_EPOCH;
use std::sync::atomic::{AtomicU64, Ordering};
```

Add a static counter for generating unique activity IDs:

```rust
static ACTIVITY_ID_COUNTER: AtomicU64 = AtomicU64::new(1);
```

In `start_game_state_emitter`, after the existing game state emission block (after `let _ = app.emit("gsi_update", &dto);`), add activity draining. The full updated function:

```rust
pub fn start_game_state_emitter(app: AppHandle) {
    let tauri_state = app.state::<TauriAppState>();
    let app_state = tauri_state.app_state.clone();

    tauri::async_runtime::spawn(async move {
        let mut last_events_processed: u64 = 0;

        loop {
            tokio::time::sleep(Duration::from_millis(200)).await;

            // Emit game state if changed
            {
                let dto = {
                    let state = match app_state.lock() {
                        Ok(s) => s,
                        Err(_) => continue,
                    };

                    if state.metrics.events_processed != last_events_processed {
                        last_events_processed = state.metrics.events_processed;
                        Some(build_game_state_dto(&state))
                    } else {
                        None
                    }
                };

                if let Some(dto) = dto {
                    let _ = app.emit("gsi_update", &dto);
                }
            }

            // Drain and emit activity events
            let entries = activity::drain_activities();
            for entry in entries {
                let id = ACTIVITY_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
                let timestamp = entry
                    .timestamp
                    .duration_since(UNIX_EPOCH)
                    .map(|d| {
                        let secs = d.as_secs() % 86400;
                        let hours = secs / 3600;
                        let minutes = (secs % 3600) / 60;
                        let seconds = secs % 60;
                        let millis = d.subsec_millis();
                        format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis)
                    })
                    .unwrap_or_else(|_| "00:00:00.000".to_string());

                let dto = ActivityEntryDto {
                    id: id.to_string(),
                    timestamp,
                    category: entry.category.as_str().to_string(),
                    message: entry.message,
                    details: entry.details,
                };
                let _ = app.emit("activity_event", &dto);
            }
        }
    });
}
```

- [ ] **Step 2: Wire activityStore to Tauri events, remove mock data**

Replace `src-ui/src/stores/activityStore.ts` with:

```typescript
import { create } from "zustand";
import type { ActivityEntry, ActivityCategory } from "../types/activity";
import { isTauri } from "../lib/tauri";

interface ActivityStore {
  entries: ActivityEntry[];
  filter: ActivityCategory | "all";
  setFilter: (filter: ActivityCategory | "all") => void;
  addEntry: (entry: ActivityEntry) => void;
  clear: () => void;
  filteredEntries: () => ActivityEntry[];
  startListening: () => Promise<() => void>;
}

export const useActivityStore = create<ActivityStore>((set, get) => ({
  entries: [],
  filter: "all",
  setFilter: (filter) => set({ filter }),
  addEntry: (entry) =>
    set((state) => ({ entries: [...state.entries.slice(-499), entry] })),
  clear: () => set({ entries: [] }),
  filteredEntries: () => {
    const { entries, filter } = get();
    return filter === "all"
      ? entries
      : entries.filter((e) => e.category === filter);
  },
  startListening: async () => {
    if (!isTauri()) return () => {};

    const { listen } = await import("@tauri-apps/api/event");

    const unlisten = await listen<ActivityEntry>("activity_event", (event) => {
      get().addEntry(event.payload);
    });

    return unlisten;
  },
}));
```

- [ ] **Step 3: Initialize activity listener in App.tsx**

In `src-ui/src/App.tsx`, add the activity store import and start listening in the init effect:

```typescript
import { useActivityStore } from "./stores/activityStore";
```

Update the `useEffect`:

```typescript
useEffect(() => {
  useConfigStore.getState().loadConfig();
  useUIStore.getState().loadInitialState();
  const gameUnlistenPromise = useGameStore.getState().startListening();
  const activityUnlistenPromise = useActivityStore.getState().startListening();
  useUpdateStore.getState().loadInitialState();

  return () => {
    gameUnlistenPromise.then((unlisten) => unlisten());
    activityUnlistenPromise.then((unlisten) => unlisten());
  };
}, []);
```

- [ ] **Step 4: Run frontend tests**

```bash
cd src-ui && npx vitest run
```

Expected: all tests pass. The mock data import is removed, so if any test imports `mockActivityLog`, it needs updating. Check for test failures and fix imports.

- [ ] **Step 5: Run Rust tests**

```bash
cargo test --quiet
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/events.rs src-ui/src/stores/activityStore.ts src-ui/src/App.tsx
git commit -m "feat: wire activity event logging from backend to frontend"
```

---

### Task 7: Config validation

**Files:**
- Modify: `src-tauri/src/commands/config.rs`

- [ ] **Step 1: Add validation function**

In `src-tauri/src/commands/config.rs`, add a validation function that checks the deserialized settings before persisting:

```rust
use dota2_scripts::config::Settings;

fn validate_settings(settings: &Settings) -> Result<(), String> {
    // Server config
    if settings.server.port == 0 {
        return Err("Server port must be greater than 0".to_string());
    }

    // Danger detection thresholds
    let dd = &settings.danger_detection;
    if dd.hp_threshold_percent > 100 {
        return Err("Danger HP threshold must be 0-100".to_string());
    }
    if dd.satanic_hp_threshold > 100 {
        return Err("Satanic HP threshold must be 0-100".to_string());
    }

    // Common survivability
    if settings.common.survivability_hp_threshold > 100 {
        return Err("Survivability HP threshold must be 0-100".to_string());
    }

    // Soul ring thresholds
    let sr = &settings.soul_ring;
    if sr.min_mana_percent > 100 {
        return Err("Soul Ring min mana must be 0-100".to_string());
    }
    if sr.min_health_percent > 100 {
        return Err("Soul Ring min health must be 0-100".to_string());
    }

    // Meepo thresholds
    let meepo = &settings.heroes.meepo;
    if meepo.dig_hp_threshold_percent > 100 {
        return Err("Meepo dig HP threshold must be 0-100".to_string());
    }
    if meepo.megameepo_hp_threshold_percent > 100 {
        return Err("Meepo MegaMeepo HP threshold must be 0-100".to_string());
    }

    Ok(())
}
```

- [ ] **Step 2: Call validation in `update_config`**

In the `update_config` function, add the validation call after deserializing `new_settings` but before writing to disk:

```rust
let new_settings: Settings =
    serde_json::from_value(config_value).map_err(|e| format!("Deserialize error: {}", e))?;

// Validate before persisting
validate_settings(&new_settings)?;

let toml_str =
    toml::to_string_pretty(&new_settings).map_err(|e| format!("TOML error: {}", e))?;
```

- [ ] **Step 3: Call validation in `update_hero_config`**

Similarly, add validation in `update_hero_config` after deserializing:

```rust
let new_settings: Settings =
    serde_json::from_value(config_value).map_err(|e| format!("Deserialize error: {}", e))?;

validate_settings(&new_settings)?;
```

- [ ] **Step 4: Run tests**

```bash
cargo test --quiet
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/config.rs
git commit -m "feat: add config validation before persistence"
```

---

### Task 8: Meepo observed state panel

**Files:**
- Modify: `src-tauri/src/ipc_types.rs`
- Create: `src-tauri/src/commands/meepo.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-ui/src/types/game.ts`
- Modify: `src-ui/src/pages/heroes/configs/MeepoConfig.tsx`

- [ ] **Step 1: Add MeepoStateDto to ipc_types.rs**

In `src-tauri/src/ipc_types.rs`, add:

```rust
/// Meepo hero-specific observed runtime state
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeepoStateDto {
    pub health_percent: u32,
    pub mana_percent: u32,
    pub in_danger: bool,
    pub alive: bool,
    pub stunned: bool,
    pub silenced: bool,
    pub poof_ready: bool,
    pub dig_ready: bool,
    pub megameepo_ready: bool,
    pub has_shard: bool,
    pub has_scepter: bool,
    pub blink_available: bool,
    pub combo_items: Vec<String>,
}
```

- [ ] **Step 2: Create the `get_meepo_state` command**

Create `src-tauri/src/commands/meepo.rs`:

```rust
use crate::ipc_types::MeepoStateDto;
use dota2_scripts::actions::heroes::meepo_state;

/// Returns the latest observed Meepo state, or null if not playing Meepo.
#[tauri::command]
pub fn get_meepo_state() -> Option<MeepoStateDto> {
    meepo_state::latest_meepo_observed_state().map(|s| MeepoStateDto {
        health_percent: s.health_percent,
        mana_percent: s.mana_percent,
        in_danger: s.in_danger,
        alive: s.alive,
        stunned: s.stunned,
        silenced: s.silenced,
        poof_ready: s.poof_ready,
        dig_ready: s.dig_ready,
        megameepo_ready: s.megameepo_ready,
        has_shard: s.has_shard,
        has_scepter: s.has_scepter,
        blink_available: s.blink_slot_key.is_some(),
        combo_items: s
            .combo_item_keys
            .into_iter()
            .map(|(name, _)| name)
            .collect(),
    })
}
```

- [ ] **Step 3: Register the command**

In `src-tauri/src/commands/mod.rs`, add:

```rust
pub mod meepo;
```

In `src-tauri/src/lib.rs`, add the command to the invoke_handler:

```rust
.invoke_handler(tauri::generate_handler![
    commands::config::get_config,
    commands::config::update_config,
    commands::config::update_hero_config,
    commands::state::get_app_state,
    commands::state::set_gsi_enabled,
    commands::state::set_standalone_enabled,
    commands::state::select_hero,
    commands::game::get_game_state,
    commands::diagnostics::get_diagnostics,
    commands::updates::get_update_state,
    commands::updates::check_for_updates,
    commands::updates::apply_update,
    commands::updates::dismiss_update,
    commands::meepo::get_meepo_state,
])
```

- [ ] **Step 4: Add MeepoObservedState TypeScript type**

In `src-ui/src/types/game.ts`, add:

```typescript
export interface MeepoObservedState {
  healthPercent: number;
  manaPercent: number;
  inDanger: boolean;
  alive: boolean;
  stunned: boolean;
  silenced: boolean;
  poofReady: boolean;
  digReady: boolean;
  megameepoReady: boolean;
  hasShard: boolean;
  hasScepter: boolean;
  blinkAvailable: boolean;
  comboItems: string[];
}
```

- [ ] **Step 5: Add observed state panel to MeepoConfig page**

In `src-ui/src/pages/heroes/configs/MeepoConfig.tsx`, add a runtime state panel at the top. Add imports and a polling hook:

```typescript
import { useEffect, useState } from "react";
import { isTauri } from "../../../lib/tauri";
import type { MeepoObservedState } from "../../../types/game";
```

Add a state-fetching hook inside the component:

```typescript
export default function MeepoConfig() {
  const config = useConfigStore((s) => s.config.heroes.meepo);
  const update = useConfigStore((s) => s.updateHeroConfig);
  const set = (updates: Partial<typeof config>) => update("meepo", updates);
  const setFarm = (updates: Partial<typeof config.farm_assist>) =>
    set({ farm_assist: { ...config.farm_assist, ...updates } });

  const [meepoState, setMeepoState] = useState<MeepoObservedState | null>(null);

  useEffect(() => {
    if (!isTauri()) return;
    let cancelled = false;

    const poll = async () => {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        while (!cancelled) {
          const state = await invoke<MeepoObservedState | null>("get_meepo_state");
          if (!cancelled) setMeepoState(state);
          await new Promise((r) => setTimeout(r, 500));
        }
      } catch {
        // Silently ignore — command may not be available
      }
    };

    poll();
    return () => { cancelled = true; };
  }, []);

  return (
    <>
      {meepoState && (
        <div className="col-span-2">
          <Card title="Live State">
            <div className="grid grid-cols-3 gap-3 text-sm">
              <div>
                <span className="text-muted">HP:</span>{" "}
                <span className={meepoState.healthPercent < 30 ? "text-danger" : "text-terminal"}>
                  {meepoState.healthPercent}%
                </span>
              </div>
              <div>
                <span className="text-muted">Mana:</span>{" "}
                <span className="text-info">{meepoState.manaPercent}%</span>
              </div>
              <div>
                <span className="text-muted">Status:</span>{" "}
                {meepoState.inDanger ? (
                  <span className="text-danger">⚠ DANGER</span>
                ) : meepoState.alive ? (
                  <span className="text-terminal">Alive</span>
                ) : (
                  <span className="text-muted">Dead</span>
                )}
              </div>
              <div>
                <span className="text-muted">Poof:</span>{" "}
                <span className={meepoState.poofReady ? "text-terminal" : "text-muted"}>
                  {meepoState.poofReady ? "Ready" : "CD"}
                </span>
              </div>
              <div>
                <span className="text-muted">Dig:</span>{" "}
                <span className={meepoState.digReady ? "text-terminal" : "text-muted"}>
                  {meepoState.digReady ? "Ready" : "CD"}
                </span>
              </div>
              <div>
                <span className="text-muted">MegaMeepo:</span>{" "}
                <span className={meepoState.megameepoReady ? "text-terminal" : "text-muted"}>
                  {meepoState.megameepoReady ? "Ready" : "CD"}
                </span>
              </div>
              <div>
                <span className="text-muted">Blink:</span>{" "}
                <span className={meepoState.blinkAvailable ? "text-terminal" : "text-muted"}>
                  {meepoState.blinkAvailable ? "Available" : "No"}
                </span>
              </div>
              <div>
                <span className="text-muted">Shard:</span>{" "}
                {meepoState.hasShard ? "✓" : "✗"}
              </div>
              <div>
                <span className="text-muted">Scepter:</span>{" "}
                {meepoState.hasScepter ? "✓" : "✗"}
              </div>
            </div>
            {meepoState.comboItems.length > 0 && (
              <div className="mt-2 text-sm">
                <span className="text-muted">Combo items ready:</span>{" "}
                {meepoState.comboItems.join(", ")}
              </div>
            )}
          </Card>
        </div>
      )}

      {/* Existing config cards below, unchanged */}
      <div className="space-y-4">
        {/* ... existing Keybindings and Combo Settings cards ... */}
      </div>
      <div className="space-y-4">
        {/* ... existing Danger Abilities and Farm Assist cards ... */}
      </div>
    </>
  );
}
```

Note to implementer: The `meepoState` panel should be inserted BEFORE the existing two column divs. Keep all existing JSX exactly as-is. The `col-span-2` class makes the live state panel span the full width of the two-column grid that `HeroPage` uses.

- [ ] **Step 6: Run tests**

```bash
cargo test --quiet && cd src-ui && npx vitest run
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/ipc_types.rs src-tauri/src/commands/meepo.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs src-ui/src/types/game.ts src-ui/src/pages/heroes/configs/MeepoConfig.tsx
git commit -m "feat: add Meepo observed state panel with live polling"
```

---

### Task 9: Rune alert audio notification

**Files:**
- Modify: `src-ui/src/App.tsx`

- [ ] **Step 1: Add rune alert hook to App.tsx**

Add a `useRuneAlert` custom hook directly in `App.tsx` (or as a separate small file — implementer's choice). The hook plays a short tone via Web Audio API when `runeTimer` transitions to a small value (e.g., ≤ 10 seconds):

```typescript
function useRuneAlert(runeTimer: number | null) {
  const lastAlertRef = useRef<number | null>(null);

  useEffect(() => {
    // Only alert when timer first appears (transitions from null or > 10 to ≤ 10)
    if (runeTimer === null || runeTimer > 10) {
      lastAlertRef.current = null;
      return;
    }

    // Don't re-alert for the same rune window
    if (lastAlertRef.current !== null && lastAlertRef.current <= 10) {
      return;
    }

    lastAlertRef.current = runeTimer;

    // Play a short alert tone using Web Audio API
    try {
      const ctx = new AudioContext();
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.frequency.value = 880;
      gain.gain.value = 0.15;
      osc.start();
      osc.stop(ctx.currentTime + 0.12);
      // Clean up after tone finishes
      setTimeout(() => ctx.close(), 500);
    } catch {
      // AudioContext may not be available
    }
  }, [runeTimer]);
}
```

Add the `useRef` import if not already present:

```typescript
import { useEffect, useRef } from "react";
```

Call the hook inside the `App` component:

```typescript
export default function App() {
  // ... existing code ...
  const game = useGameStore((s) => s.game);
  useRuneAlert(game.runeTimer);
  // ... rest of component ...
}
```

- [ ] **Step 2: Run frontend tests**

```bash
cd src-ui && npx vitest run
```

Expected: all tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-ui/src/App.tsx
git commit -m "feat: add rune alert audio notification via Web Audio API"
```

---

### Task 10: Full verification

**Files:** No new files — verification only.

- [ ] **Step 1: Run all Rust tests**

```bash
cargo test --quiet
```

Expected: all tests pass (library tests + Tauri DTO tests + activity buffer tests).

- [ ] **Step 2: Run React tests**

```bash
cd src-ui && npx vitest run
```

Expected: all tests pass.

- [ ] **Step 3: Build React production bundle**

```bash
cd src-ui && npm run build
```

Expected: production build succeeds.

- [ ] **Step 4: Build Tauri binary**

```bash
cargo build -p dota2-scripts-tauri
```

Expected: binary compiles.

- [ ] **Step 5: Build egui binary**

```bash
cargo build -p dota2-scripts
```

Expected: binary compiles (egui path unaffected by these changes).

- [ ] **Step 6: Verify no warnings in library crate**

```bash
cargo check -p dota2-scripts 2>&1
```

Expected: no new warnings (the existing `Button` unused import in `examples/mouse_test.rs` is the known baseline).

- [ ] **Step 7: Commit any final fixes**

```bash
git add -A
git commit -m "chore: final Plan 3 verification fixes"
```

---

## Explicitly Not Included

- **Auto-resize on content**: Window auto-sizing is a UX risk (causes layout jank) and adds complexity for minimal benefit. The fixed window size works well. Can revisit if users request it.
- **Keyboard shortcut display on hero pages**: Hero config pages already show key bindings via `KeyInput` components. A separate read-only display would be redundant at this stage.
