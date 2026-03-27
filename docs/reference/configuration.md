# Configuration Reference

**Purpose**: Use this page when you need the exact config surface in `config/config.toml`, the Rust fallback defaults in `src/config/settings.rs`, or the next doc to read for behavior details.

---

## Load model and constraints

| Rule | Source |
|---|---|
| Runtime loads `config/config.toml` once at startup via `Settings::load()` | `src/config/settings.rs` |
| If the file is missing or TOML parsing fails, the app falls back to `Settings::default()` for the whole config | `src/config/settings.rs` |
| Missing sections / missing keys inside a valid file fall back per-field because the structs use `#[serde(default)]` | `src/config/settings.rs` |
| The checked-in `config/config.toml` is **not** the same as the Rust fallback defaults for every field; treat both as important | `config/config.toml`, `src/config/settings.rs` |
| `RUST_LOG` overrides `[logging].level` at process start | `src/main.rs` |
| Duplicate inventory/neutral keybindings only warn; the app still starts | `src/config/settings.rs` |
| Most hotkey-like string fields must be supported by `src/input/keyboard.rs::parse_key_string()` (`Home`, `End`, `Insert`, `Delete`, `PageUp`, `PageDown`, `F1`-`F12`, or one character) | `src/input/keyboard.rs` |

**Deeper behavior docs**

- Shared survivability and neutral items: `docs/features/survivability.md`
- Danger heuristics and defensive items: `docs/features/danger-detection.md`
- Keyboard interception and replay: `docs/features/keyboard-interception.md`
- Soul Ring flow: `docs/features/soul-ring.md`
- Update system: `docs/features/updates.md`
- Hero-specific behavior: `docs/heroes/*.md`

---

## `[server]`

| Field | `config/config.toml` | Rust fallback if omitted | Notes |
|---|---:|---:|---|
| `port` | `3000` | `3000` | GSI server binds `127.0.0.1:<port>` in `src/gsi/server.rs`; Dota's GSI config must target the same port. |

## `[updates]`

| Field | `config/config.toml` | Rust fallback if omitted | Notes |
|---|---:|---:|---|
| `check_on_startup` | `true` | `true` | If true, `src/main.rs` starts a background update check. See `docs/features/updates.md`. |
| `include_prereleases` | `false` | `false` | Passed into `check_for_update(...)`; affects which GitHub Releases qualify. |

## `[keybindings]`

| Field | `config/config.toml` | Rust fallback if omitted | Notes |
|---|---:|---:|---|
| `slot0` | `"z"` | `"z"` | One-character item hotkey; used when the runtime maps `items.slot0` to a key press. |
| `slot1` | `"x"` | `"x"` | Same constraint as above. |
| `slot2` | `"c"` | `"c"` | Same constraint as above. |
| `slot3` | `"v"` | `"v"` | Same constraint as above. |
| `slot4` | `"b"` | `"b"` | Same constraint as above. |
| `slot5` | `"n"` | `"n"` | Same constraint as above. |
| `neutral0` | `"0"` | `"0"` | Neutral-item self-cast key and neutral-slot mapping. |
| `combo_trigger` | `"Home"` | `"Home"` | Present in settings, but current runtime wiring does not read this field when installing the keyboard listener; the live standalone trigger comes from `AppState.trigger_key` and hero-specific selection logic instead. See `src/ui/app.rs`, `src/main.rs`, and `docs/workflows/adding-a-hero.md`. |

**Constraint**: `slot0`-`slot5` and `neutral0` deserialize to `char`, so TOML values must be single-character strings.

**Runtime note**: the UI shows the currently active standalone trigger from `AppState.trigger_key`. Changing the selected hero updates that live key without restarting the app, but editing `config/config.toml` still requires a restart because settings load once at startup.

## `[logging]`

| Field | `config/config.toml` | Rust fallback if omitted | Notes |
|---|---:|---:|---|
| `level` | `"info"` | `"info"` | Used unless `RUST_LOG` is set. See `docs/workflows/testing-and-debugging.md`. |

## `[common]`

| Field | `config/config.toml` | Rust fallback if omitted | Notes |
|---|---:|---:|---|
| `survivability_hp_threshold` | `30` | `30` | Base HP% threshold for shared healing when danger mode is not active. See `docs/features/survivability.md`. |

## `[armlet]`

| Field | `config/config.toml` | Rust fallback if omitted | Notes |
|---|---:|---:|---|
| `enabled` | `true` | `true` | Master switch for shared armlet automation. The runtime still only acts when the current hero is alive and actually has Armlet equipped. |
| `cast_modifier` | `"Alt"` | `"Alt"` | Shared cast-side modifier paired with the quick-cast slot key. Supported values are `Alt`, `Ctrl` / `Control`, and `Shift`. Unknown values warn and fall back to `Alt`. |
| `toggle_threshold` | `320` | `320` | Shared raw-HP base threshold before any hero-specific overrides are applied. |
| `predictive_offset` | `30` | `30` | Shared extra HP buffer added to the base threshold. |
| `toggle_cooldown_ms` | `250` | `250` | Shared cooldown between armlet toggle attempts. |

See `docs/features/survivability.md`.

## `[danger_detection]`

| Field | `config/config.toml` | Rust fallback if omitted | Notes |
|---|---:|---:|---|
| `enabled` | `true` | `true` | Master switch for `src/actions/danger_detector.rs`. |
| `hp_threshold_percent` | `70` | `70` | Enter danger when HP% drops below this line. |
| `rapid_loss_hp` | `100` | `100` | Raw HP loss needed inside `time_window_ms` to flag danger. |
| `time_window_ms` | `500` | `500` | Sampling window for rapid HP loss. |
| `clear_delay_seconds` | `3` | `3` | Delay before clearing the danger state. |
| `healing_threshold_in_danger` | `50` | `50` | Shared healing threshold while danger mode is active. |
| `max_healing_items_per_danger` | `3` | `3` | Shared healing item budget per danger episode. |
| `auto_bkb` | `false` | `false` | Shared defensive-item toggle. |
| `auto_satanic` | `true` | `true` | Shared defensive-item toggle. |
| `satanic_hp_threshold` | `40` | `40` | Additional HP gate for Satanic. |
| `auto_blade_mail` | `true` | `true` | Shared defensive-item toggle. |
| `auto_glimmer_cape` | `true` | `true` | Shared defensive-item toggle. |
| `auto_ghost_scepter` | `true` | `true` | Shared defensive-item toggle. |
| `auto_shivas_guard` | `true` | `true` | Shared defensive-item toggle. |
| `auto_manta_on_silence` | `true` | `true` | Read by `src/actions/dispel.rs`; silence dispels are configured here even though they are not gated on danger. |
| `auto_lotus_on_silence` | `true` | `true` | Same note as above. |

See `docs/features/danger-detection.md` and `docs/features/survivability.md`.

## `[neutral_items]`

| Field | `config/config.toml` | Rust fallback if omitted | Notes |
|---|---:|---:|---|
| `enabled` | `true` | `false` | Checked-in config enables the feature; code fallback does not. |
| `self_cast_key` | `"0"` | `" "` (space) | `char` field; must be a single-character string. |
| `log_discoveries` | `false` | `true` | When true, `src/actions/dispatcher.rs` appends discoveries to `logs/neutral_items_discovered.txt`. |
| `use_in_danger` | `true` | `true` | Additional gate for danger-triggered neutral usage. |
| `hp_threshold` | `50` | `50` | Only used when danger criteria and feature gates allow it. |
| `allowed_items` | 14-item allowlist | empty list | Must use GSI item names such as `item_essence_ring`. Only `items.neutral0` is checked. |

See `docs/features/survivability.md`.

## `[gsi_logging]`

| Field | `config/config.toml` | Rust fallback if omitted | Notes |
|---|---:|---:|---|
| `enabled` | `false` | `false` | Enables JSONL event logging in `src/gsi/handler.rs`. |
| `output_dir` | `"logs/gsi_events"` | `"logs/gsi_events"` | Directory is created on startup if possible. |

See `docs/reference/gsi-schema-and-usage.md`.

## `[soul_ring]`

| Field | `config/config.toml` | Rust fallback if omitted | Notes |
|---|---:|---:|---|
| `enabled` | `true` | `true` | Master switch for `src/actions/soul_ring.rs`. |
| `min_mana_percent` | `100` | `90` | `100` means "always eligible on mana"; checked-in config is more permissive than the fallback. |
| `min_health_percent` | `20` | `20` | Soul Ring is skipped when HP% is `<=` this threshold. |
| `delay_before_ability_ms` | `30` | `30` | Wait between pressing Soul Ring and replaying the blocked key. |
| `trigger_cooldown_ms` | `10` | `500` | Checked-in config is far more aggressive than the fallback lockout. |
| `ability_keys` | `["q","w","e","r","d","f"]` | same list | Compared case-insensitively. |
| `intercept_item_keys` | `true` | `true` | Item-key interception still skips items in `SOUL_RING_SKIP_ITEMS`. |

See `docs/features/soul-ring.md` and `docs/features/keyboard-interception.md`.

---

## Hero sections

## `[heroes.huskar]`

| Field | `config/config.toml` | Rust fallback if omitted | Notes |
|---|---:|---:|---|
| `berserker_blood_key` | `"e"` | `"e"` | `char` field; one-character string only. |
| `berserker_blood_delay_ms` | `300` | `300` | Delay between first debuff detection and cleanse attempt. |
| `standalone_key` | `"Home"` | `"Home"` | Used by the generic combo-trigger path. |

### `[heroes.huskar.armlet]`

| Field | `config/config.toml` | Rust fallback if omitted | Notes |
|---|---:|---:|---|
| `enabled` | omitted | inherits `[armlet].enabled` | Optional per-hero override. If omitted, Huskar inherits the shared armlet master switch. |
| `toggle_threshold` | `120` | inherits `[armlet].toggle_threshold` | Checked-in Huskar config overrides the shared base threshold. |
| `predictive_offset` | `150` | inherits `[armlet].predictive_offset` | Checked-in Huskar config overrides the shared predictive buffer. |
| `toggle_cooldown_ms` | `300` | inherits `[armlet].toggle_cooldown_ms` | Checked-in Huskar config overrides the shared cooldown. |

**Compatibility note:** legacy flat Huskar keys (`armlet_toggle_threshold`, `armlet_predictive_offset`, `armlet_toggle_cooldown_ms`) are still read when the nested `[heroes.huskar.armlet]` block is absent, so older local configs do not lose Huskar-specific tuning.

See `docs/heroes/huskar.md`.

## `[heroes.legion_commander]`

| Field | `config/config.toml` | Rust fallback if omitted | Notes |
|---|---:|---:|---|
| `standalone_key` | `"Home"` | `"Home"` | Generic combo-trigger key. |

See `docs/heroes/legion_commander.md`.

## `[heroes.shadow_fiend]`

| Field | `config/config.toml` | Rust fallback if omitted | Notes |
|---|---:|---:|---|
| `raze_intercept_enabled` | `true` | `true` | Gates the `Q/W/E` intercept in `src/input/keyboard.rs`. |
| `raze_delay_ms` | `10` | `100` | Checked-in config is much faster than the code fallback. |
| `auto_bkb_on_ultimate` | `true` | `false` | Checked-in config enables auto-BKB by default. |
| `auto_d_on_ultimate` | `true` | `false` | Checked-in config enables pre-ultimate `D` by default. |
| `standalone_key` | `"Home"` | `"Home"` | This field exists in config, but `Settings::get_standalone_key("shadow_fiend")` currently hardcodes `"q"` instead of returning this value. See `src/config/settings.rs` and `docs/heroes/shadow_fiend.md`. |

See `docs/heroes/shadow_fiend.md` and `docs/features/keyboard-interception.md`.

## `[heroes.tiny]`

| Field | `config/config.toml` | Rust fallback if omitted | Notes |
|---|---:|---:|---|
| `standalone_key` | `"Home"` | `"Home"` | Generic combo-trigger key. |

See `docs/heroes/tiny.md`.

## `[heroes.largo]`

| Field | `config/config.toml` | Rust fallback if omitted | Notes |
|---|---:|---:|---|
| `amphibian_rhapsody_enabled` | `true` | `true` | Master switch for Largo automation. |
| `auto_toggle_on_danger` | `true` | `true` | Disables the ultimate when danger logic says to bail out. |
| `mana_threshold_percent` | `20` | `20` | Low-mana shutdown threshold. |
| `heal_hp_threshold` | `50` | `50` | HP gate for switching to Island Elixir. |
| `beat_interval_ms` | `995` | `995` | Manual beat timing anchor. |
| `beat_correction_ms` | `30` | `-10` | Checked-in config overrides the code fallback in the opposite direction. |
| `beat_correction_every_n_beats` | `5` | `5` | `0` disables periodic correction. |
| `q_ability_key` | `"q"` | `"q"` | One-character string only. |
| `w_ability_key` | `"w"` | `"w"` | One-character string only. |
| `e_ability_key` | `"e"` | `"e"` | One-character string only. |
| `r_ability_key` | `"r"` | `"r"` | One-character string only. |
| `standalone_key` | `"Home"` | `"Home"` | This field exists in config, but current runtime does not read a Largo-specific value. Largo manual toggles still flow through the shared trigger key stored in `AppState.trigger_key`, so this behaves like documentation/config drift today. |

See `docs/heroes/largo.md`.

## `[heroes.broodmother]`

| Field | `config/config.toml` | Rust fallback if omitted | Notes |
|---|---:|---:|---|
| `spider_micro_enabled` | `true` | `true` | Enables the middle-mouse spider macro. |
| `spider_control_group_key` | `"F3"` | `"F2"` | Parsed by `parse_key_string()`. |
| `reselect_hero_key` | `"1"` | `"F1"` | Parsed by `parse_key_string()`. |
| `attack_key` | `"a"` | `"a"` | Present in config; current spider move uses right-click rather than this field. |
| `auto_items_enabled` | `true` | `false` | Checked-in config enables the Space+right-click combo; fallback does not. |
| `auto_items_modifier` | `"Space"` | `"Space"` | The field exists in config, but current runtime interception is hardwired to the physical Space key in `src/input/keyboard.rs` rather than reading this value dynamically. |
| `auto_items` | `["orchid","bloodthorn","diffusal","disperser","nullifier","abyssal_blade"]` | empty list | Partial item-name matches against inventory item names. |
| `auto_abilities` | two inline-table entries | empty list | Each entry uses `{ index, key, hp_threshold? }`; `index` maps to `abilities.ability0`-`ability5`. |
| `auto_abilities_first` | `false` | `false` | `false` = items first. |

See `docs/heroes/broodmother.md` and `docs/reference/gsi-schema-and-usage.md`.

---

## When you change config surface area

1. Update `src/config/settings.rs`
2. Update `config/config.toml`
3. Update the affected hero/feature doc in `docs/heroes/` or `docs/features/`
4. Update this page if the top-level reference changed
5. If the change adds new GSI assumptions, also update `docs/reference/gsi-schema-and-usage.md`
