# Wave Tracker & Audio Alerts — Design Spec

**Status:** Draft
**Supersedes:** the external "dota2script" PRD (which assumed a Python/Electron greenfield app; this repo is Rust + Tauri v2 + React)
**Depends on:** GSI server (`src/gsi/`), minimap capture backend (`src/observability/minimap_capture_backend.rs`), React UI shell (`src-ui/`)

---

## Goal

Two related situational-awareness features, shipped as independent slices:

1. **Wave Tracker** — clock-driven prediction of creep wave spawn, travel, and clash points, rendered as a 2D map both in-app and as a click-through overlay positioned over Dota's own minimap.
2. **Audio Alert Engine** — a real audio subsystem in Rust with per-event, individually recognizable cues for map objectives (runes, Tormentor, neutrals, stacks).

Wave Tracker ships first — it is the genuinely new capability. Audio alerts ship last, since a working (if primitive) rune alert already exists.

---

## Current State (verified against the repo)

| Capability | Where | State |
|---|---|---|
| GSI HTTP listener | `src/gsi/server.rs` | Complete |
| Game clock parsing | `src/models/gsi_event.rs:154` | `Map` parses only `clock_time` — sufficient, see "Clock Handling" below |
| Rune alerts | `src/observability/rune_alerts.rs` | Single generic rune, fixed 120s interval, 10s lead |
| Alert sound | `src-ui/src/hooks/useRuneAlert.ts:33` | 880 Hz `AudioContext` oscillator in the WebView. No assets, no Rust audio crate |
| Dota window rect | `src/observability/minimap_capture_backend.rs:23` | `find_dota2_window_rect()` — complete |
| Minimap sub-region capture | same file, `:69` | BitBlt of a client-relative rect — complete |
| Minimap region config | `config/config.toml [minimap_capture]` | `minimap_x/y/width/height` already stored |
| Map visualization | `src-ui/src/pages/MinimapIntelligence.tsx` | Text zone rows only — no map graphic |
| State → UI transport | `src-tauri/src/events.rs:13` | 5 Hz diff-based emitter (`gsi_update`) |

Two consequences worth stating up front:

- The overlay is **much cheaper than the PRD assumes**, because window-rect discovery and the minimap rect are already solved. The overlay is "place a transparent window at a rect we already compute."
- The audio subsystem is **less complete than the PRD assumes**. There is no file playback, no volume control, no per-event routing, and playback lives in a WebView that the OS may throttle when the window is not visible.

---

## Clock Handling

Every GSI packet carries an authoritative `map.clock_time`, so the app never needs to run its own clock — it only needs to interpolate *between* packets so animation is smooth rather than stepping at packet rate.

Pause is therefore handled **by derivation, not by parsing a field**: if `clock_time` is unchanged between two consecutive packets, interpolation freezes. The next packet re-synchronises regardless. This is strictly more robust than reading `map.paused`, because it correctly handles every reason the clock might stop — pause, disconnect, GSI stall, alt-tab throttling — without needing to enumerate them.

Consequences:

- No new GSI fields are required. `Map` stays as-is.
- Worst-case error is one packet interval of drift followed by a snap-back, bounded by the GSI throttle in the `.cfg`.
- The manual-sync fallback hotkey from the original PRD is dropped. GSI is already a hard dependency of every other feature in this repo; a second clock source would be dead weight.

Pre-horn is handled by `clock_time < 0` (already the convention `rune_alerts.rs` uses), so `game_state` is not needed either.

---

## Risk Assessment: In-Game Overlay

The question is whether drawing wave positions "inside Dota" risks a ban.

**Rejected approach — do not build:** rendering into Dota's own minimap requires D3D hooking, DLL injection, or game-memory reads. This is the category anti-cheat exists to catch.

**Chosen approach:** a separate OS window — borderless, `WS_EX_LAYERED | WS_EX_TRANSPARENT`, always-on-top — positioned exactly over the minimap rect, drawing on a transparent background. It never opens a handle to the Dota process. This is architecturally the same as the Discord, Steam, OBS, and GeForce Experience overlays.

Risk verdict: **the overlay is the lowest-risk subsystem in this repository.** A draw-only external window adds no detection surface. The pre-existing keystroke simulation (`rdev` / `enigo`) and minimap screen-scraping are the components that a strict reading of the Steam Subscriber Agreement would object to, and both already ship. The overlay does not change that posture.

Practical constraints (not legal ones):

1. **Exclusive fullscreen suppresses all overlays.** Dota must run Borderless or Windowed. The UI must detect and warn rather than silently render nothing.
2. **Click-through is mandatory.** Without `WS_EX_TRANSPARENT`, the overlay eats minimap clicks and breaks click-to-move. This is the highest-risk implementation detail and gets an explicit manual test.
3. **DPI scaling.** The minimap rect is client-relative; the overlay is positioned in screen coordinates. Per-monitor DPI must be handled or the overlay drifts on scaled displays.

De-risking order: build the in-app panel (D2) first so the wave model is validated visually with zero overlay risk, then reuse the identical renderer in the overlay (D3). If the overlay proves unworkable, D2 still stands alone as a shipped feature.

---

## Accuracy Model: What Wave Tracking Can and Cannot Do

The PRD's Mirror Rule holds **only while both waves are alive and symmetric**. Once a wave is killed, eaten by a tower, or the lane settles into an off-center equilibrium, clock-derived positions become fiction.

The feature is therefore scoped as a **wave ETA and clash-point tracker**, not a live position readout:

| Output | Accuracy | Valid when |
|---|---|---|
| Next wave spawn countdown | Exact | Always (pure clock arithmetic) |
| Predicted clash point per lane | High | Laning phase, before either wave is disrupted |
| Predicted arrival-at-tower ETA | High | Laning phase |
| Animated dot positions | Estimate | Laning phase; confidence decays after ~10:00 |

Exact positional accuracy is **explicitly a non-goal**. The value is in knowing roughly where a wave is and precisely when the next one lands; a dot that is a few seconds off costs nothing.

What this does buy is a presentation requirement: the UI renders dots in a visually "predicted" style (soft/ghosted) with a decaying confidence indicator, rather than implying measurement. Over-claiming would make the feature actively misleading in exactly the moments a player would trust it. Cheap to do, and it keeps the model honest.

**Speculative correction path (D4):** the existing minimap capture already clusters colored pixels, with `min_cluster_size`/`max_cluster_size` tuned for hero blobs. A second, smaller cluster band could detect creep blobs and reconcile predictions against observations. Given that accuracy is a non-goal, this is parked — recorded so the option isn't lost, not scheduled.

---

## Audio Design

### Why the current approach must be replaced

Playback lives in `useRuneAlert.ts`, inside the Tauri WebView. When the app window is minimized or occluded — which is its normal state while you are playing Dota — the OS and the WebView runtime may throttle timers and suspend `AudioContext`. An alert engine that is unreliable precisely when it is needed is worse than no alert engine. Playback moves to Rust (`rodio`), driven off the same background task that already emits state at 5 Hz.

### Cue design principles

Under fight-time cognitive load, cues must be identifiable pre-attentively — recognized without being thought about. Pitch alone is a weak discriminator. Three orthogonal channels carry meaning:

1. **Rhythm carries identity.** A countable pulse count (1, 2, 3 hits) is the strongest discriminator and survives a noisy mix.
2. **Pitch contour carries direction.** Rising = something is becoming available. Falling = something is expiring.
3. **Timbre carries category.** Economy events (neutrals, stacks, bounty) use soft wooden tones; map objectives (Tormentor, Roshan) use low brass; runes use bright glass/bell.

Additional constraints: keep every cue ≤ 500 ms so it never masks a spell sound; band-limit to roughly 500 Hz–4 kHz to cut through Dota's mix without competing with it; normalize all cues to a common perceived loudness so no single event startles.

### Proposed cue catalogue

| Event | Cadence | Motif | Timbre | Length |
|---|---|---|---|---|
| Power rune | 2 min, from 6:00 | 2 ascending blips | glass bell | ~260 ms |
| Wisdom rune | 7 min, from 7:00 | 3 ascending notes | soft marimba | ~380 ms |
| Water rune | 2:00 and 4:00 only | single blip + short tail | filtered sine | ~200 ms |
| Bounty rune | 3 min, from 0:00 | 2 quick high ticks | metallic tick | ~180 ms |
| Tormentor | 20:00, 10 min respawn | 2-note descending | low brass | ~500 ms |
| Neutral item tier | 7 / 17 / 27 / 37 / 60 | 4-note ascending arpeggio | plucked | ~400 ms |
| Stack timing | :53 each minute | single short tick | wood click | ~120 ms |
| Wave arriving | per lane, configurable | soft double tick | muted wood | ~150 ms |

The rhythm assignment is deliberate: the two-minute event gets two pulses, the three-minute event gets two fast pulses, the seven-minute event gets three. Cadence is encoded in the cue itself, which shortens the learning curve.

### On AI-generated audio

Two distinct uses, with different verdicts.

**Text-to-SFX (ElevenLabs Sound Effects, Stable Audio) for the motifs — not recommended as the default.** Generative SFX models are tuned for cinematic texture: whooshes, impacts, ambience. For a 200 ms alert that must be identified in a split second, a cleanly synthesized motif outperforms a generated one, and it stays tunable without regenerating an asset.

**Text-to-speech for voice callouts — genuinely the right tool.** A spoken word ("power", "wisdom", "tormentor") requires zero learning and is unambiguous on first hearing. This is where AI meaningfully beats synthesis. Generate short callouts offline, ship them as `.wav`, and expose them as a selectable voice pack.

**Recommended architecture, therefore, is a hybrid:**

- **Default:** procedural motif synthesis in Rust. Zero assets, near-zero binary size, every parameter (pitch, pulse count, timbre, length) tunable from config without regenerating files.
- **Optional:** a TTS-generated voice pack, selectable per event.
- **Always:** user-supplied `.wav` / `.mp3` override per event.

Assets are **pre-generated offline and committed**, never fetched at runtime. A game-adjacent tool must not carry an API key, must not make network calls on the alert path, and must have deterministic playback latency.

---

## Deliverables

Independently shippable slices, in execution order.

> **D0 (clock foundations) was dropped.** Per the Clock Handling section, GSI's `clock_time` is authoritative on every packet and pause is derived from an unchanged clock, so no new GSI fields, no separate clock module, and no manual-sync hotkey are needed. The only surviving piece — freeze interpolation when the clock stops — is a few lines inside D1's renderer feed.

### D1 — Wave Model ✅ *shipped* *(pure Rust, no UI)*

A `wave_tracker` module exposing spawn cadence, per-lane normalized progress, predicted clash points, and per-lane ETAs. Lane geometry is a set of normalized `[0,1]` map-space polylines; travel timing is calibrated per lane rather than derived from an assumed constant, with the calibration values living in config. Fully unit-tested, zero UI risk, zero dependencies.

Deliberately does **not** hardcode pixel coordinates — waypoints are calibrated against the reference map asset in D2.

### D2 — In-App Map Panel ✅ *shipped*

React page at `/waves` rendering **vector** lane paths (decided: no bitmap asset) with
animated wave dots, clash markers, per-lane clash countdowns, and calibration controls.
`WaveMap` is presentational and takes a `compact` prop, so D3 reuses it unchanged.

Positions come from `get_wave_snapshot` at ~15Hz, driven by a locally interpolated clock
anchored to GSI's `gameTime`. Confidence decay is rendered as opacity from the start.

### D3 — Minimap Overlay Window ✅ *shipped*

Second Tauri window (`wave-overlay`): transparent, borderless, always-on-top,
click-through, positioned over the Dota minimap and following it once a second. Global
hotkey (`F8`, blocked from Dota) plus a UI button. Reuses D2's renderer via `compact`.

Two things landed differently than planned. `find_dota2_window_rect()` could **not** be
reused — it returns a client rect whose origin is always `(0, 0)`, so a new
`find_dota2_client_screen_rect()` resolves the true screen origin via `ClientToScreen`.
And DPI needed no maths: using physical units throughout makes scaled displays correct
by construction.

Exclusive-fullscreen detection was **scoped down deliberately** — it cannot be done
reliably from outside the game process, so the UI reports the window style and states
the limitation instead of promising detection.

Click-through, live transparency, placement accuracy, and DPI-scaled monitors remain
manual-test-only; see the feature doc.

### D4 — Prediction Correction *(parked)*

Reconcile D1's predictions against creep clusters detected by the existing minimap analysis, using a second cluster-size band below the hero band. Parked, not scheduled: positional accuracy is a non-goal, and D1–D3 stand without it.

### D5 — Audio Engine ✅ *shipped*

`rodio` playback in Rust with procedural motif synthesis, per-event configuration,
master volume, and `.wav`/`.mp3` override that falls back to the built-in cue on
failure. The WebView oscillator hook was **removed**, not left in place — running both
would double-fire every cue. Per-event Test buttons live on the new Alerts page.

Synthesis (`src/audio/motif.rs`) is pure DSP and fully unit-tested without an audio
device. Two details turned out to matter: inharmonic partials are what make the bell
timbre read as metallic, and a 3ms attack/release ramp is required or every cue starts
and ends on a waveform discontinuity — an audible click.

### D6 — Event Catalogue ✅ *shipped*

All seven events with independent enable/lead/volume/sound config. Tormentor and Stack
ship **off** by default (role-specific, and once-a-minute respectively).

One finding worth recording: power and bounty runes **coincide** every six minutes, so
both cues fire together at 6:00, 12:00, 18:00. Correct behaviour, now locked in by a
test — an initial test that counted fired alerts caught it.

`[rune_alerts]` was left in place driving the header countdown, with `audio_enabled`
downgraded to a documented no-op. Folding that countdown onto the power-rune schedule
is a sensible future cleanup, deliberately not bundled here.

### D7 — Voice Pack

Offline-generated TTS callouts, committed as assets, selectable per event alongside the
procedural motifs. The `sound_file` override shipped in D5 is the hook this needs, so
D7 is now purely asset generation plus a pack selector.

---

## Open Questions

All blocking questions are resolved.

*Resolved:* clock handling (GSI is authoritative, pause derived from an unchanged clock);
accuracy target (positional precision is a non-goal); map rendering (vector lane paths,
no bitmap asset); overlay geometry (sized to and positioned over the in-game minimap at
the configured rect).

Remaining unknowns are D3 implementation risks rather than open decisions: click-through
behaviour, per-monitor DPI scaling, and exclusive-fullscreen detection. Each is called
out in the D3 deliverable.
