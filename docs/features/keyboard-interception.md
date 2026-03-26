# Keyboard Interception

**Purpose**: Read this before editing blocked key behavior, adding a new intercepted key, or changing how synthetic input is replayed.

---

## Ownership map

| Path | What it owns |
|---|---|
| `src/input/keyboard.rs` | Global `rdev::grab` hook, decision tree, `HotkeyEvent` channel, Soul Ring replay helper |
| `src/actions/soul_ring.rs` | Soul Ring shared state, key eligibility rules, health/mana/cooldown gates |
| `src/actions/heroes/shadow_fiend.rs` | Shadow Fiend intercepted sequences (`Q/W/E` razes, `R` ultimate combo) |
| `src/input/simulation.rs` | High-level synthetic keys/mouse + `SIMULATING_KEYS` guard |
| `src/ui/app.rs` | Per-frame refresh of the shared `KeyboardSnapshot` |

Related but not primary owners:

- `src/actions/heroes/largo.rs` receives `HotkeyEvent::LargoQ/W/E/R`
- `src/actions/heroes/broodmother.rs` uses mouse interception plus `BROODMOTHER_ACTIVE`
- `src/state/app_state.rs` exposes `trigger_key` and `sf_enabled`

---

## Core model

The repo uses **global interception**, not per-window polling:

- `start_keyboard_listener()` spawns a thread
- that thread installs `rdev::grab(callback)`
- returning `None` from the callback **blocks** the original OS event
- returning `Some(event)` passes the event through unchanged

If the app blocks a key, it must replay the desired behavior itself.

### Cached keyboard snapshot

The hot callback no longer locks and clones full runtime config on every event.

- `main.rs` creates one `Arc<RwLock<KeyboardSnapshot>>`
- `start_keyboard_listener(...)` receives that shared snapshot
- `Dota2ScriptApp::update(...)` refreshes it every frame from current `Settings` + `AppState`
- the callback clones it only on the button/key paths that need static config

The snapshot holds only static keyboard-facing facts:

- parsed combo-trigger key
- Shadow Fiend interception flags and delays
- Broodmother callback-facing config and pre-parsed keys
- Soul Ring thresholds, ability keys, and item-slot keys

It does **not** replace live Soul Ring runtime state. Cooldowns, mana, health, alive state, Soul Ring availability, and slot-to-item contents still come from `SOUL_RING_STATE`, which is refreshed from GSI. That means moving an item between slots in-game still updates the interception path once GSI reports the new inventory layout.

### Platform note

This feature assumes the app can install a global keyboard hook on Windows. If interception stops working, check elevation/OS-hook permissions before changing logic.

---

## Decision tree in `src/input/keyboard.rs`

Current callback order on key/button input:

1. **Ignore our own simulated input**
   - `SIMULATING_KEYS` -> immediate pass-through
2. **Read snapshot once**
   - clone `KeyboardSnapshot` from the shared `RwLock`
3. **Track Space**
   - updates `MODIFIER_KEY_HELD`
4. **Broodmother Space + right-click**
   - blocks the click
   - spawns `auto_items::execute_auto_items(...)`
5. **Broodmother middle mouse**
   - blocks the click
   - sends `HotkeyEvent::BroodmotherSpiderAttack`
   - also spawns `execute_spider_attack_move(...)`
6. **Calculate Soul Ring eligibility**
   - one live `SOUL_RING_STATE` lock on the keypress path
   - `should_intercept_key_with_config(&snapshot.soul_ring)`
   - `should_trigger_with_config(&snapshot.soul_ring)`
7. **Shadow Fiend raze intercept**
   - if `snapshot.sf_enabled` and `snapshot.shadow_fiend.raze_intercept_enabled`
   - block `Q/W/E`
8. **Shadow Fiend ultimate intercept**
   - if `snapshot.sf_enabled` and `snapshot.shadow_fiend.auto_bkb_on_ultimate`
   - block `R`
9. **Largo / generic ability-key path**
   - emit `HotkeyEvent::LargoQ/W/E/R`
   - if Soul Ring should trigger, block and replay
   - otherwise pass through
10. **Item-slot Soul Ring interception**
   - blocks configured item keys when the item is mana-using and Soul Ring should fire first
11. **Standalone combo key**
   - sends `HotkeyEvent::ComboTrigger`
   - does not block the original key

Because this logic is ordered, a new intercept can easily shadow an older one. Preserve ordering deliberately.

---

## Re-emitting blocked input

Two different replay mechanisms exist.

### `src/input/keyboard.rs::simulate_key()`

Uses `rdev::simulate` to replay a blocked physical key:

- sets `SIMULATING_KEYS = true`
- emits key press + key release
- clears `SIMULATING_KEYS`

Used by Soul Ring interception because the original physical key was swallowed by `grab()`.

### `src/input/simulation.rs`

Uses enigo for higher-level combos:

- `press_key(char)`
- `mouse_click()`
- `left_click()`
- `alt_down()`
- `alt_up()`

Used by:

- Shadow Fiend raze facing (`ALT` + right-click + raze key)
- Shadow Fiend ultimate / standalone combo
- Broodmother auto-items and spider control
- self-cast item helpers like Glimmer double-tap

---

## Soul Ring interception

### State owner

`src/actions/soul_ring.rs` owns `SOUL_RING_STATE: LazyLock<Arc<Mutex<SoulRingState>>>`.

`src/gsi/handler.rs::process_gsi_events()` refreshes that state on every GSI event via `update_from_gsi(...)`, even when the main GSI automation toggle is off.

The keyboard callback now combines that live state with static config from `snapshot.soul_ring`.

### What `SoulRingState` tracks

- whether Soul Ring is present
- which slot key it uses
- whether it can cast
- hero mana percent
- hero health percent
- whether the hero is alive
- `last_triggered` cooldown lockout
- a `slot_items` map for item-key skip checks

### When a key is eligible

`should_trigger_with_config(&snapshot.soul_ring)` requires:

- `[soul_ring].enabled = true`
- Soul Ring present and castable
- hero alive
- mana below `min_mana_percent` (unless it is `100`, which means "always")
- health above `min_health_percent`
- cooldown lockout elapsed (`trigger_cooldown_ms`)

### Which keys can be intercepted

- ability keys listed in `[soul_ring].ability_keys`
- item slot keys, but only when:
  - `[soul_ring].intercept_item_keys = true`
  - the key is not Soul Ring's own slot key
  - the item is not in `SOUL_RING_SKIP_ITEMS`

### Replay flow

`spawn_soul_ring_then_key(original_key, snapshot.soul_ring.clone())`:

1. lock `SOUL_RING_STATE`
2. if eligible, mark as triggered
3. replay Soul Ring's slot key
4. wait `delay_before_ability_ms`
5. replay the original blocked key

This runs on a short-lived thread so the `grab()` callback can return immediately.

---

## Shadow Fiend interception

### Activation gate

The callback reads `snapshot.sf_enabled`, which is rebuilt from `AppState.sf_enabled`.

That source flag is updated when:

- `AppState::update_from_gsi(...)` detects `npc_dota_hero_nevermore`
- the UI manually changes `selected_hero`

### `Q/W/E` raze path

`src/input/keyboard.rs` blocks `Q/W/E` and calls `ShadowFiendState::execute_raze(...)`.

`src/actions/heroes/shadow_fiend.rs` then:

1. sleeps briefly
2. holds `ALT`
3. right-clicks to face direction
4. releases `ALT`
5. waits `heroes.shadow_fiend.raze_delay_ms`
6. presses the raze key

### `R` ultimate path

If `heroes.shadow_fiend.auto_bkb_on_ultimate = true`, the hook blocks `R` and calls `execute_ultimate_combo(...)`.

That helper:

- reads `SF_LAST_EVENT` for inventory state
- attempts BKB if available
- optionally presses `D`
- then presses `R`

### Standalone combo

The standalone hotkey is **not** a blocked-key intercept. It travels through the `HotkeyEvent` channel and ends up at `handle_standalone_trigger()`.

---

## Largo and Broodmother notes

These are still part of the interception surface even though this page centers on `keyboard.rs`, `soul_ring.rs`, and `shadow_fiend.rs`.

### Largo

- `Q/W/E/R` emit `HotkeyEvent::LargoQ/W/E/R`
- the original key is only blocked when Soul Ring also needs to fire first
- `main.rs` downcasts to `LargoScript` for manual song selection / beat-loop stop

### Broodmother

- Space + right-click blocks the click and uses cached GSI state from `src/actions/auto_items.rs`
- middle mouse blocks the click and triggers spider micro
- activation is keyed off `BROODMOTHER_ACTIVE`, not `AppState.selected_hero`

---

## Config that matters

| Area | Path | Keys |
|---|---|---|
| Soul Ring | `config/config.toml` -> `[soul_ring]` | `enabled`, `min_mana_percent`, `min_health_percent`, `delay_before_ability_ms`, `trigger_cooldown_ms`, `ability_keys`, `intercept_item_keys` |
| Shadow Fiend | `config/config.toml` -> `[heroes.shadow_fiend]` | `raze_intercept_enabled`, `raze_delay_ms`, `auto_bkb_on_ultimate`, `auto_d_on_ultimate` |
| Global hotkey | `config/config.toml` -> `[keybindings]` | slot key mappings; the live standalone trigger is read from `AppState.trigger_key` and cached as a parsed `snapshot.trigger_key` |

---

## Editing checklist

When you add or change an intercept:

1. update `src/input/keyboard.rs` decision ordering
2. verify whether the original key should be blocked or passed through
3. choose the replay mechanism (`simulate_key` vs `input/simulation.rs`)
4. ensure `SIMULATING_KEYS` still prevents self-reinterception
5. update the owning feature/hero doc if behavior changed

Related docs:

- `docs/features/soul-ring.md`
- `docs/heroes/shadow_fiend.md`
- `docs/architecture/runtime-flow.md`
