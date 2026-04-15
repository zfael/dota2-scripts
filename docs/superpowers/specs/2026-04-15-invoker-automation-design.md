# Invoker Automation Design

## Problem

Invoker is a poor fit for the repo's simplest hero-automation patterns because his value comes from:

- dynamic spell-slot preparation
- long multi-step cast sequences
- multiple viable combo families
- strong dependence on cursor position and player intent

The current runtime is good at reading **self state** from GSI and emitting serialized input, but it is not a tactical aiming system. It does not know enemy positions, pathing, or target priority. A workable Invoker design therefore needs to automate **sequencing and preparation** without pretending it can fully pilot the hero.

## Goals

1. Add Invoker as a supported hero in the repo's existing dispatcher, UI/manual-override, and standalone trigger flow.
2. Provide one reliable **manual offensive combo trigger** that sequences a configured Invoker combo profile.
3. Provide dedicated **panic** and **prep** triggers that solve high-value Invoker problems without requiring enemy-position inference.
4. Keep all offensive behavior gated by spell readiness, currently invoked spells, and item availability.
5. Preserve player control by using the current cursor/quickcast setup instead of moving the mouse or inventing targets.

## Non-goals

1. Build a full autonomous Invoker AI that chooses targets, positions the camera, or solves enemy geometry.
2. Intercept and rewrite Invoker's normal Q/W/E/R/D/F gameplay in the first pass.
3. Support every Invoker combo, talent edge case, or patch-specific upgrade interaction in the first pass.
4. Infer Aghanim's Scepter/Shard "pill" upgrade choices from GSI alone.

## Current State

### Repo capabilities Invoker can reuse

The current codebase already has the pieces needed for a strong assistant-style Invoker implementation:

- `HeroScript`-based GSI routing in `src/actions/dispatcher.rs`
- selected-hero + standalone combo dispatch through `AppState` and `HotkeyEvent::ComboTrigger`
- dedicated worker-queue patterns in Shadow Fiend and Outworld Destroyer for serialized combo execution
- high-level synthetic input helpers in `src/input/simulation.rs`
- shared survivability composition in hero scripts through `SurvivabilityActions`

These patterns favor:

- explicit hero state checks before acting
- serialized combo workers instead of ad-hoc thread spawns
- config-driven key mappings and timings
- keeping risky or highly contextual automation behind explicit user triggers

### Runtime limits that shape the design

The current GSI model only exposes:

- hero snapshot
- six ability slots
- self inventory
- map clock

It does **not** expose:

- enemy positions
- ally positions
- cursor position
- target identity
- exact cast ranges for dynamic targeting
- reliable selected Scepter/Shard upgrade choice

That means Invoker automation can safely reason about:

- orb levels
- whether a spell is already invoked
- whether a spell or item can cast
- whether the hero owns Scepter or Shard

But it cannot safely reason about:

- where to aim Tornado, EMP, Meteor, Ice Wall, or Sun Strike
- which enemy should receive Cold Snap, Vessel, or Atos
- whether Cataclysm is the correct global cast at a given moment

## Design Summary

The recommended design is a **hybrid Invoker assistant**:

- one generic standalone combo key for the primary offensive profile
- one dedicated panic hotkey for Ghost Walk
- one dedicated prep hotkey for pre-invoking the next spell pair
- optional low-risk reactive helpers kept narrow and disabled by default

This design uses automation where the repo is strong:

- spell-slot preparation
- legality checks
- deterministic key sequencing
- repeated timing-sensitive input

And avoids automation where the repo is weak:

- target selection
- cursor movement
- autonomous offensive casting from pure GSI state

## Proposed Architecture

### 1. `InvokerScript`

Add `src/actions/heroes/invoker.rs` implementing `HeroScript`.

Responsibilities:

- cache the latest Invoker GSI event
- compose shared survivability actions like the existing hero scripts
- expose manual combo, panic, and prep entry points
- translate current GSI state plus config into an executable Invoker request

Invoker should follow the Outworld Destroyer / Shadow Fiend pattern and own its own dedicated request worker instead of spawning one thread per combo.

### 2. `InvokerObservedState`

Build a focused state snapshot from GSI on each event. It should capture:

- Quas/Wex/Exort/Invoke readiness and orb levels
- currently invoked spell names in the two active spell slots
- whether core spells are ready to cast
- item-slot keys for configured combo items
- hero alive / silenced / stunned / hexed state
- Scepter and Shard ownership

This state should be derived from the six GSI ability slots, not guessed from config. For Invoker, the critical question is not just "is Tornado learned" but also "is Tornado currently in slot D or F, or must it be invoked first?"

### 3. `InvokerComboPlanner`

Add a small planner layer inside `invoker.rs` or a focused helper module. Its job is to convert:

- observed state
- configured build/profile
- requested trigger type

into an explicit sequence of low-level actions such as:

- cast orb key
- cast `invoke`
- press active spell key `D` or `F`
- press item slot key
- wait for a configured timing window

The planner should never guess a target. It only decides the correct **spell preparation and ordering**.

### 4. `InvokerRequestWorker`

Use one serialized worker queue for all Invoker requests:

- `PrimaryCombo`
- `PanicGhostWalk`
- `PrepPair`

This preserves timing consistency and prevents overlapping invoke sequences from corrupting the spell slots.

## Trigger Model

### Primary combo trigger

Invoker joins the existing generic standalone flow:

- add `HeroType::Invoker`
- route the hero in `main.rs`
- expose `heroes.invoker.standalone_key`

This trigger runs the configured primary combo profile and is the main offensive automation surface.

### Panic trigger

Add a dedicated Invoker panic hotkey through `src/input/keyboard.rs` and a new `HotkeyEvent` variant.

Default behavior:

1. verify Ghost Walk is legal to prepare/cast
2. invoke Ghost Walk if needed
3. cast Ghost Walk immediately

This is the safest high-value Invoker automation in the repo because it does not need target geometry.

### Prep trigger

Add a dedicated Invoker prep hotkey through `src/input/keyboard.rs`.

Default behavior:

- prepare a configured two-spell package without casting it

Initial supported prep packages:

- `tornado_emp`
- `meteor_blast`
- `cold_snap_forge_spirit`
- `ghost_walk_ice_wall`

If both requested spells are already invoked, the prep trigger should no-op and log that no action was needed.

### Reactive helpers

Reactive helpers are deferred from the first implementation.

The most likely later addition is:

- `auto_ghost_walk_on_danger`

Even that helper should remain opt-in when it is eventually added, because it overrides player intent and can grief fights if it fires at the wrong time. No offensive GSI-only auto-casting belongs in v1.

## Combo Profiles

The first version should deliberately support a **small** combo catalog.

### Profile 1: `qw_pickoff`

Intended for tempo/QW gameplay.

Sequence:

1. ensure Tornado is available
2. cast Tornado
3. wait configured tornado-to-EMP delay
4. ensure EMP is available
5. cast EMP
6. optionally follow with configured item casts such as Vessel if the player wants them in the profile

This combo assumes the player is responsible for cursor placement and quickcast semantics.

### Profile 2: `qe_burst`

Intended for QE pickoff with current-cursor targeting.

Sequence:

1. optionally cast Rod of Atos if present and enabled
2. ensure Sun Strike is available
3. cast Sun Strike
4. wait configured delay
5. ensure Chaos Meteor is available
6. cast Chaos Meteor
7. wait configured delay
8. ensure Deafening Blast is available
9. cast Deafening Blast

This is intentionally narrower than a full late-game Refresher combo. The goal is a stable, repeatable profile that matches the repo's input model.

### Explicitly deferred profiles

These should stay out of the first implementation:

- Blink-led initiation profiles
- Cataclysm automation that assumes global enemy lockdown
- Refresher double-Meteor double-Cataclysm late-game chains
- complex Ice Wall geometry casting

## Spell Preparation Rules

Invoker's design lives or dies on deterministic invoke behavior.

### Spell-slot assumptions

The design assumes the user exposes:

- one key for Quas
- one key for Wex
- one key for Exort
- one key for Invoke
- two active-spell keys (usually `D` and `F`)

Those keys must be config-driven because the repo cannot assume every user keeps default bindings.

### Invoke planner behavior

For any requested spell, the planner should:

1. check whether the spell is already in slot D or F
2. if yes, use that slot directly
3. otherwise cast the required orb triplet
4. cast Invoke
5. detect which active slot now contains the requested spell
6. use that slot

For two-spell preparation, the planner should invoke them in order so the final D/F state is predictable. The implementation should not assume that "first invoked is always D" without checking the live post-invoke slot names.

## Upgrade and Patch Handling

Patch 7.41b-specific Scepter/Shard changes matter, but the runtime should stay honest about what it knows.

### What can be detected

- whether the hero owns Scepter
- whether the hero owns Shard

### What cannot be safely inferred

- which permanent Scepter pill was chosen
- which permanent Shard pill was chosen

Therefore:

- Cataclysm-specific behavior must be enabled by config, not by guessing from Scepter ownership
- EMP Pull behavior must be treated as a user-declared capability if the combo profile wants to rely on it
- Ice Floe geometry should not be assumed from Scepter presence alone

## Error Handling and Safety

Invoker automation should fail closed.

### Abort conditions

Abort the request and log clearly when:

- no GSI event is available
- the hero is dead
- the hero is stunned, hexed, or otherwise unable to act
- a required spell is not learned
- a required spell cannot be invoked or cast
- a required item is missing or unusable
- another Invoker request is already running and the queue policy rejects overlap

### Safety principles

- never move the mouse
- never invent a target
- never silently fall back to a different combo profile
- never overwrite spell slots outside an explicit Invoker trigger
- keep reactive behavior opt-in and narrow

## Config Surface

Add a dedicated `[heroes.invoker]` section.

Recommended first-pass fields:

- `standalone_key`
- `panic_key`
- `prep_key`
- `quas_key`
- `wex_key`
- `exort_key`
- `invoke_key`
- `spell_slot_primary_key`
- `spell_slot_secondary_key`
- `primary_profile` (`qw_pickoff` or `qe_burst`)
- `prep_profile`
- profile timing values such as tornado/EMP and meteor/blast delays
- optional profile item allowlists such as `["spirit_vessel", "rod_of_atos"]`
- optional booleans for item usage inside a combo

The config should avoid exposing every theoretical combo branch in v1. Prefer a few opinionated profiles over a combinatorial rule engine.

## Runtime Flow

### GSI event path

1. dispatcher routes Invoker events to `InvokerScript`
2. `InvokerScript` stores the latest event snapshot
3. `InvokerScript` composes shared survivability behavior

### Primary combo path

1. keyboard hotkey emits generic `ComboTrigger`
2. `main.rs` routes it to Invoker through `selected_hero`
3. `InvokerScript::handle_standalone_trigger()` builds the configured primary request
4. Invoker worker serializes the sequence
5. planner invokes missing spells as needed
6. worker emits item/spell keys with configured delays

### Panic path

1. dedicated panic hotkey is detected before generic ability handling
2. callback emits `HotkeyEvent::InvokerPanic`
3. main-thread hotkey receiver calls the Invoker panic handler
4. worker prepares and casts Ghost Walk

### Prep path

1. dedicated prep hotkey is detected
2. callback emits `HotkeyEvent::InvokerPrep`
3. Invoker prep handler builds the configured spell-pair request
4. worker invokes the missing spells without casting them

## Testing

The implementation should be verified at three levels.

### Unit tests

Add focused unit tests for:

- ability-slot lookup by spell name
- spell invoke planning when one or both spells are already active
- planner refusal when required spells are unlearned
- profile-to-step translation
- config-driven upgrade gating for Cataclysm-sensitive logic

### Integration / fixture tests

Add or update GSI fixtures that represent:

- early QW Invoker
- QE Invoker with Meteor/Blast online
- Invoker with one requested spell already invoked
- Invoker with both requested spells already invoked
- Invoker with Scepter and Shard owned

These fixtures should prove that the planner reads the six-slot GSI ability model correctly.

### Repo verification

Run the repo's normal verification set after implementation:

- `cargo test`
- `npm --prefix src-ui test`
- `cargo build --release`

## Implementation Shape

This is one coherent hero slice and should be implemented in staged order:

1. wire Invoker into hero/state/manual-selection plumbing
2. add config surface and docs
3. implement observed-state parsing and invoke planning
4. add panic + prep triggers
5. add one primary combo profile
6. add the second primary combo profile

That order delivers value early while keeping the riskiest logic isolated in one hero module.
