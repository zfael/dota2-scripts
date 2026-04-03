# File Index

**Purpose**: Repo-wide file and directory map for agents and contributors. Use this when you need to answer two questions quickly:

1. where does this behavior live?
2. which doc also needs to change?

> **Maintenance contract**: add a row here whenever you add a new source file or a new durable doc entry point.

---

## Repo map

| Path | What lives there | Read next |
|---|---|---|
| `src/actions/` | Shared action orchestration, survivability, hero dispatch, Soul Ring, auto-items | `docs/architecture/state-and-dispatch.md` |
| `src/actions/heroes/` | Per-hero automation scripts and the `HeroScript` trait | `docs/workflows/adding-a-hero.md`, `docs/heroes/` |
| `src/gsi/` | HTTP listener, queueing, GSI processing | `docs/reference/gsi-schema-and-usage.md`, `docs/architecture/runtime-flow.md` |
| `src/input/` | Global interception and synthetic replay | `docs/features/keyboard-interception.md` |
| `src/config/` | Runtime config types, defaults, helpers | `docs/reference/configuration.md` |
| `src/state/` | Shared app/UI/runtime state | `docs/architecture/state-and-dispatch.md` |
| `src/ui/` | egui app, status, settings, manual hero selection | `docs/architecture/overview.md` |
| `src-ui/` | Tauri React frontend, settings UI, game dashboard, and Vitest coverage | `docs/superpowers/specs/2026-03-31-react-ui-design.md` |
| `src/models/` | GSI model types plus shared enums | `docs/reference/gsi-schema-and-usage.md` |
| `assets/` | Application icon and packaged visual assets | `src/main.rs` |
| `examples/` | Small standalone binaries for local input/debug experiments | `docs/workflows/testing-and-debugging.md` |
| `tests/` | Fixture-backed tests | `docs/workflows/testing-and-debugging.md` |
| `docs/` | Architecture, features, heroes, reference, workflows | `AGENTS.md` |
| `docs/superpowers/` | Design specs and implementation plans created during AI-driven work | `AGENTS.md` |

---

## Entry points and top-level files

| File | Purpose | Linked Doc |
|---|---|---|
| `src/main.rs` | Boot order, runtime wiring, keyboard listener, GSI server, update checks, egui launch | `docs/architecture/overview.md`, `docs/architecture/runtime-flow.md`, `docs/features/updates.md` |
| `config/config.toml` | Checked-in runtime config | `docs/reference/configuration.md` |
| `AGENTS.md` | Agent / contributor navigation hub | — |
| `README.md` | User-facing overview and setup | — |
| `Cargo.toml` | Crate manifest | — |
| `Cargo.lock` | Locked dependency graph for reproducible builds | — |
| `build.rs` | Windows resource embedding hook used during build | — |
| `app.rc` | Windows resource script compiled by `build.rs` | `build.rs` |
| `app.manifest` | Windows application manifest bundled with the binary | `build.rs` |
| `src/lib.rs` | Library exports | — |

---

## `src/actions/`

| File | Purpose | Linked Doc |
|---|---|---|
| `src/actions/mod.rs` | Module re-exports | — |
| `src/actions/dispatcher.rs` | Pre-dispatch hooks plus hero/common routing for every GSI event | `docs/architecture/state-and-dispatch.md`, `docs/reference/gsi-schema-and-usage.md` |
| `src/actions/armlet.rs` | Shared armlet planning, config resolution, cooldown/critical-state handling, and dual-trigger execution | `docs/features/survivability.md`, `docs/heroes/huskar.md`, `docs/reference/configuration.md` |
| `src/actions/common.rs` | Shared survivability pipeline: healing, defensive items, neutral items, and armlet job enqueueing | `docs/features/survivability.md`, `docs/features/danger-detection.md`, `docs/reference/gsi-schema-and-usage.md` |
| `src/actions/danger_detector.rs` | HP-loss heuristic and global danger state | `docs/features/danger-detection.md` |
| `src/actions/item_automation.rs` | Shared item automation metadata, cast modes, and short lockout state | `docs/features/survivability.md`, `docs/reference/configuration.md` |
| `src/actions/auto_items.rs` | Cached GSI item state and Broodmother item/ability combo execution | `docs/features/survivability.md`, `docs/reference/gsi-schema-and-usage.md` |
| `src/actions/dispel.rs` | Silence dispel logic (Manta / Lotus) | `docs/features/survivability.md`, `docs/reference/gsi-schema-and-usage.md` |
| `src/actions/soul_ring.rs` | Soul Ring shared state, gating rules, and replay helpers | `docs/features/soul-ring.md`, `docs/features/keyboard-interception.md`, `docs/reference/gsi-schema-and-usage.md` |

## `src/actions/heroes/`

| File | Purpose | Linked Doc |
|---|---|---|
| `src/actions/heroes/mod.rs` | Hero module registration and re-exports | `docs/workflows/adding-a-hero.md` |
| `src/actions/heroes/traits.rs` | `HeroScript` trait contract | `docs/architecture/state-and-dispatch.md`, `docs/workflows/adding-a-hero.md` |
| `src/actions/heroes/broodmother.rs` | Broodmother spider micro and auto-items/abilities | `docs/heroes/broodmother.md` |
| `src/actions/heroes/huskar.rs` | Huskar Berserker Blood cleanse plus shared armlet-survivability wiring | `docs/heroes/huskar.md` |
| `src/actions/heroes/largo.rs` | Largo ultimate state, beat timing, manual song hooks | `docs/heroes/largo.md` |
| `src/actions/heroes/legion_commander.rs` | Legion Commander combo automation | `docs/heroes/legion_commander.md` |
| `src/actions/heroes/meepo_macro.rs` | Meepo farm-assist macro state, gating, and pulse decisions | `docs/heroes/meepo.md` |
| `src/actions/heroes/meepo.rs` | Meepo standalone combo, GSI-driven Dig / MegaMeepo, and survivability wiring | `docs/heroes/meepo.md` |
| `src/actions/heroes/meepo_state.rs` | Read-only Meepo observed-state derivation and cache | `docs/heroes/meepo.md`, `docs/reference/gsi-schema-and-usage.md` |
| `src/actions/heroes/outworld_destroyer.rs` | Outworld Destroyer barrier, combo worker, ultimate interception support, and self-Astral helper | `docs/heroes/outworld_destroyer.md`, `docs/features/keyboard-interception.md` |
| `src/actions/heroes/shadow_fiend.rs` | Shadow Fiend raze / ultimate / standalone combo logic | `docs/heroes/shadow_fiend.md`, `docs/features/keyboard-interception.md` |
| `src/actions/heroes/tiny.rs` | Tiny standalone combo | `docs/heroes/tiny.md` |

## `src/gsi/`

| File | Purpose | Linked Doc |
|---|---|---|
| `src/gsi/server.rs` | Axum HTTP server on `127.0.0.1:<port>` plus bounded queue setup | `docs/architecture/runtime-flow.md`, `docs/reference/gsi-schema-and-usage.md` |
| `src/gsi/handler.rs` | Deserialize `GsiWebhookEvent`, log JSONL, update `AppState`, refresh shared caches, and dispatch | `docs/architecture/runtime-flow.md`, `docs/reference/gsi-schema-and-usage.md` |
| `src/gsi/mod.rs` | Module re-exports | — |

## `src/input/`

| File | Purpose | Linked Doc |
|---|---|---|
| `src/input/keyboard.rs` | Global `rdev::grab` hook and the interception decision tree | `docs/features/keyboard-interception.md`, `docs/workflows/troubleshooting.md` |
| `src/input/simulation.rs` | Synthetic key and mouse emission helpers | `docs/features/keyboard-interception.md` |
| `src/input/mod.rs` | Module re-exports | — |

## `src/config/`

| File | Purpose | Linked Doc |
|---|---|---|
| `src/config/settings.rs` | Config structs, serde defaults, load/save helpers, keybinding validation | `docs/reference/configuration.md` |
| `src/config/storage.rs` | LocalAppData config-path resolution, legacy import, and TOML merge/persist helpers | `docs/reference/configuration.md`, `docs/features/updates.md` |
| `src/config/constants.rs` | Compile-time constants and default maps | `docs/reference/configuration.md` |
| `src/config/mod.rs` | Module re-exports | — |

## `src/state/`

| File | Purpose | Linked Doc |
|---|---|---|
| `src/state/app_state.rs` | Shared runtime/UI state, `HeroType`, update state, queue metrics | `docs/architecture/state-and-dispatch.md`, `docs/workflows/adding-a-hero.md` |
| `src/state/mod.rs` | Module re-exports | — |

## `src/ui/`

| File | Purpose | Linked Doc |
|---|---|---|
| `src/ui/app.rs` | Main egui app: tabs, hero selection, status, settings, update banner | `docs/architecture/overview.md`, `docs/workflows/adding-a-hero.md` |
| `src/ui/mod.rs` | Module re-exports | — |

## `src-ui/src/`

| File | Purpose | Linked Doc |
|---|---|---|
| `src-ui/src/App.tsx` | React shell that wires stores, routing, and global hooks | `docs/superpowers/specs/2026-03-31-react-ui-design.md` |
| `src-ui/src/hooks/useRuneAlert.ts` | Frontend-owned rune alert gating and Web Audio playback | `docs/superpowers/specs/2026-03-31-react-ui-design.md`, `docs/reference/configuration.md` |

## `src/models/`

| File | Purpose | Linked Doc |
|---|---|---|
| `src/models/gsi_event.rs` | `GsiWebhookEvent` plus nested hero/item/ability/map structs | `docs/reference/gsi-schema-and-usage.md` |
| `src/models/heroes.rs` | Hero enum and internal-name mapping | `docs/workflows/adding-a-hero.md` |
| `src/models/items.rs` | Item model helpers | `docs/features/survivability.md` |
| `src/models/mod.rs` | Module re-exports | — |

## `src/update/`

| File | Purpose | Linked Doc |
|---|---|---|
| `src/update/mod.rs` | GitHub release checks plus MSI/config-template apply orchestration | `docs/features/updates.md` |
| `src/update/msi.rs` | MSI asset selection, ZIP-layout guard, temp download, and PowerShell handoff helpers | `docs/features/updates.md` |

## `src/observability/`

| File | Purpose | Linked Doc |
|---|---|---|
| `src/observability/minimap_capture.rs` | Minimap capture worker lifecycle and status publication | `docs/reference/configuration.md` |
| `src/observability/minimap_capture_state.rs` | Minimap capture status snapshot types | `docs/architecture/state-and-dispatch.md` |
| `src/observability/minimap_capture_backend.rs` | Win32 window binding and BitBlt screen capture | `docs/reference/configuration.md` |
| `src/observability/minimap_artifacts.rs` | Artifact metadata and persistence helpers | `docs/reference/configuration.md` |
| `src/observability/minimap_zones.rs` | Map zone definitions and point-to-zone classification | `docs/superpowers/specs/2026-03-31-minimap-hero-detection-design.md` |
| `src/observability/minimap_analysis.rs` | HSV color segmentation, BFS clustering, hero detection pipeline | `docs/superpowers/specs/2026-03-31-minimap-hero-detection-design.md` |
| `src/observability/minimap_baseline.rs` | Static baseline mask accumulator for filtering map fixtures | `docs/superpowers/specs/2026-03-31-minimap-hero-detection-design.md` |
| `src/observability/lane_heat.rs` | Zone activity classifier, rolling lane heat tracker, and event detection | `docs/superpowers/specs/2026-03-31-lane-heat-analysis-design.md` |

## `tests/`

| File | Purpose | Linked Doc |
|---|---|---|
| `tests/gsi_handler_tests.rs` | Fixture-backed GSI deserialization smoke tests | `docs/workflows/testing-and-debugging.md`, `docs/reference/gsi-schema-and-usage.md` |
| `tests/fixtures/` | Sample JSON payloads for Huskar, Tiny, Meepo, and Outworld Destroyer | `docs/workflows/testing-and-debugging.md`, `docs/reference/gsi-schema-and-usage.md` |
| `tests/minimap_capture_tests.rs` | Minimap capture integration tests | `docs/reference/configuration.md` |
| `tests/minimap_analysis_tests.rs` | Tests for zone mapping, color analysis, clustering, baseline, detection | `docs/superpowers/specs/2026-03-31-minimap-hero-detection-design.md` |

## `assets/`

| Path | Purpose | Linked Doc |
|---|---|---|
| `assets/icon.png` | Window/taskbar icon loaded at startup via `include_bytes!` | `src/main.rs` |

## `examples/`

| File | Purpose | Linked Doc |
|---|---|---|
| `examples/mouse_test.rs` | Local helper binary for inspecting `rdev` mouse button events | `docs/workflows/testing-and-debugging.md` |
| `examples/minimap_analyze.rs` | Standalone CLI for running hero detection on PNG captures | `docs/superpowers/specs/2026-03-31-minimap-hero-detection-design.md` |

## `docs/`

| Path | Purpose |
|---|---|
| `docs/architecture/overview.md` | System map and entry points |
| `docs/architecture/runtime-flow.md` | Startup order, queues, threads, event flow |
| `docs/architecture/state-and-dispatch.md` | `AppState`, dispatcher, hero/common composition |
| `docs/features/danger-detection.md` | Danger heuristics, thresholds, defensive response |
| `docs/features/keyboard-interception.md` | Interception ordering, blocking, replay |
| `docs/features/soul-ring.md` | Soul Ring state and trigger rules |
| `docs/features/survivability.md` | Shared healing, dispel, neutral items |
| `docs/features/updates.md` | Update check/apply/restart flow |
| `docs/heroes/meepo.md` | Meepo automation doc |
| `docs/heroes/hero-template.md` | Template for new hero docs |
| `docs/heroes/*.md` | Hero-specific automation docs |
| `docs/heroes/outworld_destroyer.md` | Outworld Destroyer automation doc |
| `docs/reference/configuration.md` | Section-by-section config reference and fallback defaults |
| `docs/reference/gsi-schema-and-usage.md` | Consumed GSI fields, event flow, debug pointers |
| `docs/reference/file-index.md` | This repo map |
| `docs/workflows/adding-a-hero.md` | End-to-end hero addition workflow |
| `docs/workflows/testing-and-debugging.md` | Test, build, fixture, and logging workflow |
| `docs/workflows/troubleshooting.md` | Runtime failure-mode guide |
| `docs/superpowers/specs/` | Archived design/spec docs for larger documentation or feature efforts |
| `docs/superpowers/plans/` | Archived implementation plans and execution breakdowns |
