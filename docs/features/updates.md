# Updates

**Purpose**: Read this before changing startup update checks, the GitHub Releases integration, or the UI flow for download/apply/restart.

---

## Ownership map

| Path | What it owns |
|---|---|
| `src/update/mod.rs` | Version check, GitHub release filtering, update apply, process restart |
| `src/main.rs` | Startup update-check trigger and initial `UpdateCheckState` transitions |
| `src/ui/app.rs` | Update banner, Settings tab controls, retry/download threads |
| `src/state/app_state.rs` | `UpdateCheckState` stored under `AppState.update_state` |
| `src/config/settings.rs` | `UpdateConfig` defaults |
| `config/config.toml` | `[updates]` checked-in values |

---

## Startup flow

Startup check is triggered from `src/main.rs`.

Current flow:

1. load `settings.updates.check_on_startup`
2. load `settings.updates.include_prereleases`
3. if startup checks are enabled:
   - clone `AppState.update_state`
   - set it to `UpdateCheckState::Checking`
   - run `check_for_update(include_prereleases)` inside `tokio::task::spawn_blocking`
4. store the result back into `update_state`

Possible state transitions at startup:

- `Idle` -> `Checking` -> `Available { .. }`
- `Idle` -> `Checking` -> `UpToDate`
- `Idle` -> `Checking` -> `Error(String)`

---

## How version checks work

`src/update/mod.rs::check_for_update(include_prereleases)`:

1. reads the current app version via `cargo_crate_version!()`
2. fetches releases using `self_update::backends::github::ReleaseList`
3. filters prereleases unless `include_prereleases` is true
4. compares the newest candidate against the current version with `bump_is_greater(...)`

Current repository constants:

| Constant | Value |
|---|---|
| `REPO_OWNER` | `zfael` |
| `REPO_NAME` | `dota2-scripts` |

Current prerelease detection in `is_prerelease(version)` treats these substrings as prereleases:

- `-rc`
- `-alpha`
- `-beta`
- `-dev`

---

## Applying an update

`src/ui/app.rs::start_update()` is the UI entry point.

Flow:

1. set `UpdateCheckState::Downloading`
2. spawn a background thread
3. call `apply_update()`
4. if successful, call `restart_application()`

`src/update/mod.rs::apply_update()` configures `self_update::backends::github::Update` with:

- repo owner/name from above
- `bin_name("dota2-scripts")`
- `show_download_progress(false)`
- `no_confirm(true)`
- `current_version(cargo_crate_version!())`

Result handling:

| Result | Behavior |
|---|---|
| `ApplyUpdateResult::Success { new_version }` | log success and try to restart immediately |
| `ApplyUpdateResult::UpToDate` | set UI state to `UpToDate` |
| `ApplyUpdateResult::Error(msg)` | set UI state to `Error(msg)` |

### Restart behavior

`restart_application()`:

1. reads `std::env::current_exe()`
2. spawns a new process from that path
3. exits the current process with `std::process::exit(0)`

If the restart spawn fails, the UI state becomes:

- `Error("Update applied but restart failed: ... Please restart manually.")`

---

## UI flow

### Update banner (`src/ui/app.rs::render_update_banner()`)

Banner states:

| `UpdateCheckState` | UI behavior |
|---|---|
| `Checking` | spinner + "Checking for updates..." |
| `Available { version, release_notes }` | green banner with **Update Now** / **Later** and collapsible release notes |
| `Downloading` | spinner + "Downloading update..." |
| `Error(msg)` | red banner with **Retry** / **Dismiss** |
| `Idle`, `UpToDate` | no banner |

`update_dismissed` is a UI-only flag on `Dota2ScriptApp`; dismissing the banner does not change saved config.

### Settings tab (`src/ui/app.rs::render_settings_tab()`)

Exposes:

- `updates.check_on_startup`
- `updates.include_prereleases`
- current build version from `env!("CARGO_PKG_VERSION")`
- **Check for Updates Now** button
- current update state summary

### Manual retry path

`retry_update_check()`:

1. reads `settings.updates.include_prereleases`
2. sets `UpdateCheckState::Checking`
3. spawns a thread
4. calls `check_for_update(...)`
5. writes `Available`, `UpToDate`, or `Error`

---

## Config

`src/config/settings.rs` defines:

| Key | Type | Default | Meaning |
|---|---|---|---|
| `updates.check_on_startup` | `bool` | `true` | Run a background update check during app startup |
| `updates.include_prereleases` | `bool` | `false` | Consider `-rc`, `-alpha`, `-beta`, `-dev` releases eligible |

Checked-in defaults in `config/config.toml` match those values.

There is currently:

- no update channel concept beyond prerelease filtering
- no scheduled background polling after startup
- no persisted "dismiss this version" state

---

## Change checklist

If you change...

| Area | Also review |
|---|---|
| `src/update/mod.rs` | `src/main.rs`, `src/ui/app.rs`, `src/state/app_state.rs` |
| `[updates]` config | `src/config/settings.rs`, `config/config.toml`, `src/ui/app.rs` |
| `UpdateCheckState` variants | startup flow, update banner, settings-tab status, retry/apply paths |

Related docs:

- `docs/architecture/overview.md`
- `docs/architecture/runtime-flow.md`
