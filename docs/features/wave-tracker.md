# Creep Wave Tracker

Clock-driven prediction of creep wave spawn, travel, and clash points.

**Owner:** `src/observability/wave_tracker.rs`
**Config:** `[wave_tracker]` — see `docs/reference/configuration.md`
**Design:** `docs/superpowers/specs/2026-07-27-wave-tracker-and-audio-alerts-design.md`

**Status:** model and in-app panel shipped. The click-through minimap overlay is a
separate deliverable; `WaveMap` already accepts a `compact` prop for it.

| Layer | File |
|---|---|
| Model | `src/observability/wave_tracker.rs` |
| IPC | `src-tauri/src/commands/waves.rs` — `get_wave_lane_paths`, `get_wave_snapshot` |
| Clock + polling | `src-ui/src/stores/waveStore.ts` |
| Renderer | `src-ui/src/components/waves/WaveMap.tsx` |
| Page | `src-ui/src/pages/WaveTracker.tsx` (route `/waves`) |

---

## What it does

Creep waves spawn on a fixed 30-second cadence, so their approximate position derives
from `map.clock_time` alone. The module is pure arithmetic: no I/O, no stored state, no
game-state access beyond the clock value the caller passes in.

`wave_snapshot(clock_time_seconds, &config)` returns:

| Field | Meaning |
|---|---|
| `next_spawn_time_seconds` | Game-clock time of the next spawn |
| `seconds_until_next_spawn` | Countdown to that spawn |
| `current_wave_age_seconds` | Seconds since the in-flight wave spawned; `None` before the horn |
| `confidence` | `High` / `Degrading` / `Low` — how much the renderer should trust positions |
| `waves` | Six `WavePosition` entries (3 lanes × 2 teams), empty before the horn |
| `clashes` | One `LaneClash` per lane: where the current pair meets and when |

---

## Coordinate space

Map points are normalised to `[0.0, 1.0]` with the origin at the **bottom-left**
(Radiant corner); `(1.0, 1.0)` is the top-right (Dire corner). Renderers flip the y-axis
for screen space as needed.

Lane progress is separately normalised: `0.0` is the Radiant barracks end of the lane and
`1.0` the Dire barracks end — for both teams' waves. A Dire wave therefore starts at
progress `1.0` and counts down.

Lane paths are hand-calibrated polylines (`TOP_LANE_PATH`, `MID_LANE_PATH`,
`BOTTOM_LANE_PATH`). `point_at(lane, progress)` walks the polyline by cumulative segment
length. These waypoints are approximations expected to be refined against whatever map
asset the renderer ends up using.

---

## Timing model

Each lane is calibrated by two numbers: **when** the waves meet and **where**.

Both waves advance linearly from their own barracks and stop at the clash point. When
the clash is off-centre the two teams travel at different rates, because they cover
different distances in the same time — which is what produces the correct asymmetry on
the side lanes.

| Lane | Meets at | Clash position | Rationale |
|---|---|---|---|
| Mid | 17s after spawn | `0.5` (centre) | Symmetric lane |
| Top | 28s after spawn | `0.42` | Radiant offlane — clash sits on the Radiant half |
| Bottom | 28s after spawn | `0.58` | Dire offlane — mirrors top |

Before the horn (`clock_time < 0`) there are no waves; the snapshot reports a countdown
to the first spawn at 0:00 with empty `waves` and `clashes`.

---

## Accuracy: read this before building on it

**Positional precision is an explicit non-goal.**

The model assumes both waves in a lane are alive and undisrupted. That holds during
laning and breaks down once a wave is killed, eaten by a tower, or the lane settles into
an off-centre equilibrium. There is no game-state feedback correcting it.

| Output | Accuracy | Valid when |
|---|---|---|
| Spawn countdown | Exact | Always — pure clock arithmetic |
| Clash point and ETA | Good | Laning phase, waves undisrupted |
| Dot positions | Estimate | Laning phase; degrades afterwards |

`Confidence` exists to keep this honest at the presentation layer. Renderers should
style predicted positions as estimates (soft or ghosted) and reflect the decaying
confidence, rather than drawing them as measurements. A dot a few seconds out of place
costs nothing; a dot that *looks* authoritative while being wrong is worse than no dot.

---

## Clock handling

GSI carries an authoritative `map.clock_time` on every packet, so nothing here runs its
own clock — `waveStore` interpolates between packets only to keep animation smooth, and
`wave_snapshot` accepts a fractional clock so dots move rather than step.

Pause is handled **by derivation, not by a field**. `interpolatedClock` extrapolates from
the last reported `gameTime`, capped at `MAX_CLOCK_DRIFT_SECONDS` (1.5s). When the game
is paused `gameTime` stops changing, the cap is reached, and the clock freezes instead of
drifting. The next packet re-anchors it. This covers pause, disconnect, GSI stall, and
alt-tab throttling identically without needing to enumerate them — which is why no extra
GSI field is parsed.

---

## Rendering

The map is drawn as vectors, not a bitmap: `WaveMap` renders the lane polylines straight
from `get_wave_lane_paths`, so the drawn lanes and the interpolated positions cannot
drift apart. It scales from a 240px overlay to a full page with no assets, and ships no
Valve artwork.

Normalised map space has its origin at the bottom-left; SVG's is at the top-left, so the
y-axis flips in `toSvg()` and nowhere else.

Dots carry a soft halo and an opacity driven by `Confidence`, so a prediction never reads
as a measurement.

---

## Tests

- `cargo test --lib wave_tracker` — 18 unit tests: spawn cadence and boundaries, pre-horn
  behaviour, wave convergence and clamping, side-lane mirror symmetry, cycle repetition,
  monotonic progress, fractional-clock interpolation, path endpoints and clamping,
  normalised-space containment, confidence decay, degenerate zero-duration config.
- `cargo test -p dota2-scripts-tauri` — lane path geometry is complete and in range.
- `npx vitest run` in `src-ui/` — renderer output, y-axis flip, confidence fade, compact
  mode, and the clock interpolation and freeze behaviour.
