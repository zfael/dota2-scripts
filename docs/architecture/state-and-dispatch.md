# State and Dispatch

**Purpose**: Read this before changing `AppState`, adding/removing a hero from routing, or moving shared logic between dispatcher/common/hero code.

---

## `AppState`

`src/state/app_state.rs` defines the top-level shared runtime state:

| Field | Type | Owner / meaning |
|---|---|---|
| `selected_hero` | `Option<HeroType>` | UI + hotkey routing for `Huskar`, `Largo`, `LegionCommander`, `ShadowFiend`, `Tiny` |
| `gsi_enabled` | `bool` | Master gate for async dispatch from `process_gsi_events()` |
| `standalone_enabled` | `bool` | Master gate for hotkey-triggered standalone combos |
| `last_event` | `Option<GsiWebhookEvent>` | Latest GSI payload for UI/status rendering |
| `metrics` | `QueueMetrics` | `events_processed`, `events_dropped`, `current_queue_depth` |
| `trigger_key` | `Arc<Mutex<String>>` | Current standalone hotkey string, updated when the active hero changes |
| `sf_enabled` | `Arc<Mutex<bool>>` | Fast flag for Shadow Fiend keyboard interception |
| `update_state` | `Arc<Mutex<UpdateCheckState>>` | UI-visible update status machine |

### Current caveats

- `HeroType` does **not** include Broodmother. Broodmother keyboard behavior is driven by `BROODMOTHER_ACTIVE` in `src/actions/heroes/broodmother.rs`.
- `metrics.events_processed` is updated in `AppState::update_from_gsi(...)`.
- `metrics.current_queue_depth` is updated in `process_gsi_events(...)`.
- `metrics.events_dropped` is incremented in `gsi_webhook_handler()` when `try_send` fails because the bounded queue is full.
- `AppState::ui_snapshot()` clones the UI-facing hot fields once so `src/ui/app.rs` can render read-only status and metrics sections without repeatedly locking `AppState`.

---

## Shared `Arc<Mutex<...>>` usage

### Top-level shared objects

| Shared object | Declared in | Shared with |
|---|---|---|
| `Arc<Mutex<AppState>>` | `src/state/app_state.rs` | `src/main.rs`, `src/gsi/handler.rs`, `src/ui/app.rs` |
| `Arc<Mutex<Settings>>` | `src/main.rs` | dispatcher, keyboard hook, hero scripts, UI, updater |
| `Arc<ActionExecutor>` | `src/actions/executor.rs` | `src/main.rs`, dispatcher, common survivability helpers, hero scripts that compose survivability |
| `Arc<Mutex<String>>` (`trigger_key`) | inside `AppState` | keyboard hook + UI + main hotkey consumer |
| `Arc<Mutex<bool>>` (`sf_enabled`) | inside `AppState` | keyboard hook + UI/GSI hero selection |
| `Arc<Mutex<UpdateCheckState>>` | inside `AppState` | startup update task + UI |

### Feature-specific shared state

These are not part of `AppState`, but they matter when tracing dispatch:

| State | Path | Purpose |
|---|---|---|
| `SOUL_RING_STATE` | `src/actions/soul_ring.rs` | Shared Soul Ring inventory/health/mana snapshot used by GSI + keyboard paths |
| `SF_LAST_EVENT` | `src/actions/heroes/shadow_fiend.rs` | Cached event for BKB/Blink checks during intercepted SF combos |
| `HP_TRACKER` | `src/actions/danger_detector.rs` | Global danger heuristic state |
| `LATEST_GSI_EVENT` | `src/actions/auto_items.rs` | Cached inventory/ability state for Broodmother auto-items |
| `DISPEL_TRIGGERED` | `src/actions/dispel.rs` | Prevent repeated Manta/Lotus usage during one silence |
| `BROODMOTHER_ACTIVE` | `src/actions/heroes/broodmother.rs` | Enables Broodmother mouse interception without using `AppState.selected_hero` |

### Locking pattern used in this repo

Common pattern:

1. lock
2. read/copy the few fields you need
3. drop the lock
4. perform input simulation, file I/O, sleeps, or dispatch

Examples:

- `src/main.rs` reads `selected_hero`, then `drop(state)` before calling `dispatch_standalone_trigger(...)`
- `src/actions/dispatcher.rs` drops the `Settings` lock before hero dispatch
- `src/actions/common.rs` gathers defensive-item config, then releases the settings lock before simulating keys
- `src/actions/soul_ring.rs` drops `SOUL_RING_STATE` before sleeping and pressing keys
- `src/ui/app.rs` now throttles keyboard-snapshot rebuilds and uses one `AppState::ui_snapshot()` per frame for read-only main-tab sections

When extending the code, keep lock scopes short. Do not hold `AppState` or `Settings` across sleeps, network/file I/O, or long-running combo logic.

---

## Dispatcher structure

`src/actions/dispatcher.rs` owns:

| Field | Type | Purpose |
|---|---|---|
| `hero_scripts` | `HashMap<String, Arc<dyn HeroScript>>` | Maps Dota hero names (for example `npc_dota_hero_huskar`) to concrete scripts |
| `executor` | `Arc<ActionExecutor>` | Schedules short GSI-driven action jobs without spawning a new OS thread per action |
| `survivability` | `SurvivabilityActions` | Fallback path for heroes without a dedicated script |

Hero registration happens once in `ActionDispatcher::new(...)`:

- `HuskarScript`
- `LargoScript`
- `LegionCommanderScript`
- `ShadowFiendScript`
- `TinyScript`
- `BroodmotherScript`

The key used in the map is `HeroScript::hero_name()`.

---

## Dispatcher responsibilities

For every GSI event, `dispatch_gsi_event()` runs only dispatch-local hooks and routing:

1. neutral item discovery logging
2. silence dispel check (queues Manta/Lotus jitter work on the shared action executor)
3. hero/default routing (calls the hero script or fallback survivability)

All shared keyboard/runtime cache refreshes are now performed upstream in the handler, before the dispatcher is called.

### How hero-specific routing works

After the shared hooks:

- if `event.hero.name` exists in `hero_scripts`, the dispatcher calls `hero_script.handle_gsi_event(event)`
- otherwise it calls `survivability.execute_default_strategy(event)`

### How common actions interact with hero scripts

Important design detail: for registered heroes, the dispatcher does **not** automatically call `execute_default_strategy()`.

Instead, the hero script decides how to compose shared behavior. In the current codebase, all registered hero scripts call shared survivability helpers themselves:

- `danger_detector::update(...)`
- `check_and_use_healing_items(...)`
- `use_defensive_items_if_danger(...)`
- `use_neutral_item_if_danger(...)`

That means:

- shared survivability changes often affect both `src/actions/common.rs` **and** multiple hero files
- moving logic between dispatcher and common helpers can easily create duplicate or missing actions
- docs for hero scripts should stay aligned when a hero overrides or sequences shared helpers differently
- the shared `ActionExecutor` now flows through those hero scripts because `SurvivabilityActions` depends on it for executor-backed armlet scheduling plus the timed Glimmer/neutral self-cast survivability sequences

### Current executor-backed paths

The current executor scope is intentionally limited to the hot GSI-driven short jobs identified in the audit:

- default/common armlet handling in `src/actions/common.rs`
- shared survivability timed self-casts in `src/actions/common.rs` (Glimmer Cape follow-up tap and neutral-item self-cast)
- silence dispel jitter in `src/actions/dispel.rs`
- Huskar armlet handling in `src/actions/heroes/huskar.rs`
- Tiny and Legion Commander standalone combo execution in `src/actions/heroes/tiny.rs` and `src/actions/heroes/legion_commander.rs`

Notably unchanged in this item:

- `src/input/keyboard.rs` thread spawns
- `src/actions/heroes/shadow_fiend.rs` combo threads
- `src/actions/heroes/largo.rs` beat-monitor thread
- standalone combo execution outside the three migrated paths

---

## `HeroScript` trait

`src/actions/heroes/traits.rs` defines the dispatcher contract:

| Method | Used by |
|---|---|
| `handle_gsi_event(&self, event: &GsiWebhookEvent)` | GSI routing from `ActionDispatcher::dispatch_gsi_event()` |
| `handle_standalone_trigger(&self)` | Keyboard-triggered standalone combo path from `main.rs` |
| `hero_name(&self) -> &'static str` | Startup registration key |
| `as_any(&self) -> &dyn Any` | Downcasting for Largo-only helpers in `main.rs` |

`as_any()` exists because `main.rs` needs access to concrete `LargoScript` methods like `select_song_manually(...)` and `deactivate_ultimate()`.

---

## Standalone dispatch path

Standalone flow is split across `AppState`, the keyboard hook, the dispatcher, and the executor:

1. `src/input/keyboard.rs` emits `HotkeyEvent::ComboTrigger`
2. `src/main.rs` reads `AppState.selected_hero` and `standalone_enabled`
3. `src/main.rs` converts `HeroType` into the game's hero name string
4. `ActionDispatcher::dispatch_standalone_trigger(hero_name)` calls the matching script
5. Tiny and Legion Commander standalone triggers enqueue onto `ActionExecutor`
6. Largo manual `Q/W/E/R` hotkeys still bypass `handle_standalone_trigger()` and use the concrete `LargoScript` methods

Special cases:

- Shadow Fiend standalone handling stays on its existing specialized path inside the hero script
- Broodmother callback actions stay in `src/input/keyboard.rs`, which uses `BROODMOTHER_ACTIVE` and a dedicated callback worker instead of the `HotkeyEvent` channel

---

## Safe edit checklist

If you change:

- `src/state/app_state.rs` -> also check `src/main.rs`, `src/gsi/handler.rs`, `src/ui/app.rs`
- `src/actions/dispatcher.rs` -> also check `docs/architecture/runtime-flow.md` and hero docs touched by the routing change
- `src/actions/heroes/traits.rs` -> every registered hero script must still compile and satisfy the trait
- hero/common survivability composition -> read `docs/features/survivability.md` and `docs/features/danger-detection.md`
