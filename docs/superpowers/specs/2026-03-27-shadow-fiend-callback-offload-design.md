# Shadow Fiend Callback Offload Design

## Goal

Remove the remaining per-intercept raw thread spawning from the Shadow Fiend keyboard-intercept paths without changing raze-facing behavior, auto-BKB sequencing, or unrelated hero/runtime behavior.

## Why this is the next slice

The current runtime still spawns a fresh OS thread for each Shadow Fiend intercepted sequence:

- `ShadowFiendState::execute_raze(...)`
- `ShadowFiendState::execute_ultimate_combo(...)`
- `ShadowFiendState::execute_standalone_combo(...)`

Broodmother callback actions, Soul Ring replay, and the Enigo-backed synthetic-input lane already use long-lived workers. Shadow Fiend is now the clearest remaining hot-path source of bursty thread creation during gameplay.

## Scope

In scope:

- Shadow Fiend Q/W/E raze interception
- Shadow Fiend R ultimate interception
- Shadow Fiend standalone combo execution path
- a dedicated Shadow Fiend request worker
- deterministic tests for request planning / worker-facing helper logic
- docs directly tied to the new worker ownership

Out of scope:

- changing `raze_delay_ms` behavior or retuning the sequence
- changing the standalone-key conflict documented in `docs/heroes/shadow_fiend.md`
- changing Soul Ring, Broodmother, Largo, or generic keyboard-intercept behavior
- redesigning the synthetic-input worker
- changing Blink / BKB / D / R combo semantics

## Approaches considered

### 1. Dedicated Shadow Fiend intercept worker (recommended)

Add one lazy FIFO worker owned by `src\actions\heroes\shadow_fiend.rs`. Keyboard interception and standalone-trigger entry points enqueue high-level Shadow Fiend sequence requests instead of calling `thread::spawn(...)` directly.

Pros:

- removes the remaining per-intercept raw thread creation from the SF path
- keeps Shadow Fiend timing logic owned by the Shadow Fiend module
- preserves the current callback ordering while shrinking callback-side work
- matches the existing worker pattern used by Soul Ring and Broodmother

Cons:

- serializes full SF requests through one worker, so back-to-back razes queue instead of racing as independent timers

### 2. Reuse an existing worker or executor

Route Shadow Fiend intercepted sequences through `ActionExecutor` or extend `src\input\simulation.rs` with broader sequencing support.

Pros:

- avoids introducing another long-lived worker

Cons:

- couples keyboard-intercept timing to broader runtime workers
- widens the blast radius well beyond one hero
- risks turning a narrow freeze-reduction slice into executor/input-lane redesign

### 3. Keep raw threads and only trim setup work

Reduce callback bookkeeping or factor out more planning helpers but keep the per-intercept `thread::spawn(...)` model.

Pros:

- smallest code change

Cons:

- leaves the hot-path thread-creation cost in place
- weakest expected freeze reduction

## Recommended design

Take approach 1.

`src\actions\heroes\shadow_fiend.rs` will own a lazy `std::sync::mpsc` queue and one long-lived worker that accepts:

```rust
enum ShadowFiendRequest {
    Raze { raze_key: char, raze_delay_ms: u64 },
    Ultimate { auto_d_on_ultimate: bool },
    Standalone { auto_bkb_on_ultimate: bool, auto_d_on_ultimate: bool },
}
```

### Ownership split

- `src\input\keyboard.rs` keeps deciding whether Q/W/E or R should be intercepted and still blocks the original key with `return None`
- `ShadowFiendState` exposes enqueue-style helpers instead of spawning raw threads directly
- the dedicated worker performs the existing timing-sensitive sequence steps
- `src\input\simulation.rs` remains the owner of actual synthetic input emission and `SIMULATING_KEYS` guard behavior

### Execution model

For `Raze` requests, the worker will preserve the current sequence:

1. sleep 50 ms
2. `alt_down()`
3. `mouse_click()`
4. sleep 50 ms
5. `alt_up()`
6. sleep `raze_delay_ms`
7. `press_key(raze_key)`

For `Ultimate` requests, the worker will:

1. lock `SF_LAST_EVENT`
2. look up a castable BKB slot from the latest live event
3. double-tap BKB if present
4. optionally press `D`
5. press `R`

For `Standalone` requests, the worker will:

1. lock `SF_LAST_EVENT`
2. verify ultimate availability
3. verify Blink availability
4. optionally resolve BKB from the latest live event
5. run the same Blink -> BKB -> D -> R sequence used today

The worker must continue reading `SF_LAST_EVENT` at execution time rather than capturing a full GSI clone in the keyboard callback. That keeps inventory / cooldown decisions aligned with the latest event and avoids widening callback-time cloning cost.

## Behavior guarantees

- Q/W/E raze interception still blocks the original key and faces toward the cursor before pressing the raze key
- R interception still blocks the original key and performs the existing BKB / D / R sequence
- standalone combo execution keeps its current semantics, including the existing trigger/key conflict limitations
- actual synthetic input ordering still flows through `src\input\simulation.rs`
- if queue submission fails unexpectedly, log a warning and fall back to the current short-lived thread path rather than silently dropping the intercepted action
- Soul Ring, Broodmother, Largo, and generic hotkey routing remain unchanged

## Key risk and mitigation

The main design trade-off is request serialization.

Today, repeated Shadow Fiend inputs can create multiple overlapping raw threads. With a dedicated worker, requests become FIFO. That may slightly change the behavior of extremely rapid repeated raze presses, but it also removes the bursty thread creation that is the point of this slice. Because all real input emission already funnels through the single synthetic-input worker, the runtime is effectively serialized at the emission layer today anyway. This design makes that sequencing explicit earlier and more observable.

To keep the rollout low-risk:

- preserve the exact per-request sleep timings
- do not retune raze or ultimate delays in the same slice
- keep fallback-to-thread logic for unexpected queue failure
- validate rapid repeated razes manually after implementation

## Testing strategy

Add deterministic tests around the worker-facing planning helpers rather than OS input itself.

Suggested coverage:

- Q/W/E interception maps to the expected `ShadowFiendRequest::Raze` payload
- ultimate enqueue path carries the expected `auto_d_on_ultimate` flag
- standalone enqueue path copies the relevant config booleans without needing a live settings lock later
- fallback helpers preserve the current sequence-building assumptions for BKB slot mapping / no-event behavior

Automated verification remains:

- `cargo test`
- `cargo build --release --target-dir target\release-verify`

Manual validation should focus on:

- rapid Q/W/E razes under pressure
- R interception with BKB available and unavailable
- standalone combo behavior remaining unchanged
- overall hitching improvement during heavy SF interception

## Docs impact

Update:

- `docs\features\keyboard-interception.md`
- `docs\architecture\runtime-flow.md`
- `docs\heroes\shadow_fiend.md`

to describe the dedicated Shadow Fiend request worker instead of per-intercept raw threads.
