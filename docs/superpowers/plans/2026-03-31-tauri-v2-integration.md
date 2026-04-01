# Tauri v2 Integration — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the React frontend (built in Plan 1) to the Rust backend via Tauri v2 IPC, replacing mock data with live state and enabling real-time game updates.

**Architecture:** Create a `src-tauri/` crate as a new binary in a Cargo workspace alongside the existing library crate. The Tauri binary imports from `dota2_scripts` library, registers IPC command handlers, starts all background tasks (GSI server, keyboard hook, update checker), and emits real-time events to the React frontend. The existing egui binary remains gated behind a feature flag.

**Tech Stack:** Tauri v2, `@tauri-apps/api` v2, Zustand stores wired to IPC, tokio broadcast channels for event push

---

## File Structure

### New files (src-tauri/)

| File | Purpose |
|---|---|
| `src-tauri/Cargo.toml` | Tauri binary crate; depends on `dota2_scripts` library + `tauri` v2 |
| `src-tauri/build.rs` | Tauri build hook (`tauri_build::build()`) |
| `src-tauri/tauri.conf.json` | Window config, frontend dist, dev URL |
| `src-tauri/capabilities/default.json` | IPC permissions |
| `src-tauri/src/main.rs` | Entry point — calls `lib::run()` |
| `src-tauri/src/lib.rs` | Tauri builder, managed state, setup hook |
| `src-tauri/src/ipc_types.rs` | Serializable DTOs matching frontend TypeScript types |
| `src-tauri/src/commands/mod.rs` | Command module re-exports |
| `src-tauri/src/commands/config.rs` | `get_config`, `update_config` commands |
| `src-tauri/src/commands/state.rs` | `get_app_state`, `set_gsi_enabled`, `set_standalone_enabled`, `select_hero` |
| `src-tauri/src/commands/game.rs` | `get_game_state`, `get_danger_state` |
| `src-tauri/src/commands/diagnostics.rs` | `get_diagnostics` |
| `src-tauri/src/commands/updates.rs` | `get_update_state`, `check_for_updates`, `apply_update`, `dismiss_update` |
| `src-tauri/src/events.rs` | Event emitter — subscribes to broadcast channel, emits Tauri events |

### New files (src-ui/)

| File | Purpose |
|---|---|
| `src-ui/src/hooks/useTauriCommand.ts` | Generic hook wrapping `invoke()` with loading/error |
| `src-ui/src/hooks/useTauriEvent.ts` | Generic hook wrapping `listen()` with cleanup |
| `src-ui/src/hooks/index.ts` | Hook barrel exports |
| `src-ui/src/lib/tauri.ts` | Tauri detection + conditional import helpers |
| `src-ui/src/stores/updateStore.ts` | Separated update state store (from gameStore) |

### Modified files (library crate)

| File | Change |
|---|---|
| `Cargo.toml` (root) | Add `[workspace]` members |
| `src/state/app_state.rs` | Add `ui_broadcast_tx` field + `Serialize` on DTOs |
| `src/gsi/handler.rs` | Emit to broadcast channel after processing |

### Modified files (src-ui/)

| File | Change |
|---|---|
| `src-ui/package.json` | Add `@tauri-apps/api` dependency |
| `src-ui/src/stores/configStore.ts` | Wire to `invoke('get_config')` + debounced write-back |
| `src-ui/src/stores/gameStore.ts` | Wire to Tauri event subscription |
| `src-ui/src/stores/activityStore.ts` | Wire to Tauri event subscription |
| `src-ui/src/stores/uiStore.ts` | Wire toggles to Tauri commands |
| `src-ui/src/App.tsx` | Add initialization effect, switch to update banner |
| `src-ui/src/components/layout/Sidebar.tsx` | Add collapse/expand |
| `src-ui/src/components/layout/StatusHeader.tsx` | Add connection indicator |

---

## Conventions

- **IPC DTOs** use `#[serde(rename_all = "camelCase")]` to match frontend TypeScript naming (except config types which stay snake_case to match `config.toml`)
- **Tauri commands** return `Result<T, String>` — errors are serialized as strings
- **Frontend hooks** detect Tauri environment via `window.__TAURI_INTERNALS__` — when absent, fall back to mock data (enables standalone dev server testing)
- **Event names** use snake_case: `gsi_update`, `activity_event`, `update_state_changed`, `danger_state_changed`
- **Config writes** are debounced 300ms on the frontend before calling `update_config`

---

## Phase 1: Tauri Project Setup

### Task 1: Create Cargo workspace and src-tauri crate

**Files:**
- Modify: `Cargo.toml` (root)
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/build.rs`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add workspace to root Cargo.toml**

Add a `[workspace]` section at the top of the root `Cargo.toml` (before `[package]`):

```toml
[workspace]
members = [".", "src-tauri"]
resolver = "2"
```

Keep all existing content unchanged.

- [ ] **Step 2: Create src-tauri/Cargo.toml**

```toml
[package]
name = "dota2-scripts-tauri"
version = "0.1.0"
edition = "2021"

[dependencies]
dota2_scripts = { path = ".." }
tauri = { version = "2", features = [] }
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = "0.3"

[build-dependencies]
tauri-build = { version = "2", features = [] }
```

- [ ] **Step 3: Create src-tauri/build.rs**

```rust
fn main() {
    tauri_build::build()
}
```

- [ ] **Step 4: Create src-tauri/src/main.rs**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    dota2_scripts_tauri::run();
}
```

- [ ] **Step 5: Create src-tauri/src/lib.rs (minimal)**

```rust
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 6: Verify workspace compiles**

Run from the workspace root:
```bash
cargo check -p dota2-scripts-tauri
```
Expected: compiles (may warn about unused imports). If `tauri::generate_context!()` fails, that's expected — we need the Tauri config (Task 2).

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock src-tauri/
git commit -m "feat(tauri): scaffold src-tauri crate with Cargo workspace"
```

---

### Task 2: Tauri configuration and capabilities

**Files:**
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/capabilities/default.json`
- Create: `src-tauri/icons/` (placeholder)

- [ ] **Step 1: Create tauri.conf.json**

```json
{
  "productName": "Dota 2 Scripts",
  "version": "0.1.0",
  "identifier": "com.dota2scripts.app",
  "build": {
    "devUrl": "http://localhost:5173",
    "frontendDist": "../src-ui/dist",
    "beforeDevCommand": "",
    "beforeBuildCommand": ""
  },
  "app": {
    "windows": [
      {
        "title": "Dota 2 Script Automation",
        "width": 1024,
        "height": 700,
        "minWidth": 900,
        "minHeight": 650,
        "resizable": true,
        "decorations": true
      }
    ],
    "security": {
      "csp": null
    }
  }
}
```

Note: `beforeDevCommand` and `beforeBuildCommand` are empty — we start the React dev server manually. This avoids issues with Tauri trying to manage the frontend process.

- [ ] **Step 2: Create capabilities/default.json**

```json
{
  "identifier": "default",
  "description": "Default capability set for Dota 2 Scripts",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "shell:allow-open"
  ]
}
```

- [ ] **Step 3: Create placeholder icon**

Create `src-tauri/icons/` directory. Copy the existing `assets/icon.png` into `src-tauri/icons/icon.png`. Also create a minimal `src-tauri/icons/icon.ico` by copying `assets/icon.ico` if it exists, or create a placeholder.

Tauri v2 needs at least one icon. If `assets/icon.png` exists:
```bash
mkdir -p src-tauri/icons
cp assets/icon.png src-tauri/icons/icon.png
```

If icon files don't exist in the expected format, create a simple 32x32 PNG placeholder.

- [ ] **Step 4: Verify Tauri config compiles**

```bash
cargo check -p dota2-scripts-tauri
```
Expected: compiles successfully. The `generate_context!()` macro should now find `tauri.conf.json`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/tauri.conf.json src-tauri/capabilities/ src-tauri/icons/
git commit -m "feat(tauri): add Tauri config, capabilities, and icons"
```

---

### Task 3: Verify Tauri app opens React webview

**Files:**
- Modify: `src-ui/package.json` (add tauri dev script)

- [ ] **Step 1: Install @tauri-apps/cli in src-ui**

```bash
cd src-ui
npm install --save-dev @tauri-apps/cli@^2
```

Add to `src-ui/package.json` scripts:
```json
{
  "scripts": {
    "tauri": "tauri"
  }
}
```

- [ ] **Step 2: Build the React frontend**

```bash
cd src-ui
npm run build
```
Expected: production build succeeds, output in `src-ui/dist/`.

- [ ] **Step 3: Build and run the Tauri app**

From the workspace root:
```bash
cargo build -p dota2-scripts-tauri
```

Then test run (the binary will be in `target/debug/dota2-scripts-tauri.exe` on Windows):
```bash
cargo run -p dota2-scripts-tauri
```

Expected: a native window opens showing the React app with mock data. If it shows a blank page, check that `frontendDist` path resolves correctly from `src-tauri/` to `../src-ui/dist`.

- [ ] **Step 4: Commit**

```bash
git add src-ui/package.json src-ui/package-lock.json
git commit -m "feat(tauri): verify Tauri opens React webview"
```

---

## Phase 2: IPC Types

### Task 4: Create serializable IPC DTOs

**Files:**
- Create: `src-tauri/src/ipc_types.rs`
- Modify: `src-tauri/src/lib.rs` (add module)

These DTOs match the frontend TypeScript types exactly. Config types use snake_case (matching `config.toml`), everything else uses camelCase.

- [ ] **Step 1: Create ipc_types.rs**

```rust
use serde::Serialize;

/// Matches frontend GameState in src-ui/src/types/game.ts
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameStateDto {
    pub hero_name: Option<String>,
    pub hero_level: u32,
    pub hp_percent: u32,
    pub mana_percent: u32,
    pub in_danger: bool,
    pub connected: bool,
    pub alive: bool,
    pub stunned: bool,
    pub silenced: bool,
    pub respawn_timer: Option<u32>,
    pub rune_timer: Option<i32>,
    pub game_time: i32,
}

/// Matches frontend AppState-related fields
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStateDto {
    pub selected_hero: Option<String>,
    pub gsi_enabled: bool,
    pub standalone_enabled: bool,
}

/// Matches frontend QueueMetrics in src-ui/src/types/game.ts
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueMetricsDto {
    pub events_processed: u64,
    pub events_dropped: u64,
    pub current_queue_depth: usize,
    pub max_queue_depth: usize,
}

/// Matches frontend syntheticInput in DiagnosticsState
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyntheticInputDto {
    pub queue_depth: usize,
    pub total_queued: u64,
    pub peak_depth: usize,
    pub completions: u64,
    pub drops: u64,
}

/// Matches frontend DiagnosticsState in src-ui/src/types/game.ts
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsDto {
    pub gsi_connected: bool,
    pub keyboard_hook_active: bool,
    pub queue_metrics: QueueMetricsDto,
    pub synthetic_input: SyntheticInputDto,
    pub soul_ring_state: String,
    pub blocked_keys: Vec<String>,
}

/// Matches frontend UpdateCheckState in src-ui/src/types/game.ts
/// Uses internally-tagged enum: { "kind": "idle" }, { "kind": "available", "version": "..." }
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
pub enum UpdateStateDto {
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "checking")]
    Checking,
    #[serde(rename = "available", rename_all = "camelCase")]
    Available {
        version: String,
        release_notes: Option<String>,
    },
    #[serde(rename = "downloading")]
    Downloading,
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "upToDate")]
    UpToDate,
}

/// Activity entry emitted to frontend
#[derive(Debug, Clone, Serialize)]
pub struct ActivityEntryDto {
    pub id: String,
    pub timestamp: String,
    pub category: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}
```

- [ ] **Step 2: Add module to lib.rs**

Update `src-tauri/src/lib.rs`:
```rust
pub mod ipc_types;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: Write serialization tests**

Add at the bottom of `src-tauri/src/ipc_types.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_state_dto_serializes_camel_case() {
        let dto = GameStateDto {
            hero_name: Some("Shadow Fiend".to_string()),
            hero_level: 25,
            hp_percent: 85,
            mana_percent: 70,
            in_danger: false,
            connected: true,
            alive: true,
            stunned: false,
            silenced: false,
            respawn_timer: None,
            rune_timer: Some(45),
            game_time: 1234,
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["heroName"], "Shadow Fiend");
        assert_eq!(json["hpPercent"], 85);
        assert_eq!(json["inDanger"], false);
        assert_eq!(json["runeTimer"], 45);
        assert!(json.get("hero_name").is_none()); // no snake_case
    }

    #[test]
    fn update_state_dto_tags_correctly() {
        let idle = UpdateStateDto::Idle;
        let json = serde_json::to_value(&idle).unwrap();
        assert_eq!(json["kind"], "idle");

        let available = UpdateStateDto::Available {
            version: "1.2.0".to_string(),
            release_notes: Some("Bug fixes".to_string()),
        };
        let json = serde_json::to_value(&available).unwrap();
        assert_eq!(json["kind"], "available");
        assert_eq!(json["version"], "1.2.0");
        assert_eq!(json["releaseNotes"], "Bug fixes");

        let up_to_date = UpdateStateDto::UpToDate;
        let json = serde_json::to_value(&up_to_date).unwrap();
        assert_eq!(json["kind"], "upToDate");
    }

    #[test]
    fn diagnostics_dto_serializes_nested() {
        let dto = DiagnosticsDto {
            gsi_connected: true,
            keyboard_hook_active: true,
            queue_metrics: QueueMetricsDto {
                events_processed: 100,
                events_dropped: 2,
                current_queue_depth: 3,
                max_queue_depth: 10,
            },
            synthetic_input: SyntheticInputDto {
                queue_depth: 0,
                total_queued: 50,
                peak_depth: 5,
                completions: 48,
                drops: 2,
            },
            soul_ring_state: "ready".to_string(),
            blocked_keys: vec!["q".to_string(), "w".to_string()],
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["gsiConnected"], true);
        assert_eq!(json["queueMetrics"]["eventsProcessed"], 100);
        assert_eq!(json["syntheticInput"]["peakDepth"], 5);
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p dota2-scripts-tauri
```
Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/ipc_types.rs src-tauri/src/lib.rs
git commit -m "feat(tauri): add IPC DTO types with serialization tests"
```

---

## Phase 3: IPC Commands — Rust Side

### Task 5: Managed state and read commands (config, app state, game state)

**Files:**
- Create: `src-tauri/src/commands/mod.rs`
- Create: `src-tauri/src/commands/config.rs`
- Create: `src-tauri/src/commands/state.rs`
- Create: `src-tauri/src/commands/game.rs`
- Modify: `src-tauri/src/lib.rs` (add managed state + command registration)

- [ ] **Step 1: Create managed state struct in lib.rs**

Update `src-tauri/src/lib.rs`:

```rust
pub mod commands;
pub mod ipc_types;

use dota2_scripts::config::Settings;
use dota2_scripts::state::AppState;
use std::sync::{Arc, Mutex};

/// Shared state managed by Tauri, accessible from all commands
pub struct TauriAppState {
    pub app_state: Arc<Mutex<dota2_scripts::state::AppState>>,
    pub settings: Arc<Mutex<Settings>>,
}

pub fn run() {
    let settings = Arc::new(Mutex::new(Settings::load()));
    let app_state = AppState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(TauriAppState {
            app_state: app_state.clone(),
            settings: settings.clone(),
        })
        .invoke_handler(tauri::generate_handler![
            commands::config::get_config,
            commands::state::get_app_state,
            commands::game::get_game_state,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 2: Create commands/mod.rs**

```rust
pub mod config;
pub mod game;
pub mod state;
```

- [ ] **Step 3: Create commands/config.rs**

```rust
use crate::TauriAppState;
use dota2_scripts::config::Settings;

/// Returns the full config as JSON (snake_case keys matching config.toml)
#[tauri::command]
pub fn get_config(state: tauri::State<'_, TauriAppState>) -> Result<Settings, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|e| format!("Failed to lock settings: {}", e))?;
    Ok(settings.clone())
}
```

- [ ] **Step 4: Create commands/state.rs**

```rust
use crate::ipc_types::AppStateDto;
use crate::TauriAppState;

/// Returns current app state (selected hero, enabled flags)
#[tauri::command]
pub fn get_app_state(state: tauri::State<'_, TauriAppState>) -> Result<AppStateDto, String> {
    let app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;

    Ok(AppStateDto {
        selected_hero: app.selected_hero.map(|h| h.to_display_name().to_string()),
        gsi_enabled: app.gsi_enabled,
        standalone_enabled: app.standalone_enabled,
    })
}
```

- [ ] **Step 5: Create commands/game.rs**

```rust
use crate::ipc_types::GameStateDto;
use crate::TauriAppState;

/// Returns current game state from the latest GSI event
#[tauri::command]
pub fn get_game_state(state: tauri::State<'_, TauriAppState>) -> Result<GameStateDto, String> {
    let app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;

    let dto = if let Some(ref event) = app.last_event {
        let rune_timer = app.rune_alerts.as_ref().and_then(|ra| {
            if ra.next_rune_time > 0 {
                Some(ra.next_rune_time - event.map.clock_time)
            } else {
                None
            }
        });

        GameStateDto {
            hero_name: app
                .selected_hero
                .map(|h| h.to_display_name().to_string()),
            hero_level: event.hero.level,
            hp_percent: event.hero.health_percent,
            mana_percent: event.hero.mana_percent,
            in_danger: false, // TODO: wire to danger detector
            connected: true,
            alive: event.hero.alive,
            stunned: event.hero.stunned,
            silenced: event.hero.silenced,
            respawn_timer: if event.hero.respawn_seconds > 0 {
                Some(event.hero.respawn_seconds)
            } else {
                None
            },
            rune_timer,
            game_time: event.map.clock_time,
        }
    } else {
        GameStateDto {
            hero_name: None,
            hero_level: 0,
            hp_percent: 100,
            mana_percent: 100,
            in_danger: false,
            connected: false,
            alive: true,
            stunned: false,
            silenced: false,
            respawn_timer: None,
            rune_timer: None,
            game_time: 0,
        }
    };

    Ok(dto)
}
```

- [ ] **Step 6: Verify commands compile**

```bash
cargo check -p dota2-scripts-tauri
```
Expected: compiles. Fix any import issues (e.g., `RuneAlertSnapshot` field names).

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/commands/ src-tauri/src/lib.rs
git commit -m "feat(tauri): add read commands for config, app state, and game state"
```

---

### Task 6: Read commands (diagnostics, update state)

**Files:**
- Create: `src-tauri/src/commands/diagnostics.rs`
- Create: `src-tauri/src/commands/updates.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs` (register new commands)

- [ ] **Step 1: Create commands/diagnostics.rs**

```rust
use crate::ipc_types::{DiagnosticsDto, QueueMetricsDto, SyntheticInputDto};
use crate::TauriAppState;

/// Returns diagnostics: GSI metrics, synthetic input, keyboard state
#[tauri::command]
pub fn get_diagnostics(state: tauri::State<'_, TauriAppState>) -> Result<DiagnosticsDto, String> {
    let app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;

    Ok(DiagnosticsDto {
        gsi_connected: app.last_event.is_some(),
        keyboard_hook_active: true, // keyboard hook is always active once started
        queue_metrics: QueueMetricsDto {
            events_processed: app.metrics.events_processed,
            events_dropped: app.metrics.events_dropped,
            current_queue_depth: app.metrics.current_queue_depth,
            max_queue_depth: 10,
        },
        synthetic_input: SyntheticInputDto {
            queue_depth: 0,
            total_queued: 0,
            peak_depth: 0,
            completions: 0,
            drops: 0,
        },
        soul_ring_state: "ready".to_string(),
        blocked_keys: vec![],
    })
}
```

- [ ] **Step 2: Create commands/updates.rs**

```rust
use crate::ipc_types::UpdateStateDto;
use crate::TauriAppState;
use dota2_scripts::state::UpdateCheckState;

/// Returns current update check state
#[tauri::command]
pub fn get_update_state(state: tauri::State<'_, TauriAppState>) -> Result<UpdateStateDto, String> {
    let app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;

    let update_state = app.update_state.lock().map_err(|e| e.to_string())?;

    let dto = match &*update_state {
        UpdateCheckState::Idle => UpdateStateDto::Idle,
        UpdateCheckState::Checking => UpdateStateDto::Checking,
        UpdateCheckState::Available {
            version,
            release_notes,
        } => UpdateStateDto::Available {
            version: version.clone(),
            release_notes: release_notes.clone(),
        },
        UpdateCheckState::Downloading => UpdateStateDto::Downloading,
        UpdateCheckState::Error(msg) => UpdateStateDto::Error {
            message: msg.clone(),
        },
        UpdateCheckState::UpToDate => UpdateStateDto::UpToDate,
    };

    Ok(dto)
}
```

- [ ] **Step 3: Update commands/mod.rs**

```rust
pub mod config;
pub mod diagnostics;
pub mod game;
pub mod state;
pub mod updates;
```

- [ ] **Step 4: Register new commands in lib.rs**

Add to the `invoke_handler` macro:
```rust
.invoke_handler(tauri::generate_handler![
    commands::config::get_config,
    commands::state::get_app_state,
    commands::game::get_game_state,
    commands::diagnostics::get_diagnostics,
    commands::updates::get_update_state,
])
```

- [ ] **Step 5: Verify compiles**

```bash
cargo check -p dota2-scripts-tauri
```

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands/
git commit -m "feat(tauri): add diagnostics and update state read commands"
```

---

### Task 7: Write commands (config update, toggles, hero selection)

**Files:**
- Modify: `src-tauri/src/commands/config.rs`
- Modify: `src-tauri/src/commands/state.rs`
- Modify: `src-tauri/src/lib.rs` (register new commands)

- [ ] **Step 1: Add update_config command to config.rs**

Append to `src-tauri/src/commands/config.rs`:

```rust
use std::fs;
use tracing::info;

/// Updates a config section and persists to config.toml
#[tauri::command]
pub fn update_config(
    section: String,
    updates: serde_json::Value,
    state: tauri::State<'_, TauriAppState>,
) -> Result<(), String> {
    let mut settings = state
        .settings
        .lock()
        .map_err(|e| format!("Failed to lock settings: {}", e))?;

    // Serialize current config to Value, merge updates into the section, deserialize back
    let mut config_value =
        serde_json::to_value(&*settings).map_err(|e| format!("Serialize error: {}", e))?;

    if let Some(section_value) = config_value.get_mut(&section) {
        if let (Some(existing_obj), Some(update_obj)) =
            (section_value.as_object_mut(), updates.as_object())
        {
            for (key, value) in update_obj {
                existing_obj.insert(key.clone(), value.clone());
            }
        }
    } else {
        return Err(format!("Unknown config section: {}", section));
    }

    // Deserialize back to Settings
    let new_settings: Settings =
        serde_json::from_value(config_value).map_err(|e| format!("Deserialize error: {}", e))?;

    // Persist to config.toml
    let toml_str =
        toml::to_string_pretty(&new_settings).map_err(|e| format!("TOML error: {}", e))?;
    fs::write("config/config.toml", &toml_str)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    *settings = new_settings;
    info!("Config section '{}' updated and persisted", section);

    Ok(())
}

/// Updates a hero-specific config section
#[tauri::command]
pub fn update_hero_config(
    hero: String,
    updates: serde_json::Value,
    state: tauri::State<'_, TauriAppState>,
) -> Result<(), String> {
    let mut settings = state
        .settings
        .lock()
        .map_err(|e| format!("Failed to lock settings: {}", e))?;

    let mut config_value =
        serde_json::to_value(&*settings).map_err(|e| format!("Serialize error: {}", e))?;

    let heroes_section = config_value
        .get_mut("heroes")
        .and_then(|h| h.as_object_mut())
        .ok_or("Missing heroes section")?;

    if let Some(hero_section) = heroes_section.get_mut(&hero) {
        if let (Some(existing_obj), Some(update_obj)) =
            (hero_section.as_object_mut(), updates.as_object())
        {
            for (key, value) in update_obj {
                existing_obj.insert(key.clone(), value.clone());
            }
        }
    } else {
        return Err(format!("Unknown hero: {}", hero));
    }

    let new_settings: Settings =
        serde_json::from_value(config_value).map_err(|e| format!("Deserialize error: {}", e))?;

    let toml_str =
        toml::to_string_pretty(&new_settings).map_err(|e| format!("TOML error: {}", e))?;
    fs::write("config/config.toml", &toml_str)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    *settings = new_settings;
    info!("Hero config '{}' updated and persisted", hero);

    Ok(())
}
```

Note: add `use dota2_scripts::config::Settings;` and `use tracing::info;` to the imports at the top of config.rs. Also add `toml` as a dependency in `src-tauri/Cargo.toml`:
```toml
toml = "0.8"
```

- [ ] **Step 2: Add write commands to state.rs**

Append to `src-tauri/src/commands/state.rs`:

```rust
use dota2_scripts::state::HeroType;

/// Toggles GSI automation on/off
#[tauri::command]
pub fn set_gsi_enabled(
    enabled: bool,
    state: tauri::State<'_, TauriAppState>,
) -> Result<(), String> {
    let mut app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;
    app.gsi_enabled = enabled;
    Ok(())
}

/// Toggles standalone script on/off
#[tauri::command]
pub fn set_standalone_enabled(
    enabled: bool,
    state: tauri::State<'_, TauriAppState>,
) -> Result<(), String> {
    let mut app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;
    app.standalone_enabled = enabled;
    Ok(())
}

/// Manually selects a hero (or clears selection with null)
#[tauri::command]
pub fn select_hero(
    hero: Option<String>,
    state: tauri::State<'_, TauriAppState>,
) -> Result<(), String> {
    let mut app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;

    let hero_type = match hero {
        Some(name) => {
            let game_name = match name.as_str() {
                "Broodmother" => "npc_dota_hero_broodmother",
                "Huskar" => "npc_dota_hero_huskar",
                "Largo" => "npc_dota_hero_largo",
                "Legion Commander" => "npc_dota_hero_legion_commander",
                "Meepo" => "npc_dota_hero_meepo",
                "Outworld Destroyer" => "npc_dota_hero_obsidian_destroyer",
                "Shadow Fiend" => "npc_dota_hero_nevermore",
                "Tiny" => "npc_dota_hero_tiny",
                _ => return Err(format!("Unknown hero: {}", name)),
            };
            HeroType::from_hero_name(game_name)
        }
        None => None,
    };

    app.selected_hero = hero_type;

    // Update hero-specific flags
    if let Some(ht) = hero_type {
        *app.sf_enabled.lock().unwrap() = ht == HeroType::ShadowFiend;
        *app.od_enabled.lock().unwrap() = ht == HeroType::OutworldDestroyer;
    } else {
        *app.sf_enabled.lock().unwrap() = false;
        *app.od_enabled.lock().unwrap() = false;
    }

    Ok(())
}
```

- [ ] **Step 3: Add update action commands to updates.rs**

Append to `src-tauri/src/commands/updates.rs`:

```rust
use dota2_scripts::update::{check_for_update, UpdateCheckResult};

/// Triggers an update check
#[tauri::command]
pub async fn check_for_updates(
    state: tauri::State<'_, TauriAppState>,
) -> Result<UpdateStateDto, String> {
    let app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;
    let update_state_arc = app.update_state.clone();
    let include_prereleases = state
        .settings
        .lock()
        .map_err(|e| e.to_string())?
        .updates
        .include_prereleases;
    drop(app);

    *update_state_arc.lock().unwrap() = UpdateCheckState::Checking;

    let result =
        tokio::task::spawn_blocking(move || check_for_update(include_prereleases))
            .await
            .map_err(|e| format!("Task join error: {}", e))?;

    let new_state = match result {
        UpdateCheckResult::Available(info) => {
            let s = UpdateCheckState::Available {
                version: info.version.clone(),
                release_notes: info.release_notes.clone(),
            };
            *update_state_arc.lock().unwrap() = s;
            UpdateStateDto::Available {
                version: info.version,
                release_notes: info.release_notes,
            }
        }
        UpdateCheckResult::UpToDate => {
            *update_state_arc.lock().unwrap() = UpdateCheckState::UpToDate;
            UpdateStateDto::UpToDate
        }
        UpdateCheckResult::Error(msg) => {
            let s = UpdateCheckState::Error(msg.clone());
            *update_state_arc.lock().unwrap() = s;
            UpdateStateDto::Error { message: msg }
        }
    };

    Ok(new_state)
}

/// Applies an available update (downloads, replaces binary)
#[tauri::command]
pub async fn apply_update(
    state: tauri::State<'_, TauriAppState>,
) -> Result<(), String> {
    let app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;
    let update_state_arc = app.update_state.clone();
    drop(app);

    *update_state_arc.lock().unwrap() = UpdateCheckState::Downloading;

    let include_prereleases = state
        .settings
        .lock()
        .map_err(|e| e.to_string())?
        .updates
        .include_prereleases;

    tokio::task::spawn_blocking(move || {
        dota2_scripts::update::apply_update(include_prereleases)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .map_err(|e| format!("Update failed: {}", e))?;

    // Restart the application
    dota2_scripts::update::restart_application()
        .map_err(|e| format!("Restart failed: {}", e))?;

    Ok(())
}

/// Dismisses update banner (resets to Idle)
#[tauri::command]
pub fn dismiss_update(state: tauri::State<'_, TauriAppState>) -> Result<(), String> {
    let app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;
    *app.update_state.lock().unwrap() = UpdateCheckState::Idle;
    Ok(())
}
```

- [ ] **Step 4: Register all new commands in lib.rs**

Update the `invoke_handler` in `src-tauri/src/lib.rs`:
```rust
.invoke_handler(tauri::generate_handler![
    commands::config::get_config,
    commands::config::update_config,
    commands::config::update_hero_config,
    commands::state::get_app_state,
    commands::state::set_gsi_enabled,
    commands::state::set_standalone_enabled,
    commands::state::select_hero,
    commands::game::get_game_state,
    commands::diagnostics::get_diagnostics,
    commands::updates::get_update_state,
    commands::updates::check_for_updates,
    commands::updates::apply_update,
    commands::updates::dismiss_update,
])
```

- [ ] **Step 5: Verify compiles**

```bash
cargo check -p dota2-scripts-tauri
```
Expected: compiles. Fix any missing imports.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/
git commit -m "feat(tauri): add write commands for config, toggles, hero selection, updates"
```

---

## Phase 4: Event Bridge

### Task 8: Add broadcast channel for UI events

**Files:**
- Modify: `src/state/app_state.rs` (add broadcast sender)
- Create: `src-tauri/src/events.rs`
- Modify: `src-tauri/src/lib.rs` (add module + setup event listener)

The approach: add an optional `tokio::sync::broadcast::Sender` to `AppState` (or pass alongside). When the Tauri app runs, it sets this sender. The GSI handler sends events through it. The Tauri event bridge subscribes and emits to the frontend.

Since `AppState` is in the library crate and we don't want to add `tokio` as a required dependency there, we'll use a different approach: store the broadcast sender in the `TauriAppState` managed state, and have the GSI processing pipeline send to it via a callback.

Actually, the simpler approach: use a **polling timer** in the Tauri setup hook that periodically reads `AppState` and emits to the frontend. This avoids modifying the library crate at all.

- [ ] **Step 1: Create events.rs with polling emitter**

Create `src-tauri/src/events.rs`:

```rust
use crate::ipc_types::GameStateDto;
use crate::TauriAppState;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

/// Starts a background task that polls AppState and emits game_state events
/// to the frontend at ~5Hz (every 200ms)
pub fn start_game_state_emitter(app: AppHandle) {
    let tauri_state = app.state::<TauriAppState>();
    let app_state = tauri_state.app_state.clone();

    tauri::async_runtime::spawn(async move {
        let mut last_events_processed: u64 = 0;

        loop {
            tokio::time::sleep(Duration::from_millis(200)).await;

            let dto = {
                let state = match app_state.lock() {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                // Only emit if there's new data
                if state.metrics.events_processed == last_events_processed {
                    continue;
                }
                last_events_processed = state.metrics.events_processed;

                build_game_state_dto(&state)
            };

            let _ = app.emit("gsi_update", &dto);
        }
    });
}

fn build_game_state_dto(
    state: &dota2_scripts::state::AppState,
) -> GameStateDto {
    if let Some(ref event) = state.last_event {
        let rune_timer = state.rune_alerts.as_ref().and_then(|ra| {
            if ra.next_rune_time > 0 {
                Some(ra.next_rune_time - event.map.clock_time)
            } else {
                None
            }
        });

        GameStateDto {
            hero_name: state
                .selected_hero
                .map(|h| h.to_display_name().to_string()),
            hero_level: event.hero.level,
            hp_percent: event.hero.health_percent,
            mana_percent: event.hero.mana_percent,
            in_danger: false, // TODO: wire danger detector
            connected: true,
            alive: event.hero.alive,
            stunned: event.hero.stunned,
            silenced: event.hero.silenced,
            respawn_timer: if event.hero.respawn_seconds > 0 {
                Some(event.hero.respawn_seconds)
            } else {
                None
            },
            rune_timer,
            game_time: event.map.clock_time,
        }
    } else {
        GameStateDto {
            hero_name: None,
            hero_level: 0,
            hp_percent: 100,
            mana_percent: 100,
            in_danger: false,
            connected: false,
            alive: true,
            stunned: false,
            silenced: false,
            respawn_timer: None,
            rune_timer: None,
            game_time: 0,
        }
    }
}
```

- [ ] **Step 2: Add events module and wire into setup hook**

Update `src-tauri/src/lib.rs` — add `pub mod events;` and use it in the setup hook:

```rust
pub mod commands;
pub mod events;
pub mod ipc_types;

// ... (keep existing imports and TauriAppState struct)

pub fn run() {
    let settings = Arc::new(Mutex::new(Settings::load()));
    let app_state = AppState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(TauriAppState {
            app_state: app_state.clone(),
            settings: settings.clone(),
        })
        .invoke_handler(tauri::generate_handler![
            // ... all commands
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            events::start_game_state_emitter(handle);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: Verify compiles**

```bash
cargo check -p dota2-scripts-tauri
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/events.rs src-tauri/src/lib.rs
git commit -m "feat(tauri): add polling event bridge for real-time game state"
```

---

### Task 9: Wire background tasks into Tauri setup

**Files:**
- Modify: `src-tauri/src/lib.rs` (full boot sequence)

This task mirrors the current `main.rs` boot sequence but within Tauri's setup hook.

- [ ] **Step 1: Implement full boot sequence in lib.rs**

Replace the `run()` function in `src-tauri/src/lib.rs` with the full boot sequence:

```rust
pub mod commands;
pub mod events;
pub mod ipc_types;

use dota2_scripts::actions::executor::ActionExecutor;
use dota2_scripts::actions::ActionDispatcher;
use dota2_scripts::config::Settings;
use dota2_scripts::gsi::start_gsi_server;
use dota2_scripts::input::keyboard::{start_keyboard_listener, KeyboardSnapshot};
use dota2_scripts::state::{AppState, HeroType, UpdateCheckState};
use dota2_scripts::update::{check_for_update, UpdateCheckResult};
use dota2_scripts::models::Hero;
use std::sync::{Arc, Mutex, RwLock};
use tracing::info;

pub struct TauriAppState {
    pub app_state: Arc<Mutex<dota2_scripts::state::AppState>>,
    pub settings: Arc<Mutex<Settings>>,
}

pub fn run() {
    // Initialize logging
    let settings = Arc::new(Mutex::new(Settings::load()));
    let log_level = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| settings.lock().unwrap().logging.level.clone());
    tracing_subscriber::fmt().with_env_filter(log_level).init();

    info!("Starting Dota 2 Script Automation (Tauri)...");

    let app_state = AppState::new();

    // Build initial keyboard snapshot
    let initial_snapshot = {
        let settings_guard = settings.lock().unwrap();
        let state_guard = app_state.lock().unwrap();
        Arc::new(RwLock::new(KeyboardSnapshot::from_runtime(
            &settings_guard,
            &state_guard,
        )))
    };

    // Initialize action dispatcher
    let action_executor = ActionExecutor::new();
    let dispatcher = Arc::new(ActionDispatcher::new(settings.clone(), action_executor));

    // Start keyboard listener
    let keyboard_config = dota2_scripts::input::keyboard::KeyboardListenerConfig {
        snapshot: initial_snapshot.clone(),
    };
    let hotkey_rx = start_keyboard_listener(keyboard_config);

    // Clone references for background tasks
    let gsi_app_state = app_state.clone();
    let gsi_dispatcher = dispatcher.clone();
    let gsi_settings = settings.clone();
    let gsi_port = settings.lock().unwrap().server.port;

    let hotkey_app_state = app_state.clone();
    let hotkey_dispatcher = dispatcher.clone();

    let update_app_state = app_state.clone();
    let update_settings = settings.clone();

    let minimap_settings = settings.clone();
    let minimap_state = app_state.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(TauriAppState {
            app_state: app_state.clone(),
            settings: settings.clone(),
        })
        .invoke_handler(tauri::generate_handler![
            commands::config::get_config,
            commands::config::update_config,
            commands::config::update_hero_config,
            commands::state::get_app_state,
            commands::state::set_gsi_enabled,
            commands::state::set_standalone_enabled,
            commands::state::select_hero,
            commands::game::get_game_state,
            commands::diagnostics::get_diagnostics,
            commands::updates::get_update_state,
            commands::updates::check_for_updates,
            commands::updates::apply_update,
            commands::updates::dismiss_update,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();

            // Start GSI server
            tauri::async_runtime::spawn(async move {
                start_gsi_server(gsi_port, gsi_app_state, gsi_dispatcher, gsi_settings).await;
            });

            // Start update check (if enabled)
            {
                let check_on_startup = update_settings.lock().unwrap().updates.check_on_startup;
                let include_prereleases =
                    update_settings.lock().unwrap().updates.include_prereleases;

                if check_on_startup {
                    let update_state = update_app_state.lock().unwrap().update_state.clone();
                    *update_state.lock().unwrap() = UpdateCheckState::Checking;

                    tokio::task::spawn_blocking(move || match check_for_update(include_prereleases)
                    {
                        UpdateCheckResult::Available(info) => {
                            *update_state.lock().unwrap() = UpdateCheckState::Available {
                                version: info.version,
                                release_notes: info.release_notes,
                            };
                        }
                        UpdateCheckResult::UpToDate => {
                            *update_state.lock().unwrap() = UpdateCheckState::UpToDate;
                        }
                        UpdateCheckResult::Error(msg) => {
                            *update_state.lock().unwrap() = UpdateCheckState::Error(msg);
                        }
                    });
                }
            }

            // Start minimap capture worker
            std::thread::spawn(move || {
                dota2_scripts::observability::minimap_capture::start_minimap_capture_worker(
                    minimap_settings,
                    minimap_state,
                );
            });

            // Start hotkey event handler
            std::thread::spawn(move || {
                while let Ok(event) = hotkey_rx.recv() {
                    handle_hotkey_event(event, &hotkey_app_state, &hotkey_dispatcher);
                }
            });

            // Start game state emitter (polls AppState → emits to frontend)
            events::start_game_state_emitter(handle);

            info!("All background tasks started");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn handle_hotkey_event(
    event: dota2_scripts::input::keyboard::HotkeyEvent,
    app_state: &Arc<Mutex<dota2_scripts::state::AppState>>,
    dispatcher: &Arc<ActionDispatcher>,
) {
    use dota2_scripts::input::keyboard::HotkeyEvent;

    match event {
        HotkeyEvent::ComboTrigger => {
            let state = app_state.lock().unwrap();
            if state.standalone_enabled {
                if let Some(hero_type) = state.selected_hero {
                    let hero_name = match hero_type {
                        HeroType::Huskar => Hero::Huskar.to_game_name(),
                        HeroType::Largo => Hero::Largo.to_game_name(),
                        HeroType::LegionCommander => Hero::LegionCommander.to_game_name(),
                        HeroType::Meepo => Hero::Meepo.to_game_name(),
                        HeroType::OutworldDestroyer => Hero::ObsidianDestroyer.to_game_name(),
                        HeroType::ShadowFiend => Hero::Nevermore.to_game_name(),
                        HeroType::Tiny => Hero::Tiny.to_game_name(),
                    };
                    info!("Triggering standalone combo for {}", hero_name);
                    drop(state);
                    dispatcher.dispatch_standalone_trigger(hero_name);
                }
            }
        }
        HotkeyEvent::MeepoFarmToggle => {
            let state = app_state.lock().unwrap();
            if state.standalone_enabled && state.selected_hero == Some(HeroType::Meepo) {
                drop(state);
                if let Some(script) = dispatcher.hero_scripts.get(Hero::Meepo.to_game_name()) {
                    if let Some(meepo) = script
                        .as_any()
                        .downcast_ref::<dota2_scripts::actions::heroes::MeepoScript>()
                    {
                        meepo.toggle_farm_assist();
                    }
                }
            }
        }
        HotkeyEvent::LargoQ | HotkeyEvent::LargoW | HotkeyEvent::LargoE => {
            let state = app_state.lock().unwrap();
            if state.standalone_enabled && state.selected_hero == Some(HeroType::Largo) {
                drop(state);
                if let Some(script) = dispatcher.hero_scripts.get(Hero::Largo.to_game_name()) {
                    if let Some(largo) = script
                        .as_any()
                        .downcast_ref::<dota2_scripts::actions::heroes::LargoScript>()
                    {
                        use dota2_scripts::actions::heroes::largo::Song;
                        let song = match event {
                            HotkeyEvent::LargoQ => Song::Bullbelly,
                            HotkeyEvent::LargoW => Song::Hotfeet,
                            HotkeyEvent::LargoE => Song::IslandElixir,
                            _ => unreachable!(),
                        };
                        largo.select_song_manually(song);
                    }
                }
            }
        }
        HotkeyEvent::LargoR => {
            let state = app_state.lock().unwrap();
            if state.standalone_enabled && state.selected_hero == Some(HeroType::Largo) {
                drop(state);
                if let Some(script) = dispatcher.hero_scripts.get(Hero::Largo.to_game_name()) {
                    if let Some(largo) = script
                        .as_any()
                        .downcast_ref::<dota2_scripts::actions::heroes::LargoScript>()
                    {
                        largo.deactivate_ultimate();
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 2: Verify compiles**

```bash
cargo check -p dota2-scripts-tauri
```

Fix any compilation errors — the main challenges will be:
- Ensuring all library types are `pub` and accessible
- Matching the exact import paths for heroes, models, etc.
- Resolving any `tokio` runtime conflicts

- [ ] **Step 3: Run existing library tests to verify no breakage**

```bash
cargo test -p dota2-scripts
```
Expected: all existing tests pass (library crate unchanged).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(tauri): wire full boot sequence into Tauri setup hook"
```

---

## Phase 5: Frontend IPC Wiring

### Task 10: Install @tauri-apps/api and create hooks

**Files:**
- Modify: `src-ui/package.json` (add dependency)
- Create: `src-ui/src/lib/tauri.ts`
- Create: `src-ui/src/hooks/useTauriCommand.ts`
- Create: `src-ui/src/hooks/useTauriEvent.ts`
- Create: `src-ui/src/hooks/index.ts`

- [ ] **Step 1: Install @tauri-apps/api**

```bash
cd src-ui
npm install @tauri-apps/api@^2
```

- [ ] **Step 2: Create lib/tauri.ts (Tauri detection helper)**

```typescript
/// Returns true when running inside a Tauri webview
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}
```

- [ ] **Step 3: Create hooks/useTauriCommand.ts**

```typescript
import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { isTauri } from "../lib/tauri";

interface UseTauriCommandResult<T> {
  data: T | null;
  loading: boolean;
  error: string | null;
  execute: (...args: unknown[]) => Promise<T | null>;
}

/**
 * Hook for invoking Tauri commands. Falls back to no-op when not in Tauri.
 * @param command The Tauri command name
 */
export function useTauriCommand<T>(command: string): UseTauriCommandResult<T> {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const execute = useCallback(
    async (...args: unknown[]): Promise<T | null> => {
      if (!isTauri()) return null;

      setLoading(true);
      setError(null);
      try {
        const result = await invoke<T>(command, args[0] as Record<string, unknown> | undefined);
        setData(result);
        return result;
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        setError(msg);
        return null;
      } finally {
        setLoading(false);
      }
    },
    [command],
  );

  return { data, loading, error, execute };
}

/**
 * Direct invoke helper for use outside React components (e.g., in stores)
 */
export async function tauriInvoke<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!isTauri()) {
    throw new Error(`Not in Tauri environment (tried to call '${command}')`);
  }
  return invoke<T>(command, args);
}
```

- [ ] **Step 4: Create hooks/useTauriEvent.ts**

```typescript
import { useEffect } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { isTauri } from "../lib/tauri";

/**
 * Hook that subscribes to a Tauri event and calls the handler on each emission.
 * Automatically unsubscribes on unmount.
 */
export function useTauriEvent<T>(
  eventName: string,
  handler: (payload: T) => void,
): void {
  useEffect(() => {
    if (!isTauri()) return;

    let unlisten: UnlistenFn | undefined;

    listen<T>(eventName, (event) => {
      handler(event.payload);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, [eventName, handler]);
}
```

- [ ] **Step 5: Create hooks/index.ts**

```typescript
export { useTauriCommand, tauriInvoke } from "./useTauriCommand";
export { useTauriEvent } from "./useTauriEvent";
```

- [ ] **Step 6: Verify builds**

```bash
cd src-ui
npm run build
```
Expected: builds without errors.

- [ ] **Step 7: Commit**

```bash
git add src-ui/package.json src-ui/package-lock.json src-ui/src/lib/ src-ui/src/hooks/
git commit -m "feat(ui): add Tauri IPC hooks and detection helpers"
```

---

### Task 11: Wire configStore to Tauri

**Files:**
- Modify: `src-ui/src/stores/configStore.ts`

The store loads config from Tauri on initialization and debounces write-backs.

- [ ] **Step 1: Rewrite configStore.ts**

```typescript
import { create } from "zustand";
import type { Settings } from "../types/config";
import { mockConfig } from "./mockData";
import { isTauri } from "../lib/tauri";

interface ConfigStore {
  config: Settings;
  loaded: boolean;
  loadConfig: () => Promise<void>;
  updateConfig: <K extends keyof Settings>(
    section: K,
    updates: Partial<Settings[K]>,
  ) => void;
  updateHeroConfig: <K extends keyof Settings["heroes"]>(
    hero: K,
    updates: Partial<Settings["heroes"][K]>,
  ) => void;
}

// Debounce timers per section
const debounceTimers: Record<string, ReturnType<typeof setTimeout>> = {};
const DEBOUNCE_MS = 300;

function debouncedPersist(section: string, updates: Record<string, unknown>) {
  if (!isTauri()) return;

  const key = `config:${section}`;
  if (debounceTimers[key]) clearTimeout(debounceTimers[key]);

  debounceTimers[key] = setTimeout(async () => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("update_config", { section, updates });
    } catch (e) {
      console.error(`Failed to persist config section '${section}':`, e);
    }
  }, DEBOUNCE_MS);
}

function debouncedPersistHero(hero: string, updates: Record<string, unknown>) {
  if (!isTauri()) return;

  const key = `hero:${hero}`;
  if (debounceTimers[key]) clearTimeout(debounceTimers[key]);

  debounceTimers[key] = setTimeout(async () => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("update_hero_config", { hero, updates });
    } catch (e) {
      console.error(`Failed to persist hero config '${hero}':`, e);
    }
  }, DEBOUNCE_MS);
}

export const useConfigStore = create<ConfigStore>((set) => ({
  config: mockConfig,
  loaded: false,

  loadConfig: async () => {
    if (!isTauri()) {
      set({ loaded: true });
      return;
    }
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const config = await invoke<Settings>("get_config");
      set({ config, loaded: true });
    } catch (e) {
      console.error("Failed to load config:", e);
      set({ loaded: true }); // use mock defaults on error
    }
  },

  updateConfig: (section, updates) => {
    set((state) => {
      const newConfig = {
        ...state.config,
        [section]: { ...state.config[section], ...updates },
      };
      debouncedPersist(section, updates as Record<string, unknown>);
      return { config: newConfig };
    });
  },

  updateHeroConfig: (hero, updates) => {
    set((state) => {
      const newConfig = {
        ...state.config,
        heroes: {
          ...state.config.heroes,
          [hero]: { ...state.config.heroes[hero], ...updates },
        },
      };
      debouncedPersistHero(hero, updates as Record<string, unknown>);
      return { config: newConfig };
    });
  },
}));
```

- [ ] **Step 2: Verify builds**

```bash
cd src-ui && npm run build
```

- [ ] **Step 3: Run existing tests**

```bash
cd src-ui && npm test -- --run
```
Expected: existing tests still pass.

- [ ] **Step 4: Commit**

```bash
git add src-ui/src/stores/configStore.ts
git commit -m "feat(ui): wire configStore to Tauri with debounced persistence"
```

---

### Task 12: Wire gameStore and uiStore to Tauri

**Files:**
- Modify: `src-ui/src/stores/gameStore.ts`
- Modify: `src-ui/src/stores/uiStore.ts`
- Create: `src-ui/src/stores/updateStore.ts`
- Modify: `src-ui/src/stores/index.ts`

- [ ] **Step 1: Rewrite gameStore.ts with Tauri event subscription**

```typescript
import { create } from "zustand";
import type { GameState, DiagnosticsState } from "../types/game";
import { isTauri } from "../lib/tauri";

interface GameStore {
  game: GameState;
  diagnostics: DiagnosticsState;
  setGame: (game: Partial<GameState>) => void;
  setDiagnostics: (diagnostics: DiagnosticsState) => void;
  startListening: () => Promise<void>;
}

export const useGameStore = create<GameStore>((set) => ({
  game: {
    heroName: null,
    heroLevel: 0,
    hpPercent: 100,
    manaPercent: 100,
    inDanger: false,
    connected: false,
    alive: true,
    stunned: false,
    silenced: false,
    respawnTimer: null,
    runeTimer: null,
    gameTime: 0,
  },
  diagnostics: {
    gsiConnected: false,
    keyboardHookActive: false,
    queueMetrics: {
      eventsProcessed: 0,
      eventsDropped: 0,
      currentQueueDepth: 0,
      maxQueueDepth: 10,
    },
    syntheticInput: {
      queueDepth: 0,
      totalQueued: 0,
      peakDepth: 0,
      completions: 0,
      drops: 0,
    },
    soulRingState: "ready",
    blockedKeys: [],
  },

  setGame: (partial) =>
    set((state) => ({ game: { ...state.game, ...partial } })),

  setDiagnostics: (diagnostics) => set({ diagnostics }),

  startListening: async () => {
    if (!isTauri()) return;

    const { listen } = await import("@tauri-apps/api/event");

    // Subscribe to real-time game state updates from Rust
    listen<GameState>("gsi_update", (event) => {
      set({ game: event.payload });
    });

    // Initial diagnostics fetch
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const diag = await invoke<DiagnosticsState>("get_diagnostics");
      set({ diagnostics: diag });
    } catch (e) {
      console.error("Failed to fetch diagnostics:", e);
    }
  },
}));
```

- [ ] **Step 2: Create updateStore.ts (separated from gameStore)**

```typescript
import { create } from "zustand";
import type { UpdateCheckState } from "../types/game";
import { isTauri } from "../lib/tauri";

interface UpdateStore {
  updateState: UpdateCheckState;
  setUpdateState: (state: UpdateCheckState) => void;
  checkForUpdates: () => Promise<void>;
  applyUpdate: () => Promise<void>;
  dismissUpdate: () => void;
  loadInitialState: () => Promise<void>;
}

export const useUpdateStore = create<UpdateStore>((set) => ({
  updateState: { kind: "idle" },

  setUpdateState: (updateState) => set({ updateState }),

  loadInitialState: async () => {
    if (!isTauri()) return;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const state = await invoke<UpdateCheckState>("get_update_state");
      set({ updateState: state });
    } catch (e) {
      console.error("Failed to load update state:", e);
    }
  },

  checkForUpdates: async () => {
    if (!isTauri()) return;
    set({ updateState: { kind: "checking" } });
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const result = await invoke<UpdateCheckState>("check_for_updates");
      set({ updateState: result });
    } catch (e) {
      set({
        updateState: {
          kind: "error",
          message: e instanceof Error ? e.message : String(e),
        },
      });
    }
  },

  applyUpdate: async () => {
    if (!isTauri()) return;
    set({ updateState: { kind: "downloading" } });
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("apply_update");
    } catch (e) {
      set({
        updateState: {
          kind: "error",
          message: e instanceof Error ? e.message : String(e),
        },
      });
    }
  },

  dismissUpdate: () => {
    set({ updateState: { kind: "idle" } });
    if (isTauri()) {
      import("@tauri-apps/api/core").then(({ invoke }) => {
        invoke("dismiss_update").catch(console.error);
      });
    }
  },
}));
```

- [ ] **Step 3: Wire uiStore toggles to Tauri commands**

Rewrite `src-ui/src/stores/uiStore.ts`:

```typescript
import { create } from "zustand";
import { isTauri } from "../lib/tauri";

interface UIStore {
  sidebarCollapsed: boolean;
  toggleSidebar: () => void;
  gsiEnabled: boolean;
  standaloneEnabled: boolean;
  setGsiEnabled: (enabled: boolean) => void;
  setStandaloneEnabled: (enabled: boolean) => void;
  loadInitialState: () => Promise<void>;
}

export const useUIStore = create<UIStore>((set) => ({
  sidebarCollapsed: false,
  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
  gsiEnabled: true,
  standaloneEnabled: false,

  setGsiEnabled: (enabled) => {
    set({ gsiEnabled: enabled });
    if (isTauri()) {
      import("@tauri-apps/api/core").then(({ invoke }) => {
        invoke("set_gsi_enabled", { enabled }).catch(console.error);
      });
    }
  },

  setStandaloneEnabled: (enabled) => {
    set({ standaloneEnabled: enabled });
    if (isTauri()) {
      import("@tauri-apps/api/core").then(({ invoke }) => {
        invoke("set_standalone_enabled", { enabled }).catch(console.error);
      });
    }
  },

  loadInitialState: async () => {
    if (!isTauri()) return;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const state = await invoke<{
        selectedHero: string | null;
        gsiEnabled: boolean;
        standaloneEnabled: boolean;
      }>("get_app_state");
      set({
        gsiEnabled: state.gsiEnabled,
        standaloneEnabled: state.standaloneEnabled,
      });
    } catch (e) {
      console.error("Failed to load app state:", e);
    }
  },
}));
```

- [ ] **Step 4: Update stores/index.ts**

```typescript
export { useConfigStore } from "./configStore";
export { useGameStore } from "./gameStore";
export { useUIStore } from "./uiStore";
export { useActivityStore } from "./activityStore";
export { useUpdateStore } from "./updateStore";
```

- [ ] **Step 5: Verify builds**

```bash
cd src-ui && npm run build
```

- [ ] **Step 6: Run tests**

```bash
cd src-ui && npm test -- --run
```

- [ ] **Step 7: Commit**

```bash
git add src-ui/src/stores/
git commit -m "feat(ui): wire game, update, and UI stores to Tauri IPC"
```

---

### Task 13: Add initialization effect to App.tsx

**Files:**
- Modify: `src-ui/src/App.tsx`

- [ ] **Step 1: Add init effect that loads all stores**

Update `src-ui/src/App.tsx` — add a `useEffect` at the top of the `App` component:

```typescript
import { useEffect } from "react";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import { Sidebar } from "./components/layout/Sidebar";
import { StatusHeader } from "./components/layout/StatusHeader";
import { ActivityTicker } from "./components/layout/ActivityTicker";
import { useGameStore } from "./stores/gameStore";
import { useActivityStore } from "./stores/activityStore";
import { useConfigStore } from "./stores/configStore";
import { useUIStore } from "./stores/uiStore";
import { useUpdateStore } from "./stores/updateStore";
import Dashboard from "./pages/Dashboard";
import Heroes from "./pages/Heroes";
import HeroDetail from "./pages/HeroDetail";
import DangerDetection from "./pages/DangerDetection";
import SoulRing from "./pages/SoulRing";
import Armlet from "./pages/Armlet";
import ActivityLog from "./pages/ActivityLog";
import Diagnostics from "./pages/Diagnostics";
import Settings from "./pages/Settings";

export default function App() {
  const game = useGameStore((s) => s.game);
  const entries = useActivityStore((s) => s.entries);

  // Initialize all stores from Tauri backend on mount
  useEffect(() => {
    useConfigStore.getState().loadConfig();
    useUIStore.getState().loadInitialState();
    useGameStore.getState().startListening();
    useUpdateStore.getState().loadInitialState();
  }, []);

  const tickerEntries = entries.slice(-3).map((e) => ({
    id: e.id,
    timestamp: e.timestamp,
    category: e.category as "action" | "danger" | "warning" | "system",
    message: e.message,
  }));

  return (
    <BrowserRouter>
      <div className="flex h-screen w-screen overflow-hidden bg-base">
        <Sidebar />
        <div className="flex flex-1 flex-col overflow-hidden">
          <StatusHeader
            heroName={game.heroName ?? undefined}
            heroLevel={game.heroLevel}
            hpPercent={game.hpPercent}
            manaPercent={game.manaPercent}
            inDanger={game.inDanger}
            connected={game.connected}
            runeTimer={game.runeTimer}
            stunned={game.stunned}
            silenced={game.silenced}
            alive={game.alive}
            respawnTimer={game.respawnTimer}
          />
          <main className="flex-1 overflow-y-auto">
            <Routes>
              <Route path="/" element={<Dashboard />} />
              <Route path="/heroes" element={<Heroes />} />
              <Route path="/heroes/:heroId" element={<HeroDetail />} />
              <Route path="/danger" element={<DangerDetection />} />
              <Route path="/soul-ring" element={<SoulRing />} />
              <Route path="/armlet" element={<Armlet />} />
              <Route path="/activity" element={<ActivityLog />} />
              <Route path="/diagnostics" element={<Diagnostics />} />
              <Route path="/settings" element={<Settings />} />
            </Routes>
          </main>
          <ActivityTicker entries={tickerEntries} />
        </div>
      </div>
    </BrowserRouter>
  );
}
```

- [ ] **Step 2: Verify builds**

```bash
cd src-ui && npm run build
```

- [ ] **Step 3: Commit**

```bash
git add src-ui/src/App.tsx
git commit -m "feat(ui): add Tauri initialization effect to App shell"
```

---

## Phase 6: Deferred UI Features

### Task 14: Update banner component

**Files:**
- Create: `src-ui/src/components/layout/UpdateBanner.tsx`
- Modify: `src-ui/src/App.tsx` (add banner above routes)

- [ ] **Step 1: Create UpdateBanner.tsx**

```typescript
import { useUpdateStore } from "../../stores/updateStore";
import { Button } from "../common/Button";

export function UpdateBanner() {
  const updateState = useUpdateStore((s) => s.updateState);
  const applyUpdate = useUpdateStore((s) => s.applyUpdate);
  const dismissUpdate = useUpdateStore((s) => s.dismissUpdate);

  if (updateState.kind !== "available") return null;

  return (
    <div className="flex items-center justify-between gap-4 border-b border-border bg-elevated px-4 py-2">
      <div className="flex items-center gap-2">
        <span className="text-sm font-medium text-gold">
          🎉 Update v{updateState.version} available
        </span>
        {updateState.releaseNotes && (
          <span className="text-xs text-subtle">
            — {updateState.releaseNotes}
          </span>
        )}
      </div>
      <div className="flex items-center gap-2">
        <Button size="sm" onClick={applyUpdate}>
          Apply Update
        </Button>
        <button
          type="button"
          onClick={dismissUpdate}
          className="text-xs text-subtle hover:text-content"
        >
          Dismiss
        </button>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Add UpdateBanner to App.tsx**

Import and place it above the `<main>` element:

```typescript
import { UpdateBanner } from "./components/layout/UpdateBanner";
```

Add inside the right column div, between `StatusHeader` and `<main>`:
```tsx
<UpdateBanner />
```

- [ ] **Step 3: Verify builds**

```bash
cd src-ui && npm run build
```

- [ ] **Step 4: Commit**

```bash
git add src-ui/src/components/layout/UpdateBanner.tsx src-ui/src/App.tsx
git commit -m "feat(ui): add update banner with apply/dismiss"
```

---

### Task 15: Connection state indicator in StatusHeader

**Files:**
- Modify: `src-ui/src/components/layout/StatusHeader.tsx`

- [ ] **Step 1: Add connection dot indicator**

In the `StatusHeader` component, find the section that displays hero info and add a connection status dot. Look for where `heroName` or `connected` is used and add:

```tsx
{/* Connection indicator */}
<div className="flex items-center gap-1.5">
  <span
    className={`inline-block h-2 w-2 rounded-full ${
      connected ? "bg-success" : "bg-danger animate-pulse"
    }`}
  />
  <span className="text-xs text-subtle">
    {connected ? "GSI Connected" : "Disconnected"}
  </span>
</div>
```

Place this near the top-left of the header, before the hero name/stats.

Later implementation note:
- Treat this indicator as freshness-based, not "we have ever received at least one GSI payload".
- Include the post-match / stale-GSI scenario in Pencil and mock designs so the header does not imply an active match after GSI goes stale.

- [ ] **Step 2: Verify builds**

```bash
cd src-ui && npm run build
```

- [ ] **Step 3: Commit**

```bash
git add src-ui/src/components/layout/StatusHeader.tsx
git commit -m "feat(ui): add GSI connection indicator to status header"
```

---

### Task 16: Sidebar collapse/expand

**Files:**
- Modify: `src-ui/src/components/layout/Sidebar.tsx`

- [ ] **Step 1: Add collapse toggle and responsive width**

Update the Sidebar component to read `sidebarCollapsed` from `uiStore` and toggle width:

Key changes:
1. Import `useUIStore`
2. Read `sidebarCollapsed` and `toggleSidebar`
3. Outer container: `w-[200px]` when expanded, `w-[60px]` when collapsed, with `transition-all duration-200`
4. Conditionally hide labels when collapsed (show only icons)
5. Add a collapse/expand toggle button at the bottom

```tsx
import { useUIStore } from "../../stores/uiStore";
import { ChevronLeft, ChevronRight } from "lucide-react";

// In the component:
const sidebarCollapsed = useUIStore((s) => s.sidebarCollapsed);
const toggleSidebar = useUIStore((s) => s.toggleSidebar);

// Container classes:
className={`flex flex-col border-r border-border bg-base transition-all duration-200 ${
  sidebarCollapsed ? "w-[60px]" : "w-[200px]"
}`}

// Nav item labels hidden when collapsed:
{!sidebarCollapsed && <span>{item.label}</span>}

// Toggle button at bottom of sidebar:
<button
  type="button"
  onClick={toggleSidebar}
  className="flex items-center justify-center border-t border-border p-3 text-subtle hover:text-content"
>
  {sidebarCollapsed ? <ChevronRight size={16} /> : <ChevronLeft size={16} />}
</button>
```

- [ ] **Step 2: Verify builds**

```bash
cd src-ui && npm run build
```

- [ ] **Step 3: Commit**

```bash
git add src-ui/src/components/layout/Sidebar.tsx
git commit -m "feat(ui): add sidebar collapse/expand toggle"
```

---

### Task 17: Page transitions and activity log expand

**Files:**
- Modify: `src-ui/src/App.tsx` (add transition wrapper)
- Modify: `src-ui/src/pages/ActivityLog.tsx` (add expand-on-click)

- [ ] **Step 1: Add fade transition to page content**

In `App.tsx`, wrap the `<Routes>` content with a simple CSS fade. Add to `src-ui/src/styles/global.css`:

```css
@keyframes page-fade-in {
  from { opacity: 0; }
  to { opacity: 1; }
}

.page-transition {
  animation: page-fade-in 150ms ease-out;
}
```

Then wrap each Route's element with a div that has this class, or apply it at the `<main>` level:
```tsx
<main className="flex-1 overflow-y-auto page-transition">
```

Note: for a true per-page transition you'd need React Transition Group or Framer Motion. The simple CSS animation on `<main>` provides a good-enough fade effect when the route changes.

- [ ] **Step 2: Add expand-on-click to ActivityLog entries**

In `src-ui/src/pages/ActivityLog.tsx`, add state for expanded entries:

```typescript
const [expandedId, setExpandedId] = useState<string | null>(null);
```

Update the entry rendering to toggle expansion on click:
```tsx
<div
  key={entry.id}
  onClick={() => setExpandedId(expandedId === entry.id ? null : entry.id)}
  className="cursor-pointer rounded px-2 py-1 hover:bg-elevated"
>
  <div className="flex gap-2">
    <span className="text-muted shrink-0">{entry.timestamp}</span>
    <span className={categoryColor[entry.category] ?? "text-content"}>
      {entry.message}
    </span>
  </div>
  {expandedId === entry.id && entry.details && (
    <div className="mt-1 pl-20 text-xs text-subtle">{entry.details}</div>
  )}
</div>
```

- [ ] **Step 3: Verify builds**

```bash
cd src-ui && npm run build
```

- [ ] **Step 4: Commit**

```bash
git add src-ui/src/styles/global.css src-ui/src/App.tsx src-ui/src/pages/ActivityLog.tsx
git commit -m "feat(ui): add page transitions and activity log expand"
```

---

## Phase 7: Cleanup and Verification

### Task 18: Feature-gate egui dependency

**Files:**
- Modify: `Cargo.toml` (root — add feature flag)

- [ ] **Step 1: Gate egui behind a feature flag**

In the root `Cargo.toml`, add a feature section:

```toml
[features]
default = ["egui-ui"]
egui-ui = ["egui", "eframe"]
```

Then change the egui/eframe dependencies to optional:
```toml
egui = { version = "0.29", optional = true }
eframe = { version = "0.29", optional = true }
```

- [ ] **Step 2: Gate the UI module behind the feature**

In `src/lib.rs`, conditionally include the UI module:
```rust
#[cfg(feature = "egui-ui")]
pub mod ui;
```

In `src/main.rs`, gate the egui-specific code:
```rust
#[cfg(feature = "egui-ui")]
mod ui;
```

The entire `main.rs` can be gated since it's the egui entry point — the Tauri binary has its own entry point in `src-tauri/`.

- [ ] **Step 3: Verify both targets compile**

```bash
# Library + Tauri (no egui)
cargo check -p dota2-scripts --no-default-features
cargo check -p dota2-scripts-tauri

# Library + egui (original binary)
cargo check -p dota2-scripts
```

- [ ] **Step 4: Verify existing tests still pass**

```bash
cargo test
```

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/lib.rs src/main.rs
git commit -m "refactor: feature-gate egui UI behind 'egui-ui' feature flag"
```

---

### Task 19: Full integration verification

**Files:** No new files — verification only.

- [ ] **Step 1: Run all Rust tests**

```bash
cargo test
```
Expected: all tests pass (library tests + Tauri DTO tests).

- [ ] **Step 2: Run React tests**

```bash
cd src-ui && npm test -- --run
```
Expected: all tests pass.

- [ ] **Step 3: Build React production bundle**

```bash
cd src-ui && npm run build
```
Expected: production build succeeds.

- [ ] **Step 4: Build Tauri binary**

```bash
cargo build -p dota2-scripts-tauri
```
Expected: binary compiles.

- [ ] **Step 5: Smoke test the Tauri app**

```bash
cargo run -p dota2-scripts-tauri
```

Expected behavior:
1. Window opens with React UI
2. Sidebar navigation works
3. Config pages show loaded config (from `config/config.toml`)
4. GSI toggle and standalone toggle work
5. When Dota 2 sends GSI events, the status header updates in real-time
6. If an update is available, the update banner appears

- [ ] **Step 6: Verify existing egui binary still works**

```bash
cargo run -p dota2-scripts
```
Expected: the original egui UI opens and works as before.

- [ ] **Step 7: Commit any final fixes**

```bash
git add -A
git commit -m "chore: final integration verification fixes"
```

---

## Deferred to Plan 3 (Future Enhancement)

These items are not blocking for the Tauri integration but should be addressed later:

- **Activity event emission from Rust**: Add `tracing`-based or channel-based activity broadcasting from hero scripts, danger detector, and common actions. Currently activity log stays with mock data.
- **Danger state wiring**: The `in_danger` field in `GameStateDto` is hardcoded to `false`. Wire it to the actual `DangerDetector` state.
- **Synthetic input metrics**: The `SyntheticInputDto` returns zeros. Wire to actual `ActionExecutor` metrics.
- **Soul ring state + blocked keys**: Wire to actual keyboard hook state for diagnostics page.
- **Config validation**: Backend validation before persisting (e.g., port range, threshold bounds).
- **Meepo observed state panel**: Display Meepo-specific real-time state on the hero detail page.
- **Rune alert audio**: Play audio alert from the frontend when rune timer triggers.
- **Keyboard shortcut display**: Show hero-specific key bindings on hero detail pages.
- **Auto-resize on content**: Dynamic window sizing based on active page content.
