# Survivability

**Purpose**: Read this before changing generic healing, defensive-item use, neutral-item saves, or silence dispels.

---

## Ownership map

| Path | What it owns |
|---|---|
| `src-ui/src/pages/Survivability.tsx` | The operator-facing **Survivability** page — the single place to configure healing, defensive items, dispels, and neutral items |
| `src/actions/armlet.rs` | Armlet / Sabatons toggle state read from `max_health`, the press decision and its check, Roshan mode, and the chord |
| `src/actions/common.rs` | Shared survivability pipeline: healing items, defensive items, neutral items |
| `src/actions/danger_detector.rs` | Global `in_danger` heuristic consumed by common and hero code |
| `src/actions/dispel.rs` | Immediate silence reaction: Manta, Flicker, Minotaur Horn, Lotus |
| `src/actions/auto_items.rs` | Cached GSI item/ability state and Space+right-click item usage; not the HP-healing loop, but part of the shared item automation surface |
| `src/config/settings.rs` | `CommonConfig`, shared `ArmletAutomationConfig`, hero armlet overrides, `DangerDetectionConfig`, `NeutralItemConfig` defaults |
| `config/config.toml` | Checked-in values for `[common]`, `[armlet]`, `[danger_detection]`, `[neutral_items]`, `[mana_automation]`, `[phase_boots_automation]`, and hero armlet overrides |

---

## Shared GSI survivability pipeline

Before hero-specific or fallback survivability logic runs, `src/actions/dispatcher.rs::dispatch_gsi_event()` now evaluates shared Armlet automation inline as the highest-priority GSI hook. That keeps Armlet off `ActionExecutor` and lets it reach the synthetic-input worker before neutral-item logging, silence dispel jitter, danger detection, healing, or other survivability actions.

### Fallback path

For heroes without a registered script, `src/actions/common.rs::SurvivabilityActions::execute_default_strategy()` runs:

1. `danger_detector::update(...)`
2. `check_and_use_healing_items(...)`
3. `use_defensive_items_if_danger(...)`
4. `use_neutral_item_if_danger(...)`

### Hero-script path

Registered hero scripts currently rely on the dispatcher-owned Armlet pre-hook, then call the same shared survivability helpers manually from their own `handle_gsi_event(...)` implementations.

That means survivability changes often affect both:

- `src/actions/common.rs`
- hero files under `src/actions/heroes/`

Read `docs/architecture/state-and-dispatch.md` before moving logic between those layers.

---

## Healing items

Owned by `src/actions/common.rs::check_and_use_healing_items()`.

### Thresholds

| Mode | Threshold source | Default |
|---|---|---|
| Lane phase | `common.lane_phase_healing_threshold` while `0 <= map.clock_time < common.lane_phase_duration_seconds` | `12` |
| Normal | `common.survivability_hp_threshold` | `30` |
| Danger | `danger_detection.healing_threshold_in_danger` | `50` |

Lane phase takes precedence over both normal and danger healing during the configured early-game window. Set `common.lane_phase_duration_seconds = 0` to disable the override. Negative pre-game clock values do not count as lane phase.

### Item order

Current code checks items in this exact order:

| Mode | Exact order |
|---|---|
| Normal | `item_cheese` -> `item_magic_stick` -> `item_faerie_fire` -> `item_magic_wand` -> `item_enchanted_mango` -> `item_greater_faerie_fire` |
| Danger | `item_cheese` -> `item_greater_faerie_fire` -> `item_enchanted_mango` -> `item_magic_wand` -> `item_magic_stick` -> `item_faerie_fire` |

### Limits

| Mode | Limit |
|---|---|
| Normal | one item per call |
| Danger | `danger_detection.max_healing_items_per_danger` per call (default `3`) |

### Castability check

For each inventory slot from `event.items.all_slots()`:

- match exact `item.name`
- require `item.can_cast == Some(true)`
- use the slot's configured key via `Settings::get_key_for_slot(...)`

---

## Armlet automation

Owned by `src/actions/armlet.rs`. The decision loop was proved first in
`examples/armlet_toggle_control.rs`, against a simulator wired with capture 1 of
`examples/sabatons_toggle_probe.rs` and beside the loop it replaced.

This path is shared across heroes and fallback survivability flow. It drives Armlet of
Mordiggian and 7.42's Sabatons of Mordiggian (Armlet wins if both are held), and only
acts when:

- `[armlet].enabled = true` after per-hero override resolution
- the hero is alive and holds `item_armlet` or `item_berserkers_sabatons`
- the hero can use items: not stunned, hexed or muted
- Dota has the keyboard. With another window in front nothing is pressed; if Dota's
  window cannot be found at all, this check stands aside rather than block
- HP is below `toggle_threshold + predictive_offset`, or Roshan mode says the next hit
  is lethal

### Reading the toggle from max HP

GSI does not say whether the toggle is on: in capture 1 the item JSON is identical
either way. `max_health` does:

| `max_health` move | Read as |
|---|---|
| rising tick over tick: the Strength ramping in, 0.75–1.0s | switched **on** |
| a one-tick drop the size of a toggle, or one that floors HP at 1–2 | switched **off** |
| death, respawn, the item just arriving, Armlet swapped for Sabatons | **off** |
| the app starting with the item already held | **unknown** |
| a level, a purchase, a one-tick rise, a small drop | not the toggle |

The size of a toggle (+500 Armlet, +600 Sabatons) is learned from the moves themselves,
never configured. Replaying capture 1 through the tracker reads all 14 of the probe's
checkpoints correctly, a press the probe never saw included.

### What it presses

| Toggle | Press |
|---|---|
| on, HP under the learned bonus | the **trick**: quick-cast, then `cast_modifier` + quick-cast, one chord. HP floors at 1 and the Strength ramps back in on top |
| on, HP at or over the bonus | nothing: the trick cannot gain anything there |
| unknown | the trick, as a probe: from off it cancels itself, which also shows it is off |
| off, while losing HP or in danger | **one press** to turn it on: the whole bonus, with no 1 HP floor at all |
| off, nothing hitting | nothing: the player switched it off on purpose |

It never presses while one of its presses is in flight or the Strength is still coming
in. `toggle_cooldown_ms` is only a floor under that.

### Checking every press

Each press is checked against what `max_health` did next:

- **Landed** — the Strength came back in. Logged at INFO with the HP before, at the
  floor and after, and how long it took.
- **Second press lost** — the trick switched it off but the press back on had not
  shown by the next tick. One press turns it back on at once.
- **Was on** — a single press switched it off, so the belief was wrong. One more press
  turns it back on.
- **No effect** — nothing moved within `press_confirm_ms`. After a trick from Unknown:
  it reads off, since a chord from off switches it on and straight back off. After a
  trick believed on: on was read from `max_health` rising, and a switch-off would have
  shown as a drop, so the presses are what is not landing — a taunt or a cyclone stops
  items and GSI says nothing. One retry at once, then it waits `press_confirm_ms`, and
  only a third miss reads as off. It never sends a single press there: on a toggle
  that is on, that switches it off. After a single press: it backs off for another
  `press_confirm_ms` instead of pressing into nothing.
- **Died** — the hero died before one of ours landed. Logged at INFO with how long
  after the press, and the HP before it and at the lowest.

It never presses at 0 HP: GSI can still say alive on the tick the hero dies.

A toggle the automation did not press is the player's. It holds off for
`press_confirm_ms` after one, so a trick done by hand is never cut in half.

### What the logs showed

The app's own logs from 2026-09-01 to 2026-11-21 (215 episodes, 973 presses) show what
the loop this replaced — the chord whenever HP was under the trigger and the cooldown
had passed — did in real games:

| What the next press found | Presses |
|---|---|
| the Strength still ramping in: HP pinned at ~240–270 while each chord cancelled the last | 354 |
| the toggle off: HP creeping up on regen while each chord switched it on and back off | 319 |
| a chord that half landed | 28 |

37 of the 111 single-press episodes ended in a death within 3s, most on the very next
tick. In the simulator the new loop dies less often in every fight it was run through —
a slow bleed under drain 4–25% of runs against 49–95% — and never presses mid-ramp.

### Self-inflicted HP

The HP a switch-off takes — ours or the player's — is declared to
`danger_detector::note_self_damage` before the detector reads the event, and left out
of Roshan mode's hit learning. Both used to read every trick as a hit the size of the
floor.

### Sabatons of Mordiggian

7.42's Sabatons (built from Armlet) is driven by the same code. Capture 1 of
`examples/sabatons_toggle_probe.rs` measured it against Armlet:

| | Armlet | Sabatons |
|---|---|---|
| Max HP per toggle | +500 | +600 |
| Press → `max_health` settled | ~0.75s | ~0.85–1.0s |
| Toggle state in GSI | none | none |
| Off at low HP | — | leaves 1–2 HP, never 0 |

Nothing about either item is hard-coded in the decision. Re-run the probe and the
simulator after a patch that touches either item.

### Roshan mode

Shared Armlet now includes an optional Roshan-specific mode under `[armlet.roshan]`.

- `enabled = false` by default
- when enabled, the configured `toggle_key` becomes a **blocked global hotkey**
- pressing that hotkey toggles a live armed/disarmed runtime state without sending the key through to Dota 2
- while armed, the module keeps learning recent HP-loss samples that look like Roshan hits

The Roshan path only asks for a trick while HP is still over the ordinary trigger, and that trick goes through the same gating as any other: never mid-ramp, only from ON, and not at all once HP reaches the bonus. Its hit learning leaves out whatever the toggle itself took in the same event.

Current trigger ladder:

1. record large recent HP drops that clear `min_sample_damage`
2. if the sample window reaches `min_confidence_hits` inside `learning_window_ms`, use the largest learned hit as predicted incoming Roshan damage
3. before confidence is reached, allow an emergency first-hit fallback when HP falls into the learned-danger zone plus `emergency_margin_hp`
4. clear stale learning after `stale_reset_ms` or whenever Roshan mode is armed/disarmed again

This exists because the current GSI model does not expose authoritative Roshan attack events. The runtime therefore infers likely Roshan hits from observed HP deltas instead of predicting an exact server-side attack timer.

For Huskar specifically, `src/actions/heroes/huskar.rs` can also gate Burning Spears while Roshan mode is armed. The optional `[heroes.huskar.roshan_spears]` block disables Burning Spears in a buffer band above the effective Armlet trigger and only re-enables it after HP recovers above a higher hysteresis line.

### Trigger shape

Armlet toggling now uses a dual-trigger sequence:

1. press the quick-cast slot key from `[keybindings]`
2. press the cast-side trigger for the same slot using `[armlet].cast_modifier`

With the checked-in config, a slot bound to `x` toggles as:

- `x`
- `Alt + x`

The runtime now sends that as one dedicated serialized worker-owned chord:

1. quick-cast slot key
2. modifier down
3. slot key again while the modifier is held
4. modifier up

That means the two casts are still **not** truly simultaneous, but Armlet no longer pays the old extra queue handoff / guard pulse between those four steps. The whole chord executes inside one synthetic-input worker command, so the second cast follows the modifier press as quickly as that worker can emit it, and the short replay-safety guard is applied once after the full chord instead of once after the first click.

### Fast-lane scheduling

Armlet now has two priority advantages over the rest of the survivability stack:

1. `dispatch_gsi_event()` evaluates `src/actions/armlet.rs::maybe_toggle(...)` inline before silence dispel, danger detection, or hero-specific survivability helpers run. That is also what lets it declare a switch-off's HP before the danger detector reads the same event.
2. `src/input/simulation.rs` places Armlet chords, and the single press that turns it on (`armlet_press`), onto an Armlet-priority backlog that the single `Enigo` worker drains before older normal queued inputs.

The worker still does **not** interrupt an input command that is already mid-execution. Instead, it preserves atomic execution of the current command and then runs the pending Armlet press next before resuming older normal queue backlog.

Supported modifier strings are:

- `Alt`
- `Ctrl` / `Control`
- `Shift`

If the configured modifier is unknown, the runtime logs a warning and falls back to `Alt`.

### Diagnostics and tuning workflow

At the default INFO level every press logs why it fired, the HP the last tick lost to
something other than the toggle, and which way the toggle read
(`Armlet trick: low HP, toggle on (HP 270/1800, lost 95 last tick, trigger 280, …)`),
and every outcome logs what it did (`Armlet trick landed: HP 270 → 1 → 495 in 870ms`,
`Armlet trick cut short: died 450ms after the press (HP 270 → 1)`, or the warnings for
a lost press). Those lines are the data for tuning the trigger: whether a trick under
heavy damage per tick saves the hero or leaves it at 1 HP to die.

The first real game on this code (2026-11-21) showed why that matters. In a practice
lobby 27 of 32 presses landed, in 865–1093ms. In a real Huskar game at 2–3k max HP,
6 of 9 tricks ended in a death 0.2–0.7s after the press, with 130–350 HP lost per tick:
a 280 HP trigger leaves one tick at that rate.

For offline work, `examples/armlet_toggle_control.rs` runs the controller and the old
loop through scripted scenarios, randomised fights under both drain and press-loss
models, and a replay of capture 1:

```powershell
cargo run --release --example armlet_toggle_control
```

For live verification at the input level, enable:

```powershell
$env:RUST_LOG="dota2_scripts::actions::armlet=debug,dota2_scripts::input::simulation=debug"; cargo run --release
```

With that filter, the logs also show the player's own toggles as the tracker reads
them and the synthetic-input worker's executed chord order and per-step timing.

### Shared defaults + hero overrides

The base armlet behavior lives in `[armlet]`.

Supported heroes can override the shared threshold/cooldown values through nested hero config blocks such as `[heroes.huskar.armlet]`. If a hero has no override, the shared defaults apply. Huskar also keeps backward compatibility with its older flat armlet keys if the nested block is absent. `press_confirm_ms` is shared only: it describes the game and the keyboard path, not the hero.

---

## Defensive items

Owned by `src/actions/common.rs::use_defensive_items_if_danger()`.

This path only runs when:

- danger detection is enabled
- `danger_detector::is_in_danger()` is true
- the hero is alive

Current activation order:

1. `item_black_king_bar`
2. `item_satanic`
3. `item_blade_mail`
4. `item_lotus_orb`
5. `item_mjollnir`
6. `item_glimmer_cape`
7. `item_ghost`
8. `item_shivas_guard`

Which of them fire is decided by `plan_defensive_items()`, a pure planner; the
caller only presses the keys it hands back.

Details:

- each item is independently enabled/disabled in `[danger_detection]`
- Mjollnir (Static Charge), Glimmer and Lotus Orb are unit-targeted, so all three are self-cast by double-tapping the bound key — `defensive_item_needs_self_cast()` in `common.rs` is the single list
- from the first self-cast item onwards, `common.rs` queues the rest of the sequence on the shared `ActionExecutor`, so the synchronous GSI lane does not wait on the double tap and later defensive items still stay behind that item's second tap. Each double tap is one input job, `[common].self_cast_tap_gap_ms` apart, with the player's clicks held in between — see "Self-cast double taps hold your clicks" in `docs/features/keyboard-interception.md`
- Satanic has a separate HP gate: `satanic_hp_threshold`

Three items are held back rather than fired blind (see "Defensive item windows"
below):

- **Ghost Scepter** waits while Glimmer Cape is running, and while Glimmer is
  firing on this same tick. They are the same panic button pressed twice, so
  spending both on one moment leaves nothing for the next one. Glimmer opens
  because it hides us on top of the magic resistance.
- **Blade Mail** is skipped whenever nothing can hit us: a live invisibility
  window, a live Glimmer or Ghost Scepter window, or either of those two firing
  on this tick. There is no incoming damage to return, and while hidden the
  activation is also what would drop the fade.
- **Lotus Orb** is skipped only while we are concealed — invisibility window,
  Glimmer window, or a Glimmer firing on this tick. Nothing is aimed at a hero it
  cannot see, and the cast would drop the fade. A Ghost Scepter window is
  deliberately *not* a hold: spells still land on a ghosted hero, so there is
  still something worth reflecting.

For the heuristics that decide when this path runs, see `docs/features/danger-detection.md`.

---

## Neutral items

Owned by `src/actions/common.rs` with item classification in `src/actions/item_automation.rs`.

Neutral-item automation is part of survivability because it is tied to low HP + danger state.

Neutral-item automation now supports three cast modes:

1. `self_cast` - neutral key -> `neutral_items.self_cast_key`
2. `no_target` - neutral key only
3. `cursor_targeted` - neutral key at the current cursor target

Current danger-supported neutrals:

- Self-cast: `item_minotaur_horn`, `item_metamorphic_mandible`
- No-target: `item_jidi_pollen_bag`, `item_ash_legion_shield`, `item_idol_of_screeauk`, `item_kobold_cup`, `item_flicker`, `item_riftshadow_prism`, `item_demonicon`
- Cursor-targeted: `item_crippling_crossbow`, `item_brick`

`item_brick` (offensive) and `item_demonicon` (Book of the Dead) are supported but left
out of the checked-in `allowed_items`.

Configured-but-unsupported neutrals are ignored at runtime unless their support status is upgraded in `src/actions/item_automation.rs`.
Pollywog Charm (ally-target), Seeds of Serenity and Witchbane (`item_heavy_blade`,
both ground-target) stay `KnownUnsupported`. In capture 1 of
`examples/neutral_cast_probe.rs`, with item keys on Quick Cast, a double tap did not
cast Pollywog; only Alt (the self-cast modifier) did. They need a modifier self-cast
before they can be automated.

The **Survivability** page shows the allowlist as a checklist of the current pool, by
tier. The pool comes from `src/models/neutral_catalog.rs`, generated from Valve's
datafeed by `scripts/generate-neutral-catalog.ps1`; rerun it after a gameplay patch.
Only supported items can be ticked. Anything in `allowed_items` that has left the pool
is listed under "Not in the current pool" so it can be removed.

Neutrals Valve cycles out of the pool keep their spec as `Inactive` instead of being
deleted, so they work again if they return. 7.42 retired `item_essence_ring` and
`item_flayers_bota`.

---

## Low mana automation

Shared low-mana automation is dispatcher-owned and runs before hero routing.

Current low-mana supported items:

- `item_arcane_boots`
- `item_mana_draught`

The feature is controlled by `[mana_automation]`. Huskar is excluded by default.

---

## Movement automation

Shared movement automation is dispatcher-owned and runs before hero routing.

Current movement-supported items:

- `item_phase_boots`

The feature is controlled by `[phase_boots_automation]`. It only fires when
`hero.xpos` / `hero.ypos` show the hero has walked at least
`phase_boots_automation.minimum_distance_units` during the current movement
segment. The runtime tracks distance from the start of the current walk instead
of relying on a single GSI sample, then resets once the hero settles in place
again. Small position jitter and stationary farming should stay below that
threshold.

---

## Invisibility hold

Owned by `src/actions/invisibility.rs`, which infers a Shadow Blade / Silver Edge
window from item cooldown edges (GSI exposes no modifiers). One gate,
`invisibility::suppresses_automation(&settings)`, is consulted by every automation
that would end that window:

| Automation | Where the gate sits |
|---|---|
| Healing items | `check_and_use_healing_items_with_danger` |
| Defensive items | `use_defensive_items_if_danger_with_snapshot` |
| Neutral items | `use_neutral_item_if_danger_with_snapshot` |
| Low-mana items | `check_and_use_mana_items` |
| Phase Boots | `eligible_movement_item` |
| Silence dispel | `dispel::check_and_dispel_silence` |
| Slark Dark Pact | `heroes::slark::plan_dark_pact` |

The gate lives at the point of the key press rather than inside the planner
functions, so the planners stay pure and their tests stay order-independent.
Slark is the exception: `plan_dark_pact` takes the answer as an argument for the
same reason.

**Not** gated, deliberately:

- Slark's Shadow Dance and Depth Shroud — they grant invisibility, so they
  replace the window rather than ending it.
- Soul Ring — it fires off your own ability keypress, which has already broken
  invisibility by the time we would act.
- Armlet — its whole job is stopping you dying to the HP drain; being seen is
  the cheaper outcome.

Turn the whole thing off with `[invisibility] suppress_automation = false`.

Blind spot inherited from the tracker: a plain right-click attack breaks
invisibility and produces no GSI signal, so the hold stays on until the window
times out.

---

## Defensive item windows

Owned by `src/actions/defensive_windows.rs`, a second cooldown-edge tracker built
the same way as the invisibility one: a `0 -> N` cooldown edge on the item is the
cast, and that item's own cooldown is the clock for its buff, so a window survives
pauses and follows whatever cooldown the patch ships.

Tracked sources, in `WINDOW_SOURCES`:

| Item | Duration |
|---|---|
| `item_glimmer_cape` | 5s |
| `item_ghost` | 4s |
| `item_mask_of_madness` | 6s (Berserk) |

Mask of Madness is not a defensive item; it is tracked only because Berserk
silences its own caster. Its window holds the silence dispel (see below), and
nothing in the defensive planner reads it.

These are deliberately **not** part of `invisibility::is_invisible()`. That gate
holds every automation in the app; these only feed the defensive planner, which
reads all three through `DefensiveGates::current()`:

| Gate | Holds |
|---|---|
| `invisibility::is_invisible()` | Blade Mail, Lotus Orb |
| Glimmer window | Blade Mail, Lotus Orb, Ghost Scepter |
| Ghost Scepter window | Blade Mail |

Ghost form is not concealment, so it is not part of `DefensiveGates::hidden()` —
it gates Blade Mail on its own reason (nobody can land the attack that Blade Mail
would punish) and leaves Lotus Orb alone.

Two differences from the invisibility tracker, both on purpose:

- nothing closes a window early. Glimmer's magic resistance outlives the fade, so
  a healing item pressed in between must not release Ghost Scepter.
- the Blade Mail and Ghost Scepter holds ignore
  `[invisibility] suppress_automation`. That switch is about not wasting a Shadow
  Blade; these are about not wasting the item being pressed.

Blind spot: a Glimmer cast on an ally looks identical from here, so a manual ally
shield holds our own Ghost Scepter for five seconds.

---

## Silence dispel

Owned by `src/actions/dispel.rs::check_and_dispel_silence()`.

This path is survivability-adjacent but **not** tied to `is_in_danger()`.

`check_and_dispel_silence()` is state bookkeeping around one pure planner,
`plan_dispel()`, which returns `Idle` / `Hold` / `Cast { slot, item }` — the same
shape as Slark's `plan_dark_pact()`, and testable without globals or a clock.

Current rules:

- `Idle` (forget the silence and reset the state) when every toggle is off, the
  hero is dead, or the hero is not silenced
- `Hold` while invisible, and while `stunned`, `hexed`, or `muted` — Dota drops
  item orders issued through those, so a press would be thrown away. The silence
  stays tracked, so the dispel fires on the first tick the lock lifts instead of
  being lost for the rest of the silence
- `Hold` while our own Mask of Madness window is open (6s from its `0 -> N`
  cooldown edge): Berserk's silence is self-inflicted and chosen, so Manta or
  Lotus must not answer it. An enemy silence that outlasts Berserk is still
  dispelled on the first tick after the window closes
- priority holds across the whole inventory: `item_manta`, then the neutral slot's
  `item_flicker`, then `item_minotaur_horn`, then `item_lotus_orb`. Instant casts
  first; Minotaur Horn's 40s cooldown is saved for when Flicker (8s) is down; Lotus
  has a cast point
- only cast if `can_cast == true` and cooldown is `0`
- add random human-like jitter of `30..100ms`
- Lotus self-casts by double-tapping, through the same one-job, click-holding double tap as the danger kit (`[common].self_cast_tap_gap_ms`). Flicker and Minotaur Horn take no target and get a single press (both measured casting from one tap in capture 1 of `examples/neutral_cast_probe.rs`)
- Flicker blinks ~500u forward as it dispels, or stays in place if a player damaged
  the hero in the last 3s
- after a press, wait `PRESS_SETTLE_MS` (600ms) for GSI to confirm it. The
  pressed item going on cooldown is the confirmation; if it is still ready after
  the window, the press never reached the game and it is pressed again
- a confirmed press ends the episode: the hero is still silenced *after* a
  dispel landed, so the silence is undispellable (Doom and other strong debuffs)
  and the second item is not burned on it
- at most `MAX_PRESSES_PER_SILENCE` (4) presses per silence, so a mis-bound key
  cannot turn into key spam

The toggles live under `[danger_detection]`:

- `auto_manta_on_silence`
- `auto_lotus_on_silence`
- `auto_neutral_on_silence` (Flicker and Minotaur Horn)

Witchbane (`item_heavy_blade`) also cleared a silence in capture 1, but from a Quick
Cast tap aimed at the cursor; it only hit the hero because the cursor was close. It
is not a dispel here until a self-targeted press is measured.

---

## `auto_items.rs` and why it belongs here

`src/actions/auto_items.rs` is not the healing loop, but it is part of the shared item-automation surface that survivability changes often touch.

It owns:

- `LATEST_GSI_EVENT` cache
- per-slot castability lookup
- Space+right-click item/ability sequence execution

`src/actions/dispatcher.rs` refreshes that cache on every GSI event with `auto_items::update_gsi_state(event)`.

`src/input/keyboard.rs` later consumes it for Broodmother's blocked right-click combo path.

If you change how shared item availability is read from GSI, check both:

- `src/actions/common.rs`
- `src/actions/auto_items.rs`

---

## UI ownership

The **Survivability** page (`/survivability`, `src-ui/src/pages/Survivability.tsx`)
is the entry point for everything on this page that an operator tunes. It reaches
across three config sections rather than mirroring one:

| Card | Config section | Keys |
|---|---|---|
| Healing Items | `[common]`, `[danger_detection]` | `survivability_hp_threshold`, `healing_threshold_in_danger`, `max_healing_items_per_danger` |
| Lane Phase | `[common]` | `lane_phase_duration_seconds`, `lane_phase_healing_threshold` |
| Defensive Items | `[danger_detection]` | `auto_bkb`, `auto_satanic`, `satanic_hp_threshold`, `auto_blade_mail`, `auto_lotus_orb`, `auto_mjollnir`, `auto_glimmer_cape`, `auto_ghost_scepter`, `auto_shivas_guard` |
| Dispels | `[danger_detection]` | `auto_manta_on_silence`, `auto_lotus_on_silence`, `auto_neutral_on_silence` |
| Neutral Items | `[neutral_items]` | `enabled`, `use_in_danger`, `hp_threshold`, `self_cast_key`, `allowed_items` |
| Invisibility | `[invisibility]` | `suppress_automation` |

The Lane Phase toggle is derived, not stored: it writes
`lane_phase_duration_seconds = 0` to disable and `480` to re-enable, matching the
runtime's "0 disables the override" contract.

Deliberately **not** on that page:

- danger *detection* heuristics — `docs/features/danger-detection.md`, `/danger`
- `[armlet]` — `/armlet`
- `[phase_boots_automation]` — `/boots`. Its invisibility hold is the exception:
  that switch stopped being about Phase Boots when the gate went crate-wide, so
  it lives in the Invisibility card here and `/boots` links across to it.
- `[mana_automation]` — TOML-only; the feature is not shaped well enough to expose yet
- `neutral_items.log_discoveries`, and every `excluded_heroes` list — TOML-only

---

## Config touchpoints

| Section | Keys currently used by survivability code |
|---|---|
| `[common]` | `survivability_hp_threshold`, `lane_phase_duration_seconds`, `lane_phase_healing_threshold` |
| `[armlet]` | `enabled`, `cast_modifier`, `toggle_threshold`, `predictive_offset`, `toggle_cooldown_ms` |
| `[armlet.roshan]` | `enabled`, `toggle_key`, `emergency_margin_hp`, `learning_window_ms`, `min_confidence_hits`, `min_sample_damage`, `stale_reset_ms` |
| `[danger_detection]` | `enabled`, `healing_threshold_in_danger`, `max_healing_items_per_danger`, `auto_bkb`, `auto_satanic`, `satanic_hp_threshold`, `auto_blade_mail`, `auto_lotus_orb`, `auto_mjollnir`, `auto_glimmer_cape`, `auto_ghost_scepter`, `auto_shivas_guard`, `auto_manta_on_silence`, `auto_lotus_on_silence`, `auto_neutral_on_silence` |
| `[heroes.<hero>.armlet]` | optional per-hero `enabled`, `toggle_threshold`, `predictive_offset`, `toggle_cooldown_ms` overrides |
| `[neutral_items]` | `enabled`, `self_cast_key`, `use_in_danger`, `hp_threshold`, `allowed_items` |
| `[mana_automation]` | `enabled`, `mana_threshold_percent`, `excluded_heroes`, `allowed_items` |
| `[phase_boots_automation]` | `enabled`, `minimum_distance_units`, `excluded_heroes` |
| `[invisibility]` | `suppress_automation` |

---

## Related docs

- `docs/features/danger-detection.md`
- `docs/features/keyboard-interception.md`
- `docs/architecture/state-and-dispatch.md`
