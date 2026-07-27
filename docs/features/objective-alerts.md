# Objective Alerts

Audio cues for clock-scheduled map objectives: runes, Tormentor, neutral item tiers,
and stack timings.

**Owner:** `src/observability/alerts.rs` (schedules + cues), `src/audio/` (synthesis + playback)
**Config:** `[alerts]` — see `docs/reference/configuration.md`
**Design:** `docs/superpowers/specs/2026-07-27-wave-tracker-and-audio-alerts-design.md`
**UI:** `src-ui/src/pages/Alerts.tsx` (route `/alerts`)

---

## Why playback lives in Rust

The previous implementation played an 880 Hz `AudioContext` tone from the WebView. The
Tauri window's normal state while you are playing Dota is minimised or occluded, and in
that state the OS and the WebView runtime may throttle timers and suspend
`AudioContext` — so the alert engine was unreliable exactly when it was needed.

Cues are now generated and played in Rust (`rodio`), driven from the GSI handler. The
WebView hook (`useRuneAlert`) was removed rather than left in place, since running both
would double-fire every cue.

---

## Schedules

| Event | Schedule | Default |
|---|---|---|
| Power Rune | Every 2 min from 6:00 | on, 15s lead |
| Wisdom Rune | Every 7 min from 7:00 | on, 20s lead |
| Water Rune | 2:00 and 4:00 only | on, 15s lead |
| Bounty Rune | Every 3 min from 0:00 | on, 15s lead |
| Tormentor | 20:00, then every 10 min | **off**, 30s lead |
| Neutral Item | 7 / 17 / 27 / 37 / 60 min | on, 10s lead |
| Stack Timing | Every minute at :53 | **off**, 5s lead |

Tormentor and Stack are off by default: Tormentor matters only to some roles, and Stack
fires every single minute, so both are noise unless you specifically want them.

Two schedule shapes cover everything: `Periodic { start, interval }` and a `Fixed` list.
Fixed schedules genuinely run out — after 4:00 there is no next water rune, and the UI
shows a dash rather than a bogus countdown.

**Power and bounty runes coincide** at 6:00, 12:00, 18:00 and so on, so both cues fire
together. This is correct and is locked in by a test.

---

## Cue design

A cue has to be identified pre-attentively — recognised without being thought about —
while a fight is happening. Pitch alone is a weak discriminator, so three orthogonal
channels carry meaning:

| Channel | Carries | Example |
|---|---|---|
| **Rhythm** | Identity | Pulse count is countable and survives a noisy mix |
| **Pitch contour** | Direction | Rising = becoming available; falling = expiring |
| **Timbre** | Category | Runes bright bell, economy wooden, objectives brass |

Pulse count deliberately encodes cadence, which shortens the learning curve: the
2-minute power rune gets **two** blips, the 7-minute wisdom rune **three** notes, bounty
**two fast** ticks.

| Event | Cue | Timbre |
|---|---|---|
| Power Rune | 2 rising blips | Bell |
| Wisdom Rune | 3 rising notes | Wood |
| Water Rune | 1 soft drop | Sine |
| Bounty Rune | 2 quick high ticks | Bell |
| Tormentor | 2-note falling | Brass |
| Neutral Item | 4-note rising arpeggio | Wood |
| Stack | 1 dry tick | Wood |

Every cue is ≤500ms so it never masks a spell sound — enforced by a test.

---

## Synthesis

`src/audio/motif.rs` is pure DSP over `f32` samples with no audio-device dependency, so
all of it is unit-tested.

- Timbre is a set of partials plus a decay rate. Bell uses **inharmonic** ratios
  (2.76×, 5.4×), which is what makes it read as metallic rather than as a chord.
- Every tone gets a 3ms attack/release ramp. Without it the waveform starts and ends on
  a non-zero sample — an instantaneous discontinuity, audible as a click on every cue.
- Cues are peak-normalised before volume is applied, so cues with different harmonic
  content sit at a consistent loudness and no single event startles.

Generating rather than shipping audio keeps the binary asset-free and makes every
property of a cue a value that can be retuned without regenerating anything.

---

## Custom sounds

Set `sound_file` on any event to a `.wav` / `.mp3` path. If the file cannot be opened or
decoded the built-in cue plays instead — a mistyped path should not silently disable an
alert. This is also the hook for a TTS voice pack (D7).

---

## Behaviour notes

- **Fire-once:** each occurrence announces once, even though the lead window spans many
  GSI packets.
- **New game:** a clock that jumps backwards clears fired state, so the previous game's
  occurrences do not suppress this game's alerts.
- **No audio device:** logged once as a warning; the rest of the app keeps working.
- **Volume:** per-event volume is multiplied by `master_volume`, both clamped to 0-1.

---

## Relationship to `[rune_alerts]`

`[rune_alerts]` is the older, generic "next rune every N seconds" timer. It still drives
the `runeTimer` countdown in the status header. Its `audio_enabled` field is now a
**no-op** — audio is owned entirely by `[alerts]`. Folding the header countdown onto the
power-rune schedule is a sensible future cleanup.

---

## Tests

- `cargo test --lib motif` — 11 tests: duration maths, render length, no clipping, volume
  scaling and clamping, click-free edges, silent gaps, timbres being distinguishable, and
  decay rates actually differing.
- `cargo test --lib alerts` — 22 tests: every schedule, exhausted fixed schedules, the
  power/bounty coincidence, fire-once, re-firing on the next occurrence, per-event and
  master switches, new-game reset, lead-time widening, cue pulse counts, volume maths.
- `cargo test -p dota2-scripts-tauri` — event key round-tripping.
- `npx vitest run` in `src-ui/` — countdown formatting and catalogue completeness.

**Not covered by automated tests:** how the cues actually sound, and whether they are
distinguishable to a human mid-fight. That needs listening, via the Test button on each
event in the Alerts page.
