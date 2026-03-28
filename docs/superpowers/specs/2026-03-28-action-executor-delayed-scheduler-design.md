# Design: ActionExecutor Delayed-Job Scheduler

## Problem

The runtime now avoids most hot-path raw thread spawning, but `ActionExecutor::enqueue_after(...)` still creates one short-lived helper thread for every non-zero delay:

- `src\actions\executor.rs` already has one long-lived executor worker for action jobs
- `enqueue()` already sends immediate jobs directly to that worker
- `enqueue_after()` still uses `thread::spawn(...)` + `thread::sleep(...)` per delayed job before forwarding to the worker
- live delayed-job usage remains in `src\actions\dispel.rs` for Manta and Lotus silence-dispel jitter

That means burst gameplay can still turn repeated delayed actions into repeated OS thread creation even though the rest of the action lane is already centralized.

## Goals

- Remove the remaining routine per-job thread spawn from `ActionExecutor::enqueue_after(...)`.
- Preserve the public `enqueue()` / `enqueue_after()` API so call sites do not need broad rewiring.
- Preserve current behavior where immediate jobs are not blocked behind future delayed jobs.
- Keep delayed jobs serialized onto the existing executor worker once they become due.
- Keep the slice small enough to test and validate independently.

## Non-goals

- No keyboard-callback worker redesign.
- No synthetic-input worker changes.
- No migration of fallback-only thread spawns in Soul Ring, Broodmother, or Shadow Fiend.
- No changes to how callers choose delays or jitter values.
- No broad executor API redesign beyond what is needed to internalize delayed scheduling.

## Options considered

### Option 1: Internal delayed-job scheduler inside `ActionExecutor`

Keep the existing executor worker, but add one dedicated scheduler thread owned by `ActionExecutor`. Immediate jobs still go straight to the worker; delayed jobs go to the scheduler, which waits until they are due and then forwards them to the worker.

**Pros**

- Removes the remaining routine per-delay helper thread.
- Preserves the current public API and most call sites unchanged.
- Keeps immediate jobs fast and unblocked by future delayed jobs.
- Keeps action execution serialized on the existing worker lane.

**Cons**

- Adds a second internal thread to `ActionExecutor`.
- Requires a small amount of new internal queue and wake-up logic.

### Option 2: Collapse delayed waiting onto the existing worker

Teach the current executor worker to hold delayed jobs and sleep until they are due.

**Pros**

- Fewer moving parts on paper.
- Only one executor-owned thread.

**Cons**

- Risks blocking immediate jobs behind a delayed wait.
- Requires a more invasive worker-loop redesign to avoid regressing latency.
- Harder to reason about behavior preservation than a separate scheduler lane.

### Option 3: Broader residual-spawn cleanup in one slice

Replace the executor delayed helper thread and also clean up remaining rare fallback spawns or other thread creation sites at the same time.

**Pros**

- Larger total reduction in thread spawning.
- Could reduce future cleanup work.

**Cons**

- Widens scope beyond the one remaining routine hotspot.
- Makes regressions harder to isolate during gameplay validation.
- Breaks the current incremental rollout discipline.

## Recommendation

Use **Option 1**.

This is the smallest change that removes the remaining routine action-lane thread spawn without mixing in unrelated keyboard or fallback behavior. It keeps the action model understandable:

- one worker executes jobs
- one scheduler waits on delayed jobs
- immediate jobs still bypass scheduler latency

## Proposed architecture

### Public API stays stable

Keep:

- `ActionExecutor::new() -> Arc<ActionExecutor>`
- `enqueue<F>(&self, label: &'static str, job: F)`
- `enqueue_after<F>(&self, label: &'static str, delay: Duration, job: F)`

No caller-facing signature changes should be required for this slice.

### Internal lanes

`ActionExecutor` should own two internal channels:

1. **executor worker channel**
   - receives ready-to-run jobs
   - keeps the existing panic-catching execution behavior

2. **scheduler command channel**
   - receives delayed jobs that are not yet due
   - is owned by one long-lived scheduler thread

Immediate jobs continue to send directly to the executor worker channel.

Non-zero delayed jobs send a scheduling request to the scheduler command channel instead of spawning a helper thread.

### Scheduled item model

Each delayed job should be wrapped in a small scheduled item containing:

- `label`
- `job`
- `due_at: Instant`
- `sequence: u64`

The sequence counter exists only to preserve stable FIFO ordering for jobs with the same due time or near-identical timing after conversion from `Duration` to `Instant`.

That ordering contract should be defined as:

- FIFO relative to accepted `enqueue_after(...)` call order inside one process
- `sequence` assigned before the job is sent to the scheduler command channel
- scheduler tie-breaking based on `(due_at, sequence)` so later internal wake-ups do not reorder already-accepted equal-deadline jobs

The scheduler thread should maintain a min-ordered collection keyed by `(due_at, sequence)`.

### Scheduler behavior

The scheduler thread should:

1. block waiting for the first delayed job when idle
2. once it has queued work, compute the nearest due item
3. wait only until either:
   - that item becomes due, or
   - a new scheduling request arrives that may have an earlier deadline
4. when one or more items are due, forward them to the executor worker channel in due-order / sequence-order

The implementation should recompute the next wait target after every scheduler-channel receive so a newly accepted earlier deadline can preempt a longer outstanding wait.

This keeps the waiting centralized without requiring a helper thread per delayed job.

### Immediate-job fast path

`enqueue()` and `enqueue_after(..., Duration::ZERO, ...)` should keep the existing fast path and send directly to the executor worker channel.

That preserves the current guarantee tested today: a delayed job scheduled first should not prevent a later immediate job from running promptly.

## Behavior expectations

- Delayed jobs still run no earlier than their requested delay.
- Immediate jobs still avoid being stuck behind future delayed jobs.
- Once a delayed job is due, it still executes on the same serialized executor worker as other action jobs.
- Existing job labels and panic handling remain intact.
- Jitter is still computed at the call site, then handed to `enqueue_after(...)`; the scheduler only owns the waiting.

## Failure handling

- If sending an immediate job to the executor worker channel fails, log a warning and fail soft as today.
- If sending a delayed job to the scheduler command channel fails, log a warning and fail soft.
- If the scheduler cannot forward a due job to the executor worker channel, log a warning and drop that job rather than recreating raw thread fallbacks.

Shutdown behavior should be explicit:

- once the scheduler command channel is disconnected, the scheduler should stop accepting new work but continue processing already-accepted delayed jobs still held in its queue
- after the command channel is disconnected and the delayed queue is empty, the scheduler thread should exit cleanly
- if the executor worker channel is disconnected, the scheduler should log the failure, drop any remaining queued delayed jobs, and exit rather than hanging on work that can no longer run

This slice is specifically about removing routine spawn behavior, so it should not silently reintroduce helper threads as a fallback path.

## Testing strategy

Add focused executor tests where behavior is deterministic:

- zero delay still uses the immediate path
- delayed jobs wait before execution
- delayed jobs do not block a later immediate job
- a newly queued earlier deadline preempts a longer existing wait
- multiple delayed jobs with the same target time preserve FIFO order relative to accepted `enqueue_after(...)` call order
- the executor still survives a panicking job
- shutdown behavior is covered either directly or through small scheduler-helper seams if a full thread-lifecycle test would be too brittle

Prefer helper seams for ordering or scheduler-item comparison if they make the tests clearer, but avoid over-abstracting the runtime just for tests.

## Documentation updates

Update `docs\architecture\runtime-flow.md` so the background-thread table no longer describes a per-job delayed helper thread and instead documents:

- the long-lived `ActionExecutor` worker thread
- the long-lived `ActionExecutor` delayed scheduler thread

If implementation details materially affect developer troubleshooting, update any nearby action-runtime notes that reference delayed executor behavior.

## Manual validation

Gameplay validation for this slice should focus on:

- silence dispel still triggering with natural jitter
- immediate executor-driven actions still feeling responsive during fights
- no obvious regression in action ordering when delayed and immediate jobs overlap
- reduced hitching in situations that previously stacked many short delayed jobs

## Risks

- Scheduler wake-up logic could accidentally delay jobs if the earliest-deadline recalculation is wrong.
- Poor tie-breaking could reorder nearly simultaneous delayed jobs.
- Over-designing the scheduler could widen the slice beyond the one remaining hotspot.

## Acceptance criteria

- `src\actions\executor.rs` no longer uses one raw `thread::spawn(...)` per non-zero delayed job.
- Existing executor tests still pass, with added deterministic coverage for delayed-ordering behavior.
- `cargo test` passes.
- `cargo build --release --target-dir target\release-verify` passes.
- `docs\architecture\runtime-flow.md` accurately describes the new executor-owned delayed scheduler thread.
