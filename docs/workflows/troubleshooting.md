# Troubleshooting

**Purpose**: Use this page when runtime behavior is wrong and you need the shortest path to the likely owner file.

---

## First 4 checks

1. **Look at the UI event/status panel**
   - if events stay at `0`, GSI is not reaching the app
2. **Run with logs**
   - ``$env:RUST_LOG="debug"; cargo run --release``
3. **Confirm the app is elevated on Windows**
   - `rdev::grab`-based interception often fails without Administrator privileges
4. **Confirm the config you edited is the one being loaded**
   - `config/config.toml`, then restart the app

---

## Symptom map

| Symptom | Likely owner files | What to check |
|---|---|---|
| No GSI events | `src/gsi/server.rs`, `src/gsi/handler.rs`, `config/config.toml` | Port, listener binding, Dota target URL, UI event count |
| Hero never auto-detects | `src/state/app_state.rs`, `src/actions/dispatcher.rs`, `src/models/heroes.rs` | Exact `hero.name`, registration, `HeroType::from_hero_name(...)` |
| Key interception does nothing | `src/input/keyboard.rs`, `src/input/simulation.rs` | Admin privileges, `grab()` startup, blocked-vs-passthrough branch |
| Soul Ring never fires | `src/actions/soul_ring.rs`, `src/input/keyboard.rs`, `config/config.toml` | Mana/HP thresholds, slot mapping, skip-list, cooldown lockout |
| Healing / defensive items never fire | `src/actions/common.rs`, `src/actions/danger_detector.rs`, `config/config.toml` | HP thresholds, danger state, item presence / castability |
| Broodmother macros do nothing | `src/actions/heroes/broodmother.rs`, `src/actions/auto_items.rs`, `src/input/keyboard.rs` | `BROODMOTHER_ACTIVE`, modifier key path, cached GSI state |
| Shadow Fiend razes are wrong | `src/input/keyboard.rs`, `src/actions/heroes/shadow_fiend.rs` | interception enabled, Dota ALT-facing cvar, `raze_delay_ms` |
| Config changes seem ignored | `src/config/settings.rs`, `config/config.toml` | parse failure fallback, unknown-key ignore, restart required |
| Startup crash or immediate exit | `src/main.rs`, `src/gsi/server.rs` | port bind failure, early hook/server failure, log output |

---

## GSI is not reaching the app

### Symptoms

- UI says `No GSI events received yet`
- `Events Processed` stays at `0`
- no `Received GSI event for hero:` logs at debug level

### Checks

1. Confirm the app started the server:
   - `src/gsi/server.rs` binds `127.0.0.1:<port>`
2. Confirm Dota is sending to the same port as `[server].port`
3. Confirm nothing else already owns the port
4. Confirm the app is still running when Dota posts

### Owner files

- `src/gsi/server.rs`
- `src/gsi/handler.rs`
- `docs/reference/gsi-schema-and-usage.md`

---

## Hero is wrong or never auto-detected

### Symptoms

- `Active Hero` stays `None`
- the wrong hero doc or automation path appears active

### Checks

1. Inspect `event.hero.name` in debug logs
2. Check `HeroType::from_hero_name(...)` in `src/state/app_state.rs`
3. Check the `hero_scripts` registration in `src/actions/dispatcher.rs`
4. If you added a new hero, confirm `src/actions/heroes/mod.rs` also exports it

### Note

Not every hero-specific feature uses the generic `HeroType` flow. Broodmother's mouse macros, for example, are keyed off `BROODMOTHER_ACTIVE` from dispatcher updates.

---

## Key interception is not firing

### Symptoms

- pressing the expected key behaves as if the app is not running
- no interception logs appear
- blocked/replayed sequences never happen

### Checks

1. Run as Administrator on Windows
2. Check startup logs for the keyboard listener
3. Look for `Error in keyboard grab listener` from `src/input/keyboard.rs`
4. Verify you are on the branch that actually blocks the original event (`return None`)
5. Confirm the feature gate is enabled in config

### Owner files

- `src/input/keyboard.rs`
- `src/input/simulation.rs`
- `docs/features/keyboard-interception.md`

---

## Shadow Fiend razes are not facing the cursor

### Checks

1. `heroes.shadow_fiend.raze_intercept_enabled = true`
2. Dota console / autoexec contains:

```text
cl_dota_alt_unit_movetodirection 1
```

3. Increase `heroes.shadow_fiend.raze_delay_ms` if the hero is not finishing the turn
4. Confirm Shadow Fiend is actually selected/enabled in app state

### Owner files

- `src/actions/heroes/shadow_fiend.rs`
- `src/input/keyboard.rs`
- `docs/heroes/shadow_fiend.md`

---

## Soul Ring or other key-assisted flows are not firing

### Checks

1. Confirm the item exists in `items.slot0`-`slot5`
2. Confirm `item.can_cast == true`
3. Confirm mana is below `min_mana_percent` unless that value is `100`
4. Confirm health is above `min_health_percent`
5. Confirm the key is in `[soul_ring].ability_keys` or is an eligible item-slot key
6. Confirm the item is not on the Soul Ring skip-list

### Owner files

- `src/actions/soul_ring.rs`
- `src/input/keyboard.rs`
- `docs/features/soul-ring.md`

---

## Shared healing / defensive items are not triggering

### Checks

1. Inspect `hero.health_percent`
2. Inspect the current danger state
3. Confirm the item is present in a checked slot and castable
4. Confirm the relevant toggle in `[danger_detection]` or `[neutral_items]`
5. Remember neutral-item logic only checks `items.neutral0`

### Owner files

- `src/actions/common.rs`
- `src/actions/danger_detector.rs`
- `src/actions/dispel.rs`
- `docs/features/danger-detection.md`
- `docs/features/survivability.md`

---

## Config is not taking effect

### Checks

1. Restart the app after editing `config/config.toml`
2. Compare `config/config.toml` against `docs/reference/configuration.md` and `src/config/settings.rs`
3. Check for typos or misplaced sections
4. Remember:
    - missing keys fall back to Rust defaults
    - unknown keys are ignored by serde unless code says otherwise
    - some checked-in values intentionally differ from fallback defaults
    - parse warnings inside `Settings::load()` happen before tracing is initialized, so fallback-to-defaults may look silent at startup

Use `docs/reference/configuration.md` to compare the checked-in config with `src/config/settings.rs`.

---

## Startup crash / bind failure

### Common causes

- another process already uses the configured GSI port
- config parse failure causes unexpected fallback behavior
- an external dependency such as the global hook fails immediately

### Owner files

- `src/main.rs`
- `src/gsi/server.rs`
- `src/config/settings.rs`

---

## Related docs

- `docs/workflows/testing-and-debugging.md`
- `docs/reference/configuration.md`
- `docs/reference/gsi-schema-and-usage.md`
- `docs/features/keyboard-interception.md`
