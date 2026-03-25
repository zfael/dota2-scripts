# Adding a Hero

**Purpose**: Use this checklist when adding a new hero script, from the Rust type and dispatcher registration through config, UI/state wiring, and required docs.

---

## Files you will usually touch

| Path | Why it matters |
|---|---|
| `src/actions/heroes/traits.rs` | Defines the `HeroScript` contract |
| `src/actions/heroes/<hero>.rs` | New hero implementation |
| `src/actions/heroes/mod.rs` | Adds the module + re-export |
| `src/actions/dispatcher.rs` | Registers the hero for GSI routing |
| `src/config/settings.rs` | Adds `<Hero>Config`, defaults, and `HeroesConfig` field |
| `config/config.toml` | Adds `[heroes.<hero>]` checked-in config |
| `src/state/app_state.rs` | Adds `HeroType` if the hero participates in the generic UI/manual-override flow |
| `src/ui/app.rs` | Manual override buttons, keybinding display, selected-hero behavior |
| `src/main.rs` | Generic standalone combo routing and any special `HotkeyEvent` handling |
| `src/input/keyboard.rs` | Needed when the hero uses direct key or mouse interception |
| `docs/heroes/<hero>.md` | Required hero doc |
| `docs/reference/configuration.md` | Update if you add or rename config fields |
| `docs/reference/gsi-schema-and-usage.md` | Update if you start depending on new GSI fields |
| `docs/reference/file-index.md` | Add the new file/doc touchpoints |
| `AGENTS.md` | Add the hero to navigation and the Hero Docs table |

---

## 1. Choose the identity and registration model

Start with the internal hero name:

- Prefer `src/models/heroes.rs` if the enum already has the hero
- Otherwise use Valve's internal-name reference and add the enum case if needed
- Your `HeroScript::hero_name()` must return the exact GSI string such as `npc_dota_hero_nevermore`

Current hero scripts live in `src/actions/heroes/` and all implement:

```rust
fn handle_gsi_event(&self, event: &GsiWebhookEvent);
fn handle_standalone_trigger(&self);
fn hero_name(&self) -> &'static str;
fn as_any(&self) -> &dyn Any;
```

See `src/actions/heroes/traits.rs`.

---

## 2. Create the hero script

Create `src/actions/heroes/<hero>.rs` and follow the existing pattern:

- store shared `Arc<Mutex<Settings>>` if the hero needs config
- keep hero-specific logic in `handle_gsi_event(...)`
- call shared survivability pieces when appropriate
- implement `handle_standalone_trigger()` only if the hero has a generic combo trigger
- expose `hero_name()` using the exact GSI hero string
- return `self` from `as_any()` if `main.rs` or another caller needs downcasting

Then register the module in `src/actions/heroes/mod.rs`:

1. `pub mod <hero>;`
2. `pub use <hero>::<HeroScriptType>;`

---

## 3. Register dispatch

Add the hero in `src/actions/dispatcher.rs::new()`:

1. construct the script with the shared settings handle
2. insert it into `hero_scripts` using `hero.hero_name().to_string()`

This is what makes `dispatch_gsi_event(...)` route matching `event.hero.name` values to the new script.

If the hero also depends on shared pre-dispatch state, check whether you need one of these existing patterns:

- cached GSI item state via `src/actions/auto_items.rs`
- a global active flag like `BROODMOTHER_ACTIVE`
- keyboard-driven side effects in `src/input/keyboard.rs`

---

## 4. Add config surface

When the hero needs config:

1. add `pub struct <Hero>Config` in `src/config/settings.rs`
2. put `#[serde(default = "default_<field>")]` on each field
3. add the `default_<field>() -> Type` helpers
4. add the fields to `impl Default for <Hero>Config`
5. add the hero to `HeroesConfig`
6. add a `[heroes.<hero>]` block to `config/config.toml`

Use the existing hero sections as templates.

Also update:

- `docs/heroes/<hero>.md`
- `docs/reference/configuration.md`

If the new config key controls intercepted keys or item/ability mapping, also check:

- `docs/features/keyboard-interception.md`
- `docs/reference/gsi-schema-and-usage.md`

---

## 5. Decide whether the hero belongs in UI/state generic flow

There are two patterns in the current codebase.

### Pattern A: generic selected-hero + combo-trigger flow

Use this when the hero should appear in the main-window manual override and respond to the generic standalone combo trigger.

Update:

- `src/state/app_state.rs`
  - add `HeroType::<Hero>`
  - extend `HeroType::from_hero_name(...)`
  - extend `to_display_name()`
- `src/ui/app.rs`
   - add the hero to the Manual Override buttons
   - add any hero-specific keybinding text if needed
- `src/config/settings.rs`
   - update `Settings::get_standalone_key(...)` if this hero should expose a hero-specific manual trigger in the UI/runtime
- `src/main.rs`
   - extend the `HotkeyEvent::ComboTrigger` hero-name match
   - add any per-hero standalone handling

### Pattern B: bespoke interception path

Use this when the hero is driven by direct keyboard/mouse interception instead of the generic combo-trigger flow.

Current examples:

- Broodmother: mouse-based macros keyed off `BROODMOTHER_ACTIVE`

For this pattern, update the interception path directly and document it in the hero doc and any affected feature doc.

### Hybrid pattern: generic flow plus bespoke interception

Some heroes participate in the generic `HeroType` / manual-override flow **and** also add custom interception paths.

Current example:

- Shadow Fiend: appears in `HeroType`, handles `HotkeyEvent::ComboTrigger`, and also layers direct `Q/W/E/R` interception in `src/input/keyboard.rs`

For this pattern, do both sets of work: wire the hero into Pattern A, then document and implement the interception-specific path.

---

## 6. Update docs before you consider the hero done

Required doc updates:

| Path | Required change |
|---|---|
| `docs/heroes/<hero>.md` | Create the hero doc from `docs/heroes/hero-template.md` and describe config, triggers, limitations, and logging |
| `AGENTS.md` | Add a row to the Hero Docs table and update navigation if the hero introduces a new workflow or notable feature path |
| `docs/reference/file-index.md` | Add the new source file row and relevant doc links |
| `docs/reference/configuration.md` | Add/update the hero config section |
| `docs/reference/gsi-schema-and-usage.md` | Update if the hero depends on new GSI fields, slots, or ability conventions |

This repo's maintenance contract is: **every hero script needs a paired doc under `docs/heroes/`.**

---

## 7. Verify

Minimum verification loop:

```powershell
cargo test
cargo build --release
$env:RUST_LOG="debug"; cargo run --release
```

Then check:

- the `Current Hero` status line shows the expected `event.hero.name`
- if the hero participates in `HeroType`/manual override flow, confirm it appears there too
- the relevant GSI fields are present in fixtures or live logs
- docs match the new config and trigger model

If the hero is GSI-heavy, add or update fixture coverage in:

- `tests/gsi_handler_tests.rs`
- `tests/fixtures/`

See `docs/workflows/testing-and-debugging.md` and `docs/workflows/troubleshooting.md`.

---

## Related docs

- `docs/architecture/state-and-dispatch.md`
- `docs/reference/configuration.md`
- `docs/reference/gsi-schema-and-usage.md`
- `docs/reference/file-index.md`
- `docs/heroes/hero-template.md`
