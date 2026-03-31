# egui Removal, Release Pipeline Migration & Update UX — Design Spec

> **Date:** 2026-03-31
> **Status:** Draft
> **Branch:** `feature/react-ui-migration`

## Problem

The Tauri/React UI is functional and replaces the egui desktop UI. Three things need to happen before this branch can ship:

1. **Remove egui** — dead code behind feature gates, unused dependencies
2. **Migrate the release pipeline** — current GitHub Actions build `cargo build --release`; needs to switch to `npx tauri build` and ship both ZIP + MSI
3. **Fix update popup** — release notes render as raw text; should render as GitHub-Flavored Markdown

## Decisions Made

| Decision | Choice | Rationale |
|----------|--------|-----------|
| egui fate | Remove entirely | Tauri/React is the only UI going forward |
| Binary name | Keep `dota2-scripts.exe` | Existing users + `self_update` hardcodes it |
| Release format | ZIP + MSI | ZIP for power users, MSI for casual users |
| Update mechanism | Keep `self_update` crate | Works fine; migrate to `tauri-plugin-updater` later |
| Markdown rendering | `react-markdown` + `remark-gfm` | Full GFM support for release notes |

---

## Area 1: egui Removal

### Files to Delete

| Path | Reason |
|------|--------|
| `src/ui/app.rs` | egui `Dota2ScriptApp` implementation |
| `src/ui/mod.rs` | Module re-exports for egui UI |

### Files to Modify

**`Cargo.toml` (root)**
- Remove the `[features]` section entirely (`default = ["egui-ui"]`, `egui-ui = ["egui", "eframe"]`)
- Remove `egui` and `eframe` from `[dependencies]` (lines 40-41: `egui = { version = "0.29", optional = true }`, `eframe = { version = "0.29", optional = true }`)

**`src/lib.rs`**
- Remove lines 8-9: `#[cfg(feature = "egui-ui")] pub mod ui;`

**`src/main.rs`**
- Remove line 10-11: `#[cfg(feature = "egui-ui")] mod ui;`
- Remove line 20-21: `#[cfg(feature = "egui-ui")] use crate::ui::Dota2ScriptApp;`
- Remove lines 252-276: egui bootstrap block (`#[cfg(feature = "egui-ui")] { ... eframe::run_native(...) }`)
- Remove lines 288-319: `load_icon()` function (egui-only)
- **Keep** lines 278-285: the non-UI fallback loop becomes the main path:
  ```rust
  // No longer feature-gated — this IS the main path for the library binary
  info!("Running without GUI (Tauri UI is a separate binary)");
  loop { std::thread::park(); }
  ```
- Remove `#[cfg(not(feature = "egui-ui"))]` gate from the loop (it's now unconditional)

**`src-tauri/Cargo.toml`**
- Remove `default-features = false` from the library dependency (no features to disable anymore):
  ```toml
  # Before
  dota2_scripts = { path = "..", package = "dota2-scripts", default-features = false }
  # After
  dota2_scripts = { path = "..", package = "dota2-scripts" }
  ```

### What stays

- `src/main.rs` still exists as the library binary entry point (headless mode for testing/CI)
- All backend code: GSI server, keyboard listener, dispatcher, hero scripts, update checker
- `assets/icon.png` — may still be used by Tauri or future needs

---

## Area 2: Release Pipeline Migration

### Binary Name Alignment

**`src-tauri/tauri.conf.json`**
- Change `productName` from `"Dota 2 Scripts"` to `"dota2-scripts"`
- This makes `npx tauri build` produce `dota2-scripts.exe` instead of `Dota 2 Scripts.exe`
- Window title stays `"Dota 2 Script Automation"` (controlled by `app.windows[0].title`)

### Workflow: `release.yml`

Replace the build job to use Tauri CLI instead of `cargo build`:

**Current flow:**
1. Checkout → Rust toolchain → `cargo build --release` → package ZIP → upload

**New flow:**
1. Checkout → Rust toolchain → Node.js setup → `npm ci` (root) → `npx tauri build`
2. Collect artifacts:
   - ZIP: `dota2-scripts-{VERSION}-x86_64-pc-windows-msvc.zip` containing `dota2-scripts.exe` + `config/config.example.toml`
   - MSI: from `src-tauri/target/release/bundle/msi/`
3. Upload both as release assets

**Key details:**
- The Cargo.toml version sync script (lines 30-69) stays — it updates both root and src-tauri Cargo.toml versions from the git tag
- Must also sync `src-tauri/tauri.conf.json` version field to match the git tag
- The ZIP is manually assembled (same as today) from `src-tauri/target/release/dota2-scripts.exe`
- The MSI is produced by Tauri's bundler automatically

**Version sync addition:**
```yaml
# Sync tauri.conf.json version
- name: Sync tauri.conf.json version
  shell: pwsh
  run: |
    $version = "${{ steps.version.outputs.version }}"
    $conf = Get-Content src-tauri/tauri.conf.json | ConvertFrom-Json
    $conf.version = $version
    $conf | ConvertTo-Json -Depth 10 | Set-Content src-tauri/tauri.conf.json
```

**Build step replacement:**
```yaml
# Old
- name: Build
  run: cargo build --release --target ${{ matrix.target }}

# New
- name: Install frontend dependencies
  run: npm ci

- name: Build Tauri app
  run: npx tauri build --target ${{ matrix.target }}
```

**Artifact packaging:**
```yaml
# ZIP (same format as before, compatible with self_update)
- name: Package ZIP
  shell: pwsh
  run: |
    $exe = "src-tauri/target/${{ matrix.target }}/release/dota2-scripts.exe"
    $zip = "dota2-scripts-${{ steps.version.outputs.tag }}-${{ matrix.target }}.zip"
    New-Item -ItemType Directory -Path staging
    Copy-Item $exe staging/
    Copy-Item config/config.example.toml staging/
    Compress-Archive -Path staging/* -DestinationPath $zip

# MSI (new, from Tauri bundler output)
- name: Collect MSI
  shell: pwsh
  run: |
    $msi = Get-ChildItem "src-tauri/target/${{ matrix.target }}/release/bundle/msi/*.msi" | Select-Object -First 1
    $dest = "dota2-scripts-${{ steps.version.outputs.tag }}-${{ matrix.target }}.msi"
    Copy-Item $msi.FullName $dest
```

### Workflows: `auto-tag.yml` and `promote-release.yml`

No changes needed — they only manage git tags and trigger `release.yml`.

### self_update Compatibility

The `self_update` crate in `src/update/mod.rs` works by:
1. Fetching GitHub Releases
2. Finding a ZIP asset matching the target triple
3. Extracting it and replacing the binary named `dota2-scripts`

This continues to work because:
- ZIP asset name format is unchanged: `dota2-scripts-{tag}-x86_64-pc-windows-msvc.zip`
- Binary inside the ZIP is still `dota2-scripts.exe`
- The MSI is an additional asset that `self_update` ignores

---

## Area 3: Update Popup Markdown Rendering

### Current State

`UpdateBanner.tsx` shows release notes as inline plain text:
```tsx
<span className="text-xs text-subtle">
  — {updateState.releaseNotes}
</span>
```

This doesn't render markdown headers, bullet lists, bold text, or links.

### Design

**Add dependencies:**
- `react-markdown` — React component for rendering markdown
- `remark-gfm` — plugin for GitHub-Flavored Markdown (tables, strikethrough, task lists)

**UX change:**
- The inline banner stays compact (shows version + "View Release Notes" link)
- Clicking "View Release Notes" opens a modal/expanded panel with fully rendered markdown
- The modal uses `react-markdown` with `remark-gfm` for proper GFM rendering
- Styled to match the app's dark theme (Dota 2 aesthetic)

**Component structure:**
```
UpdateBanner (compact bar)
  └─ "View Release Notes" button
      └─ ReleaseNotesModal (overlay)
          └─ <ReactMarkdown remarkPlugins={[remarkGfm]}>{notes}</ReactMarkdown>
```

**Markdown styling:**
- Use Tailwind `prose` classes (from `@tailwindcss/typography`) or custom styles
- Headers: gold/accent color
- Lists: proper bullets/numbers
- Links: clickable, open in external browser
- Code blocks: dark background with monospace font

**Decision:** Use `@tailwindcss/typography` plugin for the `prose` class to style markdown output. This gives us good defaults without custom CSS for every element.

---

## Out of Scope

- Migrating from `self_update` to `tauri-plugin-updater` (future work)
- Code signing for MSI installer (future work)
- Auto-update without user confirmation (future work)
- Removing `src/main.rs` binary entirely (kept for headless/CI use)

---

## Testing

- `cargo test` — all existing Rust tests pass (no UI tests exist)
- `cargo build -p dota2-scripts` — headless binary builds without egui deps
- `cargo build -p dota2-scripts-tauri` — Tauri binary builds
- `npx tauri build` — produces both `.exe` and `.msi`
- `cd src-ui && npx vitest run` — React tests pass
- Manual: run the app, check update banner renders markdown correctly
