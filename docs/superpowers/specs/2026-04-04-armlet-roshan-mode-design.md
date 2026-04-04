# Armlet Roshan Mode Design

## Problem

The current shared armlet automation is driven by a generic low-HP threshold:

- require shared armlet automation to be enabled
- require the hero to be alive
- require Armlet of Mordiggian to be equipped
- require current HP to be below `toggle_threshold + predictive_offset`
- respect stun-state and toggle cooldown rules

That works for general survivability, but Roshan is a special case. The player wants an extra mode they can arm before doing Roshan so armlet can react to Roshan-sized hits more safely than the normal generic threshold.

The repo does not currently receive authoritative "Roshan is attacking" or "Roshan hit you" events from GSI. It only sees periodic hero state snapshots such as current HP. That means a Roshan-focused design must work from observed HP drops, not from exact pre-hit timing.

## Goals

1. Add a shared Roshan-specific armlet option that is default-off.
2. Let the player toggle Roshan mode on or off directly in-game with a dedicated hotkey.
3. Persist the Roshan feature settings and hotkey through the existing config flow.
4. Expose the Roshan feature and hotkey in the UI.
5. Add Roshan-specific armlet logic that estimates dangerous incoming Roshan hits from recent observed HP drops.
6. Keep existing shared armlet behavior intact outside Roshan mode.

## Non-Goals

1. Do not build full boss-fight inference that claims exact Roshan attack timing.
2. Do not rely on new GSI fields or external game-memory reads.
3. Do not replace the current armlet cooldown, stun, or critical-retry logic.
4. Do not make the first version hero-specific; this remains a shared armlet feature for any supported armlet hero.

## Selected Approach

Use a **manually armed Roshan mode** layered on top of the existing shared armlet automation.

When Roshan mode is armed, the armlet module continues to run the current low-HP threshold logic, but also keeps a short rolling memory of suspicious recent HP drops. Those drops are treated as Roshan-like hit samples. The module derives a conservative next-hit estimate from those samples and triggers armlet when current HP falls into a predicted lethal zone.

This approach is preferred because it matches the real data available in this repo:

- it does not pretend the app knows Roshan's exact attack windup
- it improves protection after the first observed dangerous hit
- it remains additive to current armlet behavior instead of replacing it
- it keeps ownership inside the existing shared armlet system

## Current State

### Shared armlet automation

`src/actions/armlet.rs::maybe_toggle(...)` currently resolves effective armlet config and evaluates:

- current HP
- `toggle_threshold`
- `predictive_offset`
- stun state
- elapsed cooldown since the last toggle
- critical retry state

It then either:

- toggles
- forces a critical retry
- skips because HP is safe
- skips because the hero is stunned
- skips because the toggle is still on cooldown

### Existing runtime boundaries

The relevant boundaries already exist:

- `src/actions/armlet.rs` owns armlet-specific runtime state such as last-toggle time and critical retry tracking
- `src/input/keyboard.rs` owns global hotkey interception and a `KeyboardSnapshot`
- `src-ui/src/pages/Armlet.tsx` already owns the shared armlet settings UI
- `src-ui/src/stores/configStore.ts` already persists config section updates through Tauri `invoke(...)`

These patterns support adding one more armlet-owned runtime mode plus one more keyboard hotkey without reshaping unrelated systems.

## Configuration

Extend shared armlet config with Roshan-specific fields, defaulting the feature off.

Recommended shape:

```toml
[armlet]
enabled = true
cast_modifier = "Alt"
toggle_threshold = 320
predictive_offset = 30
toggle_cooldown_ms = 250

[armlet.roshan]
enabled = false
toggle_key = "Insert"
emergency_margin_hp = 60
learning_window_ms = 5000
min_confidence_hits = 2
min_sample_damage = 80
stale_reset_ms = 6000
```

Direction notes:

- `enabled` is the persisted feature gate
- `toggle_key` is the in-game hotkey used to arm or disarm Roshan mode at runtime
- `emergency_margin_hp` is extra HP padding above the learned hit estimate
- `learning_window_ms` limits how far back Roshan samples remain relevant
- `min_confidence_hits` controls when the learned estimate is trusted as more than a first-hit fallback
- `min_sample_damage` filters out small unrelated HP changes
- `stale_reset_ms` clears old learning data when Roshan interaction pauses

These values should live under `[armlet]` rather than `[keybindings]` because they are feature-specific and only matter to the armlet subsystem.

## UI and Runtime Controls

### Armlet page

Extend `src-ui/src/pages/Armlet.tsx` with a Roshan section that exposes:

- Roshan feature enable toggle
- Roshan toggle key input
- Roshan estimator tuning inputs
- live Roshan mode status (`armed` / `disarmed`)
- optional learning status (`learning` / `confident`)

### Persistence

The frontend should persist Roshan config through the existing shared config store by updating the `armlet` section through the existing debounced Tauri command flow.

### In-game toggle behavior

The Roshan hotkey should be intercepted by the keyboard hook and **blocked** from reaching Dota 2. Pressing it should only toggle Roshan mode inside the app; it should not also cast or trigger anything in-game.

## Proposed Architecture

### 1. Armlet-owned Roshan runtime state

Keep Roshan-mode runtime state inside `src/actions/armlet.rs`, alongside the current armlet timing state.

The Roshan runtime state should own:

- whether Roshan mode is currently armed
- recent suspected-hit samples
- the currently learned Roshan hit estimate
- timestamps for freshness / stale reset

This keeps high-frequency decision logic close to the armlet evaluator and avoids threading Roshan-specific state through unrelated hero or survivability layers.

### 2. Config structs and frontend types

Extend:

- `src/config/settings.rs`
- `config/config.toml`
- frontend config types used by `src-ui`

The new config must remain shared, not hero-specific.

### 3. Keyboard integration

Add Roshan toggle support to `KeyboardSnapshot` and the global hotkey decision path in `src/input/keyboard.rs`.

Recommended shape:

- parse the configured Roshan toggle key from armlet config
- add a new hotkey event such as `HotkeyEvent::ArmletRoshanToggle`
- block the original key when the event fires

### 4. Backend/UI runtime bridge

Add a small runtime bridge so the React UI can:

- read whether Roshan mode is currently armed
- toggle it directly if needed
- reflect hotkey-driven changes in the UI

This should follow the same style as the current Tauri runtime-state bridge rather than inventing an unrelated persistence path.

## Roshan Estimator Behavior

### Sampling rules

Roshan sample collection is active only while:

- shared armlet automation is enabled
- armlet Roshan support is enabled in config
- Roshan mode is currently armed
- the hero is alive
- Armlet is present

For each GSI event:

1. compare current HP to the prior live HP seen while Roshan mode was armed
2. only consider **downward** HP deltas
3. ignore deltas below `min_sample_damage`
4. keep only samples inside `learning_window_ms`
5. clear samples entirely when data becomes stale for longer than `stale_reset_ms`

### Learned hit estimate

Use a conservative estimate for the next Roshan hit.

The recommended first-pass rule is:

- once confidence is high enough, set the learned hit estimate to the **largest recent valid sample**

Using the largest recent valid sample is intentionally safer than averaging. This feature is meant to avoid dying to Roshan, so overestimating slightly is better than underestimating.

### Decision ladder

Roshan mode is additive. The evaluator should apply this order:

1. Evaluate the existing shared armlet logic exactly as it works today.
2. If existing logic already says toggle or critical retry, keep that result.
3. If Roshan mode is not armed, stop there.
4. If Roshan mode is armed but confidence is still low, use the first-hit emergency fallback:
   - after a suspicious HP drop, if current HP is at or below `observed_drop + emergency_margin_hp`, trigger armlet
5. If Roshan mode is armed and confidence is high enough, compute:
   - `predicted_lethal_zone = learned_hit_estimate + emergency_margin_hp`
   - if current HP is at or below that zone, trigger armlet

This lets the feature react aggressively even before it has many samples, then become more stable once it has learned Roshan-sized damage.

## Runtime Flow

1. User enables the Roshan armlet feature in the Armlet page and chooses a toggle hotkey.
2. Frontend persists the Roshan settings through the existing `armlet` config update path.
3. User presses the Roshan hotkey during the match.
4. `src/input/keyboard.rs` blocks the original key and emits `HotkeyEvent::ArmletRoshanToggle`.
5. Backend flips Roshan mode in armlet runtime state and notifies the UI runtime bridge.
6. On later GSI ticks, `src/actions/armlet.rs::maybe_toggle(...)` reads:
   - current shared armlet config
   - current Roshan runtime mode
   - recent Roshan sample state
7. The evaluator applies:
   - existing armlet decision rules first
   - Roshan emergency fallback or learned-hit protection second
8. When Roshan mode is turned off, or the hero dies, Roshan learning state resets.

## Safety and Failure Behavior

### Invalid or missing hotkey

If the configured Roshan toggle key cannot be parsed, the backend should treat the hotkey as unavailable and expose that clearly in the UI instead of silently pretending the feature is armed through keyboard input.

### Weak confidence

If Roshan mode is armed but the estimator does not yet have enough valid recent samples, the runtime should degrade to:

- normal shared armlet behavior
- plus the first-hit emergency fallback

### State reset

Roshan learning state should reset when:

- Roshan mode is manually turned off
- the hero dies
- Armlet is not equipped anymore
- the feature is disabled in config
- the sample window becomes stale

### Logging

Add logs for:

- Roshan mode armed / disarmed transitions
- Roshan sample capture decisions
- learned-hit estimate updates
- Roshan-triggered armlet decisions and why they fired

Those logs are important because this feature is heuristic and will need real-play tuning.

## Important Limitation

This design cannot guarantee survival against an unknown very first lethal Roshan hit. The repo does not receive authoritative pre-hit Roshan attack data, so the feature cannot know the exact first hit before it lands.

The goal of Roshan mode is narrower and realistic:

- react aggressively after the first dangerous observed hit
- learn Roshan-sized damage quickly
- protect more reliably against subsequent Roshan hits than the current generic low-HP threshold alone

## Testing Strategy

Add or extend tests in three layers.

### Armlet unit tests

Cover:

1. valid Roshan sample collection from HP drops
2. small-delta filtering by `min_sample_damage`
3. rolling-window pruning
4. stale-state reset
5. learned-hit estimate choosing the largest valid recent sample
6. first-hit emergency fallback
7. Roshan-triggered decision coexisting correctly with cooldown and critical-retry rules
8. Roshan state reset on death / disable / mode-off

### Keyboard/runtime tests

Cover:

1. Roshan toggle key parsing in `KeyboardSnapshot`
2. Roshan hotkey event planning
3. blocked-key behavior for the Roshan toggle
4. runtime Roshan mode flipping on and off correctly

### Frontend/store tests

Cover:

1. Armlet page rendering Roshan controls
2. Roshan keybind edits updating the config store correctly
3. Roshan config persistence using the existing debounced `update_config` path
4. live Roshan mode status updating from backend runtime state

## Documentation Updates Required

When implemented, update:

- `docs/features/survivability.md`
- `docs/reference/configuration.md`
- `docs/features/keyboard-interception.md`
- any armlet-specific UI docs or migration notes that describe the shared armlet page

## Recommended Implementation Order

1. Add Roshan config structs/defaults and checked-in config entries.
2. Add armlet-owned Roshan runtime state and evaluator helpers.
3. Extend keyboard snapshot and hotkey routing with the Roshan toggle event.
4. Add backend/UI runtime bridge for Roshan mode status.
5. Extend the React Armlet page with Roshan controls and live status.
6. Add unit and store tests.
7. Update docs.

## Summary

The recommended design keeps Roshan support as a **shared armlet extension**, not a hero-specific special case.

It does that by combining:

- a default-off persisted Roshan feature gate
- a blocked in-game hotkey that arms or disarms Roshan mode
- armlet-owned Roshan runtime state
- a conservative damage estimator learned from recent HP drops
- an emergency first-hit fallback for high-risk post-hit states

That is the best fit for the repo's current GSI limits and shared survivability architecture.
