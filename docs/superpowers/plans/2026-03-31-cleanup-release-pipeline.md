# egui Removal, Release Pipeline & Update UX Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the egui UI framework entirely, migrate the release pipeline to Tauri builds, and fix the update banner to render GitHub-Flavored Markdown release notes.

**Architecture:** The egui code is behind feature gates (`egui-ui`) in 4 files. We remove the gates and dead code, delete `src/ui/`, update `release.yml` to use `npx tauri build`, and replace the inline release notes text in `UpdateBanner.tsx` with a modal using `react-markdown`.

**Tech Stack:** Rust (Cargo features), GitHub Actions, Tauri v2 CLI, React, react-markdown, remark-gfm, @tailwindcss/typography

---

## File Map

| File | Action | Purpose |
|------|--------|---------|
| `Cargo.toml` | Modify | Remove `[features]` section, `egui`/`eframe` deps |
| `src/lib.rs` | Modify | Remove `#[cfg(feature = "egui-ui")] pub mod ui;` |
| `src/main.rs` | Modify | Remove egui bootstrap, feature gates, `load_icon()` |
| `src/ui/mod.rs` | Delete | egui module exports |
| `src/ui/app.rs` | Delete | egui `Dota2ScriptApp` |
| `src-tauri/Cargo.toml` | Modify | Remove `default-features = false` |
| `src-tauri/tauri.conf.json` | Modify | Change `productName` to `dota2-scripts` |
| `.github/workflows/release.yml` | Modify | Switch to Tauri build, add MSI, sync tauri.conf.json version |
| `src-ui/package.json` | Modify | Add `react-markdown`, `remark-gfm`, `@tailwindcss/typography` |
| `src-ui/src/components/layout/UpdateBanner.tsx` | Modify | Add expandable release notes modal with markdown rendering |

---

### Task 1: Remove egui Dependencies and Feature Gates from Root Cargo.toml

**Files:**
- Modify: `Cargo.toml` (root)

- [ ] **Step 1: Remove the features section and egui dependencies**

In `Cargo.toml`, remove the `[features]` section (lines 18-20) and the egui/eframe dependencies (lines 39-41):

```toml
# DELETE these lines (18-20):
[features]
default = ["egui-ui"]
egui-ui = ["egui", "eframe"]

# DELETE these lines (39-41):
# UI
egui = { version = "0.29", optional = true }
eframe = { version = "0.29", features = ["default"], optional = true }
```

Also remove the `# UI` comment (line 39). The resulting `[dependencies]` section should go straight from `rdev` (line 48) to `# Utilities` (line 50) with no UI section in between.

- [ ] **Step 2: Verify the library builds without egui**

Run: `cargo check -p dota2-scripts --quiet`
Expected: Compilation errors in `src/main.rs` (references to egui types still exist). This is expected — Task 2 will fix them.

- [ ] **Step 3: Commit**

```
git add Cargo.toml
git commit -m "chore: remove egui-ui feature and dependencies from Cargo.toml"
```

---

### Task 2: Remove egui Code from src/main.rs

**Files:**
- Modify: `src/main.rs`

**Context:** After Task 1, the egui types no longer exist. We need to remove all references to them and make the headless loop unconditional.

- [ ] **Step 1: Remove egui-gated imports and module declaration**

Remove these lines from the top of `src/main.rs`:

```rust
// DELETE line 10-11:
#[cfg(feature = "egui-ui")]
mod ui;

// DELETE line 20-21:
#[cfg(feature = "egui-ui")]
use crate::ui::Dota2ScriptApp;
```

- [ ] **Step 2: Replace the UI boot section with unconditional headless loop**

Replace the entire block from line 251 to line 285 (the `// Start UI on main thread` comment through the end of the `#[cfg(not(feature = "egui-ui"))]` block):

```rust
// REMOVE lines 251-285 (the two #[cfg] blocks) and replace with:
    // Block the main thread so background tasks keep running
    // (The Tauri binary in src-tauri/ provides the GUI)
    info!("Backend running (headless mode). Use the Tauri app for the GUI.");
    loop {
        std::thread::park();
    }
```

- [ ] **Step 3: Remove the load_icon function**

Delete lines 288-319 entirely (the `#[cfg(feature = "egui-ui")] fn load_icon()` function and everything inside it).

- [ ] **Step 4: Clean up unused imports**

After removing the egui code, some imports may become unused. Check if `image` is still used anywhere in main.rs — it was only used in `load_icon()`. The `image` crate is still used by `src/observability/` so keep it in Cargo.toml but it should no longer be imported in main.rs.

Also verify these imports are still needed (they should be — they're used by the boot sequence):
- `crate::actions::executor::ActionExecutor` ✓
- `crate::actions::ActionDispatcher` ✓
- `crate::config::Settings` ✓
- `crate::gsi::start_gsi_server` ✓
- `crate::input::keyboard::{start_keyboard_listener, KeyboardSnapshot}` ✓
- `crate::state::{AppState, UpdateCheckState}` ✓
- `crate::update::{check_for_update, UpdateCheckResult}` ✓
- `std::sync::{Arc, Mutex, RwLock}` ✓
- `tracing::info` ✓
- `tracing_subscriber` ✓

- [ ] **Step 5: Verify main.rs compiles**

Run: `cargo check -p dota2-scripts --quiet`
Expected: Warnings about unused variables/imports are acceptable. No errors.

- [ ] **Step 6: Commit**

```
git add src/main.rs
git commit -m "refactor: remove egui bootstrap and feature gates from main.rs"
```

---

### Task 3: Remove egui Module from lib.rs and Delete src/ui/

**Files:**
- Modify: `src/lib.rs`
- Delete: `src/ui/mod.rs`
- Delete: `src/ui/app.rs`

- [ ] **Step 1: Remove the feature-gated module from lib.rs**

In `src/lib.rs`, remove lines 8-9:

```rust
// DELETE:
#[cfg(feature = "egui-ui")]
pub mod ui;
```

The file should now be:
```rust
pub mod actions;
pub mod config;
pub mod gsi;
pub mod input;
pub mod models;
pub mod observability;
pub mod state;
pub mod update;
```

- [ ] **Step 2: Delete the src/ui/ directory**

```
Remove-Item -Recurse -Force src/ui
```

- [ ] **Step 3: Verify everything compiles and tests pass**

Run: `cargo test --quiet`
Expected: All tests pass. No references to the `ui` module remain.

Run: `cargo check -p dota2-scripts-tauri --quiet`
Expected: Tauri crate compiles fine (it never imported the `ui` module).

- [ ] **Step 4: Commit**

```
git add src/lib.rs
git rm -r src/ui/
git commit -m "refactor: delete egui UI module (src/ui/)"
```

---

### Task 4: Update Tauri Dependency and Product Name

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/tauri.conf.json`

- [ ] **Step 1: Remove default-features = false from Tauri's library dependency**

In `src-tauri/Cargo.toml`, line 7, change:

```toml
# Before:
dota2_scripts = { path = "..", package = "dota2-scripts", default-features = false }

# After:
dota2_scripts = { path = "..", package = "dota2-scripts" }
```

This was only needed because the root crate had `default = ["egui-ui"]`. Now that the feature is gone, `default-features = false` is unnecessary.

- [ ] **Step 2: Change productName to produce dota2-scripts.exe**

In `src-tauri/tauri.conf.json`, line 2, change:

```json
// Before:
"productName": "Dota 2 Scripts",

// After:
"productName": "dota2-scripts",
```

This makes `npx tauri build` produce `dota2-scripts.exe` instead of `Dota 2 Scripts.exe`, keeping compatibility with the existing `self_update` configuration which hardcodes `bin_name("dota2-scripts")`.

The window title is controlled separately by `app.windows[0].title` which stays as `"Dota 2 Script Automation"`.

- [ ] **Step 3: Verify both crates compile**

Run: `cargo check -p dota2-scripts --quiet && cargo check -p dota2-scripts-tauri --quiet`
Expected: Both compile with no errors.

- [ ] **Step 4: Commit**

```
git add src-tauri/Cargo.toml src-tauri/tauri.conf.json
git commit -m "chore: clean up Tauri dependency and set binary name to dota2-scripts"
```

---

### Task 5: Migrate release.yml to Tauri Build

**Files:**
- Modify: `.github/workflows/release.yml`

**Context:** The current workflow uses `cargo build --release`. We need to switch to `npx tauri build` which compiles the Rust backend AND bundles the React frontend into a single `.exe`. We also add MSI as an additional release asset.

- [ ] **Step 1: Update the build job**

Replace the entire `build` job in `.github/workflows/release.yml` with:

```yaml
  build:
    name: Build Windows
    runs-on: windows-latest
    strategy:
      matrix:
        include:
          - target: x86_64-pc-windows-msvc
            artifact_name: dota2-scripts.exe

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install Node.js
        uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: npm
          cache-dependency-path: src-ui/package-lock.json

      - name: Sync Cargo.toml version with git tag
        shell: pwsh
        run: |
          $TAG = "${{ github.ref_name }}"
          # Strip 'v' prefix from tag to get semver (v1.0.0 -> 1.0.0)
          $VERSION = $TAG -replace '^v', ''
          
          Write-Host "Syncing Cargo.toml version to: $VERSION"
          
          # Update root Cargo.toml [package] version
          $lines = Get-Content "Cargo.toml"
          $inPackage = $false
          $versionUpdated = $false
          
          for ($i = 0; $i -lt $lines.Count; $i++) {
            if ($lines[$i] -match '^\[package\]') {
              $inPackage = $true
            }
            elseif ($lines[$i] -match '^\[' -and $inPackage) {
              break
            }
            elseif ($inPackage -and -not $versionUpdated -and $lines[$i] -match '^version\s*=') {
              $lines[$i] = "version = `"$VERSION`""
              $versionUpdated = $true
              Write-Host "Updated root Cargo.toml line $($i + 1): $($lines[$i])"
            }
          }
          
          if (-not $versionUpdated) {
            Write-Error "Failed to find version in root [package] section"
            exit 1
          }
          $lines | Set-Content "Cargo.toml"
          
          # Update src-tauri/Cargo.toml [package] version
          $tauriLines = Get-Content "src-tauri/Cargo.toml"
          $inPackage = $false
          $versionUpdated = $false
          
          for ($i = 0; $i -lt $tauriLines.Count; $i++) {
            if ($tauriLines[$i] -match '^\[package\]') {
              $inPackage = $true
            }
            elseif ($tauriLines[$i] -match '^\[' -and $inPackage) {
              break
            }
            elseif ($inPackage -and -not $versionUpdated -and $tauriLines[$i] -match '^version\s*=') {
              $tauriLines[$i] = "version = `"$VERSION`""
              $versionUpdated = $true
              Write-Host "Updated src-tauri/Cargo.toml line $($i + 1): $($tauriLines[$i])"
            }
          }
          $tauriLines | Set-Content "src-tauri/Cargo.toml"
          
          # Update src-tauri/tauri.conf.json version
          $conf = Get-Content "src-tauri/tauri.conf.json" -Raw | ConvertFrom-Json
          $conf.version = $VERSION
          $conf | ConvertTo-Json -Depth 10 | Set-Content "src-tauri/tauri.conf.json"
          Write-Host "Updated tauri.conf.json version to: $VERSION"

      - name: Generate Cargo.lock
        run: cargo generate-lockfile

      - name: Cache cargo registry
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry/index/
            ~/.cargo/registry/cache/
            ~/.cargo/git/db/
            target/
          key: ${{ runner.os }}-${{ matrix.target }}-cargo-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-${{ matrix.target }}-cargo-

      - name: Install frontend dependencies
        run: npm ci
        working-directory: src-ui

      - name: Build Tauri app
        run: npx tauri build --target ${{ matrix.target }}
        working-directory: src-ui

      - name: Package ZIP artifact
        shell: pwsh
        run: |
          $VERSION = "${{ github.ref_name }}"
          $TARGET = "${{ matrix.target }}"
          $PACKAGE_DIR = "artifacts/dota2-scripts-${VERSION}-${TARGET}"
          
          New-Item -ItemType Directory -Force -Path $PACKAGE_DIR
          New-Item -ItemType Directory -Force -Path "$PACKAGE_DIR/config"
          
          # Copy executable from Tauri build output
          Copy-Item "src-tauri/target/${{ matrix.target }}/release/${{ matrix.artifact_name }}" -Destination "$PACKAGE_DIR/dota2-scripts.exe"
          Copy-Item "config/config.toml" -Destination "$PACKAGE_DIR/config/config.example.toml"
          
          Compress-Archive -Path "$PACKAGE_DIR/*" -DestinationPath "artifacts/dota2-scripts-${VERSION}-${TARGET}.zip" -Force
          
          Write-Host "ZIP package contents:"
          Get-ChildItem -Recurse $PACKAGE_DIR

      - name: Collect MSI installer
        shell: pwsh
        run: |
          $VERSION = "${{ github.ref_name }}"
          $TARGET = "${{ matrix.target }}"
          $MSI_DIR = "src-tauri/target/${{ matrix.target }}/release/bundle/msi"
          
          if (Test-Path $MSI_DIR) {
            $msi = Get-ChildItem "$MSI_DIR/*.msi" | Select-Object -First 1
            if ($msi) {
              $dest = "artifacts/dota2-scripts-${VERSION}-${TARGET}.msi"
              Copy-Item $msi.FullName $dest
              Write-Host "MSI installer: $dest"
            } else {
              Write-Host "No MSI found in $MSI_DIR"
            }
          } else {
            Write-Host "MSI directory not found: $MSI_DIR"
          }

      - name: Upload ZIP artifact
        uses: actions/upload-artifact@v4
        with:
          name: dota2-scripts-${{ github.ref_name }}-${{ matrix.target }}-zip
          path: artifacts/dota2-scripts-${{ github.ref_name }}-${{ matrix.target }}.zip

      - name: Upload MSI artifact
        uses: actions/upload-artifact@v4
        if: hashFiles(format('artifacts/dota2-scripts-{0}-{1}.msi', github.ref_name, matrix.target)) != ''
        with:
          name: dota2-scripts-${{ github.ref_name }}-${{ matrix.target }}-msi
          path: artifacts/dota2-scripts-${{ github.ref_name }}-${{ matrix.target }}.msi
```

- [ ] **Step 2: Update the release job's artifact download and upload**

In the `release` job, update the artifact download step to get both ZIP and MSI, and update the release asset upload:

Replace the `Download all artifacts` step (line 209-212):
```yaml
      - name: Download all artifacts
        uses: actions/download-artifact@v4
        with:
          path: artifacts
```

Replace the `Prepare release body` step — update the Downloads section (lines 236-237) to include MSI:
```yaml
          ### 📥 Downloads
          - **Windows (ZIP)**: `dota2-scripts-${{ github.ref_name }}-x86_64-pc-windows-msvc.zip` (includes exe + example config)
          - **Windows (MSI)**: `dota2-scripts-${{ github.ref_name }}-x86_64-pc-windows-msvc.msi` (installer)
```

Also update the Features section (lines 244-248) to be current:
```yaml
          ### ⚡ Features
          - Hero-specific automation (Broodmother, Huskar, Largo, Legion Commander, Meepo, Outworld Destroyer, Shadow Fiend, Tiny)
          - GSI event-based action dispatch
          - Danger detection and survivability automation
          - Standalone hotkey triggers
          - React-based UI with live game state
```

Replace the `Create Release` step's files list (lines 259-260) to include both artifacts:
```yaml
      - name: Create Release
        uses: softprops/action-gh-release@v1
        with:
          files: |
            artifacts/dota2-scripts-${{ github.ref_name }}-x86_64-pc-windows-msvc-zip/dota2-scripts-${{ github.ref_name }}-x86_64-pc-windows-msvc.zip
            artifacts/dota2-scripts-${{ github.ref_name }}-x86_64-pc-windows-msvc-msi/dota2-scripts-${{ github.ref_name }}-x86_64-pc-windows-msvc.msi
          body_path: release_body.md
          draft: false
          prerelease: ${{ contains(github.ref, '-rc') }}
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

- [ ] **Step 3: Commit**

```
git add .github/workflows/release.yml
git commit -m "ci: migrate release pipeline to Tauri build with ZIP + MSI artifacts"
```

---

### Task 6: Add Markdown Dependencies for Update Banner

**Files:**
- Modify: `src-ui/package.json`

- [ ] **Step 1: Install react-markdown, remark-gfm, and @tailwindcss/typography**

Run from the `src-ui/` directory:

```
cd src-ui
npm install react-markdown remark-gfm @tailwindcss/typography
```

This adds:
- `react-markdown` — React component that renders markdown as React elements
- `remark-gfm` — Plugin for GitHub-Flavored Markdown (tables, strikethrough, task lists, autolinks)
- `@tailwindcss/typography` — Provides `prose` classes for styling rendered HTML content

- [ ] **Step 2: Verify tests still pass**

Run: `cd src-ui && npx vitest run`
Expected: All 31 tests pass (adding dependencies shouldn't break anything).

- [ ] **Step 3: Commit**

```
git add src-ui/package.json src-ui/package-lock.json
git commit -m "chore: add react-markdown, remark-gfm, and tailwind typography"
```

---

### Task 7: Implement Release Notes Modal with Markdown Rendering

**Files:**
- Modify: `src-ui/src/components/layout/UpdateBanner.tsx`

- [ ] **Step 1: Rewrite UpdateBanner.tsx with expandable release notes modal**

Replace the entire contents of `src-ui/src/components/layout/UpdateBanner.tsx` with:

```tsx
import { useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { useUpdateStore } from "../../stores/updateStore";
import { Button } from "../common/Button";

function ReleaseNotesModal({
  version,
  notes,
  onClose,
  onApply,
}: {
  version: string;
  notes: string;
  onClose: () => void;
  onApply: () => void;
}) {
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
      onClick={onClose}
    >
      <div
        className="mx-4 flex max-h-[80vh] w-full max-w-2xl flex-col rounded-lg border border-border bg-elevated shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between border-b border-border px-6 py-4">
          <h2 className="text-lg font-semibold text-content">
            Release Notes — v{version}
          </h2>
          <button
            type="button"
            onClick={onClose}
            className="text-subtle hover:text-content"
          >
            ✕
          </button>
        </div>
        <div className="flex-1 overflow-y-auto px-6 py-4">
          <div className="prose prose-invert prose-sm max-w-none prose-headings:text-gold prose-a:text-gold prose-strong:text-content prose-code:rounded prose-code:bg-base prose-code:px-1.5 prose-code:py-0.5 prose-code:text-content prose-pre:bg-base prose-li:text-subtle">
            <ReactMarkdown remarkPlugins={[remarkGfm]}>{notes}</ReactMarkdown>
          </div>
        </div>
        <div className="flex items-center justify-end gap-3 border-t border-border px-6 py-4">
          <button
            type="button"
            onClick={onClose}
            className="rounded px-4 py-2 text-sm text-subtle hover:text-content"
          >
            Close
          </button>
          <Button onClick={onApply} className="px-4 py-2 text-sm">
            Apply Update
          </Button>
        </div>
      </div>
    </div>
  );
}

export function UpdateBanner() {
  const updateState = useUpdateStore((s) => s.updateState);
  const applyUpdate = useUpdateStore((s) => s.applyUpdate);
  const dismissUpdate = useUpdateStore((s) => s.dismissUpdate);
  const [showNotes, setShowNotes] = useState(false);

  if (updateState.kind !== "available") return null;

  return (
    <>
      <div className="flex items-center justify-between gap-4 border-b border-border bg-elevated px-4 py-2">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-gold">
            🎉 Update v{updateState.version} available
          </span>
          {updateState.releaseNotes && (
            <button
              type="button"
              onClick={() => setShowNotes(true)}
              className="text-xs text-subtle underline decoration-dotted hover:text-content"
            >
              View Release Notes
            </button>
          )}
        </div>
        <div className="flex items-center gap-2">
          <Button onClick={applyUpdate} className="h-7 px-3 text-xs">
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
      {showNotes && updateState.releaseNotes && (
        <ReleaseNotesModal
          version={updateState.version}
          notes={updateState.releaseNotes}
          onClose={() => setShowNotes(false)}
          onApply={() => {
            setShowNotes(false);
            applyUpdate();
          }}
        />
      )}
    </>
  );
}
```

- [ ] **Step 2: Verify the component renders correctly**

Run: `cd src-ui && npx vitest run`
Expected: All tests pass. The UpdateBanner component doesn't have dedicated tests yet — existing tests should not be affected.

- [ ] **Step 3: Verify the build succeeds**

Run: `cd src-ui && npm run build`
Expected: Production build succeeds with no TypeScript errors.

- [ ] **Step 4: Commit**

```
git add src-ui/src/components/layout/UpdateBanner.tsx
git commit -m "feat: add release notes modal with GitHub-Flavored Markdown rendering"
```

---

### Task 8: Full Verification

**Files:** None (verification only)

- [ ] **Step 1: Run all Rust tests**

Run: `cargo test --quiet`
Expected: All tests pass. No references to egui remain. No compilation errors.

- [ ] **Step 2: Check both Rust crates compile**

Run: `cargo check -p dota2-scripts --quiet && cargo check -p dota2-scripts-tauri --quiet`
Expected: Both crates compile. The `dota2-scripts` crate no longer depends on egui. The Tauri crate compiles without `default-features = false`.

- [ ] **Step 3: Run React tests**

Run: `cd src-ui && npx vitest run`
Expected: All 31+ tests pass.

- [ ] **Step 4: Build React frontend**

Run: `cd src-ui && npm run build`
Expected: Production build succeeds.

- [ ] **Step 5: Verify Tauri dev mode works**

Run from repo root: `npx tauri dev`
Expected: Vite starts, Rust compiles, Tauri window opens with the React UI. All backend systems start (GSI server, keyboard listener, etc.). Close the window after verifying.

- [ ] **Step 6: Verify src/ui/ directory is gone**

Run: `Test-Path src/ui`
Expected: `False`

- [ ] **Step 7: Verify no egui references remain in Rust code**

Run: `rg "egui|eframe" --type rust`
Expected: No matches (or only matches in comments/docs that are acceptable).

- [ ] **Step 8: Commit verification results**

No commit needed — this is a verification-only task. If all checks pass, the cleanup is complete.
