# ActionExecutor Delayed Scheduler Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `ActionExecutor::enqueue_after(...)`'s per-job helper thread with one internal delayed-job scheduler while preserving immediate-job responsiveness and delayed-job ordering.

**Architecture:** Keep the public executor API stable and split executor internals into two long-lived lanes: a ready-job worker and a delayed-job scheduler. The scheduler owns deadline ordering and wake-ups, then forwards due work onto the existing serialized worker lane so caller behavior stays unchanged outside the removed helper thread spawn.

**Tech Stack:** Rust, std::sync::mpsc, std::thread, std::time::{Duration, Instant}, tracing, cargo test, cargo build

---

## File structure and responsibilities

- Modify: `src\actions\executor.rs`
  - Add the delayed-scheduler internals, helper seams, and deterministic unit tests.
  - Keep `ActionExecutor::new()`, `enqueue()`, and `enqueue_after()` stable for callers.
- Modify: `docs\architecture\runtime-flow.md`
  - Replace the per-job delayed helper thread description with the new long-lived delayed scheduler thread.

## Task 1: Lock down scheduler behavior with tests and helper seams

**Files:**
- Modify: `src\actions\executor.rs`
- Test: `src\actions\executor.rs`

- [ ] **Step 1: Add scheduler-only helper types before wiring threads**

Add small internal helpers in `src\actions\executor.rs` so tests can reason about scheduling without mocking threads:

```rust
struct ScheduledAction {
    due_at: Instant,
    sequence: u64,
    message: ActionMessage,
}

fn dispatch_mode_for_delay(delay: Duration) -> DispatchMode { /* existing helper */ }

fn scheduled_action_cmp(left: &ScheduledAction, right: &ScheduledAction) -> std::cmp::Ordering {
    left.due_at
        .cmp(&right.due_at)
        .then_with(|| left.sequence.cmp(&right.sequence))
}
```

- [ ] **Step 2: Write deterministic unit tests for the new ordering rules**

Add tests in `src\actions\executor.rs` covering:

```rust
#[test]
fn scheduled_action_orders_earlier_deadline_first() { /* earlier due_at wins */ }

#[test]
fn scheduled_action_orders_equal_deadline_by_sequence() { /* lower sequence wins */ }
```

- [ ] **Step 3: Add runtime-facing regression tests for scheduler behavior**

Add tests in `src\actions\executor.rs` covering:

```rust
#[test]
fn zero_delay_enqueue_after_uses_immediate_fast_path() { /* enqueue_after(Duration::ZERO, ...) runs promptly */ }

#[test]
fn delayed_job_does_not_block_immediate_job() { /* keep existing regression */ }

#[test]
fn earlier_delayed_job_preempts_longer_existing_wait() { /* 80ms then 20ms */ }

#[test]
fn equal_deadline_delayed_jobs_run_fifo() { /* accepted call order preserved via helper seam or shared barrier */ }
```

- [ ] **Step 4: Run focused executor tests before implementation**

Run: `cargo test executor -- --nocapture`

Expected: at least the scheduler-specific new tests fail until the delayed scheduler is implemented, while existing tests still compile.

- [ ] **Step 5: Commit the test-first slice**

```bash
git add src/actions/executor.rs
git commit -m "test: add delayed scheduler coverage"
```

## Task 2: Implement the delayed scheduler inside ActionExecutor

**Files:**
- Modify: `src\actions\executor.rs`
- Test: `src\actions\executor.rs`

- [ ] **Step 1: Add the internal scheduler channels and sequence counter**

Update `ActionExecutor` internals in `src\actions\executor.rs` to hold:

```rust
pub struct ActionExecutor {
    ready_tx: Sender<ActionMessage>,
    delayed_tx: Sender<SchedulerMessage>,
    next_sequence: AtomicU64,
}

enum SchedulerMessage {
    Schedule(ScheduledAction),
}
```

- [ ] **Step 2: Implement the ready-job worker without changing panic handling**

Keep the worker loop behavior equivalent to today:

```rust
while let Ok(message) = ready_rx.recv() {
    match message {
        ActionMessage::Run { label, job } => run_job_with_panic_guard(label, job),
    }
}
```

- [ ] **Step 3: Implement the delayed scheduler thread**

Use one long-lived scheduler thread in `src\actions\executor.rs` that:

```rust
loop {
    // idle: block on delayed_rx.recv()
    // active: recv_timeout(until next due item)
    // on new item: insert and recompute earliest due time
    // on timeout or due items ready: forward due work to ready_tx in due/sequence order
}
```

Implementation requirements:
- assign `sequence` before sending to the scheduler channel
- zero-delay work keeps the direct ready-channel fast path
- if `delayed_tx.send(...)` fails, log a warning and fail soft
- if immediate `ready_tx.send(...)` fails, keep today’s warn-and-fail-soft behavior unchanged
- if forwarding due work to `ready_tx` fails, log once per failed job, drop remaining queued delayed jobs, and exit the scheduler thread cleanly
- when the delayed channel disconnects, drain already-accepted delayed work before exit

- [ ] **Step 4: Add shutdown-behavior coverage**

Add deterministic tests or small helper-seam tests in `src\actions\executor.rs` that validate both shutdown paths from the spec:

```rust
#[test]
fn delayed_scheduler_drains_accepted_work_after_schedule_channel_disconnect() {
    // prove accepted delayed work still forwards before exit
}

#[test]
fn delayed_scheduler_drops_remaining_work_when_ready_channel_disconnects() {
    // prove failed forwarding causes queued delayed work to be dropped and scheduler exit
}
```

Prefer a narrow helper seam if a full thread-lifecycle test would be brittle.

- [ ] **Step 5: Update `enqueue_after()` to use the scheduler**

Implement the caller path in `src\actions\executor.rs`:

```rust
match dispatch_mode_for_delay(delay) {
    DispatchMode::Immediate => ready_tx.send(ActionMessage::Run { label, job }),
    DispatchMode::Delayed(delay) => {
        let scheduled = ScheduledAction {
            due_at: Instant::now() + delay,
            sequence: self.next_sequence.fetch_add(1, Ordering::Relaxed),
            message: ActionMessage::Run { label, job },
        };
        delayed_tx.send(SchedulerMessage::Schedule(scheduled))
    }
}
```

- [ ] **Step 6: Run focused executor tests**

Run: `cargo test executor -- --nocapture`

Expected: all executor tests pass, including the new earlier-deadline and FIFO regressions.

- [ ] **Step 7: Commit the scheduler implementation**

```bash
git add src/actions/executor.rs
git commit -m "perf: add action executor delayed scheduler"
```

## Task 3: Update docs and verify the repo

**Files:**
- Modify: `docs\architecture\runtime-flow.md`
- Modify: `src\actions\executor.rs` (only if verification exposes a scheduler bug or flaky test)

- [ ] **Step 1: Update runtime-flow docs**

Update both the prose and the thread table in `docs\architecture\runtime-flow.md`:

```md
Immediate jobs still go straight to the worker lane, while delayed jobs now wait inside the executor-owned scheduler and do not require a helper thread per `enqueue_after(...)` call.

| `src/actions/executor.rs` | ActionExecutor worker thread | When `ActionDispatcher::new(...)` constructs the executor | Runs queued ready action jobs FIFO |
| `src/actions/executor.rs` | ActionExecutor delayed scheduler thread | When `ActionDispatcher::new(...)` constructs the executor | Owns delayed-job deadlines and forwards due jobs onto the worker lane |
```

- [ ] **Step 2: Run full automated verification**

Run:

```bash
cargo test
cargo build --release --target-dir target/release-verify
```

Expected:
- `cargo test` passes
- `cargo build --release --target-dir target/release-verify` passes
- only pre-existing warnings remain unless this slice introduces a new one that must be fixed

- [ ] **Step 3: Review the scoped diff**

Run:

```bash
git --no-pager diff -- src/actions/executor.rs docs/architecture/runtime-flow.md
```

Check:
- no per-delay helper `thread::spawn(...)` remains in `src\actions\executor.rs`
- immediate path still bypasses the delayed scheduler
- docs describe the new long-lived scheduler thread accurately

- [ ] **Step 4: Commit docs + verification follow-up**

```bash
git add src/actions/executor.rs docs/architecture/runtime-flow.md
git commit -m "docs: update action executor runtime flow"
```

- [ ] **Step 5: Manual handoff notes**

Record the post-implementation gameplay checklist for later user validation:

```text
- Silence dispel still fires with natural jitter.
- Immediate executor-driven actions still feel responsive when delayed jobs are active.
- No obvious action reordering when delayed and immediate jobs overlap.
- Busy fights hitch less when multiple delayed executor jobs are scheduled.
```
