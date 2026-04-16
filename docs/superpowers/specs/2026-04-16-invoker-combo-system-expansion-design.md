# Invoker Combo System Expansion Design

## Problem

The current Invoker profile system can sequence spells and items, but it is still
too narrow for the next set of real gameplay scenarios the user wants to support.

Today:

- the seeded preset pack is limited to **QW Pickoff**, **QE Burst**,
  **Ghost Walk Panic**, and **Meteor + Blast Prep**
- the Invoker item catalog does not include important combo items such as
  **Refresher Orb** or **Eul's Scepter**
- the global "next enabled combo profile" hotkey exists, but it is hardcoded and
  not operator-configurable
- missing combo items are skipped at runtime, but that behavior is not surfaced
  clearly in the UI model
- `wait_for_cooldown` is not a true manual-cast model; it still presses the spell
  slot before waiting, which makes manual-targeted spells such as **Sun Strike**
  feel wrong
- there is no explicit cast behavior for cases such as **Cataclysm**
  `Alt + D/F` repeated activation

The user wants better built-in Invoker combo presets, optional item support,
proper manual spell continuation, and a configurable way to cycle the active
combo profile in game.

## Scope Split

This overall request should be split into two projects.

### Project 1: Invoker combo system expansion

This design covers:

- preset expansion
- richer spell cast semantics
- more supported combo items
- configurable cycle hotkey
- clearer UI/runtime treatment of optional item steps

### Project 2: Invoker lane-pressure micro

This design does **not** cover:

- dual-unit control for Forge Spirit + hero harassment
- issuing attack orders for both hero and summons
- spirit-targeted buff logic

Those behaviors require a different subsystem than the current linear
spell-and-item profile runner. They should be designed separately.

## Goals

- Expand Invoker presets around common modern combos without replacing the
  current profile system
- Add first-class support for missing-but-important combo items
- Model true manual spell continuation for targeted casts such as Sun Strike
- Model special cast patterns such as Cataclysm `Alt + D/F` repeat casting
- Make the global "cycle next enabled combo profile" hotkey configurable in
  config and UI
- Keep runtime behavior explicit, observable, and aligned with the editor

## Non-Goals

- No summon-unit micro or attack-order automation in this pass
- No generalized action DSL for every possible keypress pattern
- No overlay window or in-game on-top UI in this pass
- No build recommender or auto-selection of the best combo from game state
- No per-step cursor-positioning or aim prediction

## Recommended Approach

Keep the current Invoker profile model, but extend each step with a small,
explicit **cast behavior** concept.

This is the best trade-off because:

- it preserves the existing profile editor shape
- it keeps current combo/prep semantics intact
- it solves the concrete gaps the user called out
- it avoids dragging the feature into a much larger action-DSL rewrite

The profile runner remains linear. The new behavior is expressed by a few
additional step-level choices rather than by introducing a second execution
engine.

## Alternatives Considered

### 1. Preset-only expansion with no runtime model changes

This would let us ship more seeded profiles quickly, but it would not solve the
two most important behavioral gaps:

- true manual Sun Strike continuation
- Cataclysm-style modifier repeat casting

It would make the presets look richer while still feeling wrong in play.

### 2. Full action DSL

We could replace spell/item steps with a broader action system such as:

- key chord
- repeated press
- attack order
- unit select
- item use
- manual wait

This would be more powerful, but it is too much scope for the current need. It
would increase editor complexity, runtime complexity, and testing cost well
beyond what is necessary to support the requested Invoker scenarios.

## Execution Model

The current Invoker step shape should be extended, not replaced.

### Current Shape

- `kind`
- `target`
- `delay_after_ms`
- `completion_mode`
- `completion_timeout_ms`
- `notes`

### Proposed Shape

- `kind`
- `target`
- `delay_after_ms`
- `completion_mode`
- `completion_timeout_ms`
- `cast_behavior`
- `notes`

`cast_behavior` should describe **how the runner activates the prepared spell or
item step** once it is ready.

Defaults:

- existing steps default to `cast_behavior = "normal"`
- item steps remain `cast_behavior = "normal"` in this pass

### Cast Behaviors

The first pass should support these behaviors:

- `normal`
  - current single keypress behavior
- `manual_wait_cooldown`
  - prepare the spell
  - do **not** press the D/F slot key
  - wait for the player to cast the spell and for cooldown to start
- `alt_cast`
  - press `Alt + slot key` once
- `double_tap`
  - press the slot key twice with a short fixed interval
- `alt_double_tap`
  - press `Alt + slot key` twice with a short fixed interval

`cast_behavior` applies primarily to spell steps in this first pass. Item steps
remain normal single-use steps unless a later need proves otherwise.

### Relationship to Completion Mode

The system should answer two separate questions for each step:

1. **How is the step activated?** -> `cast_behavior`
2. **When is the step complete?** -> `completion_mode`

That separation matters because:

- `manual_wait_cooldown` changes activation behavior
- `wait_for_cooldown` changes completion detection

For true manual targeted spells, the design should pair:

- `cast_behavior = "manual_wait_cooldown"`
- `completion_mode = "wait_for_cooldown"`

That combination means:

1. invoke or preload the spell
2. leave the spell ready on D/F
3. wait for the player's manual cast to consume it
4. continue only after cooldown starts

This corrects the current mismatch where the runtime presses the spell key on the
player's behalf before waiting.

### Special Casting Behavior

Cataclysm should use an explicit step behavior rather than hidden spell-specific
logic.

Recommended handling:

- the step still targets `invoker_sun_strike`
- when the relevant preset or custom profile wants Cataclysm behavior, it uses
  `cast_behavior = "alt_double_tap"`

This keeps the profile model declarative and lets the runtime execute the exact
modifier pattern intentionally.

## Item Support Model

The Invoker item catalog should be expanded to include at least:

- `item_refresher`
- `item_cyclone`

The current missing-item runtime behavior should stay:

- if the configured item is present, use it
- if it is missing, skip that step and continue

That behavior already matches the user's request and should become an explicit
documented rule rather than an implicit implementation detail.

## Preset Strategy

This design should add new seeded presets, but only for scenarios that the
expanded runner can support cleanly.

### Keep Existing Presets

- QW Pickoff
- QE Burst
- Ghost Walk Panic
- Meteor + Blast Prep

### Add New Presets

- **Meta Catch**
  - Tornado
  - EMP
  - optional Cold Snap follow-up
- **Shotgun Burst**
  - Atos or Eul's
  - Sun Strike or Cataclysm behavior
  - Chaos Meteor
  - Deafening Blast
- **Ice Floe Root Lockdown**
  - Ice Wall
  - follow-up Meteor or Sun Strike/Cataclysm
- **Refresher Sequence**
  - first AoE pass
  - Refresher Orb
  - second AoE pass

### Laning Pressure Handling

The requested **Laning Pressure Combo** should be included only as a
**partial-support preset** in this project:

- Forge Spirit

It should **not** attempt:

- self-Alacrity
- Cold Snap
- spirit-targeted Alacrity
- selecting the Forge Spirit
- issuing hero-plus-spirit attack orders

Those would create hidden promises the current architecture cannot satisfy
reliably.

## UI and Config Model

### Cycle Hotkey

The global Invoker cycle key should move from a hardcoded runtime constant into
user-visible config.

Recommended home:

- `heroes.invoker.cycle_combo_profiles_hotkey`

Why this location:

- it is Invoker-specific behavior, not a shared global hotkey for every hero
- it belongs with the rest of the Invoker profile runner controls
- it matches the existing mental model for per-hero hotkeys in this repo

The React UI should expose this field in the Invoker configuration page.

Default value:

- `Delete`

### Step Editor

For spell steps, the profile editor should expose:

- completion mode
- completion timeout when cooldown wait is selected
- cast behavior

For item steps:

- keep cast behavior fixed to `normal` in v1
- optionally show helper text that missing items are skipped automatically

### Editor Clarity

The step summary and execution preview should make these behaviors visible. A
manual Sun Strike step should be easy to distinguish from an auto-cast step when
scanning a profile.

## Runtime Rules

### Manual Targeted Spell Rule

When a spell step uses:

- `cast_behavior = "manual_wait_cooldown"`
- `completion_mode = "wait_for_cooldown"`

the runner should:

1. prepare or invoke the spell as usual
2. leave it ready on the correct slot
3. wait for cooldown start
4. abort the remaining profile if the timeout expires

The runner must **not** auto-press D/F in this mode.

### Optional Item Rule

Optional combo items are not a separate step type. The runtime should treat any
configured item step as:

- use when present
- skip when absent
- continue without failing the whole combo

### Cycle Hotkey Rule

The cycle hotkey continues to rotate only through enabled combo profiles, but it
should now use the configured Invoker hotkey value rather than a hardcoded key.

## Logging and Observability

When implemented, runtime logging should make the new behaviors visible:

- spell prepared for manual cast
- waiting for cooldown start
- cooldown confirmed
- timeout aborted profile
- executing `alt_cast` / `double_tap` / `alt_double_tap`
- optional item skipped because it was not found
- cycle hotkey changed active combo profile

This is important because the new behaviors are more nuanced than today's simple
single-press flow.

## Testing

Implementation should add targeted coverage for:

- config serialization defaults for `cast_behavior`
- preset defaults include new combo profiles and item ids
- item catalog and editor include Refresher and Eul's
- `manual_wait_cooldown` does not auto-press the slot key
- `wait_for_cooldown` continues only after real cooldown start
- timeout abort still works for manual spell steps
- `alt_double_tap` produces the intended key pattern
- optional missing item steps are skipped without aborting the profile
- cycle hotkey can be configured and is reflected in keyboard planning
- UI step editor renders and persists cast-behavior choices

## Implementation Notes

- This design is intentionally limited to the current linear Invoker profile
  runner
- Spell-specific behavior should stay data-driven through `cast_behavior`, not
  through hidden special cases spread across the runner
- Documentation updates should cover:
  - `docs/heroes/invoker.md`
  - `docs/reference/configuration.md`
- The separate **Invoker lane-pressure micro** design should be created later if
  the user wants true Forge Spirit harassment automation
