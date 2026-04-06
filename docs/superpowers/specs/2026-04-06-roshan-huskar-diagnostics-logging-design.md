# Roshan / Huskar Diagnostics Logging Design

## Problem

The current Roshan Armlet and Huskar Burning Spears automation already logs major actions, but it does not log enough of the branch decisions and reset reasons to explain why Huskar still dies in some Roshan fights. When the user reports "I still died after stun," the current logs do not make it easy to tell whether the failure came from cooldown, stun timing, learned-hit confidence, deferred recovery, state reset, or Burning Spears gating.

## Goals

1. Add temporary high-signal diagnostics for Roshan Armlet and Huskar Roshan Spears behavior.
2. Capture enough runtime detail to explain both actions and no-op decisions during live Roshan tests.
3. Keep the implementation scoped to logging and observability only; do not change combat behavior yet.
4. Produce logs that are readable enough for short manual test sessions and detailed enough to guide the next implementation pass.

## Non-Goals

1. Do not retune Roshan thresholds, hysteresis, or timing in this change.
2. Do not add a new telemetry backend, file format, or persistent session recorder.
3. Do not introduce a new config surface unless the logging change proves it is necessary later.
4. Do not keep the final verbosity level fixed forever; this pass is intentionally temporary and can be reduced after tuning.

## Selected Approach

Use a temporary **high-signal verbose logging pass**:

- log every meaningful Roshan Armlet and Huskar Roshan Spears decision branch
- log important no-op reasons near dangerous thresholds
- log every Roshan learning/reset transition with an explicit reason
- keep the current behavior unchanged so the next round of manual tests measures the existing logic more clearly

This is preferred over full per-tick tracing because per-tick logs would create too much noise for normal test review. It is preferred over end-of-session summaries because those would still hide the exact branch that failed during a lethal Roshan sequence.

## Logging Surfaces

### `src/actions/armlet.rs`

This remains the source of truth for:

- normal Armlet decision evaluation
- Roshan-mode learning state
- Roshan emergency fallback
- Roshan learned-hit trigger
- Roshan stun-recovery defer / resync flow

Add diagnostics here for:

1. Roshan-active evaluation context
2. Roshan recovery action results
3. learned-hit / fallback trigger details
4. explicit skip reasons near dangerous states
5. learning-state resets and why they happened

### `src/actions/heroes/huskar.rs`

This remains the source of truth for:

- Huskar-specific Burning Spears gate thresholds
- app-owned disable / re-enable tracking
- ownership resets

Add diagnostics here for:

1. effective trigger and disable / re-enable lines
2. whether Roshan mode was armed
3. whether Burning Spears was present
4. whether the app believed it owned the disabled state
5. why the gate chose disable / re-enable / clear / no-op

### Reset and transition paths

Where Roshan or Huskar state is cleared because of death, disarm, missing Armlet, feature disable, or similar conditions, add explicit reasoned logs so unexpected resets are visible in test output.

## Runtime Logging Design

### Armlet Roshan diagnostics

Add logs that answer:

#### Why did Armlet fire?

For any Roshan-triggered toggle, log the exact cause:

- normal toggle
- immediate Roshan post-stun protection
- deferred-hit resync trigger
- learned-hit trigger
- emergency fallback trigger

Include at least:

- hero name
- current HP
- base threshold
- predictive offset
- effective trigger
- cooldown
- stun state
- predicted or observed hit when relevant
- lethal zone when relevant
- sample count when relevant

#### Why did Armlet not fire?

When Roshan mode is active and health is near the danger region, log important no-op reasons such as:

- cooldown still active
- hero still stunned
- recovery logic deferred and is awaiting next hit
- not enough valid samples yet
- current HP still above the computed lethal zone

These logs should focus on meaningful branches rather than every safe idle tick far away from danger.

#### What happened to Roshan learning state?

Whenever learning or defer state is cleared, log the reason:

- hero died
- Armlet automation disabled
- Armlet item not found
- Roshan mode not active
- stale reset
- explicit mode arm/disarm transition

This is important because silent resets make later failures hard to interpret.

### Huskar Roshan Spears diagnostics

Add logs that answer:

#### Why was Burning Spears disabled or re-enabled?

For disable / re-enable actions, log:

- current HP
- effective Armlet trigger
- disable line
- re-enable line
- Roshan mode armed state
- app ownership state

#### Why was Burning Spears left alone?

When the gate evaluates to no-op near the threshold band, log enough detail to explain why:

- Roshan mode armed or not
- config enabled or not
- Burning Spears ability present or not
- current ownership state
- current HP versus disable / re-enable lines

#### Why was ownership cleared?

When the app gives up ownership of the Spears-off state, log the reason clearly, such as:

- hero died
- Roshan mode disarmed
- feature disabled
- Burning Spears missing / unparseable

## Log Level Strategy

This pass is intentionally verbose for manual Roshan test sessions.

- put high-value branch decisions at `info`
- put supporting numeric context and near-miss reasons at `info` when they are directly relevant to a dangerous Roshan window
- keep low-value background chatter at `debug`

The goal is to make one live test run explainable from normal logs without needing a second debug-only pass.

After the tuning work is done, some of these logs can be downgraded to `debug`.

## Implementation Notes

1. Prefer small helper functions or reusable formatted reason strings if the new logs would otherwise duplicate threshold math.
2. Do not change any automation thresholds or decision outcomes in this logging pass.
3. Stay close to the existing logging style in `armlet.rs` and `huskar.rs`.
4. If a log is emitted from a hot path, make it conditional on a meaningful Roshan-danger branch rather than every benign tick.

## Testing Strategy

1. Keep existing Rust and frontend tests passing.
2. Add focused tests only if extracting helper logic for logging reasons makes unit coverage practical.
3. Validate the change primarily through manual Roshan sessions using the new logs.
4. Review resulting logs after a test run to identify which branch caused deaths:
   - late trigger
   - cooldown block
   - stun defer that waited too long
   - missing or stale samples
   - unexpected reset
   - Burning Spears still on or re-enabled too early

## Expected Outcome

After this logging pass, a single failing Roshan test should produce enough evidence to answer:

1. what Armlet Roshan branch the runtime took
2. whether it had enough sample confidence
3. whether stun recovery deferred or protected immediately
4. whether Burning Spears was still contributing self-damage
5. whether any state reset disabled protection unexpectedly

That evidence should make the next implementation pass a tuning problem instead of a blind guess.
