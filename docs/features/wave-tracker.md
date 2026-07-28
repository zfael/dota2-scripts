# Creep Wave Tracker

Clock-driven prediction of creep wave spawn, travel, and clash points.

**Owner:** `src/observability/wave_tracker.rs`
**Config:** `[wave_tracker]` — see `docs/reference/configuration.md`
**Design:** `docs/superpowers/specs/2026-07-27-wave-tracker-and-audio-alerts-design.md`

**Status:** model, in-app panel, and click-through minimap overlay all shipped.

| Layer | File |
|---|---|
| Model | `src/observability/wave_tracker.rs` |
| Overlay geometry | `src/observability/wave_overlay.rs` |
| Window discovery | `src/observability/minimap_capture_backend.rs` |
| IPC | `src-tauri/src/commands/waves.rs`, `src-tauri/src/commands/overlay.rs` |
| Clock + polling | `src-ui/src/stores/waveStore.ts` |
| Renderer | `src-ui/src/components/waves/WaveMap.tsx` |
| Page | `src-ui/src/pages/WaveTracker.tsx` (route `/waves`) |
| Overlay view | `src-ui/src/pages/WaveOverlay.tsx` |

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

Lane paths are polylines (`TOP_LANE_PATH`, `MID_LANE_PATH`, `BOTTOM_LANE_PATH`) fitted to
in-game tower positions — towers sit on the lane, so their centres are the only landmarks
precise enough to calibrate against. `point_at(lane, progress)` walks the polyline by
cumulative segment length. All three paths run corner to corner between the two bases at
`(0.15, 0.15)` and `(0.85, 0.85)`, and the ring the side lanes trace runs at `0.12`/`0.88`
on each axis.

The map is symmetric under a 180° rotation about its centre, which swaps the teams and
carries the top lane onto the bottom lane. `BOTTOM_LANE_PATH` is therefore the reverse
complement of `TOP_LANE_PATH` rather than an independent fit, and mid is its own mirror.
Three tests assert this, which is what keeps a retune of one lane from silently skewing
the map.

These waypoints remain approximations and retuning them is expected — but retune them in
*map* space. Where map space lands on Dota's minimap panel is a separate, per-resolution
concern owned by `[wave_overlay]`'s `map_offset_*`/`map_scale_*`; see
[Overlay alignment](#overlay-alignment).

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

## Minimap overlay

A second Tauri window (`wave-overlay`) — borderless, transparent, always-on-top,
click-through — positioned over Dota's in-game minimap.

**It never touches the Dota process.** No injection, no hooking, no memory reads. It is
an ordinary OS window placed on top, architecturally the same as the Discord, Steam, or
GeForce overlays. Rendering *into* Dota's own minimap would require D3D hooking and is
deliberately not done.

### Placement

`find_dota2_client_screen_rect()` resolves the Dota client area in **screen**
coordinates — note that `find_dota2_window_rect()` cannot be used here, as it returns a
client rect whose origin is always `(0, 0)`, fine for BitBlt but useless for positioning.
`overlay_bounds()` then translates the `[minimap_capture]` region by that origin and
applies the manual offsets.

All geometry is in physical pixels, matching Tauri's `PhysicalPosition` / `PhysicalSize`.
This is what keeps the overlay correct on scaled displays with no DPI maths of our own.

Dota can be moved, resized, or dragged to another monitor at any time and there is no
notification for it from outside the process, so a background task re-reads the position
once a second while the overlay is visible and only touches the window when the bounds
actually change. If Dota disappears, the overlay hides itself rather than leaving a stale
rectangle floating over the desktop.

### Overlay alignment

Placing the window correctly is only half of being aligned. The window is sized to Dota's
whole minimap **panel** — the region `[minimap_capture]` describes — but the playable map
texture is inset inside that panel's bezel and corner buttons, and is not centred within
it. Stretching normalised map space across the window therefore lands the lanes a few
percent out: small in absolute terms, but a wave dot only has to miss by a couple of
pixels to sit in the trees instead of the lane.

`WaveMap` takes a `MapCalibration` (`map_offset_x/y`, `map_scale_x/y` under
`[wave_overlay]`) describing where map space sits inside the box, applied about the
centre. The in-app panel is a bare square and uses the identity; only the overlay passes
anything else.

The SVG sets `preserveAspectRatio="none"` deliberately. The default, `xMidYMid meet`,
fits the square `viewBox` inside the overlay's taller window and letterboxes the
remainder — which would silently override the vertical half of any calibration.

How large the inset is depends on resolution and Dota's UI scale, so it cannot be a
constant. The shipped defaults were fitted to tower positions measured off a 2560x1440
borderless capture at the stock `[minimap_capture]` region. To re-derive them for another
setup, enable **Calibration Mode** in the Wave Tracker page's Minimap Overlay card: the
overlay then draws its lane lines plus a dashed box around the area it treats as the map,
with a centre crosshair. Adjust until the box frames Dota's map and the lines sit on
Dota's lanes. Config changes apply live, so no restart is needed.

Do not correct a placement error by editing the lane waypoints in `wave_tracker.rs` — that
makes the in-app panel wrong in order to make the overlay right, since the panel renders
the same geometry with no calibration applied.

### Window lifetime

Closing the main window also closes the overlay, wired up in the Tauri `setup` hook.
Tauri exits when its last window closes, and the overlay is transparent,
click-through, and hidden from the taskbar — left open on its own it would keep the
process alive as an invisible ghost that can only be killed from Task Manager.

### Click-through

`set_ignore_cursor_events(true)` is the single most important property of this window.
Without it the overlay swallows minimap clicks and breaks click-to-move. Verify this by
hand after any change to window creation.

### Transparency

The overlay shares the main bundle, selected by an `?overlay=1` query parameter (a
parameter rather than a route because the app uses `BrowserRouter`). `main.tsx` applies
`body.overlay-mode` **before first paint**, which drops the global stylesheet's opaque
background and its 900×650 minimum — either one would hide the minimap underneath.

### Exclusive fullscreen

No overlay can draw over exclusive fullscreen; Dota must be in Borderless or Windowed.
This cannot be reliably detected from outside the game, so `detect_dota2_window_mode()`
reports the window *style* only (`Windowed` when it has a caption, otherwise
`Borderless`, which means "borderless or fullscreen"). The UI states the limitation
rather than promising detection.

### Hotkey

`[wave_overlay] toggle_key` (default `F8`) is intercepted by the existing rdev hook as
`HotkeyEvent::WaveOverlayToggle` and **blocked** from reaching Dota, so pick a key Dota
does not need. The keyboard listener starts before the Tauri builder runs, so the handler
reaches the app through an `AppHandle` published during setup.

---

## Tests

- `cargo test --lib wave_tracker` — 18 unit tests: spawn cadence and boundaries, pre-horn
  behaviour, wave convergence and clamping, side-lane mirror symmetry, cycle repetition,
  monotonic progress, fractional-clock interpolation, path endpoints and clamping,
  normalised-space containment, confidence decay, degenerate zero-duration config.
- `cargo test --lib wave_overlay` — placement translation, following a moved window,
  manual offsets, negative multi-monitor coordinates, zero-sized regions.
- `cargo test --lib keyboard` — the overlay hotkey plans its event and is only parsed
  when the overlay is enabled.
- `cargo test -p dota2-scripts-tauri` — lane path geometry is complete and in range.
- `npx vitest run` in `src-ui/` — renderer output, y-axis flip, confidence fade, compact
  mode, overlay-window detection, and the clock interpolation and freeze behaviour.

**Not covered by automated tests** (they need a running Dota 2 and a real compositor):
click-through actually passing clicks to the game, transparency against the live
minimap, placement accuracy at various resolutions, and behaviour on a DPI-scaled or
secondary monitor. These need manual verification.
