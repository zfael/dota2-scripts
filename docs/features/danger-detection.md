# Danger Detection

**Purpose**: Read this before tuning HP heuristics, changing what counts as danger, or altering the automated response that danger enables.

---

## Source of truth

| Path | What it owns |
|---|---|
| `src/actions/danger_detector.rs` | Cross-event HP tracker; `update(...)` owns the current danger decision and `is_in_danger()` exposes the persisted flag |
| `src/actions/common.rs` | Healing, defensive items, and neutral items; the shared survivability pass reuses one current-event danger result instead of re-reading the tracker mid-pass |
| `src/actions/dispel.rs` | Silence dispels configured under `[danger_detection]`, but not gated by `in_danger` |
| `src/config/settings.rs` | `DangerDetectionConfig` defaults and serde wiring |
| `config/config.toml` | Checked-in runtime values |
| `src/ui/app.rs` | Danger Detection tab and main-tab danger indicator |

---

## Current heuristic

`src/actions/danger_detector.rs::update(event, config)` uses a process-wide `HP_TRACKER` mutex with:

- `last_hp`
- `last_hp_percent`
- `last_update`
- `danger_detected`
- `danger_start_time`

### Entering danger

Danger turns on when **either** condition is true:

1. **Rapid HP loss**
   - `hp_delta > rapid_loss_hp`
   - `time_delta_ms < time_window_ms`
2. **Low HP while still losing HP**
   - `current_hp_percent < hp_threshold_percent`
   - `hp_delta > 0`

Current checked-in defaults from `config/config.toml`:

| Key | Default |
|---|---|
| `hp_threshold_percent` | `70` |
| `rapid_loss_hp` | `100` |
| `time_window_ms` | `500` |

### Leaving danger

Danger only clears when:

- the current event no longer satisfies the enter conditions, **and**
- at least `clear_delay_seconds` have elapsed since `danger_start_time`

Current default:

| Key | Default |
|---|---|
| `clear_delay_seconds` | `3` |

### Important implementation details

- The tracker resets immediately when the hero dies.
- The first live GSI event only seeds the tracker; it never triggers danger.
- The clear timer is measured from when danger was first entered, not from the latest safe event.
- `is_in_danger()` is global process state, not stored inside `AppState`.

---

## What danger changes

Danger does not act on its own. `danger_detector::update(...)` still owns the cross-event tracker, and common survivability now computes one current-event `in_danger` result up front and reuses that snapshot through the healing, defensive-item, and neutral-item checks for the same GSI event. Direct `danger_detector::is_in_danger()` reads outside that shared pass are unchanged.

### 1. Healing thresholds

Owned by `src/actions/common.rs::check_and_use_healing_items()` and the event-snapshot variant used by the shared survivability pass.

| Mode | Threshold source | Default | Max items per call |
|---|---|---|---|
| Normal | `common.survivability_hp_threshold` | `30` | `1` |
| Danger | `danger_detection.healing_threshold_in_danger` | `50` | `danger_detection.max_healing_items_per_danger` (`3`) |

**Code nuance**: `max_healing_items_per_danger` is enforced per call to `check_and_use_healing_items()`. The current implementation does **not** track a once-per-danger-window total across multiple GSI events.

### 2. Healing item order

Also owned by `check_and_use_healing_items()`.

Current hardcoded priority:

| Mode | Exact order in code |
|---|---|
| Normal | `item_cheese` -> `item_faerie_fire` -> `item_magic_wand` -> `item_enchanted_mango` -> `item_greater_faerie_fire` |
| Danger | `item_cheese` -> `item_greater_faerie_fire` -> `item_enchanted_mango` -> `item_magic_wand` -> `item_faerie_fire` |

Hardcoded heal values used in the doc/review context:

| Item | Heal assumed by code/comments |
|---|---|
| `item_cheese` | `2000` |
| `item_greater_faerie_fire` | `350` |
| `item_enchanted_mango` | `175` |
| `item_magic_wand` | `100` |
| `item_faerie_fire` | `85` |

### 3. Defensive items

Owned by `src/actions/common.rs::use_defensive_items_if_danger()` and the event-snapshot variant used by the shared survivability pass.

Current activation order:

1. `item_black_king_bar`
2. `item_satanic`
3. `item_blade_mail`
4. `item_glimmer_cape`
5. `item_ghost`
6. `item_shivas_guard`

Behavior details:

- each item must be enabled in config
- the item must exist in inventory
- `item.can_cast` must be `true`
- all eligible enabled items are attempted in one pass
- `item_glimmer_cape` is double-tapped in `use_item()` for self-cast
- `item_satanic` has its own HP gate: `hp_percent <= satanic_hp_threshold`

### 4. Neutral items in danger

Owned by `src/actions/common.rs::use_neutral_item_if_danger()` and the event-snapshot variant used by the shared survivability pass.

Danger is also used as a gate for neutral item self-cast automation when all of these are true:

- the current danger result is `true` (either the per-event snapshot in the shared pass or a direct `is_in_danger()` read elsewhere)
- `[neutral_items].enabled = true`
- `[neutral_items].use_in_danger = true`
- `event.hero.health_percent < neutral_items.hp_threshold`
- the neutral slot item is in `neutral_items.allowed_items`
- the neutral item can cast

---

## Silence dispels configured under `[danger_detection]`

`src/actions/dispel.rs::check_and_dispel_silence()` reads:

- `danger_detection.auto_manta_on_silence`
- `danger_detection.auto_lotus_on_silence`

These toggles are **related to the danger feature area**, but they do **not** require `is_in_danger() == true`.

Current behavior:

- reset `DISPEL_TRIGGERED` when silence ends
- while silenced, trigger at most once per silence
- prefer `item_manta` first
- otherwise try `item_lotus_orb`
- execute on a short background thread with `30..100ms` random jitter
- Lotus uses a double-tap for self-cast

---

## Configuration knobs

Defaults come from `src/config/settings.rs`; checked-in values live in `config/config.toml`.

| Key | Type | Default | Exposed in `src/ui/app.rs`? | Effect |
|---|---|---|---|---|
| `enabled` | `bool` | `true` | Yes | Master gate for `danger_detector::update()` and danger-aware behavior |
| `hp_threshold_percent` | `u32` | `70` | Yes | Low-HP trigger threshold |
| `rapid_loss_hp` | `u32` | `100` | Yes | HP loss needed for burst-damage trigger |
| `time_window_ms` | `u64` | `500` | Yes | Window for burst-damage trigger |
| `clear_delay_seconds` | `u64` | `3` | Yes | Minimum time from danger start before clearing |
| `healing_threshold_in_danger` | `u32` | `50` | Yes | Danger-mode healing threshold |
| `max_healing_items_per_danger` | `u32` | `3` | Yes | Per-call cap while in danger |
| `auto_bkb` | `bool` | `false` | Yes | Auto-use BKB while in danger |
| `auto_satanic` | `bool` | `true` | Yes | Auto-use Satanic while in danger |
| `satanic_hp_threshold` | `u32` | `40` | Yes | Additional HP gate for Satanic |
| `auto_blade_mail` | `bool` | `true` | Yes | Auto-use Blade Mail while in danger |
| `auto_glimmer_cape` | `bool` | `true` | Yes | Auto-use Glimmer Cape while in danger |
| `auto_ghost_scepter` | `bool` | `true` | Yes | Auto-use Ghost Scepter while in danger |
| `auto_shivas_guard` | `bool` | `true` | Yes | Auto-use Shiva's Guard while in danger |
| `auto_manta_on_silence` | `bool` | `true` | No | Use Manta when silenced |
| `auto_lotus_on_silence` | `bool` | `true` | No | Use Lotus Orb when silenced |

---

## UI ownership

`src/ui/app.rs` currently exposes:

- a red `⚠️ IN DANGER` indicator on the **Main** tab
- a **Danger Detection** tab for:
  - heuristic sliders
  - danger healing sliders
  - defensive item toggles
  - `satanic_hp_threshold`
  - saving settings

It does **not** currently expose:

- `auto_manta_on_silence`
- `auto_lotus_on_silence`
- neutral-item danger settings

Those remain TOML-only.

---

## Related docs

- `docs/features/survivability.md` - shared healing / dispel / neutral-item behavior
- `docs/architecture/runtime-flow.md` - where danger detection runs in the GSI pipeline
- `docs/architecture/state-and-dispatch.md` - why `in_danger` lives outside `AppState`
