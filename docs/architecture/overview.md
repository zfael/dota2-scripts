# Architecture Overview

**Purpose**: Start here when you need the repo-wide map before editing runtime code, docs, or config.

---

## What this application is

`dota2-scripts` is a Windows-focused Rust desktop app for Dota 2 automation.

It combines four runtime surfaces:

1. **GSI ingestion** via axum (`src/gsi/`) on `http://127.0.0.1:<configured port>` (default `3000`)
2. **Keyboard interception** via rdev (`src/input/keyboard.rs`)
3. **Synthetic input emission** via enigo / rdev (`src/input/simulation.rs`, `src/input/keyboard.rs`)
4. **Desktop UI + updates** via egui/eframe and `src/update/mod.rs`

At runtime, Dota 2 sends GSI payloads, the app updates shared state, dispatches hero/common actions, optionally intercepts player keypresses, and exposes status/configuration in the UI.

---

## Main entry points

| Path | Entry point | Why it matters |
|---|---|---|
| `src/main.rs` | `main()` | Boot order: settings, logging, `AppState`, dispatcher, keyboard listener, GSI server, update check, UI |
| `src/gsi/server.rs` | `start_gsi_server()` | Creates the bounded GSI queue and starts the axum server + event processor |
| `src/gsi/handler.rs` | `gsi_webhook_handler()`, `process_gsi_events()` | Accepts POST bodies, logs JSONL, updates `AppState`, fans events out to the dispatcher |
| `src/actions/dispatcher.rs` | `ActionDispatcher::new()`, `dispatch_gsi_event()` | Registers hero scripts and runs pre-dispatch shared hooks |
| `src/input/keyboard.rs` | `start_keyboard_listener()` | Starts the global `rdev::grab` hook and returns `Receiver<HotkeyEvent>` |
| `src/ui/app.rs` | `Dota2ScriptApp` | Main egui app; owns Main / Danger Detection / Settings tabs and update banner |
| `src/update/mod.rs` | `check_for_update()`, `apply_update()`, `restart_application()` | GitHub Releases integration |

---

## Major subsystems

| Subsystem | Primary files | Responsibilities |
|---|---|---|
| GSI ingress | `src/gsi/server.rs`, `src/gsi/handler.rs`, `src/models/gsi_event.rs` | Receive Dota 2 POSTs, deserialize `GsiWebhookEvent`, queue and dispatch work |
| Shared runtime state | `src/state/app_state.rs` | Global `Arc<Mutex<AppState>>` used by UI, GSI processing, hotkey handling, and update status |
| Action dispatch | `src/actions/dispatcher.rs`, `src/actions/heroes/traits.rs` | Register hero scripts, run cross-cutting hooks, select hero-specific vs fallback handling |
| Common survivability | `src/actions/common.rs`, `src/actions/danger_detector.rs`, `src/actions/dispel.rs`, `src/actions/auto_items.rs` | Healing items, defensive items, neutral items, armlet toggle, silence dispels, cached item state |
| Hero scripts | `src/actions/heroes/*.rs` | Per-hero behavior layered on top of shared survivability |
| Keyboard interception | `src/input/keyboard.rs`, `src/actions/soul_ring.rs`, `src/actions/heroes/shadow_fiend.rs` | Block/replace physical keypresses for Soul Ring, SF razes/ultimate, Largo timing, Broodmother mouse macros |
| Input simulation | `src/input/simulation.rs` | Emit synthetic keys/mouse and guard against self-reinterception with `SIMULATING_KEYS` |
| UI and config | `src/ui/app.rs`, `src/config/settings.rs`, `config/config.toml` | Show status, edit settings, save config |
| Auto-update | `src/update/mod.rs`, `src/main.rs`, `src/ui/app.rs` | Startup checks, download/apply, restart, banner/state reporting |
| Observability | `src/observability/minimap_capture.rs` | Minimap capture worker; runs on its own background thread independent of GSI dispatch |

---

## Runtime map

```text
src/main.rs
├─ loads Settings from config/config.toml
├─ creates Arc<Mutex<AppState>>
├─ creates ActionDispatcher
├─ starts src/input/keyboard.rs
├─ spawns src/gsi/server.rs
├─ optionally spawns src/update/mod.rs check
└─ runs src/ui/app.rs on the main thread

GSI POST
  -> src/gsi/server.rs
  -> src/gsi/handler.rs
  -> src/actions/dispatcher.rs
  -> src/actions/heroes/*.rs and/or src/actions/common.rs

Physical keypress
  -> src/input/keyboard.rs
  -> src/actions/soul_ring.rs and/or src/actions/heroes/shadow_fiend.rs
  -> src/input/simulation.rs or rdev simulate helpers
```

---

## Where to start for common changes

| Change you need to make | Read next |
|---|---|
| Startup order, queueing, async/thread ownership | `docs/architecture/runtime-flow.md` |
| `AppState`, hero registration, common-action composition | `docs/architecture/state-and-dispatch.md` |
| HP danger heuristics and defensive items | `docs/features/danger-detection.md` |
| Keyboard interception / blocked keys | `docs/features/keyboard-interception.md` |
| Shared healing / dispel / neutral-item behavior | `docs/features/survivability.md` |
| Update check / download / restart behavior | `docs/features/updates.md` |
