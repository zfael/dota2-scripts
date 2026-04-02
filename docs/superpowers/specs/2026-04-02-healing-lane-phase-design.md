# Lane-Phase Healing Threshold Override

## Problem

Healing items currently use static HP thresholds:

- normal mode uses `common.survivability_hp_threshold`
- danger mode uses `danger_detection.healing_threshold_in_danger`

That works in many fights, but it is too eager during the early laning phase. In lane, especially mid, spending a healing item around 30-50% HP is often wasteful. The desired behavior is to hold healing items until the hero is close to dying, then return to the current logic once lane phase is over.

## Goals

- Add a global early-game healing threshold override for all heroes
- Make the lane-phase duration configurable
- Make the lane-phase healing threshold configurable
- Ensure the early-game rule overrides both normal and danger healing thresholds
- Keep the change narrow to healing-threshold selection only

## Non-Goals

- No per-hero healing thresholds
- No lane detection based on hero position or role
- No item-specific early-game thresholds
- No changes to healing item priority order
- No changes to danger detection heuristics
- No UI work in this slice

## Selected Approach

Use a time-gated global override in the healing-threshold resolver.

For the first configured number of seconds, survivability logic will use a stricter healing threshold regardless of whether danger detection is active. After that window expires, the code will fall back to the current threshold selection logic.

This approach is preferred because it is predictable, easy to tune, and fits the repo's existing config-driven survivability design.

## Configuration

Add two new fields under `[common]`:

- `lane_phase_duration_seconds = 480`
- `lane_phase_healing_threshold = 12`

Design note:

- `lane_phase_duration_seconds = 0` disables the override without needing an extra `enabled` flag

These fields belong in `CommonConfig` because the behavior is global and sits above both normal and danger-specific threshold selection.

## Behavior

The healing-threshold resolver should evaluate thresholds in this order:

1. If `lane_phase_duration_seconds > 0` and `event.map.clock_time >= 0` and `event.map.clock_time < lane_phase_duration_seconds`, use `lane_phase_healing_threshold`
2. Else if `in_danger` and danger detection is enabled, use `danger_detection.healing_threshold_in_danger`
3. Else use `common.survivability_hp_threshold`

Implications:

- during the first 8 minutes, the early-lane rule always wins
- after 8 minutes, current normal vs danger behavior remains unchanged
- pre-game negative clock values do not count as lane phase

## Implementation Notes

The change should stay focused in the shared survivability path:

- extend the healing-threshold helper in `src/actions/common.rs` so it can read `event.map.clock_time`
- add serde defaults and config fields in `src/config/settings.rs`
- add checked-in defaults in `config/config.toml`

The rest of healing behavior should remain unchanged:

- same healing item ordering
- same castability checks
- same max-items-per-call behavior

## Testing

Add or update unit tests in `src/actions/common.rs` to cover:

- lane-phase threshold chosen before the cutoff
- cutoff boundary behavior at exactly `480` seconds
- post-lane fallback to danger threshold
- post-lane fallback to normal threshold
- disabled override when `lane_phase_duration_seconds = 0`
- negative clock time falling through to existing logic

## Documentation Updates

Update:

- `docs/features/survivability.md`
- `docs/features/danger-detection.md`
- `docs/reference/configuration.md`

The docs should clearly state that lane-phase healing is a global override sourced from `[common]` and that it takes precedence over the danger healing threshold during the configured early-game window.
