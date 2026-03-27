# Item 5 Design: Deduplicate GSI Refresh Work Before Hero Dispatch

## Problem

The next likely freeze contributor is the GSI event path doing more per-event work than necessary.

Today `src\gsi\handler.rs::process_gsi_events()` already refreshes keyboard-supporting runtime state for every event:

- `soul_ring::update_from_gsi(...)`
- `auto_items::update_gsi_state(...)`
- `BROODMOTHER_ACTIVE`
- `SF_LAST_EVENT`

But when `gsi_enabled` is on, `src\actions\dispatcher.rs::dispatch_gsi_event()` repeats part of that same refresh work again before hero/common automation runs:

- `soul_ring::update_from_gsi(...)`
- `auto_items::update_gsi_state(...)`
- `BROODMOTHER_ACTIVE`

That means every enabled GSI event pays duplicate cache updates, duplicate lock work, and duplicate whole-event clones in hot helper paths such as `auto_items::update_gsi_state(...)`.

## Goals

- Reduce per-event GSI work without changing gameplay behavior.
- Make one runtime path the canonical owner of keyboard-supporting GSI refreshes.
- Keep the slice narrow and low-risk.
- Preserve current behavior when full GSI automation is disabled.
- Add regression tests that prove keyboard-supporting caches still refresh correctly.

## Non-goals

- No redesign of the GSI queue itself.
- No `Arc<GsiWebhookEvent>` refactor in this slice.
- No broad hero-script rewrites.
- No changes to gameplay logic for Soul Ring, Broodmother, Shadow Fiend, or auto-items beyond removing duplicate refresh calls.
- No optimization of downstream hero-specific event clones yet.

## Current duplicate work

The current GSI flow is:

1. `gsi_webhook_handler()` enqueues `GsiWebhookEvent`
2. `process_gsi_events()`:
   - updates `AppState` with `event.clone()`
   - refreshes keyboard-supporting runtime state
   - checks `gsi_enabled`
   - calls `dispatcher.dispatch_gsi_event(&event)` if enabled
3. `dispatch_gsi_event()` repeats shared refresh work before actual dispatch

The highest-value duplicate in the current narrow slice is `auto_items::update_gsi_state(event)`, which stores `Some(event.clone())` into `LATEST_GSI_EVENT`. Duplicating that clone for every enabled event is avoidable.

## Options considered

### Option 1: Deduplicate the refresh path and keep ownership in `process_gsi_events()`

Make `process_gsi_events()` the single owner of keyboard-supporting GSI refresh work, and remove duplicate refresh calls from `dispatch_gsi_event()`.

**Pros**

- Lowest-risk behavior change.
- Directly removes repeated clone/lock work on every enabled GSI event.
- Keeps the current queue, dispatcher, and hero/common automation structure intact.
- Preserves the existing guarantee that keyboard-supporting state stays fresh even when `gsi_enabled` is off.

**Cons**

- Does not eliminate all full-event clones in the repo.
- Leaves broader cache-shape improvements for a later slice.

### Option 2: Replace full-event caches with narrower snapshots

Keep the current ownership split but change caches like `LATEST_GSI_EVENT` and `SF_LAST_EVENT` to store smaller structs.

**Pros**

- Larger long-term reduction in cloning pressure.

**Cons**

- Wider refactor across keyboard paths and hero logic.
- Higher regression risk because more consumers need to change at once.

### Option 3: Refactor the GSI event pipeline around shared ownership

Move GSI events behind `Arc` or another shared representation and thread that through state, caches, and dispatch.

**Pros**

- Highest theoretical throughput improvement.

**Cons**

- Too broad for the current low-risk slice.
- Touches serialization, state, caches, and many consumers at once.

## Recommendation

Use **Option 1**.

This is the safest slice that still removes clearly duplicated hot-path work. It also clarifies ownership: `process_gsi_events()` owns event ingestion plus shared cache refresh, while `dispatch_gsi_event()` owns automation dispatch only.

## Proposed architecture

### Canonical refresh owner

Keep one helper in `src\gsi\handler.rs` that refreshes keyboard-supporting runtime state from the latest event.

That helper remains responsible for:

- `soul_ring::update_from_gsi(...)`
- `auto_items::update_gsi_state(...)`
- `BROODMOTHER_ACTIVE`
- `SF_LAST_EVENT`

It should run for every received event before the `gsi_enabled` gate so keyboard-triggered features still see fresh runtime state even when full GSI automation is disabled.

### Dispatcher boundary

`src\actions\dispatcher.rs::dispatch_gsi_event()` should stop performing shared “latest-event cache refresh” work that is already guaranteed upstream.

After this change, the dispatcher should only:

1. run truly dispatch-local pre-dispatch hooks
2. route to the matching hero script or default survivability path

In this slice, that means:

- keep neutral-item discovery logging in the dispatcher
- keep silence-dispel checks in the dispatcher
- remove duplicate shared refresh calls that belong to the GSI ingress pipeline

### Behavior safety

The slice must preserve these runtime guarantees:

- `LATEST_GSI_EVENT` still updates even when `gsi_enabled` is false
- `SF_LAST_EVENT` still updates even when `gsi_enabled` is false
- `BROODMOTHER_ACTIVE` still tracks the current hero even when `gsi_enabled` is false
- Soul Ring runtime state still refreshes on every GSI event
- enabled GSI automation still reaches the same hero/default handlers as before

## Scope boundaries

### In scope

- `src\gsi\handler.rs`
- `src\actions\dispatcher.rs`
- tests in `src\gsi\handler.rs` and any directly affected modules
- architecture docs that describe the GSI event path

### Out of scope

- changing the queue size or queue semantics
- changing hero-script signatures
- changing `AppState.last_event` ownership
- shrinking `LATEST_GSI_EVENT` / `SF_LAST_EVENT` payload shape
- downstream event-clone cleanup inside hero scripts

## Testing strategy

### Automated

- Keep existing GSI handler tests passing.
- Add or update tests proving keyboard-supporting runtime state refreshes when `gsi_enabled` is disabled.
- Add coverage proving enabled GSI dispatch no longer needs dispatcher-owned duplicate refreshes to keep:
  - `LATEST_GSI_EVENT`
  - `SF_LAST_EVENT`
  - `BROODMOTHER_ACTIVE`
  - Soul Ring runtime state
  fresh.
- Run repo-standard verification:
  - `cargo test`
  - `cargo build --release --target-dir target\release-verify`

### Manual

- Play with GSI automation enabled and confirm no gameplay regression for Soul Ring, Shadow Fiend, Broodmother, and auto-items.
- Toggle GSI automation off and confirm keyboard-driven features still react to the newest GSI state.
- Watch for smoother behavior under heavy GSI traffic compared with the current baseline.

## Expected outcome

After this slice, each GSI event should refresh shared keyboard-supporting state exactly once before dispatch, rather than paying duplicate upstream + dispatcher refresh work whenever automation is enabled.
