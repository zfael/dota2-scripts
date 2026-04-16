# Invoker Manual Step Completion Design

## Problem

The new Invoker profile builder can express ordered spell and item sequences, but every step currently completes the same way: execute the step, then sleep for `delay_after_ms`.

That works for fully automated casts, but it breaks down for manual-targeted spells such as Sun Strike:

- the runner can invoke the spell correctly
- the player still needs to aim and confirm the cast manually
- the runner has no way to know whether that manual cast actually happened
- follow-up steps can fire too early because the profile advances on a fixed delay instead of real spell usage

For QE Burst specifically, this makes `Sun Strike -> Chaos Meteor -> Deafening Blast` feel unreliable when Sun Strike is intended to be manually aimed.

## Goals

- Let individual Invoker profile steps define how completion is detected
- Support manual-targeted spell steps that pause until the spell truly enters cooldown
- Abort the remaining profile cleanly if manual confirmation never happens
- Avoid adding lag to normal non-manual profile steps
- Keep the model and UI aligned with the existing per-step editor

## Non-Goals

- No global cooldown polling for every hero or every automation path
- No auto-aim or cursor prediction for Sun Strike or any other targeted spell
- No generalized item cooldown-wait mode in this first pass
- No branching logic such as "if cooldown does not start, try another spell"

## Recommended Approach

Extend `InvokerProfileStep` with a per-step completion policy.

Most steps stay on the current fixed-delay behavior. Manual-targeted spell steps opt into a cooldown-based completion mode that waits until the spell enters cooldown before the runner advances.

This keeps the default fast path unchanged while giving mixed profiles, such as QE Burst, a way to pause only on the steps that need human confirmation.

## Alternatives Considered

### 1. Always wait for cooldown on every spell step

This would be simple to describe, but it would make fast automated combos feel sticky and would unnecessarily serialize steps that do not need confirmation.

### 2. Make manual confirmation a profile-level setting

This is easier than per-step configuration, but it is too coarse for Invoker. Real combos often mix one manual spell with several fully automated follow-ups.

## Configuration Model

Add two fields to `InvokerProfileStep`:

- `completion_mode`
- `completion_timeout_ms`

### Step Shape

Each Invoker step becomes:

- `kind`
- `target`
- `delay_after_ms`
- `completion_mode`
- `completion_timeout_ms`
- `notes`

### Completion Modes

- `fixed_delay` - current behavior; the step completes after execution plus `delay_after_ms`
- `wait_for_cooldown` - spell-only behavior; the runner waits until the target spell enters cooldown or the timeout expires

### Defaults

- Existing steps default to `completion_mode = "fixed_delay"`
- Existing steps default to `completion_timeout_ms = 3000`
- `completion_timeout_ms` is ignored when `completion_mode = "fixed_delay"`

### Initial Preset Adjustment

The seeded **QE Burst** profile should mark its **Sun Strike** step as:

- `completion_mode = "wait_for_cooldown"`
- `completion_timeout_ms = 3000`

The Meteor and Blast follow-up steps remain fixed-delay steps.

## Runtime Design

For each step, the runner now answers two separate questions:

1. How do I perform the step?
2. What counts as completion for this step?

The first answer is the existing spell/item execution logic. The second answer comes from `completion_mode`.

### `fixed_delay`

This stays exactly as it works today:

1. Execute the step
2. Sleep for `delay_after_ms`
3. Continue

### `wait_for_cooldown`

This mode applies only to spell steps.

1. Check whether the target spell is already on cooldown before attempting the step
2. If the spell is already on cooldown, log a skip and continue to the next step
3. Otherwise, invoke the spell if needed
4. Press the active spell-slot key once so the user can finish the cast naturally
5. Poll the latest Invoker GSI snapshot until the target spell reports `cooldown > 0`
6. Once cooldown starts, apply `delay_after_ms`
7. Continue to the next step
8. If the timeout expires first, abort the remaining profile

### Cooldown Detection Rule

For `wait_for_cooldown`, a step is considered complete only when the latest GSI event reports the target spell with `cooldown > 0`.

The runner does not treat keypress detection as success. Manual intent is not enough; the cast must actually be consumed.

`delay_after_ms` keeps the same meaning in both modes: it is the post-completion delay before the next step begins.

### Polling Model

The wait loop should poll the existing `INVOKER_LAST_EVENT` snapshot on a short interval of **25 ms**.

This does not add a new global worker. It only runs inside the existing dedicated Invoker request thread while a `wait_for_cooldown` step is active.

### Impact on Other Automation

Manual waiting blocks only the current Invoker profile request. It does **not** block:

- GSI ingestion
- the main app thread
- other hero automation workers

It does delay later queued Invoker profile requests until the current one finishes or aborts, which is acceptable because the queue is already FIFO and profile execution is intentionally serialized.

## Edge Cases

### Spell already on cooldown

If the step begins and the target spell is already visible in the observed GSI snapshot with `cooldown > 0`, the runner should log that the manual step is already consumed and skip directly to the next step.

### Spell cannot be observed

If the runner cannot resolve the target spell in the latest Invoker GSI snapshot when it needs cooldown confirmation, it should abort the remaining profile. Continuing would risk drifting out of sync with the actual spell state.

### Hero becomes unavailable

If the hero dies or becomes disabled while waiting for cooldown confirmation, abort the profile immediately.

### Timeout

If `completion_timeout_ms` expires before cooldown starts, abort the remaining profile and log the spell name plus timeout value.

## UI Design

The existing step editor should gain a small **Completion** section for each step.

### Step Controls

For spell steps, show:

- completion mode selector
- timeout input in milliseconds when `wait_for_cooldown` is selected

For item steps in this first pass:

- keep `completion_mode` forced to `fixed_delay`
- hide or disable cooldown-wait controls with helper text if needed

### Editor Behavior

- Switching a step to `wait_for_cooldown` should reveal the timeout field inline
- The step summary should make the mode visible so manual steps are easy to scan
- Preset steps that use cooldown waiting should render as normal editable steps, not special-cased UI

## Logging and Observability

When a step uses `wait_for_cooldown`, the runtime should log:

- that the step entered manual cooldown wait
- whether the spell was already on cooldown and skipped
- whether cooldown started successfully
- whether the wait aborted due to timeout
- whether the wait aborted because the hero became unavailable

This keeps live debugging aligned with the current Invoker logging model.

## Testing

Add targeted coverage for:

- config defaults include the new completion fields
- planner preserves completion policy on spell steps
- manual step continues when cooldown starts
- manual step skips when cooldown is already active
- manual step aborts when timeout expires
- manual step aborts if the hero becomes unavailable during the wait
- QE Burst mixed behavior: Sun Strike waits for cooldown, then Meteor and Blast continue in order
- UI editor renders completion controls and persists changes

## Implementation Notes

- The change should extend the existing Invoker profile model rather than introducing a second step type
- The wait helper should stay local to the Invoker runner
- Documentation for `docs/heroes/invoker.md` and `docs/reference/configuration.md` should be updated if this design is implemented
