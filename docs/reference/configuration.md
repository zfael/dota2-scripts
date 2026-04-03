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
| `lane_phase_duration_seconds` | `480` | `480` | Duration of the global lane-phase healing override. Set to `0` to disable it. |
| `lane_phase_healing_threshold` | `12` | `12` | HP% threshold used during `0 <= map.clock_time < lane_phase_duration_seconds`, before normal or danger healing thresholds are considered. |

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

## `[rune_alerts]`

| Field | `config/config.toml` | Rust fallback if omitted | Notes |
|---|---:|---:|---|
| `enabled` | `true` | `true` | Enables backend rune-alert window tracking. The Tauri UI may still show the countdown even when alerts are disabled. |
| `alert_lead_seconds` | `10` | `10` | Number of seconds before a rune spawn when the alert window opens. |
| `interval_seconds` | `120` | `120` | Rune cadence used by `src/observability/rune_alerts.rs`. |
| `audio_enabled` | `true` | `true` | In the Tauri app, gates the frontend Web Audio tone. Setting this to `false` silences audio without removing the timer. |

See `src/observability/rune_alerts.rs`, `src-ui/src/hooks/useRuneAlert.ts`, and `src-ui/src/pages/Settings.tsx`.

## `[minimap_capture]`

| Field | `config/config.toml` | Rust fallback if omitted | Notes |
|---|---:|---:|---|
| `enabled` | `false` | `false` | Enables the read-only minimap capture worker. |
| `minimap_x` | `10` | `10` | Minimap crop X offset relative to the Dota 2 client area. |
| `minimap_y` | `815` | `815` | Minimap crop Y offset relative to the Dota 2 client area. |
| `minimap_width` | `260` | `260` | Minimap crop width relative to the Dota 2 client area. |
| `minimap_height` | `260` | `260` | Minimap crop height relative to the Dota 2 client area. |
| `capture_interval_ms` | `1000` | `1000` | Worker capture cadence. |
| `sample_every_n` | `30` | `30` | Persist every Nth successful capture. |
| `artifact_output_dir` | `"logs/minimap_capture"` | `"logs/minimap_capture"` | Output directory for sampled captures and diagnostics. |

## `[minimap_analysis]`

| Field | `config/config.toml` | Rust fallback if omitted | Notes |
|---|---:|---:|---|
| `enabled` | `false` | `false` | Enable hero detection analysis on captured frames |
| `baseline_frames` | `10` | `10` | Number of initial frames used to build the static baseline mask |
| `baseline_threshold` | `0.8` | `0.8` | Fraction of frames a pixel must appear in to be considered static |
| `min_cluster_size` | `20` | `20` | Minimum pixel count for a cluster to be considered a hero icon |
| `max_cluster_size` | `200` | `200` | Maximum pixel count (larger clusters are not hero icons) |
| `red_hue_max` | `15.0` | `15.0` | Upper bound of red hue range (0–15) |
| `red_hue_min_wrap` | `340.0` | `340.0` | Lower bound of red hue wrap range (340–360) |
| `red_min_saturation` | `40.0` | `40.0` | Minimum HSV saturation for red detection |
| `red_min_value` | `30.0` | `30.0` | Minimum HSV brightness for red detection |
| `green_hue_min` | `80.0` | `80.0` | Lower bound of green hue range |
| `green_hue_max` | `160.0` | `160.0` | Upper bound of green hue range |
| `green_min_saturation` | `35.0` | `35.0` | Minimum HSV saturation for green detection |
| `green_min_value` | `25.0` | `25.0` | Minimum HSV brightness for green detection |

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

## `[heroes.outworld_destroyer]`

| Field | `config/config.toml` | Rust fallback if omitted | Notes |
|---|---:|---:|---|
| `standalone_key` | `"Home"` | `"Home"` | Generic combo-trigger key used for the OD engage combo. |
| `objurgation_key` | `"e"` | `"e"` | One-character synthetic key used whenever the script casts `Objurgation`. |
| `arcane_orb_key` | `"q"` | `"q"` | One-character synthetic key used for optional post-ultimate Orb presses. |
| `astral_imprisonment_key` | `"w"` | `"w"` | One-character synthetic key used for the self-Astral helper. |
| `auto_objurgation_on_danger` | `true` | `true` | Enables danger-driven passive `Objurgation`. |
| `objurgation_hp_threshold_percent` | `55` | `55` | HP% gate for passive `Objurgation`. |
| `objurgation_min_mana_percent` | `25` | `25` | Minimum mana% required before passive `Objurgation` is allowed. |
| `objurgation_trigger_cooldown_ms` | `1500` | `1500` | Local anti-spam cooldown between passive `Objurgation` attempts. |
| `ultimate_intercept_enabled` | `true` | `true` | If true, `src/input/keyboard.rs` checks OD's `R` before the generic Soul Ring / Largo path. |
| `auto_bkb_on_ultimate` | `true` | `true` | If true, OD attempts BKB before `Sanity's Eclipse`. |
| `auto_objurgation_on_ultimate` | `true` | `true` | If true, OD attempts `Objurgation` before `Sanity's Eclipse`. |
| `post_bkb_delay_ms` | `50` | `50` | Delay after BKB in the intercepted or standalone combo. |
| `post_blink_delay_ms` | `100` | `100` | Delay after Blink in the standalone combo. |
| `astral_self_cast_enabled` | `false` | `false` | Enables the dedicated self-Astral panic hotkey. |
| `astral_self_cast_key` | `"F5"` | `"F5"` | Parsed by `parse_key_string()` and intercepted separately from the main combo trigger. |
| `combo_items` | `["sheepstick","bloodthorn"]` | empty list | Ordered item-name substrings used during the standalone combo. |
| `combo_item_spam_count` | `1` | `1` | Number of presses per configured combo item. |
| `combo_item_delay_ms` | `50` | `50` | Delay between combo-item presses. |
| `post_ultimate_arcane_orb_presses` | `0` | `0` | Optional number of Arcane Orb presses after the ultimate. |
| `arcane_orb_press_interval_ms` | `30` | `30` | Delay between post-ultimate Orb presses. |

See `docs/heroes/outworld_destroyer.md` and `docs/features/keyboard-interception.md`.

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

## `[heroes.meepo]`

| Field | `config/config.toml` | Rust fallback if omitted | Notes |
|---|---:|---:|---|
| `standalone_key` | `"Home"` | `"Home"` | Generic combo-trigger key for Meepo's standalone combo. |
| `earthbind_key` | `"q"` | `"q"` | One-character key for Earthbind ability. |
| `poof_key` | `"w"` | `"w"` | One-character key for Poof ability. |
| `dig_key` | `"d"` | `"d"` | One-character key for Dig ability. |
| `megameepo_key` | `"f"` | `"f"` | One-character key for Megameepo ability. |
| `post_blink_delay_ms` | `80` | `80` | Delay after Blink before continuing combo sequence. |
| `combo_items` | `["sheepstick", "disperser"]` | same list | Ordered item-name substrings used during standalone combo. |
| `combo_item_spam_count` | `1` | `1` | Number of presses per configured combo item. |
| `combo_item_delay_ms` | `40` | `40` | Delay between combo-item presses. |
| `earthbind_press_count` | `2` | `2` | Number of Earthbind presses in combo sequence. |
| `earthbind_press_interval_ms` | `30` | `30` | Delay between Earthbind presses. |
| `poof_press_count` | `3` | `3` | Number of Poof presses in combo sequence. |
| `poof_press_interval_ms` | `35` | `35` | Delay between Poof presses. |
| `auto_dig_on_danger` | `true` | `true` | Enables automatic Dig cast when in danger state. |
| `dig_hp_threshold_percent` | `32` | `32` | HP% threshold for auto-Dig when danger-triggered. |
| `auto_megameepo_on_danger` | `true` | `true` | Enables automatic Megameepo cast when in danger state. |
| `megameepo_hp_threshold_percent` | `45` | `45` | HP% threshold for auto-Megameepo when danger-triggered. |
| `defensive_trigger_cooldown_ms` | `1500` | `1500` | Local anti-spam cooldown between defensive ability attempts. |

### `[heroes.meepo.farm_assist]`

| Field | `config/config.toml` | Rust fallback if omitted | Notes |
|---|---:|---:|---|
| `enabled` | `true` | `true` | Master switch for Meepo's manual farm-assist mode. The mode still stays idle until the toggle hotkey is pressed. |
| `toggle_key` | `"End"` | `"End"` | Parsed by `parse_key_string()` and only exposed when Meepo is the selected hero. |
| `pulse_interval_ms` | `700` | `700` | Minimum delay between farm-assist pulses. |
| `minimum_mana_percent` | `35` | `35` | Farm pulses suspend when current mana% drops below this threshold. |
| `minimum_health_percent` | `45` | `45` | Farm pulses suspend when current HP% drops below this threshold. |
| `right_click_after_poof` | `true` | `true` | If true, a right-click at current cursor position follows each successful Poof pulse. |
| `suspend_on_danger` | `true` | `true` | If true, `in_danger` suspends farm assist immediately. |
| `suspend_after_manual_combo_ms` | `2500` | `2500` | Cooldown window after the manual standalone combo before farm assist can rearm. |
| `poof_press_count` | `1` | `1` | Number of Poof presses per farm pulse. |
| `poof_press_interval_ms` | `35` | `35` | Delay between Poof presses inside one farm pulse. |

### `[heroes.meepo.armlet]`

| Field | `config/config.toml` | Rust fallback if omitted | Notes |
|---|---:|---:|---|
| `enabled` | omitted | inherits `[armlet].enabled` | Optional per-hero override for the shared armlet master switch. |
| `toggle_threshold` | omitted | inherits `[armlet].toggle_threshold` | Optional per-hero override for the shared armlet base threshold. |
| `predictive_offset` | omitted | inherits `[armlet].predictive_offset` | Optional per-hero override for the shared predictive buffer. |
| `toggle_cooldown_ms` | omitted | inherits `[armlet].toggle_cooldown_ms` | Optional per-hero override for the shared cooldown. |

---

## When you change config surface area

1. Update `src/config/settings.rs`
2. Update `config/config.toml`
3. Update the affected hero/feature doc in `docs/heroes/` or `docs/features/`
4. Update this page if the top-level reference changed
5. If the change adds new GSI assumptions, also update `docs/reference/gsi-schema-and-usage.md`
