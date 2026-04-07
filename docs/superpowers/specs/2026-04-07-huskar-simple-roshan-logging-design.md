# Huskar Roshan Summary Logging Design

## Problem

The current Huskar Roshan diagnostics are still too hard to use during a live Roshan attempt. When Burning Spears does not disable as expected, the logs may stay quiet in exactly the moments that matter, leaving too much ambiguity about whether the gate evaluated, whether Burning Spears was detected, and whether the app actually tried to emit Alt+W.

## Goals

1. Emit one easy-to-spot Huskar Roshan summary line on every GSI tick while Roshan mode is armed.
2. Emit one explicit info-level line immediately before the app sends the Burning Spears Alt+W toggle.
3. Preserve existing gate behavior and thresholds.
4. Keep the change scoped to Huskar Roshan observability only.

## Non-Goals

1. Do not change disable or re-enable thresholds.
2. Do not change Armlet Roshan behavior.
3. Do not add config flags or new UI controls.

## Selected Approach

Use an always-on Huskar Roshan summary line while Roshan mode is armed:

- log `hp`
- log `effective_trigger`
- log `disable_line`
- log `reenable_line`
- log `roshan_armed`
- log `config_enabled`
- log `burning_spears_present`
- log `owned_by_app`
- log `chosen_action`

Then add a second explicit `info!` line right before `emit_burning_spear_toggle(...)` so a live log tells us whether Alt+W was actually attempted.

This is intentionally noisier than the previous near-threshold-only no-op logs because the current failure mode is lack of visibility, not excess data.

## Implementation Notes

1. Reuse the existing Huskar threshold helpers and gate evaluation.
2. Add a small helper to map the chosen gate action to a stable log label if that keeps the runtime code clearer.
3. Keep the existing richer branch-specific logs; the new summary line complements them rather than replacing them.
4. Add focused unit coverage only for any extracted helper logic.

## Expected Outcome

After this change, a single Roshan attempt should make it obvious whether Huskar Roshan Spears logic is:

1. running every tick while Roshan mode is armed,
2. seeing Burning Spears in the GSI payload,
3. deciding `Disable`, `Reenable`, `ClearOwnership`, or `None`,
4. and actually attempting the Alt+W toggle.
