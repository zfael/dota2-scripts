# Design: Standalone Combo Offload

## Problem

Most of the session's freeze-reduction work has already moved hot gameplay paths off raw per-event thread creation, but the standalone combo path still has one important blocking behavior:

- `src\main.rs` consumes `HotkeyEvent::ComboTrigger` on a dedicated hotkey consumer thread
- that thread currently calls `ActionDispatcher::dispatch_standalone_trigger(...)` inline
- `dispatch_standalone_trigger(...)` immediately calls `HeroScript::handle_standalone_trigger()`
- Tiny and Legion Commander implement standalone combos with long inline `thread::sleep(...)` chains

That means one standalone combo can block the hotkey consumer thread for roughly a second or more, delaying later combo triggers or other keyboard-driven standalone actions even though the rest of the runtime has been moving toward explicit async lanes.

## Goals

- Remove long standalone combo execution from the hotkey consumer thread.
- Preserve the current trigger key, hero routing, and combo step ordering.
- Reuse existing runtime infrastructure where possible instead of adding more ad hoc threads.
- Keep the slice narrow enough to validate with gameplay testing after implementation.

## Non-goals

- No redesign of Tiny or Legion combo timings.
- No changes to Shadow Fiend's dedicated worker flow.
- No changes to Largo's dedicated hotkey handling.
- No broad cleanup of every small keyboard-side `sleep(...)` in the repo.
- No UI or GSI pipeline changes.

## Options considered

### Option 1: Route standalone combo execution through `ActionExecutor`

Keep the hotkey consumer thread responsible only for deciding that a combo should run, then enqueue the actual standalone combo execution onto `ActionExecutor`.

**Pros**

- Reuses an existing async lane instead of introducing new workers.
- Keeps the hotkey handler responsive immediately after the trigger is received.
- Small blast radius: mostly `main.rs`, `dispatcher.rs`, and the affected hero scripts.

**Cons**

- Long combos would now occupy the shared executor worker while they run.
- Requires care to keep existing hero routing semantics unchanged.

### Option 2: Add a dedicated standalone-combo worker

Create one runtime-owned worker specifically for standalone combo requests and route Tiny/Legion execution there.

**Pros**

- Strong isolation from the executor lane used by GSI-driven actions.
- Clear ownership boundary for long combo sequences.

**Cons**

- Adds another queue/worker concept to the runtime.
- Wider change than necessary for the immediate hotspot.
- More design surface for a single incremental slice.

### Option 3: Add per-hero combo workers

Give Tiny and Legion Commander their own request queues or workers similar to Shadow Fiend.

**Pros**

- Maximum isolation between heroes.
- Future room for hero-specific queue policy.

**Cons**

- Overbuilt for only two current problem paths.
- More threads, more lifecycle code, more docs surface.
- Harder to justify before proving a shared offload is insufficient.

## Recommendation

Use **Option 1**.

This is the smallest, most behavior-preserving way to stop long standalone combos from blocking the hotkey consumer thread. It reuses a runtime lane that already exists, keeps trigger detection simple, and stays aligned with the incremental rollout strategy used throughout this session.

## Proposed architecture

### Hotkey thread stays thin

`src\main.rs` should continue to:

- receive `HotkeyEvent::ComboTrigger`
- check `AppState.standalone_enabled`
- resolve the selected hero

But after it has determined the target hero, it should no longer run the full combo inline via `dispatch_standalone_trigger(...)`.

Instead, it should hand the standalone combo execution off to an async lane immediately.

### Dispatcher remains the routing boundary

`ActionDispatcher` should remain the central place that knows how to route a standalone trigger to the correct hero script.

This slice should add an async-aware standalone dispatch path for the specific heroes in scope rather than pushing hero lookup logic into `main.rs`.

That keeps responsibilities clear:

- `main.rs` decides *that* a combo was requested
- `ActionDispatcher` decides *which* hero script handles it
- the chosen async lane decides *where* the long combo work executes

### Target scope: Tiny and Legion Commander

The practical hotspot is the long sleep-heavy combo logic in:

- `src\actions\heroes\tiny.rs`
- `src\actions\heroes\legion_commander.rs`

These combos currently run inline through `handle_standalone_trigger()`. After the offload, their internal combo timing can stay the same, but they should no longer execute on the hotkey consumer thread.

The async offload path should therefore be explicitly scoped to:

- `npc_dota_hero_tiny`
- `npc_dota_hero_legion_commander`

All other standalone-routing behavior should stay on its current path in this slice unless the implementation requires a tiny shared helper with unchanged behavior.

Shadow Fiend and Largo remain out of scope because they already use special handling:

- Shadow Fiend already has a dedicated request worker
- Largo uses distinct hotkey event variants and dedicated handling

## Execution model

### Preferred lane

Standalone combo execution for Tiny and Legion Commander should enqueue onto `ActionExecutor` using an immediate job.

That gives this slice:

- no new worker type
- no new delay primitive
- one clear offload point for the hotkey consumer thread

### Serialization expectation

This design intentionally accepts that Tiny or Legion standalone combos will serialize on the executor worker once triggered.

That is a deliberate trade-off for this slice:

- **improved keyboard responsiveness immediately**
- **minimal new infrastructure**

If later gameplay testing shows that long standalone combos interfere too much with executor-driven survivability work, that would justify a follow-up slice for a dedicated standalone-combo worker. That follow-up is not part of this design.

## Behavior expectations

- The trigger key and selected-hero gating stay unchanged.
- Tiny and Legion combo step order and existing sleeps remain unchanged in this slice.
- Tiny and Legion should continue using their existing latest cached GSI context at execution time rather than introducing a new event snapshot contract at hotkey-press time.
- If no hero is selected or no event context exists, behavior should still fail the same way it does today (log / no-op according to current hero behavior).
- Shadow Fiend and Largo standalone behavior remain unchanged.
- Heroes already using specialized standalone handling should not gain an extra `ActionExecutor` hop in this slice.

## Testing strategy

Add deterministic tests where the current behavior can be expressed clearly:

- Tiny/Legion standalone routing chooses off-thread execution instead of inline long-running execution
- the scoped async path still routes the correct hero names
- Tiny and Legion standalone trigger entry points no longer require the hotkey thread to stay blocked for the whole combo path
- Shadow Fiend and Largo remain on their current standalone path

Testing should stay focused on the routing/offload contract, not on re-validating the full gameplay timing of every combo step.

Manual gameplay validation should cover:

- Tiny standalone combo still fires in the same order and timing
- Legion Commander standalone combo still fires in the same order and timing
- repeated combo hotkeys no longer make keyboard-triggered behavior feel stalled
- Shadow Fiend and Largo standalone behavior remain unchanged

## Documentation updates

Update the runtime-flow documentation anywhere it currently implies that standalone combos are executed inline on the hotkey consumer thread.

If the dispatcher or hero docs materially change in responsibility, update the matching docs for those areas as part of implementation.

## Risks

- Long standalone combos may now occupy the executor worker long enough to delay other executor jobs.
- If async standalone routing is implemented partly in `main.rs` and partly in hero scripts, responsibilities could become muddled.
- Over-widening into unrelated keyboard cleanup would make regressions harder to isolate.

## Acceptance criteria

- The hotkey consumer thread no longer runs the long Tiny or Legion standalone combo sequences inline.
- Standalone combo routing still respects `standalone_enabled` and selected-hero gating.
- Tiny and Legion combo step timing/order remain unchanged.
- `cargo test` passes.
- `cargo build --release --target-dir target\release-verify` passes.
- Runtime docs accurately describe the new standalone combo execution path.
