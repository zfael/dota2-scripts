# Meepo Phase 3 Design

## Problem statement

The remaining Meepo roadmap item is larger than a single safe implementation slice:

- farm automation
- map pressure / split-push behavior
- objective helpers such as Tormentor or Roshan

In the current repo, those concerns do not share the same constraints. The runtime can press keys, issue right-clicks at the current cursor, and schedule delayed jobs, but it still does **not** have:

- per-clone Meepo telemetry
- world-coordinate pathing helpers
- camp/objective visibility from GSI
- generic multi-hotkey support beyond the current hero-specific paths

Because of that, “phase 3” must be decomposed before implementation.

## Decomposition

### Phase 3A: Meepo farm assist (recommended first slice)

A conservative, manually armed farm helper that improves repetitive lane / camp clearing without pretending the app can fully route clones around the map.

### Phase 3B: Objective helper

A separate helper for Tormentor / Roshan-style sequences once phase 3A establishes reusable macro state and hotkey plumbing.

### Phase 3C: Map pressure / split-push orchestration

Deferred until the app has reliable clone-aware telemetry or a deliberately manual control model for per-clone actions.

This spec covers **Phase 3A only**.

## Goal

Add a safe Meepo farm-assist mode that:

- is armed or disarmed by an explicit hotkey
- uses only signals the runtime already knows
- stops immediately when the hero is in danger or no longer Meepo
- helps with repetitive farm pulses while the player still owns cursor placement and broader movement decisions

This is intentionally **not** a full autopilot route planner.

## Approaches considered

### 1. Manual-armed farm assist with scheduled pulses (recommended)

Add a dedicated Meepo farm-assist toggle hotkey and a small state machine. When armed, Meepo periodically issues a conservative farm pulse based on current observed state and cursor position.

Each pulse may:

- cast `Poof` only when ready and safe
- optionally right-click at the current cursor position
- respect mana / health / danger gates

Pros:

- fits the current input model
- keeps player control over cursor and macro direction
- avoids fake clone-routing assumptions
- easy to pause on danger or hero changes

Cons:

- less autonomous than a true route planner
- requires one new Meepo-specific hotkey path

### 2. Configured waypoint route planner

Add configurable lane/jungle waypoints and attempt to move / clear automatically.

Pros:

- looks closer to “real macro automation”

Cons:

- the app does not currently own cursor-to-world translation
- no clone positions or camp state are available
- high risk of brittle or unsafe movement

### 3. Objective helper first

Skip farming and build Tormentor / Roshan helpers first.

Pros:

- smaller tactical surface
- could reuse manual activation

Cons:

- does not solve Meepo’s core economic gameplay loop first
- still needs macro-mode plumbing later

## Recommendation

Use **Approach 1** and explicitly define phase 3A as:

> “A manually armed, cursor-directed farm assist for the currently controlled Meepo context.”

That gives a useful macro slice without over-claiming pathing intelligence the repo does not have.

## Scope

### In scope

- one new Meepo farm-assist toggle hotkey
- Meepo macro state machine for armed / suspended behavior
- conservative pulse scheduling using `ActionExecutor`
- `Poof`-driven farm pulses gated by observed state
- optional right-click follow-up at current cursor
- automatic suspension on danger, death, stun/silence, hero swap, or manual combo trigger
- UI visibility for armed / suspended farm-assist state
- config and tests

### Out of scope

- autonomous camp routing
- clone-by-clone movement
- net / Poof combat chains
- Tormentor / Roshan helpers
- split-push orchestration
- any behavior that depends on inferred clone positions

## Proposed architecture

### 1. New `meepo_macro.rs` helper

Add a dedicated helper module, likely:

- `src/actions/heroes/meepo_macro.rs`

This keeps `src/actions/heroes/meepo.rs` from absorbing macro state, toggle logic, pulse planning, and gating all in one file.

Suggested responsibilities:

- macro state enum
- pure pulse-gating helpers
- suspend-reason modeling
- small execution planner for a single farm pulse

Suggested types:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeepoMacroMode {
    Inactive,
    Armed,
    Suspended(MeepoMacroSuspendReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeepoMacroSuspendReason {
    Danger,
    Disabled,
    HeroChanged,
    ManualCombo,
    UnableToCast,
}
```

### 2. Expand Meepo observed state just enough for farming

Phase 2 intentionally kept `MeepoObservedState` conservative. Phase 3A should extend it only where the farm pulse truly needs extra known state:

- `poof_ready`
- optional `earthbind_ready` only if later wanted for manual farming combos

No clone-ready arrays or guessed clone counts should be added.

### 3. Meepo farm-assist config

Extend `MeepoConfig` with a nested block, for example:

```toml
[heroes.meepo.farm_assist]
enabled = true
toggle_key = "End"
pulse_interval_ms = 700
minimum_mana_percent = 35
minimum_health_percent = 45
right_click_after_poof = true
suspend_on_danger = true
suspend_after_manual_combo_ms = 2500
poof_press_count = 1
poof_press_interval_ms = 35
```

Notes:

- `toggle_key` should be separate from the existing standalone combo key
- `poof_key` still reuses the main Meepo ability binding already defined in `[heroes.meepo]`
- the nested config keeps phase-3 settings isolated from phase-1 combat config

### 4. Hotkey plumbing

Add a new Meepo-specific hotkey event path:

- `HotkeyEvent::MeepoFarmToggle`

`KeyboardSnapshot` should expose the parsed Meepo farm-assist toggle key only when:

- standalone scripts are enabled
- selected hero is Meepo
- `[heroes.meepo.farm_assist].enabled = true`

This keeps the listener logic aligned with current hero-specific hotkey handling such as Largo’s extra keys.

### 5. Runtime data flow

```text
User presses Meepo farm toggle key
  -> keyboard listener emits HotkeyEvent::MeepoFarmToggle
  -> main hotkey loop routes to MeepoScript
  -> MeepoScript toggles macro mode between Inactive and Armed

Every Meepo GSI event
  -> handler refreshes MeepoObservedState
  -> MeepoScript refreshes final in_danger-aware snapshot
  -> MeepoScript evaluates farm pulse gates
  -> if Armed and safe, enqueue a farm pulse on ActionExecutor
  -> pulse may cast Poof and optionally right-click at cursor
```

### 6. Suspension rules

Farm assist must be conservative. It should immediately suspend or refuse pulses when:

- Meepo is not the active hero anymore
- the hero is dead
- `in_danger == true`
- the hero is stunned or silenced
- `poof_ready == false`
- current HP% or mana% are below configured thresholds

When the manual combo trigger fires, farm assist should enter `Suspended(ManualCombo)` for a short cooldown window before rearming automatically or requiring a fresh toggle. The simpler version is:

- suspend for `suspend_after_manual_combo_ms`
- then return to `Armed`

## Pulse behavior

Each farm pulse should remain intentionally small:

1. confirm farm-assist mode is `Armed`
2. confirm current observed state still passes gates
3. press `Poof` `poof_press_count` times with configured interval
4. if `right_click_after_poof = true`, issue a right-click at current cursor position
5. record `last_pulse_at`

Important constraint:

- the pulse is **cursor-directed**, not coordinate-directed
- the player is still responsible for placing the cursor over the camp, wave, or movement direction

## Error handling

- If Meepo farm assist is toggled without any Meepo GSI snapshot yet, do not fire actions; store `Suspended(UnableToCast)` and log a warning.
- If the hotkey is pressed while another hero is selected, ignore it cleanly.
- If a pulse is skipped because `Poof` is unavailable or mana/HP are too low, do not spam warnings every tick; use stateful suspend / resume logs instead.

## Testing strategy

### Unit tests

Add pure tests for:

- macro mode toggling
- pulse-gating decisions
- suspension on danger / death / stun / silence
- resume-after-manual-combo timing logic
- `KeyboardSnapshot` parsing of the new Meepo farm-assist key

### Integration-level tests

Add focused Meepo tests that verify:

- the hotkey path emits the new event only when Meepo is selected
- GSI refresh can suspend or clear farm-assist state when hero changes
- observed-state extension exposes `poof_ready`

### Verification

Run:

- `cargo test`
- `cargo build --release --target-dir target\release-verify`

## Documentation updates

If phase 3A is implemented, update at least:

- `docs/heroes/meepo.md`
- `docs/reference/configuration.md`
- `docs/reference/file-index.md`
- `docs/reference/gsi-schema-and-usage.md`

## Success criteria

Phase 3A is successful when:

1. Meepo has a dedicated farm-assist toggle key.
2. The runtime exposes an explicit farm-assist mode for Meepo.
3. Farm pulses use only known state from GSI / observed state.
4. The helper suspends cleanly on danger or invalid conditions.
5. The player retains cursor and macro-direction control.
6. No fake clone-routing or map-coordinate logic is introduced.

## Deferred work

Keep these out of the implementation plan for this slice:

- Tormentor / Roshan helpers
- split-push / lane pressure planners
- clone-distributed farm orchestration
- pathing by configured map waypoints
