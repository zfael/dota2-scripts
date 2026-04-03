# Updates

**Purpose**: Read this before changing startup update checks, the GitHub Releases integration, or the UI flow for download/apply/restart.

---

## Ownership map

| Path | What it owns |
|---|---|
| `src/update/mod.rs` | Version check, GitHub release filtering, MSI/template apply flow |
| `src/update/msi.rs` | MSI asset selection, ZIP-layout guard, temp downloads, PowerShell handoff |
| `src/main.rs` | Startup update-check trigger and initial `UpdateCheckState` transitions |
| `src/ui/app.rs` | Update banner, Settings tab controls, retry/download threads |
| `src/state/app_state.rs` | `UpdateCheckState` stored under `AppState.update_state` |
| `src/config/settings.rs` | `UpdateConfig` defaults plus live-config load/save through LocalAppData |
| `src/config/storage.rs` | Live config path resolution, legacy import, merge helpers |
| `config/config.toml` | Checked-in template used for defaults, packaging, and release assets |

---

## Startup flow

Startup check is triggered from Rust boot and surfaced through the React/Tauri frontend.

Current flow:

1. Rust loads `settings.updates.check_on_startup`
2. Rust loads `settings.updates.include_prereleases`
3. if startup checks are enabled:
   - clone `AppState.update_state`
   - set it to `UpdateCheckState::Checking`
   - run `check_for_update(include_prereleases)` in a background task
   - store `Available`, `UpToDate`, or `Error` back into `update_state`
4. React loads update state on mount with `get_update_state()`
5. if the first frontend snapshot is `Idle` or `Checking`, the update store re-reads `get_update_state()` on a short bounded interval until the state becomes terminal or the retry window expires

Possible state transitions at startup:

- `Idle` -> `Checking` -> `Available { .. }`
- `Idle` -> `Checking` -> `UpToDate`
- `Idle` -> `Checking` -> `Error(String)`

---

## How version checks work

`src/update/mod.rs::check_for_update(include_prereleases)`:

1. reads the current app version via `cargo_crate_version!()`
2. fetches releases from the GitHub Releases API
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
4. `apply_update()` downloads the latest MSI asset plus a matching `config.template.toml`
5. merge the downloaded template into `%LOCALAPPDATA%\dota2-scripts\config\config.toml` with local values winning
6. spawn a hidden PowerShell handoff that waits for the current PID to exit, runs `msiexec /i ... /qn /norestart`, then relaunches the app

Result handling:

| Result | Behavior |
|---|---|
| `ApplyUpdateResult::Success { new_version }` | reserved for success-shaped returns; the MSI path normally exits the process after launching the handoff |
| `ApplyUpdateResult::UpToDate` | set UI state to `UpToDate` |
| `ApplyUpdateResult::Error(msg)` | set UI state to `Error(msg)` |

### Config merge behavior

The live writable config is `%LOCALAPPDATA%\dota2-scripts\config\config.toml`.

Startup bootstrap rules:

1. if LocalAppData already has a live config, use it
2. else if an older install-local `config\config.toml` exists next to the executable, import it once
3. else seed LocalAppData from the embedded checked-in `config/config.toml`

Update merge rules:

1. download the release's `dota2-scripts-<tag>-config.template.toml`
2. merge template -> local live config recursively
3. when keys conflict, **local values win**
4. keys present only in the local config are preserved

### ZIP vs MSI behavior

- GitHub Releases still publish both ZIP and MSI assets.
- The app only supports **MSI-driven** in-app updates going forward.
- ZIP-style installs are expected to install the MSI manually once before using the in-app updater.
- The MSI upgrade path relaunches the app automatically after a successful silent install.

---

## UI flow

### Update banner (`src-ui/src/components/layout/UpdateBanner.tsx`)

Banner states:

| `UpdateCheckState` | UI behavior |
|---|---|
| `Checking` | spinner + "Checking for updates..." |
| `Available { version, release_notes }` | green banner with **Update Now** / **Later** and collapsible release notes |
| `Downloading` | spinner + "Downloading update..." |
| `Error(msg)` | red banner with **Retry** / **Dismiss** |
| `Idle`, `UpToDate` | no banner |

The banner no longer depends on a single startup snapshot. On app mount, the frontend update store performs an initial `get_update_state()` read and, if the backend is still `Idle` or `Checking`, briefly re-reads state until the startup check reaches `Available`, `UpToDate`, or `Error`.

### Settings tab (`src/ui/app.rs::render_settings_tab()`)

Exposes:

- `updates.check_on_startup`
- `updates.include_prereleases`
- current build version from `env!("CARGO_PKG_VERSION")`
- **Check for Updates Now** button
- current update state summary

### Manual retry path

Manual retry (`src-ui/src/stores/updateStore.ts::checkForUpdates()`):

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

Checked-in defaults in `config/config.toml` match those values, and the live LocalAppData config inherits from that template at first bootstrap.

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
