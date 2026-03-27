# Survivability

**Purpose**: Read this before changing generic healing, defensive-item use, neutral-item saves, or silence dispels.

---

## Ownership map

| Path | What it owns |
|---|---|
| `src/actions/armlet.rs` | Shared armlet planning, config resolution, cooldown tracking, critical retry handling, and dual-trigger execution |
| `src/actions/common.rs` | Shared survivability pipeline: armlet job enqueueing, healing items, defensive items, neutral items |
| `src/actions/danger_detector.rs` | Global `in_danger` heuristic consumed by common and hero code |
| `src/actions/dispel.rs` | Immediate Manta/Lotus reaction to silence |
| `src/actions/auto_items.rs` | Cached GSI item/ability state and Space+right-click item usage; not the HP-healing loop, but part of the shared item automation surface |
| `src/config/settings.rs` | `CommonConfig`, shared `ArmletAutomationConfig`, hero armlet overrides, `DangerDetectionConfig`, `NeutralItemConfig` defaults |
| `config/config.toml` | Checked-in values for `[common]`, `[armlet]`, `[danger_detection]`, `[neutral_items]`, and hero armlet overrides |

---

## Shared GSI survivability pipeline

### Fallback path

For heroes without a registered script, `src/actions/common.rs::SurvivabilityActions::execute_default_strategy()` runs:

1. armlet handling (short background thread)
2. `danger_detector::update(...)`
3. `check_and_use_healing_items(...)`
4. `use_defensive_items_if_danger(...)`
5. `use_neutral_item_if_danger(...)`

### Hero-script path

Registered hero scripts currently call the same shared helpers manually from their own `handle_gsi_event(...)` implementations.

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
| Normal | `common.survivability_hp_threshold` | `30` |
| Danger | `danger_detection.healing_threshold_in_danger` | `50` |

### Item order

Current code checks items in this exact order:

| Mode | Exact order |
|---|---|
| Normal | `item_cheese` -> `item_faerie_fire` -> `item_magic_wand` -> `item_enchanted_mango` -> `item_greater_faerie_fire` |
| Danger | `item_cheese` -> `item_greater_faerie_fire` -> `item_enchanted_mango` -> `item_magic_wand` -> `item_faerie_fire` |

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

Owned by `src/actions/common.rs::use_neutral_item_if_danger()`.

Neutral-item automation is part of survivability because it is tied to low HP + danger state.

Current requirements:

- hero alive
- `[neutral_items].enabled = true`
- `[neutral_items].use_in_danger = true`
- `danger_detector::is_in_danger() == true`
- `event.hero.health_percent < neutral_items.hp_threshold`
- neutral item present in `event.items.neutral0`
- neutral item name included in `neutral_items.allowed_items`
- `neutral_item.can_cast == Some(true)`

When triggered, the code:

1. validates the neutral item against the existing danger, HP, allowed-item, and `can_cast` gates
2. queues a self-cast sequence on the shared `ActionExecutor`
3. inside that executor job, presses the neutral slot key, waits 50ms, then presses `neutral_items.self_cast_key`

---

## Silence dispel

Owned by `src/actions/dispel.rs::check_and_dispel_silence()`.

This path is survivability-adjacent but **not** tied to `is_in_danger()`.

Current rules:

- if the hero is not silenced, reset `DISPEL_TRIGGERED`
- while silenced, trigger at most once per silence
- prefer `item_manta`
- otherwise try `item_lotus_orb`
- only cast if `can_cast == true` and cooldown is `0`
- add random human-like jitter of `30..100ms`
- Lotus self-casts by double-tapping

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

## Config touchpoints

| Section | Keys currently used by survivability code |
|---|---|
| `[common]` | `survivability_hp_threshold` |
| `[armlet]` | `enabled`, `cast_modifier`, `toggle_threshold`, `predictive_offset`, `toggle_cooldown_ms` |
| `[danger_detection]` | `enabled`, `healing_threshold_in_danger`, `max_healing_items_per_danger`, `auto_bkb`, `auto_satanic`, `satanic_hp_threshold`, `auto_blade_mail`, `auto_glimmer_cape`, `auto_ghost_scepter`, `auto_shivas_guard`, `auto_manta_on_silence`, `auto_lotus_on_silence` |
| `[heroes.<hero>.armlet]` | optional per-hero `enabled`, `toggle_threshold`, `predictive_offset`, `toggle_cooldown_ms` overrides |
| `[neutral_items]` | `enabled`, `self_cast_key`, `use_in_danger`, `hp_threshold`, `allowed_items` |

---

## Related docs

- `docs/features/danger-detection.md`
- `docs/features/keyboard-interception.md`
- `docs/architecture/state-and-dispatch.md`
