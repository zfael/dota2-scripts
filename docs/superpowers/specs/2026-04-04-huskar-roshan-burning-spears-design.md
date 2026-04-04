# Huskar Roshan Burning Spears Gate Design

## Problem

The current Roshan support has two useful layers already:

1. shared Armlet Roshan mode that learns Roshan-sized hits
2. Roshan stun-recovery deferral that waits for the next real hit when recovery is survivable

Real play still exposes another failure mode for Huskar:

- Burning Spears keeps draining HP while Huskar is approaching the Armlet trigger
- after a Roshan stun, that self-damage can push Huskar too close to or below the Armlet threshold before the next Roshan decision window
- even if the Roshan stun logic defers correctly, self-inflicted HP loss can still sabotage the recovery timing

The user wants a Roshan-only Huskar follow-up that automatically toggles Burning Spears off near the Armlet trigger, then turns it back on after HP has clearly recovered from the Armlet save.

## Goals

1. Add a Huskar-only Roshan behavior that reduces self-damage from Burning Spears near the Armlet trigger.
2. Keep it scoped to **Huskar while Armlet Roshan mode is armed**.
3. Auto-disable Burning Spears before HP drifts too close to the Armlet threshold.
4. Re-enable Burning Spears only after HP has clearly recovered again from the Armlet save.
5. Preserve the existing Roshan stun-recovery logic instead of replacing it.

## Non-Goals

1. Do not make Burning Spears automation global for all Huskar play.
2. Do not make this a separate manual Huskar mode outside Roshan mode.
3. Do not replace shared Armlet decision logic with Huskar-specific boss logic.
4. Do not assume GSI exposes authoritative Burning Spears autocast state.

## Selected Approach

Use a **threshold-band spear gate**:

- while Huskar + Armlet Roshan mode are active, compute the effective Armlet trigger from the existing shared Armlet config
- add a fixed HP buffer above that trigger
- once Huskar enters that danger band, auto-send `Alt + <burning spear key>` once to disable Burning Spears
- keep Burning Spears logically off while Huskar remains in the dangerous Roshan window
- re-enable Spears only after HP has recovered above a higher re-enable line

This is preferred over a post-stun-only solution because the user wants protection any time HP gets close to the Armlet trigger during Roshan mode, not only after stuns. It is also preferred over “leave Spears off for the whole Roshan session” because that would be safer but unnecessarily blunt and hurts DPS more than needed.

## Ownership

Split the behavior across the existing boundaries:

- `src/actions/armlet.rs`
  - continues to own shared Roshan mode, learned-hit estimation, and stun-recovery defer logic
  - remains the source of the effective Armlet trigger line
- `src/actions/heroes/huskar.rs`
  - owns Huskar-specific Burning Spears toggle behavior
  - tracks whether the app disabled Spears itself
  - decides when to send `Alt + burning_spear_key`
- `src/config/settings.rs`
  - adds Huskar-only config for the Roshan spear gate
- `src-ui/src/components/heroes/configs/HuskarConfig.tsx`
  - exposes those Huskar-only settings in the existing Huskar config panel

Burning Spears automation belongs in `huskar.rs`, not `armlet.rs`, because the ability keybinding and toggle semantics are hero-specific even though the trigger band depends on shared Armlet thresholds.

## Important Data Constraint

The current GSI model exposes:

- ability name
- `ability_active`
- `can_cast`
- cooldown
- level

It does **not** expose a trustworthy “the app knows Burning Spears autocast is currently on/off” contract.

That means the app must treat Burning Spears ownership conservatively:

- if the app disables Spears, it can later re-enable Spears
- if the app did **not** disable Spears, it must not guess and flip them
- if ownership becomes ambiguous, the app should clear its tracking and stop toggling instead of trying to force the state

## Configuration

Add a new Huskar-only config block:

```toml
[heroes.huskar.roshan_spears]
enabled = false
burning_spear_key = "w"
disable_buffer_hp = 60
reenable_buffer_hp = 100
```

### Meaning

- `enabled`
  - master gate for the Roshan-only Burning Spears behavior
  - default `false` because it actively toggles an autocast ability
- `burning_spear_key`
  - the actual Burning Spears key to pair with `Alt`
  - default `"w"` to match common Dota bindings
- `disable_buffer_hp`
  - extra HP above the effective Armlet trigger where the app turns Spears off
- `reenable_buffer_hp`
  - extra HP above the effective Armlet trigger where the app may turn Spears back on

These should live under the Huskar config rather than shared Armlet config because the behavior is hero-specific.

## Runtime Behavior

### Effective thresholds

Let:

- `effective_armlet_trigger = resolved.toggle_threshold + resolved.predictive_offset`
- `spears_disable_line = effective_armlet_trigger + disable_buffer_hp`
- `spears_reenable_line = effective_armlet_trigger + reenable_buffer_hp`

The re-enable line is intentionally higher than the disable line to create hysteresis and avoid rapid on/off flapping around a single threshold.

### Disable flow

When all of the following are true:

1. current hero is Huskar
2. shared Armlet Roshan mode is armed
3. Huskar Roshan Spears config is enabled
4. Burning Spears ability is present and usable as a toggle target
5. HP is at or below `spears_disable_line`
6. the app has not already disabled Spears for this cycle

Then:

- send `Alt + burning_spear_key` once
- set `spears_disabled_by_app = true`
- keep the state latched without resending every GSI tick

### Hold-off state

While `spears_disabled_by_app = true`:

- do **not** keep spamming `Alt + key`
- do **not** re-enable just because stun ended
- keep Spears logically suppressed while Huskar remains in the low-HP Roshan window

### Re-enable flow

Only re-enable when all of the following are true:

1. `spears_disabled_by_app = true`
2. shared Armlet Roshan mode is still armed
3. Huskar Roshan Spears config is still enabled
4. HP has recovered above `spears_reenable_line`

Then:

- send `Alt + burning_spear_key` once
- clear `spears_disabled_by_app`

This matches the user’s requirement that Spears come back only after the Armlet recovery is successful and HP is back up again.

## Interaction with Roshan Stun Recovery

The two systems should cooperate like this:

- Roshan stun-recovery deferral still decides whether Armlet should toggle immediately or wait for the next hit
- the Huskar spear gate runs in parallel and reduces self-inflicted HP drift while Huskar is approaching the Armlet threshold
- spear suppression must **not** override an immediate Armlet-protection decision
- spear suppression is preventive HP management, not a replacement for Armlet timing logic

In other words:

- Armlet Roshan logic still owns “when do we save?”
- Huskar spear gating owns “stop spending HP on Spears while we are entering the save band”

## State Reset Rules

Clear `spears_disabled_by_app` and stop managing Burning Spears when:

- Roshan mode is manually disarmed
- the hero dies
- current hero is no longer Huskar
- Huskar Roshan Spears config is disabled
- Burning Spears ability is missing or unparseable

Do **not** automatically send a re-enable toggle during ambiguous resets unless the app still has high confidence it owns the off-state. The safer default is to clear tracking and stop toggling rather than guessing.

## Logging

Add clear logs for:

- entering the spear-off danger band
- disabling Burning Spears due to Roshan threshold protection
- re-enabling Burning Spears after HP recovery
- clearing Burning Spears ownership state without toggling

These logs are important because this feature combines inferred Roshan timing with inferred Burning Spears ownership and will need real-play tuning.

## UI

Extend the Huskar config panel with a Roshan Spears section that exposes:

- enable toggle
- Burning Spears key
- disable buffer HP
- re-enable buffer HP

This should remain in the Huskar page, not the shared Armlet page, because the feature is hero-specific.

## Testing Strategy

### Huskar unit tests

Add tests in `src/actions/heroes/huskar.rs` or nearby pure helpers for:

1. disabling Spears when HP enters the disable band
2. not repeatedly resending the disable toggle every tick
3. not re-enabling until HP rises above the re-enable line
4. only re-enabling if the app disabled Spears itself
5. clearing ownership state on death/disarm/hero change/feature-off

### Armlet/Roshan regression tests

Keep the current Roshan stun-recovery tests passing and add any focused regression test needed to show the spear gate does not break the defer-and-resync flow.

### Frontend/config tests

If the Huskar UI exposes the new config, add or extend tests to verify the new Huskar Roshan Spears settings persist through the existing config store path.

## Documentation Updates Required

When implemented, update:

- `docs/heroes/huskar.md`
- `docs/reference/configuration.md`
- any shared survivability/Armlet docs that mention Huskar Roshan tuning

## Recommended Implementation Order

1. Add Huskar Roshan Spears config structs/defaults and checked-in config entries.
2. Add Huskar-only spear gate runtime state and pure helper functions.
3. Wire the helper into `HuskarScript::handle_gsi_event(...)`.
4. Add Huskar tests for disable/re-enable/ownership/reset behavior.
5. Expose the config in the React Huskar panel.
6. Update docs.

## Summary

The recommended design adds a **Roshan-only Huskar Burning Spears threshold-band gate**:

- turn Spears off before HP drifts too close to the effective Armlet trigger
- keep Spears off through the dangerous Roshan window
- only turn Spears back on after HP has clearly recovered again

That directly addresses the remaining self-damage problem without broadening Huskar automation outside Roshan mode.
