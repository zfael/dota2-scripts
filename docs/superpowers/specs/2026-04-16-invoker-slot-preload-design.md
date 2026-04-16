# Invoker Slot Preload Design

## Problem

The current Invoker profile runner still has a slot-order bug in its runtime
model.

Today, the code assumes that when a new spell is invoked:

- the newly invoked spell lands on the secondary slot
- the previous slot state shifts in the opposite direction

That is the reverse of the behavior described by the user and observed in live
play:

- newly invoked spell lands on the primary slot (`D`)
- the previous primary spell shifts to the secondary slot (`F`)

This mismatch makes ordered combos unreliable. A profile such as
`Tornado -> EMP` can still feel inverted or stale even when the declared order
is correct.

At the same time, the current step-by-step planner does not preload spell pairs.
The user wants combo execution to feel more natural by preparing the next two
spell steps up front and then advancing through the combo using the real Invoker
slot rules.

## Goals

- Correct the runtime's `D`/`F` slot model so it matches real Invoker invoke
  behavior
- Preload up to two spell steps at combo start
- Execute preloaded spell pairs in a way that matches actual slot occupancy
- Keep manual cooldown-wait behavior compatible with the preload model
- Add regression coverage so ordered profiles such as `Tornado -> EMP` cannot
  silently invert again

## Non-Goals

- No new general-purpose combo DSL
- No auto-targeting or cursor prediction
- No cooldown-aware item preload logic
- No attempt to force every current spell onto `D` if that requires extra
  re-invokes

## Recommended Approach

Replace the current single-step Invoker spell planner with a
**pair-aware preload runner** that uses actual slot semantics.

The runner should:

1. Preload up to two pending spell steps in profile order
2. Track where those spells actually land in `D` and `F`
3. Execute the loaded pair oldest-to-newest
4. Refill the queue only after the current step is truly complete

This keeps behavior aligned with live Invoker slot mechanics instead of forcing
an artificial "current spell is always `D`" rule.

## Alternatives Considered

### 1. Fix the existing slot model only

This would correct the obvious bug, but it would still keep the current
step-by-step feel and would not deliver the smoother preload behavior the user
wants.

### 2. Force the current spell onto `D`

This would require invoking in reverse order or re-invoking more aggressively to
keep the current cast on the primary slot. It is more complex and less faithful
to how the actual invoke pipeline behaves.

## Execution Model

### Core Slot Semantics

The design should treat slot meanings explicitly:

- `primary_slot` = `D`
- `secondary_slot` = `F`
- invoking a new spell places that spell into `D`
- the previous `D` spell shifts to `F`
- the previous `F` spell is discarded

This means that when two spells are preloaded in **profile order**, the first
prepared spell naturally ends up on `F` and the second prepared spell ends up on
`D`.

### Preload and Cast Order

For spell steps, the runner should preload in declared order and cast based on
actual resulting slot positions.

For example, for:

`Tornado -> EMP -> Sun Strike`

the spell flow should be:

1. Invoke `Tornado` -> `D = Tornado`
2. Invoke `EMP` -> `D = EMP`, `F = Tornado`
3. Cast `Tornado` from `F`
4. Cast `EMP` from `D`
5. Invoke `Sun Strike` -> `D = Sun Strike`
6. Cast `Sun Strike` from `D`

So the rule is:

- when two spells are loaded, cast **`F` first, then `D`**
- when one spell is loaded, cast its actual slot

### Why Not Force `D` First

If the runner tries to keep the current spell on `D` at all times, it either has
to:

- invoke spells in reverse preparation order, or
- re-invoke more often as the combo advances

That adds complexity and can introduce extra delay. The recommended model keeps
the runner close to real slot behavior instead.

### Manual Step Interaction

Manual-targeted steps should continue to use the existing cooldown confirmation
idea, but now within the preload model.

If a manual step is the next spell to cast:

1. preload the pair in profile order
2. cast the manual step from its actual slot
3. wait for cooldown confirmation
4. only then continue with refill and remaining execution

For example, if a manual spell is the oldest loaded spell, it may be on `F`.
That is acceptable. The runner should press the actual slot for the pending
profile step rather than trying to force it onto `D`.

## Components and Boundaries

### `src/actions/heroes/invoker.rs`

This remains the primary file for the change.

Its responsibilities should become:

- observe current `D`/`F` state from GSI
- prepare up to two pending spell steps
- model how invokes mutate slot state
- execute the next declared spell from the slot it actually occupies
- handle manual cooldown waits before refill continues

### No New UI Surface Required for v1

The profile builder already expresses spell order. The preload change is a
runtime behavior correction and redesign, not a new authoring feature.

Docs may mention that ordered spell pairs are now preloaded and executed using
real slot occupancy, but no new profile field is required for this first pass.

## Error Handling

The runner should abort the remaining profile when its internal expectations and
observed GSI state drift too far apart to trust execution.

Abort conditions should include:

- expected next spell is not observed in either slot after preload
- manual step never enters cooldown before timeout
- hero dies or becomes disabled during a manual wait

Skip behavior should remain:

- if a manual step is already on cooldown before its turn, skip it and continue

## Testing

Add targeted regression coverage for:

- newly invoked spell lands on `D`, previous `D` shifts to `F`
- preloading two spells in profile order results in cast order `F` then `D`
- `Tornado -> EMP` cannot invert even when the pair is preloaded
- trailing final spell naturally lands on `D`
- manual cooldown-wait still works when the manual step is cast from its actual
  loaded slot
- stale slot assumptions from the old reversed model are removed from tests

## Documentation Impact

If implemented, update:

- `docs/heroes/invoker.md`
- `docs/reference/configuration.md` only if any config wording needs to clarify
  runtime preload behavior

The docs should explain that spell profiles are still authored in natural cast
order, but the runtime may preload them into `F` then `D` because that is how
Invoked spell slots actually rotate.
