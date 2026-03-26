# Runtime Flow

**Purpose**: Use this doc to trace execution order, queueing, thread ownership, and blocked key paths.

---

## Startup sequence

Boot starts in `src/main.rs::main()`:

1. **Load settings** from `config/config.toml` into `Arc<Mutex<Settings>>`
2. **Initialize tracing** using `RUST_LOG` or `settings.logging.level`
3. **Create shared state** with `AppState::new()` (`Arc<Mutex<AppState>>`)
4. **Create dispatcher** with `ActionDispatcher::new(settings.clone())`
5. **Start keyboard hook** with `start_keyboard_listener()`
   - spawns the `rdev::grab` thread in `src/input/keyboard.rs`
   - returns `Receiver<HotkeyEvent>` to `main.rs`
6. **Spawn GSI server task**
   - `main.rs` calls `tokio::spawn(start_gsi_server(...))`
7. **Optionally spawn update check**
   - guarded by `settings.updates.check_on_startup`
   - uses `tokio::task::spawn_blocking` and writes into `AppState.update_state`
8. **Spawn hotkey handler thread**
   - consumes `HotkeyEvent`s from the keyboard hook
   - dispatches standalone combos and Largo manual song actions
9. **Run egui UI on the main thread**
   - `eframe::run_native(...)`

---

## GSI event path

### 1. HTTP ingress

`src/gsi/server.rs`:

- creates a bounded Tokio `mpsc::channel::<GsiWebhookEvent>(10)`
- spawns `process_gsi_events(rx, ...)`
- binds axum to `127.0.0.1:{port}`
- routes `POST /` to `gsi_webhook_handler`

### 2. Queue handoff

`src/gsi/handler.rs::gsi_webhook_handler()`:

- receives `Json<GsiWebhookEvent>`
- calls `tx.try_send(event)`
- returns:
  - `200 OK` on success
  - `503 Service Unavailable` if the queue is full, after incrementing `AppState.metrics.events_dropped`
  - `500 Internal Server Error` if the channel is closed

### 3. Event processing

`src/gsi/handler.rs::process_gsi_events()`:

1. optionally appends the raw event to a JSONL session file when `settings.gsi_logging.enabled`
2. locks `AppState` and calls `state.update_from_gsi(event.clone())`
3. updates `state.metrics.current_queue_depth = rx.len()`
4. logs hero death / respawn transitions via the `WAS_ALIVE` mutex
5. checks `state.gsi_enabled`
6. if enabled, calls `dispatcher.dispatch_gsi_event(&event)` inline on the GSI processor task

### 4. Dispatcher responsibilities

`src/actions/dispatcher.rs::dispatch_gsi_event()` always runs these pre-dispatch hooks first:

1. `log_neutral_item_discovery(event, &settings)`
2. `soul_ring::update_from_gsi(&event.items, &event.hero, &settings)`
3. `dispel::check_and_dispel_silence(event, &settings, &executor)`
4. update `BROODMOTHER_ACTIVE`
5. `auto_items::update_gsi_state(event)`

Then it routes by hero name:

- **Known hero script** -> `hero_script.handle_gsi_event(event)`
- **No hero script** -> `SurvivabilityActions::execute_default_strategy(event)`

### 5. Hero/common action path

Current hero scripts in `src/actions/heroes/*.rs` all compose shared survivability manually:

- `danger_detector::update(...)`
- `SurvivabilityActions::check_and_use_healing_items(...)`
- `SurvivabilityActions::use_defensive_items_if_danger(...)`
- `SurvivabilityActions::use_neutral_item_if_danger(...)`

The fallback path for unsupported heroes calls `execute_default_strategy()` instead, which performs the same shared survivability pipeline plus per-event armlet handling.

### 6. Action executor lane

`src/actions/executor.rs` owns one runtime-created worker thread for short GSI-driven action jobs that previously spawned raw OS threads on demand.

Current item-2 users are:

- default/common armlet handling in `SurvivabilityActions::execute_default_strategy(...)`
- silence dispel jitter in `dispel::check_and_dispel_silence(...)`
- Huskar armlet handling in `HuskarScript::handle_gsi_event(...)`

The action executor is intentionally narrow in this rollout item:

- keyboard-triggered Shadow Fiend combo threads are unchanged
- Largo's long-lived beat-monitor thread is unchanged
- standalone combo threads are unchanged

---

## Keyboard event path

`src/input/keyboard.rs::start_keyboard_listener()` spawns a dedicated OS thread and installs `rdev::grab(callback)`.

### Callback order

For each intercepted event, the callback does this in order:

1. **Pass through simulated events**
   - if `SIMULATING_KEYS` is set, return `Some(event)` immediately
2. **Track modifier state**
   - update `MODIFIER_KEY_HELD` on Space press/release
3. **Broodmother Space + right-click**
   - if `MODIFIER_KEY_HELD` and `BROODMOTHER_ACTIVE`, spawn `auto_items::execute_auto_items(...)`
   - return `None` to block the original right-click
4. **Broodmother middle mouse**
   - send `HotkeyEvent::BroodmotherSpiderAttack`
   - also spawn `BroodmotherScript::execute_spider_attack_move(...)`
   - return `None`
5. **Compute Soul Ring intercept eligibility**
   - uses `SOUL_RING_STATE.should_intercept_key(...)`
   - also checks `SOUL_RING_STATE.should_trigger(...)`
6. **Shadow Fiend raze intercept**
   - if `sf_enabled` and `settings.heroes.shadow_fiend.raze_intercept_enabled`
   - block `Q/W/E`, call `ShadowFiendState::execute_raze(...)`
7. **Shadow Fiend ultimate intercept**
   - if `sf_enabled` and `auto_bkb_on_ultimate`
   - block `R`, call `ShadowFiendState::execute_ultimate_combo(...)`
8. **Largo / generic ability-key path**
   - emit Largo events for `Q/W/E/R`
   - if Soul Ring should trigger, block and replay
   - otherwise pass the key through
9. **Item-key Soul Ring interception**
   - for slot keys mapped in config
   - spawn `spawn_soul_ring_then_key(...)` and block the original item key
10. **Standalone combo trigger**
   - compare key to `AppState.trigger_key`
   - send `HotkeyEvent::ComboTrigger`
   - do **not** block the original key

### Re-emitting blocked input

Two emitters are used:

| Owner | API | Used for |
|---|---|---|
| `src/input/keyboard.rs` | `simulate_key()` via `rdev::simulate` | Replaying the exact blocked key after Soul Ring |
| `src/input/simulation.rs` | `press_key`, `mouse_click`, `alt_down`, `alt_up` via enigo | Higher-level combos like SF raze facing, BKB double-tap, right-click macros |

---

## Hotkey event path

The `Receiver<HotkeyEvent>` returned by `start_keyboard_listener()` is consumed by the thread spawned in `src/main.rs`.

That thread:

- gates generic combo triggers on `AppState.standalone_enabled`
- uses `AppState.selected_hero` to decide which hero gets `dispatch_standalone_trigger(...)`
- handles Largo `Q/W/E/R` events by downcasting to `LargoScript`
- handles Broodmother spider events directly

Important nuance: `AppState.selected_hero` only models `Huskar`, `Largo`, `LegionCommander`, `ShadowFiend`, and `Tiny`. Broodmother keyboard behavior uses `BROODMOTHER_ACTIVE` instead.

---

## Background tasks and threads

| Where it starts | Task/thread | Start condition | Notes |
|---|---|---|---|
| `src/main.rs` | Tokio task for `start_gsi_server(...)` | Always | Owns the axum listener |
| `src/gsi/server.rs` | Tokio task for `process_gsi_events(...)` | Always | Consumes the bounded GSI queue |
| `src/main.rs` | `spawn_blocking(check_for_update)` | Only when `updates.check_on_startup` is true | Writes `UpdateCheckState` |
| `src/main.rs` | Hotkey consumer thread | Always | Handles `HotkeyEvent`s from the keyboard hook |
| `src/input/keyboard.rs` | `rdev::grab` thread | Always | Global hook; blocks forever |
| `src/input/keyboard.rs` | `spawn_soul_ring_then_key(...)` | Per intercepted Soul Ring key | Short-lived |
| `src/input/keyboard.rs` | Broodmother auto-items thread | Per Space+right-click | Short-lived |
| `src/input/keyboard.rs` | Broodmother spider macro thread | Per middle click | Short-lived |
| `src/actions/common.rs` | Armlet thread inside `execute_default_strategy()` | Per fallback GSI event when armlet is present | Short-lived |
| `src/actions/dispel.rs` | Manta/Lotus thread | On first silenced event with enabled item | Short-lived; adds 30-100ms jitter |
| `src/actions/heroes/shadow_fiend.rs` | Raze / ultimate / standalone combo threads | On intercepted SF key / standalone trigger | Short-lived |
| `src/actions/heroes/largo.rs` | Beat monitoring thread | Once in `LargoScript::new()` | Long-lived singleton guarded by `BEAT_THREAD_STARTED` |
| `src/ui/app.rs` | Update apply thread | User clicks **Update Now** | Calls `apply_update()` then `restart_application()` |
| `src/ui/app.rs` | Manual retry thread | User clicks **Retry** / **Check for Updates Now** | Calls `check_for_update()` |

---

## Change-impact guide

| If you edit... | Re-check |
|---|---|
| `src/main.rs`, `src/gsi/server.rs`, `src/gsi/handler.rs` | boot order, queueing, `AppState` updates, update-check startup behavior |
| `src/input/keyboard.rs` | blocked-vs-pass-through behavior, `SIMULATING_KEYS`, hotkey channel flow |
| `src/actions/dispatcher.rs` | pre-dispatch side effects, hero registration, fallback path |
| `src/actions/heroes/largo.rs` | singleton beat-thread startup and Largo hotkey handling |
| `src/ui/app.rs`, `src/update/mod.rs` | `UpdateCheckState` transitions and restart flow |
