# Armlet Roshan Stun-Recovery Design

## Problem

The first Roshan-mode pass improved shared Armlet behavior by learning Roshan-sized HP drops and triggering Armlet from a learned lethal zone. In real play, one failure mode remains:

1. Roshan stuns the hero.
2. Armlet cannot toggle during the stun.
3. The stun ends.
4. The current Roshan logic can immediately decide to toggle from the already-dangerous HP state.
5. That recovery toggle can land in the same window as Roshan's next hit, which defeats the timing benefit and can still get the hero killed.

The user feedback points to a better outcome: if the hero can safely survive one more Roshan hit after stun recovery, do **not** toggle immediately on recovery. Instead, wait for the next Roshan-sized hit, then toggle to resynchronize Armlet with Roshan's cadence.

## Goals

1. Make Roshan mode aware of stun recovery timing.
2. Avoid immediate post-stun Roshan toggles when the hero can safely tank one more learned Roshan hit.
3. Re-sync Armlet timing by waiting for the next Roshan-sized hit after stun recovery, then toggling.
4. Preserve the current emergency behavior when the hero cannot safely survive another learned Roshan hit.
5. Keep this as a runtime Armlet improvement without adding new UI or config unless strictly necessary.

## Non-Goals

1. Do not add exact Roshan attack-timer prediction.
2. Do not add a new user-facing Roshan stun tuning panel for this pass.
3. Do not replace the existing cooldown, stun, or critical-retry rules globally.
4. Do not build a full boss-fight state machine beyond what is needed for post-stun re-sync.

## Approaches Considered

### 1. Fixed post-stun timer

After stun recovery, suppress Roshan toggles for a short fixed delay.

**Pros**
- Smallest implementation
- Easy to reason about

**Cons**
- Brittle against Roshan attack-speed changes and GSI cadence
- Can still miss the desired post-hit re-sync window
- Adds another magic timing constant with no strong data source

### 2. Post-stun next-hit deferral

When stun ends, use the best current Roshan-hit estimate. If the hero can survive one more Roshan hit, enter a short-lived **wait-for-next-hit** mode. In that mode, Roshan toggles are deferred until the next valid Roshan-sized hit sample is observed, then Armlet toggles immediately after that hit.

**Pros**
- Matches the user's requested behavior
- Re-syncs from observed Roshan hits instead of guessing with a timer
- Works with the current HP-delta-based Roshan model

**Cons**
- Adds a small amount of Armlet runtime state
- Needs careful reset behavior so defer state does not leak across unrelated events

### 3. Full Roshan cadence tracker

Track hit phases, stun phases, hit counts, and recovery windows as a larger internal state machine.

**Pros**
- Most expressive long-term model
- Leaves room for future Roshan-specific behavior

**Cons**
- Higher complexity than the problem currently needs
- Harder to verify with the repo's limited GSI signal quality
- More likely to overfit to one fight pattern

## Selected Approach

Use **post-stun next-hit deferral**.

This is the best fit for the current runtime model because the repo still only knows about Roshan indirectly through observed HP drops. A fixed timer would guess at cadence. A full cadence tracker would be more complex than the current problem needs. Deferring to the next confirmed Roshan-sized hit keeps the design anchored in actual observed damage while solving the specific "toggle on recovery into the same swing" failure mode.

## High-Level Behavior

Roshan mode gains a small post-stun recovery flow:

1. Detect when the hero becomes stunned while Roshan mode is armed.
2. Detect the transition from stunned -> not stunned.
3. On recovery, calculate the best current Roshan hit estimate:
   - prefer the learned largest recent hit when confidence exists
   - otherwise fall back to the latest observed Roshan-sized hit if available
4. If there is no usable Roshan hit estimate, keep current Roshan behavior.
5. If there is an estimate and the hero **cannot** safely survive one more learned Roshan hit, keep the current immediate-protection behavior.
6. If there is an estimate and the hero **can** survive one more learned Roshan hit, enter **wait-for-next-hit** mode.
7. While in wait-for-next-hit mode, ignore immediate post-stun Roshan toggles based only on stale pre-recovery danger.
8. When the next valid Roshan-sized hit is observed after recovery, toggle Armlet immediately after that hit to re-sync.

## Safety Rule

The key branching rule is:

- if `health <= predicted_hit + emergency_margin_hp`, the hero is still in immediate danger and Roshan mode may toggle as soon as allowed
- if `health > predicted_hit + emergency_margin_hp`, the hero is treated as able to survive one more learned Roshan hit, so Roshan mode should defer and wait for the next valid hit sample before toggling

This reuses the existing Roshan lethal-zone concept instead of adding a new config knob.

## Runtime State Changes

Extend the Armlet Roshan runtime state with recovery-specific fields:

- `was_stunned_last_tick`
- `awaiting_post_stun_hit`
- `stun_recovery_estimate_damage`
- `stun_recovery_started_at_ms` or equivalent freshness marker if needed for diagnostics

These fields stay in `src/actions/armlet.rs` with the existing Roshan learning state.

### State transitions

#### Entering stun

When Roshan mode is armed and the hero becomes stunned:

- mark `was_stunned_last_tick = true`
- clear any old `awaiting_post_stun_hit` state from earlier fights

#### Recovering from stun

When the next GSI event shows the hero is no longer stunned after having been stunned:

1. compute the best current Roshan hit estimate
2. if no estimate exists, do nothing special
3. if the hero is still in immediate Roshan lethal range, do not defer
4. otherwise set:
   - `awaiting_post_stun_hit = true`
   - `stun_recovery_estimate_damage = predicted_hit`

#### Re-sync hit observed

While `awaiting_post_stun_hit = true`:

- only a **new valid Roshan-sized downward HP sample** can end the defer state
- that first valid post-recovery hit should trigger a dedicated Roshan re-sync toggle decision
- after that, clear the defer state and return to normal Roshan behavior

#### Reset cases

Clear the defer state when:

- Roshan mode is disarmed
- the hero dies
- Armlet is absent
- shared Armlet is disabled
- Roshan learning goes stale
- another stun starts before the awaited post-recovery hit arrives

## Decision Ladder

The Roshan decision order becomes:

1. Run the existing Armlet decision first.
2. If existing logic already wants `Toggle` or `CriticalRetry`, keep it.
3. If Roshan mode is off, stop.
4. If the hero is currently stunned, do not Roshan-toggle.
5. If the hero just recovered from stun:
   - if still immediately lethal, allow current Roshan behavior
   - if survivable for one more learned hit, enter defer mode and do **not** toggle yet
6. If defer mode is active:
   - wait only for the next valid Roshan-sized hit sample
   - ignore stale "already low HP" Roshan triggers until that hit arrives
7. On the first valid post-recovery hit:
   - trigger a dedicated Roshan re-sync toggle
   - clear defer mode
8. Resume normal Roshan learning / lethal-zone behavior after re-sync

## Logging

Add Armlet debug/info logs for:

- entering Roshan stun tracking
- entering post-stun defer mode and why
- skipping immediate post-stun toggle because one more hit is survivable
- firing the post-stun re-sync toggle on the next observed Roshan-sized hit
- clearing defer state and why

These logs are important because the user will likely need to field-test whether the defer heuristic feels right.

## Testing Strategy

Add focused Armlet tests for:

1. entering defer mode on stun recovery when health can survive one more learned Roshan hit
2. not entering defer mode when health is already inside the learned lethal zone
3. ignoring small HP drops while waiting for the post-stun hit
4. firing a dedicated Roshan re-sync toggle on the first valid Roshan-sized hit after recovery
5. clearing defer state on death, disarm, stale reset, or another stun
6. preserving existing Armlet decisions outside the post-stun flow

No frontend or config tests are required for this pass unless the implementation ends up exposing new runtime state to the UI.

## Implementation Notes

This follow-up should stay behavior-focused:

- **no new config by default**
- **no new Armlet page controls by default**
- reuse the existing Roshan estimate and `emergency_margin_hp`
- add only the minimum runtime state needed to distinguish "danger now" from "wait for the next post-stun hit"

If later testing shows the heuristic needs user tuning, a future pass can add a dedicated post-stun buffer config. That is intentionally out of scope for this change.

## Summary

The recommended change is to make Roshan mode **stun-recovery aware** by adding a small defer-and-resync flow:

- if recovering from stun is still immediately lethal, protect now
- if one more learned Roshan hit is survivable, wait for that hit
- toggle immediately after the next confirmed Roshan-sized hit to re-sync Armlet timing

That directly addresses the observed failure case without overcomplicating the Roshan system or expanding the UI/config surface.
