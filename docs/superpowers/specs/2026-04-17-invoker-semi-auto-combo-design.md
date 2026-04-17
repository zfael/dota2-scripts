# Invoker Semi-Auto Combo Design

## Summary

Add a combo-only profile execution style for Invoker so existing combo profiles can run in one of two ways:

1. `automatic` - current behavior, unchanged
2. `semi_auto` - item steps still execute automatically, but spell steps are only prepared onto the configured secondary invoked-spell slot and are cast by the player

This keeps the current profile authoring model intact while adding a new workflow where the player repeatedly presses the same invoked-spell key, waits for that spell to enter cooldown, and lets the app prepare the next spell in sequence onto that same slot.

## Problem

The current Invoker combo planner assumes combo profiles are fully automatic once triggered. That works for players who want the entire authored spell sequence cast for them, but it does not support a guided manual flow where the app handles invocation order while the player decides when to cast each spell.

The requested workflow is:

- combo profiles remain authored exactly as they are today
- a combo profile can be marked as semi-auto in the editor
- when triggered, the app should execute item steps automatically
- spell steps should be prepared one at a time onto the configured secondary invoked-spell slot
- once that prepared spell enters cooldown, the app should immediately prepare the next spell onto the same slot
- the runner should keep going even if the player temporarily deviates from the authored slot state

## Goals

- Preserve existing Invoker profile authoring and defaults
- Add a profile-level way to switch a combo between fully automatic and semi-auto spell execution
- Keep prep profiles unchanged
- Keep combo item steps automatic in semi-auto mode
- Make semi-auto center on the configured `spell_slot_secondary_key` instead of introducing a new hardcoded key
- Allow semi-auto to recover from manual deviations instead of aborting the profile

## Non-Goals

- Do not replace the existing `combo` vs `prep` profile distinction
- Do not introduce a per-step semi-auto authoring model
- Do not add a separate semi-auto timeout setting
- Do not change non-Invoker standalone behavior
- Do not preserve two-spell preload behavior in semi-auto mode

## Current State

Invoker profiles currently have:

- a profile-level `mode` of `combo` or `prep`
- ordered `steps` of kind `spell` or `item`
- spell-step cast behavior and completion settings

The current combo runner:

- plans consecutive spell steps in one- or two-spell batches
- preloads real `D` and `F` slots based on authored order
- auto-casts spell steps when `mode = "combo"`
- skips casting spell steps when `mode = "prep"`

This is the right foundation for fully automatic and prep flows, but it does not express "prepare the next spell for me on one fixed slot, then wait for my real cast."

## Proposed Design

### 1. Profile model

Keep `InvokerProfileMode` unchanged:

- `combo`
- `prep`

Add a new combo-focused execution field to `InvokerProfile`:

```toml
execution_style = "automatic" | "semi_auto"
```

Default:

- existing profiles deserialize as `automatic`
- newly created combo profiles default to `automatic`
- prep profiles may still carry the field for schema consistency, but runtime behavior ignores it

This keeps "what kind of profile is this?" separate from "how should combo spell steps execute?"

### 2. React UI

Add a combo-only control in the Invoker profile editor for the new field.

Expected behavior:

- combo profiles show an execution-style control
- prep profiles hide or disable the control because prep already means invoke-only
- the control uses clear labels such as `Automatic` and `Semi-auto`
- configured profile cards show a small inline indicator when a combo uses semi-auto
- the active combo header can continue to show the active profile name without extra logic changes, though showing the execution style next to the name is acceptable if it stays compact

The new control belongs with the existing profile-level fields such as name, hotkey, mode, and build tag. This is a profile-wide behavior, not a per-step option.

### 3. Runtime split

Automatic combo profiles keep the current runtime path unchanged.

Semi-auto combo profiles use a new runner with different spell semantics:

1. Execute leading item steps automatically using the existing item behavior
2. Prepare exactly one spell step onto the configured secondary invoked-spell slot
3. Stop and wait for that spell to enter cooldown in GSI
4. When cooldown is observed, advance the queue
5. Execute any item steps now at the front of the queue automatically
6. Prepare the next spell onto the same secondary slot
7. Repeat until the profile is exhausted

This makes semi-auto a one-slot guided flow instead of a pair-preload flow.

### 4. Semi-auto spell semantics

In semi-auto mode, spell steps still use:

- authored order
- authored target ids

Spell steps no longer use these fields to control runtime execution:

- `cast_behavior`
- `completion_mode`
- spell-step `delay_after_ms`

Instead, spell progression is defined as:

`prepared spell enters cooldown -> immediately prepare next spell`

Rationale:

- the player is responsible for the actual spell cast
- the player asked for the next spell to be invoked as soon as the monitored slot spell goes on cooldown
- keeping spell-step completion and timing logic active in semi-auto would fight the one-key manual workflow

Item steps retain their current semantics, including `delay_after_ms`.

### 5. Slot targeting

Semi-auto always targets the configured `spell_slot_secondary_key`.

It does not:

- hardcode literal `F`
- introduce a new semi-auto-specific keybinding
- use pair-aware `D` and `F` preloading

This keeps the requested workflow centered on one repeated cast key while still respecting the existing configurable slot layout.

### 6. Session model

Semi-auto cannot be modeled as a single synchronous `run_profile` call because it must survive across multiple manual casts. It needs an in-flight Invoker-owned session.

The session should snapshot the relevant configuration and profile data at trigger time, including:

- profile id and display name
- remaining queued steps
- configured secondary slot key
- orb and invoke keys needed to prepare future spells
- currently watched spell, if any

The snapshot avoids mutating an already-running semi-auto combo when the user edits the profile in the UI. Retiggering starts a fresh session using the updated config.

### 7. Trigger behavior

Both existing trigger paths should keep working:

- generic active-combo trigger
- per-profile Invoker hotkeys

When a semi-auto combo is triggered:

- any existing semi-auto session is replaced
- the new session starts immediately
- the runner performs automatic item steps until it reaches the first spell wait point

This keeps user intent simple: the latest trigger wins.

### 8. Deviation and recovery

Semi-auto must not abort just because the player temporarily changes the invoked slots.

Expected behavior:

- if the player manually invokes or casts something else mid-sequence, the session remains alive
- the runner still waits for the currently prepared planned spell to enter cooldown when that spell is the active wait target
- when it is time to advance, the runner re-prepares the next remaining planned spell onto the configured secondary slot instead of assuming the previous slot layout still exists

This makes semi-auto behave like a persistent guide rail rather than a fragile scripted transaction.

### 9. Waiting rules and cleanup

Semi-auto waits indefinitely for a prepared spell to enter cooldown.

It does not use the existing spell-step completion timeout for semi-auto waiting.

The session ends when:

- all remaining steps are consumed
- the hero becomes unavailable, such as dead, stunned, hexed, or silenced
- another Invoker profile trigger replaces the session

On cleanup, the runner should emit a clear log message explaining why the session ended.

### 10. Logging and operator visibility

Semi-auto should emit explicit tracing for the operator-facing event history and debugging logs. Useful messages include:

- profile triggered in semi-auto mode
- automatic item step executed
- spell prepared onto secondary slot
- observed cooldown for watched spell
- advancing to next planned spell
- replacing an in-flight semi-auto session
- ending session because queue completed or hero became unavailable

The goal is to make the new mode understandable from logs without reading code.

## Data Flow

### Automatic combo

No change:

- build plan
- preload one or two spells
- auto-cast spell steps
- honor existing per-step completion behavior

### Semi-auto combo

- trigger profile
- create semi-auto session snapshot
- consume automatic item steps until next spell
- invoke that spell onto the configured secondary slot
- mark that spell as watched
- on future GSI updates, observe when watched spell enters cooldown
- consume newly available automatic item steps
- invoke next queued spell onto the configured secondary slot
- repeat until exhausted or canceled

## Error Handling

- Missing optional combo items remain best-effort and are logged as skipped
- If a semi-auto spell cannot be prepared from the current config or spell recipe, end the session with a clear log entry
- If no GSI event is available when a trigger occurs, do not create a session
- If the hero is unavailable when advancement is attempted, clear the session
- If the profile id no longer exists when a trigger arrives, do not start a session

Do not silently swallow these cases; they should be visible in tracing just like current Invoker runner skips and aborts.

## Testing Strategy

### Rust

- serde/default coverage for the new `execution_style` field
- plan/session tests showing `automatic` still uses current behavior
- semi-auto tests for:
  - item then spell progression
  - repeated spell preparation onto the configured secondary slot
  - advancement when watched spell enters cooldown
  - recovery after slot deviation
  - session replacement when a new profile is triggered
  - cleanup when hero becomes unavailable

### React

- profile editor renders and persists the new combo-only control
- prep profiles do not expose an actionable semi-auto control
- profile list renders a visible semi-auto indicator
- mock data and config types include the new field with automatic defaults

### Documentation

- update `docs/heroes/invoker.md`
- update configuration examples and field tables for the new profile field
- describe the behavioral differences between automatic combo, semi-auto combo, and prep

## Migration and Compatibility

- Existing checked-in config remains valid
- Existing saved user config remains valid
- Existing combo profiles behave exactly as today until explicitly changed to `semi_auto`
- Existing prep profiles behave exactly as today

## Implementation Notes

- Prefer a dedicated semi-auto runtime path instead of trying to retrofit the current pair-preload logic with special cases
- Keep automatic combo logic behavior-identical to reduce regression risk
- Treat the semi-auto session as an Invoker-owned state machine driven by GSI updates and trigger events

## Open Decisions Resolved During Brainstorming

- Item steps remain automatic in semi-auto mode
- Semi-auto uses the configured secondary invoked-spell key, not a hardcoded literal key
- Semi-auto recovers from manual deviations by re-preparing the next planned spell instead of aborting
- Semi-auto keeps exactly one planned spell ready on the monitored secondary slot
- Semi-auto waits indefinitely for the player to cast the prepared spell

