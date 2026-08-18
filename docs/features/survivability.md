# Survivability

**Purpose**: Read this before changing generic healing, defensive-item use, neutral-item saves, or silence dispels.

---

## Ownership map

| Path | What it owns |
|---|---|
| `src-ui/src/pages/Survivability.tsx` | The operator-facing **Survivability** page — the single place to configure healing, defensive items, dispels, and neutral items |
| `src/actions/armlet.rs` | Shared armlet planning, config resolution, cooldown tracking, critical retry handling, and dual-trigger execution |
| `src/actions/common.rs` | Shared survivability pipeline: armlet job enqueueing, healing items, defensive items, neutral items |
| `src/actions/danger_detector.rs` | Global `in_danger` heuristic consumed by common and hero code |
| `src/actions/dispel.rs` | Immediate Manta/Lotus reaction to silence |
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

Owned by `src/actions/armlet.rs`.

This path is shared across heroes and fallback survivability flow. The runtime only acts when:

- `[armlet].enabled = true` after per-hero override resolution
- the hero is alive
- the hero currently has `item_armlet`
- the hero is not stunned at the moment a normal toggle would fire
- current HP is below `toggle_threshold + predictive_offset`
- the shared cooldown has elapsed

### Roshan mode

Shared Armlet now includes an optional Roshan-specific mode under `[armlet.roshan]`.

- `enabled = false` by default
- when enabled, the configured `toggle_key` becomes a **blocked global hotkey**
- pressing that hotkey toggles a live armed/disarmed runtime state without sending the key through to Dota 2
- while armed, the module keeps learning recent HP-loss samples that look like Roshan hits

The Roshan path only runs when the normal Armlet decision would otherwise be `SkipSafe`. It does not bypass the existing cooldown or critical-retry handling.

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

1. `dispatch_gsi_event()` evaluates `src/actions/armlet.rs::maybe_toggle(...)` inline before silence dispel, danger detection, or hero-specific survivability helpers run.
2. `src/input/simulation.rs` places Armlet chords onto an Armlet-priority backlog that the single `Enigo` worker drains before older normal queued inputs.

The worker still does **not** interrupt an input command that is already mid-execution. Instead, it preserves atomic execution of the current command and then runs the pending Armlet chord next before resuming older normal queue backlog.

Supported modifier strings are:

- `Alt`
- `Ctrl` / `Control`
- `Shift`

If the configured modifier is unknown, the runtime logs a warning and falls back to `Alt`.

### Diagnostics and tuning workflow

For offline tuning, `src/actions/armlet.rs` now includes replay-model tests that compare threshold and cooldown behavior against sample HP timelines, plus an ignored matrix test that prints comparison rows for several candidate configs.

For live verification, enable:

```powershell
$env:RUST_LOG="dota2_scripts::actions::armlet=debug,dota2_scripts::input::simulation=debug"; cargo run --release
```

With that filter, the logs show both:

- the armlet module's resolved trigger / cooldown decisions
- the synthetic-input worker's executed armlet chord order and per-step timing

Use that combination when you need to confirm whether a real toggle emitted the intended `slot-key -> modifier down -> slot-key -> modifier up` sequence or when you are comparing threshold / cooldown settings during gameplay tests.

### Shared defaults + hero overrides

The base armlet behavior lives in `[armlet]`.

Supported heroes can override the shared threshold/cooldown values through nested hero config blocks such as `[heroes.huskar.armlet]`. If a hero has no override, the shared defaults apply. Huskar also keeps backward compatibility with its older flat armlet keys if the nested block is absent.

### Critical retry

When a toggle fires at extremely low HP (below half the configured base threshold), the module records a critical retry marker. If a later event shows HP still critically low or lower, the module forces one more dual-trigger toggle even if that suggests the previous toggle likely failed to flip the item state cleanly.

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
4. `item_glimmer_cape`
5. `item_ghost`
6. `item_shivas_guard`

Details:

- each item is independently enabled/disabled in `[danger_detection]`
- Glimmer is self-cast by double-tapping the bound key
- when Glimmer appears in the shared defensive-item sequence, `common.rs` queues the Glimmer self-cast tail on the shared `ActionExecutor`, so the synchronous GSI lane does not sleep for the 50ms follow-up timing and later defensive items still stay behind Glimmer's second tap
- Satanic has a separate HP gate: `satanic_hp_threshold`

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

- Self-cast: `item_essence_ring`, `item_minotaur_horn`, `item_metamorphic_mandible`
- No-target: `item_jidi_pollen_bag`, `item_ash_legion_shield`, `item_idol_of_screeauk`, `item_kobold_cup`
- Cursor-targeted: `item_crippling_crossbow`

Configured-but-unsupported neutrals are ignored at runtime unless their support status is upgraded in `src/actions/item_automation.rs`.

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

## Silence dispel

Owned by `src/actions/dispel.rs::check_and_dispel_silence()`.

This path is survivability-adjacent but **not** tied to `is_in_danger()`.

`check_and_dispel_silence()` is state bookkeeping around one pure planner,
`plan_dispel()`, which returns `Idle` / `Hold` / `Cast { slot, item }` — the same
shape as Slark's `plan_dark_pact()`, and testable without globals or a clock.

Current rules:

- `Idle` (forget the silence and reset the state) when both toggles are off, the
  hero is dead, or the hero is not silenced
- `Hold` while invisible, and while `stunned`, `hexed`, or `muted` — Dota drops
  item orders issued through those, so a press would be thrown away. The silence
  stays tracked, so the dispel fires on the first tick the lock lifts instead of
  being lost for the rest of the silence
- prefer `item_manta` across the whole inventory (it is instant; Lotus has a cast
  point), then `item_lotus_orb`
- only cast if `can_cast == true` and cooldown is `0`
- add random human-like jitter of `30..100ms`
- Lotus self-casts by double-tapping
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
| Defensive Items | `[danger_detection]` | `auto_bkb`, `auto_satanic`, `satanic_hp_threshold`, `auto_blade_mail`, `auto_glimmer_cape`, `auto_ghost_scepter`, `auto_shivas_guard` |
| Dispels | `[danger_detection]` | `auto_manta_on_silence`, `auto_lotus_on_silence` |
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
| `[danger_detection]` | `enabled`, `healing_threshold_in_danger`, `max_healing_items_per_danger`, `auto_bkb`, `auto_satanic`, `satanic_hp_threshold`, `auto_blade_mail`, `auto_glimmer_cape`, `auto_ghost_scepter`, `auto_shivas_guard`, `auto_manta_on_silence`, `auto_lotus_on_silence` |
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
