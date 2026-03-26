# AGENTS.md — Dota 2 Scripts

Navigation layer for AI agents and contributors working in this repo.
Start here. Follow links to the source or doc file relevant to your task.

---

## Project Snapshot

| Item | Value |
|---|---|
| Language | Rust (edition 2021) |
| UI | egui / eframe |
| GSI server | axum on `127.0.0.1:<configured port>` (default `3000`) |
| Key simulation | rdev + enigo |
| Logging | tracing |
| Config | `config/config.toml` (TOML, serde) |
| Entry point | `src/main.rs` |
| Tests | `tests/gsi_handler_tests.rs`, `src/actions/soul_ring.rs` unit test, fixtures in `tests/fixtures/` |

Supported heroes: **Broodmother, Huskar, Largo, Legion Commander, Shadow Fiend, Tiny**

---

## If You Want To…

| Goal | Go here |
|---|---|
| Understand the overall system | `docs/architecture/overview.md` |
| Trace boot order, threads, and a GSI event end-to-end | `docs/architecture/runtime-flow.md` |
| Understand `AppState`, hero routing, and common action composition | `docs/architecture/state-and-dispatch.md` |
| Add a new hero script | `docs/workflows/adding-a-hero.md` |
| Run or write tests | `docs/workflows/testing-and-debugging.md` |
| Debug a broken feature | `docs/workflows/troubleshooting.md` |
| Find a config key, fallback default, or section owner | `docs/reference/configuration.md` |
| Trace which GSI fields the app actually consumes | `docs/reference/gsi-schema-and-usage.md` |
| Tune danger heuristics, healing thresholds, or defensive items | `docs/features/danger-detection.md` |
| Trace blocked keys, Soul Ring replay, or SF interception | `docs/features/keyboard-interception.md` |
| Understand Soul Ring automation | `docs/features/soul-ring.md` |
| Understand shared survivability / dispel / neutral-item behavior | `docs/features/survivability.md` |
| Understand startup checks, update UI, and restart flow | `docs/features/updates.md` |
| Find a specific source file | `docs/reference/file-index.md` |
| Read a hero's automation docs | Hero Docs table below |

### Hero Docs

| Hero | Internal Name | Doc | Source |
|---|---|---|---|
| Broodmother | `npc_dota_hero_broodmother` | `docs/heroes/broodmother.md` | `src/actions/heroes/broodmother.rs` |
| Huskar | `npc_dota_hero_huskar` | `docs/heroes/huskar.md` | `src/actions/heroes/huskar.rs` |
| Largo | `npc_dota_hero_largo` | `docs/heroes/largo.md` | `src/actions/heroes/largo.rs` |
| Legion Commander | `npc_dota_hero_legion_commander` | `docs/heroes/legion_commander.md` | `src/actions/heroes/legion_commander.rs` |
| Shadow Fiend | `npc_dota_hero_nevermore` | `docs/heroes/shadow_fiend.md` | `src/actions/heroes/shadow_fiend.rs` |
| Tiny | `npc_dota_hero_tiny` | `docs/heroes/tiny.md` | `src/actions/heroes/tiny.rs` |

---

## Code Map

### `src/actions/`

| File | Purpose |
|---|---|
| `actions/dispatcher.rs` | Runs pre-dispatch hooks, then routes GSI events to hero scripts or fallback common actions |
| `actions/common.rs` | Shared survivability pipeline: armlet, healing, defensive items, neutral items |
| `actions/danger_detector.rs` | Global HP tracker; exposes `in_danger` to common and hero code |
| `actions/auto_items.rs` | Cached GSI item state + Space/right-click item/ability orchestration |
| `actions/dispel.rs` | Silence dispel / cleanse logic (Manta, Lotus) |
| `actions/soul_ring.rs` | Soul Ring shared state, intercept rules, and combo helper |
| `actions/heroes/traits.rs` | `HeroScript` trait — implement this to add a hero |
| `actions/heroes/broodmother.rs` | Broodmother automation |
| `actions/heroes/huskar.rs` | Huskar armlet + Berserker Blood automation |
| `actions/heroes/largo.rs` | Largo Amphibian Rhapsody beat-timing automation |
| `actions/heroes/legion_commander.rs` | Legion Commander combo automation |
| `actions/heroes/shadow_fiend.rs` | SF raze direction-facing + BKB-on-ultimate |
| `actions/heroes/tiny.rs` | Tiny standalone combo |

### `src/gsi/`

| File | Purpose |
|---|---|
| `gsi/server.rs` | axum HTTP server; listens on `127.0.0.1:<configured port>` and owns the bounded event queue |
| `gsi/handler.rs` | Deserialises `GsiWebhookEvent`, updates `AppState`, and calls dispatcher |
| `gsi/mod.rs` | Module re-exports |

### `src/input/`

| File | Purpose |
|---|---|
| `input/keyboard.rs` | rdev hook; blocks/replays keys for Soul Ring, SF, Largo, Broodmother |
| `input/simulation.rs` | Emits synthetic key presses / mouse input and guards against re-interception |
| `input/mod.rs` | Module re-exports |

### `src/config/`

| File | Purpose |
|---|---|
| `config/settings.rs` | All config structs with `#[serde(default)]`; one struct per feature/hero area |
| `config/constants.rs` | Compile-time constants |
| `config/mod.rs` | Module re-exports |
| `config/config.toml` | User-editable runtime config (not in `src/`) |

### `src/state/`

| File | Purpose |
|---|---|
| `state/app_state.rs` | `AppState` struct; wrapped in `Arc<Mutex<AppState>>` |
| `state/mod.rs` | Module re-exports |

### `src/ui/`

| File | Purpose |
|---|---|
| `ui/app.rs` | egui window, hero selector, danger settings, update banner/settings |
| `ui/mod.rs` | Module re-exports |

### `src/models/`

| File | Purpose |
|---|---|
| `models/gsi_event.rs` | `GsiWebhookEvent` and nested GSI payload structs |
| `models/heroes.rs` | Hero-related model types |
| `models/items.rs` | Item-related model types |
| `models/mod.rs` | Module re-exports |

### `src/update/`

| File | Purpose |
|---|---|
| `update/mod.rs` | GitHub Releases update-check, apply, and restart flow |

### `tests/`

| File | Purpose |
|---|---|
| `tests/gsi_handler_tests.rs` | Fixture-backed GSI deserialization smoke tests |
| `tests/fixtures/` | Sample GSI JSON payloads used by tests |

### `docs/`

| Path | Purpose |
|---|---|
| `docs/architecture/overview.md` | System overview, subsystem map, and entry points |
| `docs/architecture/runtime-flow.md` | Boot order, GSI path, keyboard path, and background tasks |
| `docs/architecture/state-and-dispatch.md` | `AppState`, shared locks, dispatcher routing, hero/common composition |
| `docs/features/danger-detection.md` | Danger heuristics, thresholds, defensive items, and related config |
| `docs/features/keyboard-interception.md` | rdev hook, Soul Ring replay, SF/Largo/Broodmother interception |
| `docs/features/survivability.md` | Shared healing, dispel, neutral-item, and item-state behavior |
| `docs/features/updates.md` | Startup checks, update UI, download/apply, restart |
| `docs/heroes/broodmother.md` | Broodmother hero doc |
| `docs/heroes/huskar.md` | Huskar hero doc |
| `docs/heroes/largo.md` | Largo hero doc |
| `docs/heroes/legion_commander.md` | Legion Commander hero doc |
| `docs/heroes/shadow_fiend.md` | Shadow Fiend hero doc |
| `docs/heroes/tiny.md` | Tiny hero doc |
| `docs/heroes/hero-template.md` | Template for new hero docs |
| `docs/features/soul-ring.md` | Soul Ring feature doc |
| `docs/reference/file-index.md` | Full file → purpose → doc cross-reference |
| `docs/reference/configuration.md` | Config sections, checked-in values, Rust fallback defaults |
| `docs/reference/gsi-schema-and-usage.md` | Consumed GSI fields, event flow, fixture references |
| `docs/workflows/adding-a-hero.md` | Step-by-step hero addition workflow |
| `docs/workflows/testing-and-debugging.md` | Test, build, fixture, and logging workflow |
| `docs/workflows/troubleshooting.md` | Common runtime failure modes and owner files |
| `docs/superpowers/specs/` | Archived design/spec docs for larger documentation or feature efforts |
| `docs/superpowers/plans/` | Archived implementation plans and execution breakdowns |

---

## Read Before Editing

| You are changing… | Read first |
|---|---|
| Any hero script in `src/actions/heroes/` | The matching hero doc for the current repo state; `docs/workflows/adding-a-hero.md` |
| `src/actions/dispatcher.rs` | `docs/architecture/state-and-dispatch.md`, `docs/architecture/runtime-flow.md` |
| `src/actions/danger_detector.rs` | `docs/features/danger-detection.md` |
| `src/actions/common.rs` | `docs/features/survivability.md`, `docs/features/danger-detection.md` |
| `src/actions/auto_items.rs` | `docs/features/survivability.md` |
| `src/actions/dispel.rs` | `docs/features/survivability.md`, `docs/features/danger-detection.md` |
| `src/actions/soul_ring.rs` | `docs/features/keyboard-interception.md`, `docs/features/soul-ring.md` |
| `src/input/keyboard.rs` | `docs/features/keyboard-interception.md` |
| `src/config/settings.rs` | `docs/reference/configuration.md` plus the affected hero/feature doc |
| `src/state/app_state.rs` | `docs/architecture/state-and-dispatch.md` |
| `src/gsi/handler.rs` or `src/gsi/server.rs` | `docs/architecture/runtime-flow.md`, `docs/reference/gsi-schema-and-usage.md` |
| `src/main.rs` | `docs/architecture/overview.md`, `docs/architecture/runtime-flow.md`, `docs/features/updates.md` |
| `src/ui/app.rs` | The affected feature doc (`danger-detection`, `updates`, or hero docs) |
| `src/models/gsi_event.rs` | `docs/reference/gsi-schema-and-usage.md`, `docs/reference/file-index.md`, and `docs/architecture/runtime-flow.md` |
| `src/update/mod.rs` | `docs/features/updates.md` |

---

## Documentation Maintenance Contract

1. **Every hero script needs a paired doc.** All hero docs live under `docs/heroes/`. Use `docs/heroes/hero-template.md` when creating a new hero doc. When you add or change `src/actions/heroes/<hero>.rs`, update the matching hero doc.
2. **Config changes require doc updates.** When you add a field to any `*Config` struct in `src/config/settings.rs`, add it to the configuration table in the relevant feature or hero doc.
3. **File-index is the authoritative map.** After adding a new source file, add it to `docs/reference/file-index.md`.
4. **Keep `AGENTS.md` navigation current.** If you add a new workflow or feature doc, add a row to the "If You Want To…" table above.

---

## Git / Commit Workflow

- **Use Conventional Commits by default.** Prefer standard prefixes such as `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, and `chore:` so commit intent is easy to scan from history.
- **Keep commit messages plain by default.** When writing or proposing a commit message, do not add extra trailer lines or extra formatting unless the user explicitly asks for them.
- **Do not stage session-state artifacts by default.** Copilot session plan files and other local session-state artifacts are not part of normal repo commits unless the user explicitly asks to include them.
- **Keep commit scope tight.** Before committing, verify the staged set contains only the repo files for the current slice and excludes unrelated docs, scratch files, or local planning artifacts.

---

## References

| Doc | What it covers |
|---|---|
| `docs/architecture/overview.md` | Module structure, entry points, subsystem map |
| `docs/architecture/runtime-flow.md` | Startup sequence, queueing, keyboard/GSI/runtime threads |
| `docs/architecture/state-and-dispatch.md` | `AppState`, shared locks, dispatcher and hero/common composition |
| `docs/features/danger-detection.md` | HP heuristics, healing escalation, defensive-item behavior |
| `docs/features/keyboard-interception.md` | Global hook ordering, Soul Ring replay, SF interception |
| `docs/features/survivability.md` | Shared healing, dispel, neutral items, common item state |
| `docs/features/updates.md` | Startup checks, update banner, download/apply/restart |
| `docs/reference/configuration.md` | Config sections, checked-in values, Rust fallback defaults |
| `docs/reference/gsi-schema-and-usage.md` | Consumed GSI fields, event flow, fixture-backed references |
| `docs/reference/file-index.md` | Every file → purpose → linked doc |
| `docs/workflows/adding-a-hero.md` | End-to-end hero addition checklist |
| `docs/workflows/testing-and-debugging.md` | `cargo test`, `cargo build --release`, `RUST_LOG`, fixtures |
| `docs/workflows/troubleshooting.md` | GSI connectivity, config drift, key intercept failures |
| `README.md` | User-facing overview and installation |
