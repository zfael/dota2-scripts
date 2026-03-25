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

### Platform note

This feature assumes the app can install a global keyboard hook on Windows. If interception stops working, check elevation/OS-hook permissions before changing logic.

---

## Decision tree in `src/input/keyboard.rs`

Current callback order on key/button input:

1. **Ignore our own simulated input**
   - `SIMULATING_KEYS` -> immediate pass-through
2. **Track Space**
   - updates `MODIFIER_KEY_HELD`
3. **Broodmother Space + right-click**
   - blocks the click
   - spawns `auto_items::execute_auto_items(...)`
4. **Broodmother middle mouse**
   - blocks the click
   - sends `HotkeyEvent::BroodmotherSpiderAttack`
   - also spawns `execute_spider_attack_move(...)`
5. **Calculate Soul Ring eligibility**
   - `SOUL_RING_STATE.should_intercept_key(...)`
   - `SOUL_RING_STATE.should_trigger(...)`
6. **Shadow Fiend raze intercept**
   - if `sf_enabled` and `raze_intercept_enabled`
   - block `Q/W/E`
7. **Shadow Fiend ultimate intercept**
   - if `sf_enabled` and `auto_bkb_on_ultimate`
   - block `R`
8. **Largo / generic ability-key path**
   - emit Largo events for `Q/W/E/R`
   - if Soul Ring should trigger, block and replay
   - otherwise pass through
9. **Item-slot Soul Ring interception**
   - blocks configured item keys when the item is mana-using and Soul Ring should fire first
10. **Standalone combo key**
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

`src/actions/dispatcher.rs` refreshes that state on every GSI event via `update_from_gsi(...)`.

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

`should_trigger(settings)` requires:

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

`spawn_soul_ring_then_key(original_key, settings)`:

1. lock `SOUL_RING_STATE`
2. if eligible, mark as triggered
3. replay Soul Ring's slot key
4. wait `delay_before_ability_ms`
5. replay the original blocked key

This runs on a short-lived thread so the `grab()` callback can return immediately.

---

## Shadow Fiend interception

### Activation gate

The keyboard hook reads `AppState.sf_enabled`.

That flag is updated when:

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
- optionally double-taps BKB
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
| Global hotkey | `config/config.toml` -> `[keybindings]` | slot key mappings; `combo_trigger` exists in config, but the live standalone trigger currently comes from `AppState.trigger_key` |

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
