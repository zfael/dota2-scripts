# Phase Boots Movement Automation Design

## Problem

The runtime already automates several shared item behaviors:

- danger-driven defensive items
- danger-driven neutral items
- low-mana restoration items
- hotkey-driven combo items

It does **not** automate movement-speed items based on actual pathing.

For Phase Boots, that creates a gap between player intent and shared automation:

- the player wants Phase Boots to fire while the hero is truly traveling
- the trigger must **not** fire when the hero is functionally stationary, such as farming in place
- the repo already models `hero.xpos` / `hero.ypos`, but current runtime logic does not consume those fields

The design needs a shared movement-aware automation path that uses real position changes instead of guessing from combat activity or keyboard intent.

## Goals

1. Add shared Phase Boots automation that triggers from real movement rather than hero-specific logic.
2. Trigger only when the hero's position changes by a meaningful amount between GSI events.
3. Avoid false triggers from tiny coordinate jitter, turning in place, or farming while stationary.
4. Allow re-triggering during continued travel once Phase Boots becomes castable again.
5. Keep the behavior centralized in the dispatcher/common shared automation path.

## Non-Goals

1. Build a general movement-intent AI for every mobility item in the game.
2. Infer pathing from mouse clicks, attack state, or animation state.
3. Move this behavior into individual hero scripts.
4. Change Space+right-click combo logic, Soul Ring interception, or other keyboard-driven systems.

## Selected Approach

Use a new shared **movement-triggered automation lane** in `src/actions/common.rs`.

This lane will:

- classify `item_phase_boots` as a supported shared automation item
- compare the current GSI position sample to the previous sample
- treat the hero as walking only when the distance moved clears a configurable minimum threshold
- fire Phase Boots through the same shared item execution pattern already used by other dispatcher-owned automation

This is preferred over a hero-script solution or a standalone Phase Boots module because it matches the repo's established design: shared item behaviors live in shared automation layers, remain config-driven, and use short local lockouts to prevent duplicate bursts.

## Current State

### Position data exists but is unused

`src/models/gsi_event.rs` already includes:

- `hero.xpos`
- `hero.ypos`

The current GSI usage docs explicitly note that positions are modeled but not consumed by runtime logic.

### Shared item automation already has the right extension points

The repo already contains shared automation patterns that fit this feature:

- `src/actions/item_automation.rs` defines shared trigger families, cast modes, support status, and a global short lockout mechanism
- `src/actions/common.rs` owns shared trigger evaluators such as low-mana and danger automation
- `src/actions/dispatcher.rs` runs shared automation before hero routing

Phase Boots also already exists as a known item name in item metadata, and Soul Ring explicitly treats it as a non-mana item that should not participate in mana-cost interception.

## Behavior

After the change, Phase Boots automation should follow these rules:

1. The first valid GSI sample for a hero only records the current position. It does not trigger anything.
2. On later GSI samples, calculate the travel distance between the current sample and the previous sample.
3. Treat the hero as **walking** only when that distance is at least the configured minimum threshold.
4. Only trigger Phase Boots when all of the following are true:
   - the feature is enabled
   - the hero is alive
   - the hero is not excluded by config
   - a previous position sample exists
   - the movement threshold is crossed
   - `item_phase_boots` exists in an inventory slot
   - `item.can_cast == Some(true)`
   - the shared short movement trigger lockout is clear
5. Update the cached position after evaluation so continued travel can be measured on the next event.

Expected user-visible behavior:

- moving across the map triggers Phase Boots automatically
- attacking or farming in place does not trigger it
- tiny movement jitter does not trigger it
- if the hero keeps running and Phase Boots becomes ready again later, it can trigger again

## Proposed Architecture

### 1. Extend shared item automation metadata

Add a `Movement` variant to `TriggerFamily` in `src/actions/item_automation.rs`.

Add a supported shared automation spec for:

- `item_phase_boots`

The spec should use:

- trigger family: `Movement`
- cast mode: `NoTarget`
- support: `Supported`
- neutral flag: `false`

This keeps movement-triggered items inside the same shared registry used by other automation families.

### 2. Add a small shared movement snapshot

Introduce a minimal movement snapshot structure in shared automation infrastructure, not in hero scripts.

It only needs to store:

- hero name
- alive/dead state
- previous `xpos`
- previous `ypos`

Because GSI currently represents the local controlled hero only, one shared snapshot is sufficient. The snapshot should reset when:

- the hero is dead
- the hero identity changes

This keeps movement classification isolated and avoids bloating hero files with repeated coordinate comparison logic.

### 3. Add a shared movement evaluator in `common.rs`

Add a shared evaluator such as:

- `eligible_movement_item(...)`
- `check_and_use_movement_items(...)`

Responsibilities:

- load `phase_boots_automation` config
- read the previous movement snapshot
- compute distance moved
- apply the minimum-distance gate
- confirm Phase Boots presence and castability
- acquire the shared short lockout
- plan and enqueue the no-target key sequence
- update the movement snapshot for the next event

The evaluator should stay in `common.rs` because this repo already places dispatcher-owned shared trigger evaluation there.

### 4. Dispatcher ownership

Call the new movement automation lane from `src/actions/dispatcher.rs` after shared low-mana automation and before hero-specific routing.

That preserves the current high-level order:

1. Armlet
2. neutral discovery logging
3. silence dispel
4. low-mana shared automation
5. movement shared automation
6. hero routing / fallback survivability

This keeps Phase Boots global without changing how hero scripts compose shared survivability helpers.

## Runtime Flow

1. GSI event arrives and normal handler-side cache refresh continues unchanged.
2. Dispatcher runs armlet, neutral discovery logging, and silence dispel exactly as today.
3. Dispatcher runs low-mana shared automation exactly as today.
4. Dispatcher runs movement shared automation:
   - read current hero position
   - compare to previous snapshot
   - classify movement against `minimum_distance_units`
   - if the hero is truly walking and Phase Boots is eligible, enqueue a single no-target item-key press
   - store the current position as the new baseline
5. Dispatcher routes to the hero script or fallback shared survivability path.

The movement path should use the shared executor-backed item sequence pattern so input remains serialized with other shared automation work.

## Config Direction

Add a dedicated config section:

```toml
[phase_boots_automation]
enabled = true
minimum_distance_units = 100
excluded_heroes = []
```

### Why a dedicated section

Although the trigger family becomes part of the shared item automation model, the operator-facing config should stay specific to the actual feature being shipped.

This avoids over-generalizing the configuration before there are more movement-triggered items to support.

### Default values

- `enabled = true`
- `minimum_distance_units = 100`
- `excluded_heroes = []`

`100` is the chosen default for implementation. It is large enough to ignore small positional noise while still responding quickly to real travel between GSI updates.

## Safety and Error Handling

The movement path should fail closed:

- if there is no previous sample, record state and do nothing
- if the hero is dead, clear/reset the snapshot and do nothing
- if Phase Boots is absent or not castable, do not trigger
- if the movement distance is below threshold, do not trigger
- if the lockout is active, do not trigger

The short lockout is only a duplicate-burst guard. It must not replace the real item castability check.

No broad fallback behavior is needed. When the movement signal is ambiguous, the runtime should simply avoid firing.

## Testing Strategy

Add focused unit coverage around movement gating and shared automation eligibility.

Required cases:

1. first movement sample does not trigger
2. zero-distance movement does not trigger
3. sub-threshold movement does not trigger
4. threshold-crossing movement with castable Phase Boots is eligible
5. dead heroes do not trigger and reset snapshot state
6. excluded heroes do not trigger
7. missing or non-castable Phase Boots does not trigger
8. continued travel can re-trigger after the lockout clears and item readiness returns

Tests should stay at the helper/evaluator level rather than trying to assert full OS-level input emission timing.

## Documentation Impact

If this design is implemented, update:

- `docs/features/survivability.md`
- `docs/reference/configuration.md`
- `docs/reference/gsi-schema-and-usage.md`

The GSI schema docs must stop describing positions as modeled-but-unused and instead document that `hero.xpos` / `hero.ypos` now drive shared Phase Boots movement automation.

## Summary

The recommended design adds Phase Boots as a new **shared movement-triggered automation** feature.

It reuses the repo's existing shared item-automation patterns while adding one missing input signal: actual position change over time.

That delivers the requested behavior:

- trigger while truly walking
- do not trigger while farming in place
- ignore tiny jitter
- re-trigger during long travel when the item becomes ready again
