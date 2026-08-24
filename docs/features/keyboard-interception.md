# Keyboard Interception

**Purpose**: Read this before editing blocked key behavior, adding a new intercepted key, or changing how synthetic input is replayed.

---

## Ownership map

| Path | What it owns |
|---|---|
| `src/input/keyboard.rs` | Global `rdev::grab` hook, decision tree, `HotkeyEvent` channel, Soul Ring replay helper, Armlet Roshan toggle hotkey, Invoker panic/prep hotkeys |
| `src/actions/heroes/outworld_destroyer.rs` | Outworld Destroyer intercepted-sequence planning and dedicated request worker (`R` combo, self-Astral, standalone combo) |
| `src/actions/soul_ring.rs` | Soul Ring shared state, key eligibility rules, health/mana/cooldown gates |
| `src/actions/heroes/shadow_fiend.rs` | Shadow Fiend intercepted-sequence planning and dedicated request worker (`Q/W/E` razes, `R` ultimate combo, standalone combo) |
| `src/actions/heroes/magnus.rs` | Magnus directional ultimate planning, GSI readiness gate, and dedicated request worker |
| `src/actions/heroes/slark.rs` | Slark directional Pounce planning, GSI readiness gate, and dedicated request worker |
| `src/actions/heroes/mirana.rs` | Mirana directional Leap planning, GSI readiness gate, and dedicated request worker |
| `src/actions/heroes/earth_spirit.rs` | Earth Spirit remnant-combo planning, two GSI readiness gates, and dedicated request worker |
| `src/actions/heroes/invoker.rs` | Invoker combo profiles, invoke planning, dedicated request worker (panic Ghost Walk, prep pairs, primary combo) |
| `src/input/simulation.rs` | High-level synthetic keys/mouse emission + `SIMULATING_KEYS` guard |
| `src/gsi/handler.rs` | Rebuilds the shared `KeyboardSnapshot` when GSI detection changes the hero |
| `src-tauri/src/commands/state.rs` | Rebuilds it on manual hero selection and runtime toggles |
| `src/ui/app.rs` | Per-frame refresh of the shared `KeyboardSnapshot` (legacy egui binary only) |

Related but not primary owners:

- `src/actions/heroes/largo.rs` receives `HotkeyEvent::LargoQ/W/E/R`
- `src/actions/heroes/broodmother.rs` uses mouse interception plus `BROODMOTHER_ACTIVE`
- `src/state/app_state.rs` exposes `trigger_key`, `sf_enabled`, and `od_enabled`

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

- `main.rs` (headless) and `src-tauri/src/lib.rs` (desktop app) each create one `Arc<RwLock<KeyboardSnapshot>>`
- `start_keyboard_listener(...)` receives that shared snapshot
- the callback clones it only on the button/key paths that need static config

**Every per-hero intercept flag on the snapshot is frozen until something
rebuilds it.** The snapshot is built once at startup, before any hero is known,
so these are the rebuild triggers:

| Trigger | Where |
|---|---|
| GSI detection changes the active hero | `src/gsi/handler.rs::process_gsi_events` |
| Manual hero selection | `src-tauri/src/commands/state.rs::select_hero` |
| GSI / standalone / Invoker-profile toggles | `src-tauri/src/commands/state.rs` |
| Any config write | `src-tauri/src/commands/config.rs` |
| Per-frame throttled refresh (**legacy egui binary only**) | `src/ui/app.rs` |

The GSI rebuild is what makes hero intercepts go live without any UI
interaction. It is gated on `AppState::update_from_gsi` reporting a hero change,
so it costs nothing on the steady-state event stream. Without it, picking a hero
in-game leaves Shadow Fiend / Magnus / Slark / Mirana / Earth Spirit / Snapfire intercepts inert until an
unrelated UI toggle happens to refresh the cache — the failure mode covered by
`process_gsi_events_rebuilds_the_keyboard_snapshot_on_hero_detection`.

The snapshot holds only static keyboard-facing facts:

- parsed combo-trigger key
- Shadow Fiend interception flags and delays
- Outworld Destroyer interception flags, keys, and combo config
- Broodmother callback-facing config and pre-parsed keys
- Magnus intercept flags, pre-parsed ultimate key, and turn delay
- Slark intercept flags, pre-parsed Pounce key, and turn delay
- Mirana intercept flags, pre-parsed Leap key, and turn delay
- Earth Spirit intercept flags, pre-parsed grip and roll keys, the shared remnant key, the silence remnant delay, and the roll's double-tap, delay, and remnant self-cast settings
- Soul Ring thresholds, ability keys, and item-slot keys
- Armlet Roshan toggle key when `[armlet.roshan].enabled = true`
- HUD portrait capture key from `[hud]`

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
   - enqueues auto-items/ability execution to the Broodmother callback worker
5. **Broodmother middle mouse**
   - blocks the click
   - enqueues spider micro to the Broodmother callback worker
6. **Calculate Soul Ring eligibility**
   - one live `SOUL_RING_STATE` lock on the keypress path
   - `should_intercept_key_with_config(&snapshot.soul_ring)`
   - `should_trigger_with_config(&snapshot.soul_ring)`
7. **Shadow Fiend raze intercept**
    - if `snapshot.sf_enabled` and `snapshot.shadow_fiend.raze_intercept_enabled`
    - block `Q/W/E`
    - enqueue the raze sequence onto Shadow Fiend's dedicated worker
8. **Shadow Fiend ultimate intercept**
    - if `snapshot.sf_enabled` and `snapshot.shadow_fiend.auto_bkb_on_ultimate`
    - block `R`
    - enqueue the ultimate sequence onto the same dedicated worker
9. **Snapfire directional cookie intercept**
    - if `snapshot.snapfire_enabled` and `snapshot.snapfire.enabled`
    - block the configured trigger key (default `Space`)
    - enqueue the `ALT down -> right-click (face cursor) -> wait -> self-cast W -> ALT up` sequence onto the dedicated Snapfire worker
10. **Magnus directional ultimate intercept**
    - if `snapshot.magnus_enabled` and `snapshot.magnus.enabled`
    - block the configured ultimate key (default `R`) **only** when
      `MagnusState::can_intercept_ultimate()` passes, or when
      `require_ability_ready = false`
    - enqueue the `ALT down -> right-click (face cursor) -> ALT up -> wait -> press R`
      sequence onto the dedicated Magnus worker
    - on a failed readiness check the branch falls through and `R` reaches Dota
11. **Slark directional Pounce intercept**
    - if `snapshot.slark_enabled` and `snapshot.slark.enabled`
    - block the configured Pounce key (default `W`) **only** when
      `SlarkState::can_intercept_pounce()` passes, or when
      `require_ability_ready = false`
    - enqueue the `ALT down -> right-click (face cursor) -> ALT up -> wait -> press W`
      sequence onto the dedicated Slark worker
    - on a failed readiness check the branch falls through and `W` reaches Dota
12. **Mirana directional Leap intercept**
    - if `snapshot.mirana_enabled` and `snapshot.mirana.enabled`
    - block the configured Leap key (default `E`) **only** when
      `MiranaState::can_intercept_leap()` passes, or when
      `require_ability_ready = false`
    - enqueue the `ALT down -> right-click (face cursor) -> ALT up -> wait -> press E`
      sequence onto the dedicated Mirana worker
    - on a failed readiness check the branch falls through and `E` reaches Dota
13. **Earth Spirit remnant intercepts**
    - if `snapshot.earth_spirit_enabled` and `snapshot.earth_spirit.enabled`
    - block the configured grip key (default `E`) when
      `EarthSpiritState::can_intercept_grip()` passes, or when
      `require_grip_ready = false`, and enqueue
      `press remnant -> wait -> press grip`
    - block the configured roll key (default `W`) when
      `EarthSpiritState::can_intercept_roll()` passes, or when
      `require_roll_ready = false`, and enqueue
      `press roll [-> wait -> press roll] -> wait -> self-cast remnant`
    - **the two combos press the remnant on opposite sides**: the grip resolves
      on the press so its remnant must already exist, while the roll's ~600ms
      windup is spent aiming a remnant into the path it already committed to
    - each combo has its own toggle, key, delay, and gate; a failed readiness
      check falls through and that key reaches Dota
14. **Outworld Destroyer intercepts**
    - if `snapshot.od_enabled` and `heroes.outworld_destroyer.ultimate_intercept_enabled`
    - block `R` only when `Sanity's Eclipse` is ready
    - enqueue `BKB -> Objurgation -> R` onto the dedicated OD worker
    - optionally block the configured self-Astral panic hotkey and double-tap Astral on self
15. **Armlet Roshan toggle**
    - if `[armlet.roshan].enabled = true` and the configured hotkey matches
    - emit `HotkeyEvent::ArmletRoshanToggle`
    - block the original key so it does not also reach Dota 2
16. **HUD portrait capture**
    - if `[hud] capture_portrait_key` matches (default `F9`)
    - emit `HotkeyEvent::CaptureHudPortrait`
    - block the original key: it is ours while calibrating
    - planned alongside the other global hotkeys in `plan_global_hotkey_press_event`
17. **Largo / generic ability-key path**
    - emit `HotkeyEvent::LargoQ/W/E/R`
    - if Soul Ring should trigger, block and replay
    - otherwise pass through
18. **Item-slot Soul Ring interception**
     - blocks configured item keys when the item is mana-using and Soul Ring should fire first
19. **Standalone combo key**
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

Used by the Soul Ring replay worker because the original physical key was swallowed by `grab()`.

### `src/input/simulation.rs`

Uses a lazy, single-consumer enigo worker for higher-level combos and still owns the actual synthetic input emission for those sequences:

- `press_key(char)`
- `mouse_click()`
- `left_click()`
- `alt_down()`
- `alt_up()`

The helper API keeps its prior blocking timing semantics, but each call now submits work onto one unbounded FIFO queue owned inside `src/input/simulation.rs` and waits for the worker to finish that command. The worker thread is started lazily on first use and owns the real `Enigo` instance, so higher-level combo code no longer contends on an implicit global `Mutex<Enigo>` in caller threads.

The Enigo-backed worker now tracks queue depth, queued total, peak depth, drops, and completions. Those metrics are for the synthetic-input lane only and are exposed via `synthetic_input_metrics()` in the debug UI. Soul Ring replay remains a separate path with its own dedicated worker.

**Key chars are lowercased before they reach enigo.** `Key::Unicode('W')` makes
enigo synthesize `Shift+W`, which Dota reads as "queue the ability" rather than
"cast it". Hand-written config is lowercase, but the UI's `KeyInput` uppercases
single characters, so any `char` key rebound through a hero config panel would
otherwise arrive shifted. `normalize_key_char` in `src/input/simulation.rs`
applies to `press_key`, `key_down`, `key_up`, and the armlet chord.

`SIMULATING_KEYS` is still managed by this path:

- `press_key`, `mouse_click`, `left_click`, and `alt_up` keep the brief post-action guard window before the worker restores the flag
- `alt_down` sets the flag and keeps it active across later queued commands until the matching queued `alt_up` runs
- this preserves FIFO replay ordering for Shadow Fiend facing sequences without changing the `rdev::simulate` Soul Ring replay path

Used by:

- Shadow Fiend raze facing (`ALT` + right-click + raze key)
- Shadow Fiend ultimate / standalone combo
- Outworld Destroyer ultimate / self-Astral / standalone combo
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

A key is only intercepted when the thing bound to it is **known to spend mana**, priced
from `src/actions/mana_costs.rs`. See `docs/features/soul-ring.md` for how that table is
built and regenerated.

- ability keys listed in `[soul_ring].ability_keys`, but only when the bound ability is
  learned, not passive, and has a non-zero mana cost at its current level
- item slot keys, but only when:
  - `[soul_ring].intercept_item_keys = true`
  - the key is not Soul Ring's own slot key
  - the slot holds an item with a non-zero mana cost

Nothing is intercepted while the hero is silenced, muted, or hexed — the press would be
dropped by the game anyway.

### Replay flow

`spawn_soul_ring_then_key(original_key, snapshot.soul_ring.clone())`:

1. enqueues a `SoulRingReplayRequest` onto one dedicated lazy worker queue
2. the callback returns immediately

The lazy worker thread:

1. receives the request from the unbounded FIFO queue
2. locks `SOUL_RING_STATE`
3. computes the replay plan (`TriggerThenOriginal` or `OriginalOnly`) while still holding the lock
4. if eligible, marks state as triggered, then releases the lock
5. replays Soul Ring's slot key via `rdev::simulate`
6. waits `delay_before_ability_ms`
7. replays the original blocked key via `rdev::simulate`

If the worker queue is unexpectedly closed, `spawn_soul_ring_then_key` falls back to spawning a short-lived thread with identical replay logic.

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

1. enqueues one `Raze` request onto a dedicated Shadow Fiend worker
2. the worker sleeps briefly
3. the worker calls `src/input/simulation.rs` helpers to hold `ALT`
4. the worker calls `src/input/simulation.rs` helpers to right-click and face direction
5. the worker releases `ALT`
6. the worker waits `heroes.shadow_fiend.raze_delay_ms`
7. the worker presses the raze key through `src/input/simulation.rs`

### `R` ultimate path

If `heroes.shadow_fiend.auto_bkb_on_ultimate = true`, the hook blocks `R` and calls `execute_ultimate_combo(...)`.

That helper:

- enqueues one `Ultimate` request onto the same dedicated Shadow Fiend worker
- reads `SF_LAST_EVENT` for inventory state
- attempts BKB if available
- optionally presses `D`
- then presses `R`
- uses `src/input/simulation.rs` for the actual synthetic key presses

### Standalone combo

The standalone hotkey is **not** a blocked-key intercept. It travels through the `HotkeyEvent` channel and ends up at `handle_standalone_trigger()`.

That standalone-key conflict remains unchanged in this slice and is still out of scope here: the checked-in config exposes `heroes.shadow_fiend.standalone_key`, but current runtime wiring still conflicts with the raze-intercept path when that path uses `Q`.

---

## Outworld Destroyer interception

### Activation gate

The callback reads `snapshot.od_enabled`, which is rebuilt from `AppState.od_enabled`.

That source flag is updated when:

- `AppState::update_from_gsi(...)` detects `npc_dota_hero_obsidian_destroyer`
- the UI manually changes `selected_hero`

### `R` ultimate path

If `heroes.outworld_destroyer.ultimate_intercept_enabled = true`, the hook checks `R` before the generic ability-key path.

When `Sanity's Eclipse` is ready, the hook:

1. blocks the original `R`
2. enqueues one `Ultimate` request onto the OD worker
3. optionally uses BKB
4. optionally uses `Objurgation`
5. presses `R`
6. optionally presses Arcane Orb after the ultimate

If the ultimate is not ready, the hook does not swallow `R`.

### Self-Astral path

If `heroes.outworld_destroyer.astral_self_cast_enabled = true`, the hook also watches the configured `astral_self_cast_key`.

That path:

1. blocks the dedicated panic hotkey
2. checks whether `Astral Imprisonment` is ready from cached GSI state
3. enqueues a request that double-taps the configured Astral key

### Standalone combo

Like Tiny and Legion Commander, OD also uses the generic standalone combo trigger routed through `HotkeyEvent::ComboTrigger` and `handle_standalone_trigger()`.

The standalone combo itself runs on the OD worker and currently sequences Blink, optional BKB, configured combo items, optional `Objurgation`, `Sanity's Eclipse`, and optional Arcane Orb follow-up presses.

---

## Largo and Broodmother notes

These are still part of the interception surface even though this page centers on `keyboard.rs`, `soul_ring.rs`, and `shadow_fiend.rs`.

### Largo

- `Q/W/E/R` emit `HotkeyEvent::LargoQ/W/E/R`
- the original key is only blocked when Soul Ring also needs to fire first
- `main.rs` downcasts to `LargoScript` for manual song selection / beat-loop stop

### Snapfire

- activation is gated on `snapshot.snapfire_enabled`, derived from `selected_hero == Some(HeroType::Snapfire)`
- the configured trigger key (default `Space`) is blocked and enqueues one `CookieLeap` request onto the dedicated Snapfire worker
- the worker holds `ALT`, right-clicks to face the cursor, waits `turn_delay_ms`, self-casts the cookie key (`W`), then releases `ALT`
- `W` itself is never intercepted, so manual ally cookies still work
- because the trigger defaults to `Space`, it shares the `MODIFIER_KEY_HELD` Space tracking, but only fires while Snapfire is the active hero (Broodmother's Space handling is gated separately on `BROODMOTHER_ACTIVE`)

### Magnus

- activation is gated on `snapshot.magnus_enabled`, derived from `selected_hero == Some(HeroType::Magnus)`
- the configured ultimate key (default `R`) is blocked and enqueues one `DirectionalUltimate` request onto the dedicated Magnus worker
- the worker holds `ALT`, right-clicks to face the cursor, releases `ALT`, waits `turn_delay_ms`, then presses the ultimate key
- when `center_camera_on_ultimate = true` it then double-taps the configured hero-select key to recentre the camera. This runs **after** the cast, via `simulate_key` rather than the enigo helpers so named keys like `F1` work
- ALT is held only across the right-click — Reverse Polarity takes no target, so the cast does not need the modifier (this is the one difference from the Snapfire sequence)
- unlike Snapfire, the intercept is **gated on GSI**: `MagnusState::can_intercept_ultimate()` reads `MAGNUS_LAST_EVENT` and requires `magnataur_reverse_polarity` to have `level > 0 && can_cast`. A failed check leaves the key unblocked
- Skewer is never intercepted

### Slark

- activation is gated on `snapshot.slark_enabled`, derived from `selected_hero == Some(HeroType::Slark)`
- the configured Pounce key (default `W`) is blocked and enqueues one `DirectionalPounce` request onto the dedicated Slark worker
- the worker holds `ALT`, right-clicks to face the cursor, releases `ALT`, waits `turn_delay_ms`, then presses the Pounce key
- ALT is released before the ability press for the same reason as Magnus: Pounce takes no target, and holding ALT over an ability key pings it instead of casting it
- like Magnus, the intercept is **gated on GSI**: `SlarkState::can_intercept_pounce()` reads `SLARK_LAST_EVENT` and requires `slark_pounce` to have `level > 0 && can_cast`. A failed check leaves the key unblocked
- Dark Pact, Saltwater Shiv, and Shadow Dance are never intercepted

### Mirana

- activation is gated on `snapshot.mirana_enabled`, derived from `selected_hero == Some(HeroType::Mirana)`
- the configured Leap key (default `E`) is blocked and enqueues one `DirectionalLeap` request onto the dedicated Mirana worker
- the worker holds `ALT`, right-clicks to face the cursor, releases `ALT`, waits `turn_delay_ms`, then presses the Leap key — byte-for-byte the Slark sequence, because Leap and Pounce are the same kind of ability
- ALT is released before the ability press for the same reason as Magnus and Slark: Leap takes no target, and holding ALT over an ability key pings it instead of casting it
- the intercept is **gated on GSI**: `MiranaState::can_intercept_leap()` reads `MIRANA_LAST_EVENT` and requires `mirana_leap` to have `level > 0 && can_cast`. A failed check leaves the key unblocked
- **Leap is charge-based**, and how GSI reports `can_cast` for a banked charge is unverified against a live payload. If the gate proves wrong in-game the symptom is a silently inert feature; `require_ability_ready = false` is the escape hatch
- Sacred Arrow, Starstorm, and Moonlight Shadow are never intercepted

### Earth Spirit

- activation is gated on `snapshot.earth_spirit_enabled`, derived from `selected_hero == Some(HeroType::EarthSpirit)`
- **two independent intercepts, not one.** Both of Earth Spirit's signature plays are a Stone Remnant plus one more ability aimed at the same cursor position, so each is remapped onto its second key:
  - the grip key (default `E`) enqueues `press remnant -> wait -> press grip`. That is the silence: Geomagnetic Grip pulls the remnant back through whoever stands between the cursor and Earth Spirit
  - the roll key (default `W`) enqueues `press roll -> wait -> self-cast remnant`, optionally pressing the roll key a second time to fire it. A roll through a remnant travels 1600 units instead of 800
- **the roll is the one combo that casts before placing.** Rolling Boulder has a ~600ms windup, and a remnant dropped into the path during it still counts, so casting first locks the direction in
- **the roll's remnant is self-cast**, via ALT held across the press (`roll_remnant_alt`), a double-tap (`roll_remnant_double_tap`), or both. Self-cast puts the stone on Earth Spirit, where the roll starts, so the boulder passes through it with no aiming. ALT is held across both taps rather than pulsed, since Dota reads the modifier when the cast resolves
- all three roll delays share one ceiling: their sum must stay under `ROLLING_BOULDER_WINDUP_MS`
- unlike the Magnus / Slark / Mirana combos there is **no right-click**: both abilities take a point, and neither cares which way Earth Spirit is facing
- each intercept is **gated on GSI** against its own ability: `can_intercept_grip()` requires `earth_spirit_geomagnetic_grip`, `can_intercept_roll()` requires `earth_spirit_rolling_boulder`, both with `level > 0 && can_cast`. A failed check leaves that key unblocked
- **neither gate reads Stone Remnant.** It is charge-based, and GSI's `can_cast` is unreliable for charge abilities — the same trap flagged for Mirana's Leap above. Gating on it would leave both combos inert while remnants are visibly banked; the trade is that a zero-charge press still fires the combo
- `roll_double_tap` exists because Rolling Boulder is commonly left off quickcast, where the first press only arms the cursor. With it off the combo sends a single press and the double-tap delay is skipped entirely
- Boulder Smash and Magnetize are never intercepted

### Broodmother

- Broodmother callback actions now queue to one dedicated worker instead of spawning raw threads from the callback
- Space + right-click blocks the click and enqueues auto-items/ability execution to the Broodmother callback worker
- middle mouse blocks the click and enqueues spider micro to the same worker
- activation is keyed off `BROODMOTHER_ACTIVE`, not `AppState.selected_hero`
- Soul Ring remains on its own separate dedicated replay worker

### Invoker

- Invoker hotkeys now come from `heroes.invoker.profiles[].hotkey`
- The keyboard layer emits `HotkeyEvent::InvokerProfile(<id>)` for enabled Invoker profiles
- Profile hotkeys only fire when `AppState.selected_hero == Invoker`
- Hotkeys do not block the original key - they pass through to Dota 2
- Requests are enqueued to a dedicated Invoker worker that handles named profile planning and execution
- The generic standalone trigger still resolves to the first enabled combo profile for compatibility with shared runtime plumbing
- No orb interception - Invoker does not modify Q/W/E/R/D/F core gameplay

---

## Config that matters

| Area | Path | Keys |
|---|---|---|
| Soul Ring | `config/config.toml` -> `[soul_ring]` | `enabled`, `min_mana_percent`, `min_health_percent`, `delay_before_ability_ms`, `trigger_cooldown_ms`, `ability_keys`, `intercept_item_keys` |
| Armlet Roshan | `config/config.toml` -> `[armlet.roshan]` | `enabled`, `toggle_key` |
| Shadow Fiend | `config/config.toml` -> `[heroes.shadow_fiend]` | `raze_intercept_enabled`, `raze_delay_ms`, `auto_bkb_on_ultimate`, `auto_d_on_ultimate` |
| Magnus | `config/config.toml` -> `[heroes.magnus]` | `enabled`, `ultimate_key`, `turn_delay_ms`, `require_ability_ready`, `center_camera_on_ultimate`, `camera_center_key`, `camera_center_delay_ms` |
| Slark | `config/config.toml` -> `[heroes.slark]` | `enabled`, `pounce_key`, `turn_delay_ms`, `require_ability_ready` |
| Mirana | `config/config.toml` -> `[heroes.mirana]` | `enabled`, `leap_key`, `turn_delay_ms`, `require_ability_ready` |
| Earth Spirit | `config/config.toml` -> `[heroes.earth_spirit]` | `enabled`, `remnant_key`, `silence_combo_enabled`, `grip_key`, `silence_remnant_delay_ms`, `require_grip_ready`, `roll_combo_enabled`, `roll_key`, `roll_double_tap`, `roll_double_tap_delay_ms`, `roll_to_remnant_delay_ms`, `roll_remnant_alt`, `roll_remnant_double_tap`, `roll_remnant_double_tap_delay_ms`, `require_roll_ready` |
| HUD anchors | `config/config.toml` -> `[hud]` | `capture_portrait_key` (blocked from reaching Dota), plus the stored portrait fractions |
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
- `docs/heroes/magnus.md`
- `docs/heroes/slark.md`
- `docs/heroes/mirana.md`
- `docs/heroes/earth_spirit.md`
- `docs/features/hud-anchors.md`
- `docs/architecture/runtime-flow.md`
