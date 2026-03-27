# Item 6 Design: Reuse One Danger Decision Per GSI Event

## Problem

The next likely low-risk performance hotspot is danger detection and its immediate survivability consumers.

Today `src\actions\common.rs::execute_default_strategy()` calls:

1. `danger_detector::update(event, config)` once
2. later `danger_detector::is_in_danger()` inside healing logic
3. later `danger_detector::is_in_danger()` again inside defensive-item logic
4. later `danger_detector::is_in_danger()` again inside neutral-item logic

That means one GSI event can take multiple locks on the global `HP_TRACKER` state even though the current event’s danger result was already computed at the top of the pass.

## Goals

- Preserve current danger behavior exactly.
- Reduce same-event lock traffic in the danger/survivability path.
- Keep the slice tightly scoped to `danger_detector` and its immediate `common.rs` consumers, with only small adjacent wins if they stay low-risk.
- Keep manual gameplay behavior identical for healing, defensive items, and neutral items.

## Non-goals

- No heuristic redesign.
- No changes to the meaning of “in danger.”
- No refactor of unrelated hero scripts.
- No broader queue/executor/input-lane changes.
- No migration of danger state into `AppState`.

## Current bottleneck

The current pattern is:

1. `danger_detector::update(...)` locks the tracker, updates cross-event state, and returns the current danger result.
2. `common.rs` discards that returned boolean.
3. Later helpers reacquire the tracker through `danger_detector::is_in_danger()`.

That repeated lock/read pattern is unnecessary for one event’s survivability pass because the current event’s danger decision is already known.

## Options considered

### Option 1: Event-local danger snapshot

Call `danger_detector::update(...)` once per event, keep the returned boolean in `execute_default_strategy()`, and pass that value through to the immediate survivability helpers that need it.

**Pros**

- Lowest-risk change.
- Preserves exact heuristic behavior.
- Directly removes repeated same-event tracker locks.
- Keeps the global tracker for cross-event state without redesigning it.

**Cons**

- Does not optimize callers outside the default/common survivability path.
- Requires a few helper signatures to accept the current event’s danger result.

### Option 2: Tracker-only cleanup

Leave the call graph alone and only tweak internal locking in `danger_detector.rs`.

**Pros**

- Smallest code change.

**Cons**

- Much smaller payoff.
- Does not remove repeated `is_in_danger()` calls from `common.rs`.

### Option 3: Per-pass survivability snapshot

Build one small per-event snapshot containing danger plus a few related thresholds/config reads, then feed all common survivability helpers from it.

**Pros**

- Slightly larger lock/config-read reduction.
- Good long-term direction if common survivability keeps growing.

**Cons**

- Wider than necessary for the current low-risk slice.
- Easier to overbuild.

## Recommendation

Use **Option 1**, with only tiny pieces of Option 3 if they stay fully local and obviously low-risk.

The important change is to compute the current event’s danger result once and reuse it throughout the same survivability pass, rather than re-locking the tracker for information we already have.

## Proposed architecture

### Canonical danger computation

Keep `danger_detector::update(event, config)` as the canonical place that:

- reads prior tracker state
- updates tracker fields for the current event
- returns the current danger result

Do **not** change the heuristic or its state machine.

### Common survivability flow

In `src\actions\common.rs::execute_default_strategy()`:

1. call `danger_detector::update(...)` once
2. store the returned boolean for the current event
3. pass that event-local danger result into the immediate common survivability helpers that need it

Likely consumers:

- healing threshold selection
- defensive-item gating
- neutral-item gating

That means `common.rs` should stop calling `danger_detector::is_in_danger()` repeatedly inside the same event flow when it already has the answer.

### Public API boundary

Keep the current public `SurvivabilityActions` API stable for hero scripts that already call:

- `check_and_use_healing_items(event)`
- `use_defensive_items_if_danger(event)`
- `use_neutral_item_if_danger(event)`

This slice should stay narrow by avoiding a fan-out signature change across hero scripts.

Recommended implementation shape:

- keep existing public methods unchanged
- add private/internal helper variants inside `common.rs` that accept the already-computed danger result for the default/common pass

For example:

```rust
pub fn check_and_use_healing_items(&self, event: &GsiWebhookEvent) {
    let in_danger = crate::actions::danger_detector::is_in_danger();
    self.check_and_use_healing_items_with_danger(event, in_danger);
}

fn check_and_use_healing_items_with_danger(
    &self,
    event: &GsiWebhookEvent,
    in_danger: bool,
) {
    // shared logic
}
```

Apply the same pattern to defensive/neutral-item helpers if needed.

### Adjacent low-risk win

If it falls out naturally while changing those helper signatures, it is acceptable to gather a small amount of danger-related config once per pass instead of reacquiring the same settings values in multiple tiny scopes.

This is only in scope if:

- the change stays inside `common.rs`
- behavior remains identical
- it does not create a broader “survivability snapshot” abstraction that is larger than the slice needs

## Scope boundaries

### In scope

- `src\actions\danger_detector.rs`
- `src\actions\common.rs`
- focused tests in the touched modules
- documentation for danger-detection/runtime flow if the code path description changes

### Out of scope

- hero-local callers outside the immediate common survivability path
- changing existing public `SurvivabilityActions` method signatures across hero scripts
- UI redesign
- changing danger config knobs
- changing silence-dispel behavior
- moving tracker ownership to a different subsystem

## Testing strategy

### Automated

- Add focused tests that prove same-event behavior is unchanged while using one computed danger result.
- Keep or extend unit coverage for:
  - danger update semantics
  - healing threshold behavior
  - defensive-item danger gating
  - neutral-item danger gating
- Run repo-standard verification:
  - `cargo test`
  - `cargo build --release --target-dir target\release-verify`

### Manual

- Verify healing items still trigger at the same HP situations as before.
- Verify danger-triggered defensive items still fire exactly when expected.
- Verify neutral items still use the same danger gate.
- Watch for reduced hitching during heavy combat without changing live danger behavior.

## Expected outcome

After this slice, each GSI event should compute danger once and reuse that answer for the current common survivability pass, reducing repeated `HP_TRACKER` lock traffic while keeping the danger heuristic and gameplay behavior unchanged.
