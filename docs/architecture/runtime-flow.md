# Runtime Flow

**Purpose**: Use this doc to trace execution order, queueing, thread ownership, and blocked key paths.

---

## Startup sequence

Boot starts in `src/main.rs::main()`:

1. **Load settings** from `config/config.toml` into `Arc<Mutex<Settings>>`
2. **Initialize tracing** using `RUST_LOG` or `settings.logging.level`
3. **Create shared state** with `AppState::new()` (`Arc<Mutex<AppState>>`)
4. **Build the initial keyboard snapshot**
   - `main.rs` creates `Arc<RwLock<KeyboardSnapshot>>`
   - snapshot is built from current `Settings` + `AppState`
5. **Create dispatcher** with `ActionDispatcher::new(settings.clone())`
6. **Start keyboard hook** with `start_keyboard_listener()`
   - `KeyboardListenerConfig` now contains the shared snapshot
   - spawns the `rdev::grab` thread in `src/input/keyboard.rs`
   - returns `Receiver<HotkeyEvent>` to `main.rs`
7. **Spawn GSI server task**
   - `main.rs` calls `tokio::spawn(start_gsi_server(...))`
8. **Optionally spawn update check**
   - guarded by `settings.updates.check_on_startup`
   - uses `tokio::task::spawn_blocking` and writes into `AppState.update_state`
9. **Spawn hotkey handler thread**
   - consumes `HotkeyEvent`s from the keyboard hook
   - dispatches standalone combos and Largo manual song actions
10. **Run egui UI on the main thread**
   - `eframe::run_native(...)`
   - `Dota2ScriptApp::update(...)` refreshes the shared `KeyboardSnapshot` every frame from current `Settings` + `AppState`

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
4. refreshes keyboard-supporting runtime state from the latest GSI event even if full GSI automation is disabled:
   - `soul_ring::update_from_gsi(...)`
   - `auto_items::update_gsi_state(...)`
   - `BROODMOTHER_ACTIVE`
   - `SF_LAST_EVENT` for Shadow Fiend keyboard combos
5. logs hero death / respawn transitions via the `WAS_ALIVE` mutex
6. checks `state.gsi_enabled`
7. if enabled, calls `dispatcher.dispatch_gsi_event(&event)` inline on the GSI processor task

### 4. Dispatcher responsibilities

`src/actions/dispatcher.rs::dispatch_gsi_event()` always runs these pre-dispatch hooks first:

1. `log_neutral_item_discovery(event, &settings)`
2. `dispel::check_and_dispel_silence(event, &settings, &executor)`

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

- immediate jobs now send straight to the executor worker channel instead of spawning a zero-delay helper thread first
- delayed jobs still sleep off-worker before sending, so timer/jitter waits do not block immediate jobs that are already queued
- keyboard-triggered Shadow Fiend combo threads are unchanged
- Largo keeps its own long-lived scheduled beat worker; it sleeps until the next beat deadline or a state-change wake-up and uses a cached beat-config snapshot instead of locking shared settings every cycle
- standalone combo threads are unchanged

---

## Keyboard event path

`src/input/keyboard.rs::start_keyboard_listener()` spawns a dedicated OS thread and installs `rdev::grab(callback)`.

`main.rs` and `ui/app.rs` share one `Arc<RwLock<KeyboardSnapshot>>` with that listener:

- `main.rs` creates the initial snapshot before the hook starts
- `Dota2ScriptApp::update(...)` refreshes it every frame
- the callback clones it only on the button/key paths that need static config

The snapshot only carries static keyboard-relevant config:

- parsed combo-trigger key
- Shadow Fiend interception flags and delays
- Broodmother callback-facing config and pre-parsed keys
- Soul Ring thresholds, ability keys, and item-slot keys

Live Soul Ring state is still read from `SOUL_RING_STATE` so inventory moves, cooldowns, mana, health, and hero-alive state stay current with GSI.

### Callback order

For each intercepted event, the callback does this in order:

1. **Pass through simulated events**
   - if `SIMULATING_KEYS` is set, return `Some(event)` immediately
2. **Read the keyboard snapshot once**
   - clone `KeyboardSnapshot` from the shared `RwLock`
3. **Track modifier state**
   - update `MODIFIER_KEY_HELD` on Space press/release
4. **Broodmother Space + right-click**
   - if `MODIFIER_KEY_HELD` and `BROODMOTHER_ACTIVE`, enqueue auto-items/ability execution to the Broodmother callback worker
   - return `None` to block the original right-click
5. **Broodmother middle mouse**
   - enqueue spider micro to the Broodmother callback worker
   - return `None`
6. **Compute Soul Ring intercept eligibility**
   - takes one live `SOUL_RING_STATE` lock on the keypress path
   - uses snapshot-backed `should_intercept_key_with_config(...)`
   - also checks `should_trigger_with_config(...)`
7. **Shadow Fiend raze intercept**
   - if `snapshot.sf_enabled` and `snapshot.shadow_fiend.raze_intercept_enabled`
   - block `Q/W/E`, call `ShadowFiendState::execute_raze(...)`
8. **Shadow Fiend ultimate intercept**
   - if `snapshot.sf_enabled` and `snapshot.shadow_fiend.auto_bkb_on_ultimate`
   - block `R`, call `ShadowFiendState::execute_ultimate_combo(...)`
9. **Largo / generic ability-key path**
   - emit Largo events for `Q/W/E/R`
   - if Soul Ring should trigger, enqueue replay work to the dedicated Soul Ring worker and block the original key
   - otherwise pass the key through
10. **Item-key Soul Ring interception**
   - for slot keys mapped in config
   - call `spawn_soul_ring_then_key(...)`, which enqueues work to the dedicated Soul Ring worker, and block the original item key
11. **Standalone combo trigger**
   - compare key to `snapshot.trigger_key`
   - send `HotkeyEvent::ComboTrigger`
   - do **not** block the original key

### Re-emitting blocked input

Two emitters are used:

| Owner | API | Used for |
|---|---|---|
| `src/input/keyboard.rs` | `simulate_key()` via `rdev::simulate` | Replaying the exact blocked key after Soul Ring |
| `src/input/simulation.rs` | `press_key`, `mouse_click`, `left_click`, `alt_down`, `alt_up` submit work to a lazy worker thread and wait for completion | Higher-level combos like SF raze facing, BKB double-tap, right-click macros |

`src/input/simulation.rs` now owns one explicit synthetic-input lane:

- the first helper call lazily starts a dedicated worker thread
- that worker owns the single `Enigo` instance for the process
- callers submit commands onto an unbounded FIFO `std::sync::mpsc` queue, then wait for the worker to finish that command
- the worker, not the caller thread, performs the small post-action guard delays for replay-safe input, so helper timing semantics stay stable while lane ownership is centralized
- `alt_down()` keeps `SIMULATING_KEYS` active across later queued commands until the matching queued `alt_up()` runs
- the synthetic-input worker now exposes queue metrics via `synthetic_input_metrics()`: current depth, queued total, peak depth, completed total, and dropped total are visible in the debug UI
- these metrics are separate from both GSI queue metrics and the Soul Ring replay worker

---

## Hotkey event path

The `Receiver<HotkeyEvent>` returned by `start_keyboard_listener()` is consumed by the thread spawned in `src/main.rs`.

That thread:

- gates generic combo triggers on `AppState.standalone_enabled`
- uses `AppState.selected_hero` to decide which hero gets `dispatch_standalone_trigger(...)`
- handles Largo `Q/W/E/R` events by downcasting to `LargoScript`

Important nuance: `AppState.selected_hero` only models `Huskar`, `Largo`, `LegionCommander`, `ShadowFiend`, and `Tiny`. Broodmother keyboard behavior uses `BROODMOTHER_ACTIVE` and the dedicated Broodmother callback worker, not the hotkey event channel.

---

## Background tasks and threads

Largo no longer uses a tight polling loop. Its dedicated worker blocks on a timed wait until the next scheduled beat or an explicit wake caused by song, ultimate, or config-state changes, and steady-state scheduling reads the cached Largo beat config stored in worker state.

| Where it starts | Task/thread | Start condition | Notes |
|---|---|---|---|
| `src/main.rs` | Tokio task for `start_gsi_server(...)` | Always | Owns the axum listener |
| `src/gsi/server.rs` | Tokio task for `process_gsi_events(...)` | Always | Consumes the bounded GSI queue |
| `src/main.rs` | `spawn_blocking(check_for_update)` | Only when `updates.check_on_startup` is true | Writes `UpdateCheckState` |
| `src/main.rs` | Hotkey consumer thread | Always | Handles `HotkeyEvent`s from the keyboard hook |
| `src/input/keyboard.rs` | `rdev::grab` thread | Always | Global hook; blocks forever |
| `src/actions/executor.rs` | ActionExecutor worker thread | When `ActionDispatcher::new(...)` constructs the executor | Runs queued short GSI-driven jobs FIFO; immediate jobs go straight to this worker |
| `src/input/simulation.rs` | Synthetic-input worker thread | First call to a simulation helper | Owns `Enigo`; drains one unbounded FIFO queue |
| `src/input/keyboard.rs` | Soul Ring replay worker thread | First intercepted Soul Ring key | Long-lived lazy singleton; drains one unbounded FIFO queue of `SoulRingReplayRequest`s; uses `rdev::simulate` for replay |
| `src/input/keyboard.rs` | Broodmother callback worker thread | First Broodmother callback action | Long-lived lazy singleton; drains one unbounded FIFO queue of `BroodmotherCallbackRequest`s; handles both Space+right-click auto-items/abilities and middle-mouse spider micro |
| `src/actions/executor.rs` | Delayed enqueue helper thread | Per non-zero `enqueue_after(...)` job | Short-lived; sleeps off-worker before sending the job to the executor lane |
| `src/actions/heroes/shadow_fiend.rs` | Raze / ultimate / standalone combo threads | On intercepted SF key / standalone trigger | Short-lived |
| `src/actions/heroes/largo.rs` | Largo scheduled beat worker | Once in `LargoScript::new()` | Long-lived singleton guarded by `BEAT_THREAD_STARTED`; timed wait until next beat or state-change wake-up |
| `src/ui/app.rs` | Update apply thread | User clicks **Update Now** | Calls `apply_update()` then `restart_application()` |
| `src/ui/app.rs` | Manual retry thread | User clicks **Retry** / **Check for Updates Now** | Calls `check_for_update()` |

---

## Change-impact guide

| If you edit... | Re-check |
|---|---|
| `src/main.rs`, `src/gsi/server.rs`, `src/gsi/handler.rs` | boot order, queueing, `AppState` updates, update-check startup behavior |
| `src/input/keyboard.rs` | blocked-vs-pass-through behavior, `SIMULATING_KEYS`, hotkey channel flow |
| `src/actions/dispatcher.rs` | pre-dispatch side effects, hero registration, fallback path |
| `src/actions/heroes/largo.rs` | singleton scheduled-worker startup, cached beat-config updates, and Largo hotkey handling |
| `src/ui/app.rs`, `src/update/mod.rs` | `UpdateCheckState` transitions and restart flow |
