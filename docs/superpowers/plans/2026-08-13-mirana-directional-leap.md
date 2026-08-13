# Mirana Directional Leap Implementation Plan

**Goal:** Add a Mirana hero that intercepts the Leap key (default **E**) and faces the cursor before casting, so Leap goes where the mouse is pointing instead of wherever Mirana happened to be facing.

**Architecture:** A direct copy of the Slark directional Pounce path. Leap, like Pounce, is a no-target ability that leaps along the hero's **facing at cast time**, so the combo is `ALT down → right-click (face cursor) → ALT up → wait turn_delay_ms → press leap key`. ALT is released *before* the ability press: Leap takes no target, and ALT held over an ability key pings it to allies instead of casting it. A new `MiranaScript` (`HeroScript`) owns a dedicated request worker; the keyboard callback gates the intercept on Mirana being the active hero (`selected_hero`) and on `mirana_leap` being levelled and castable.

**Tech Stack:** Rust 2021, `rdev` (global hook), `enigo`-backed `src/input/simulation.rs`, `serde`/TOML config, `tracing`, React/TS UI.

**Reference implementation:** `src/actions/heroes/slark.rs`, `docs/heroes/slark.md`.

---

## Why Slark's path and not Snapfire's

| | Snapfire cookie | Slark Pounce / **Mirana Leap** |
|---|---|---|
| Ability targeting | self-cast (needs the ALT modifier on the key press) | no target |
| ALT held across the ability press? | **yes** | **no** — ALT over an ability key pings it |
| Intercepted key | a separate trigger (Space) | the ability key itself |
| GSI readiness gate | none | yes, matched by ability name |

Mirana matches the right-hand column on every row.

---

## File Structure

| File | Responsibility | Create/Modify |
|---|---|---|
| `src/models/heroes.rs` | `Hero::Mirana` + `npc_dota_hero_mirana` (already present) | No change |
| `src/state/app_state.rs` | `HeroType::Mirana` + `from_hero_name` / `to_display_name` | Modify |
| `src/config/settings.rs` | `MiranaConfig`, defaults, `HeroesConfig.mirana` | Modify |
| `config/config.toml` | `[heroes.mirana]` block | Modify |
| `src/actions/heroes/mirana.rs` | `MiranaScript`, worker, `MiranaState::execute_directional_leap`, readiness gate | Create |
| `src/actions/heroes/mod.rs` | module + re-export | Modify |
| `src/actions/dispatcher.rs` | register `MiranaScript` | Modify |
| `src/input/keyboard.rs` | `MiranaKeyboardSnapshot`, snapshot fields, intercept branch | Modify |
| `src/main.rs` | `HeroType::Mirana` arm in the `ComboTrigger` hero-name match | Modify |
| `src-tauri/src/lib.rs`, `src-tauri/src/commands/state.rs` | `HeroType::Mirana` arms (exhaustive matches) | Modify |
| `tests/fixtures/mirana_event.json` | GSI fixture backing the readiness tests | Create |
| `src-ui/src/types/config.ts`, `types/game.ts`, `components/heroes/configs/MiranaConfig.tsx`, `configs/index.ts`, `stores/mockData.ts` | UI config panel + hero listing | Create/Modify |
| `docs/heroes/mirana.md` | hero doc (maintenance contract) | Create |
| `AGENTS.md`, `docs/reference/file-index.md`, `docs/reference/configuration.md`, `docs/features/keyboard-interception.md` | doc maintenance contract | Modify |

**Activation note:** `mirana_enabled` is derived in `KeyboardSnapshot::from_runtime` from `state.selected_hero == Some(HeroType::Mirana)`, which GSI hero detection and the Tauri manual-override command both already set. No new `Arc<Mutex<bool>>` flag and no new Tauri command — same model as Snapfire, Magnus, and Slark.

---

## Task 1: Add `HeroType::Mirana`

- Add `Mirana,` to the `HeroType` enum in `src/state/app_state.rs`, plus arms in `from_hero_name` and `to_display_name`.
- Test: `mirana_hero_type_maps_name_and_display`, mirroring `slark_hero_type_maps_name_and_display`.
- The enum is matched exhaustively in `src/main.rs` and `src-tauri/src/lib.rs`, so both need the new arm before anything compiles.

## Task 2: Add `MiranaConfig`

Four fields, the same shape as the Slark intercept half:

| Field | Type | Default | Purpose |
|---|---|---|---|
| `enabled` | bool | `true` | Master toggle for the intercept. |
| `leap_key` | char | `'e'` | Leap ability key; also the key intercepted. |
| `turn_delay_ms` | u64 | `200` | Delay after the facing right-click before the Leap cast. |
| `require_ability_ready` | bool | `true` | Pass the key through when Leap is unlevelled or on cooldown. |

`turn_delay_ms` starts at Slark's proven `200`, not the `60` Magnus and Snapfire use — 60ms was tuned for a facing turn that only has to be *approximately* right for a pull, while a leap that fires before the turn finishes lands somewhere else entirely.

Add the `default_mirana_*` helpers, the `impl Default`, the `HeroesConfig` field, and the `[heroes.mirana]` block in `config/config.toml`.

Test: `mirana_config_defaults_gate_leap_on_readiness`.

## Task 3: Create the hero script

`src/actions/heroes/mirana.rs`, modelled on `slark.rs` minus the GSI-driven cleanse and escape:

- `MIRANA_LAST_EVENT: Arc<Mutex<Option<GsiWebhookEvent>>>`, refreshed on every `handle_gsi_event`.
- `MIRANA_REQUEST_QUEUE` worker + `MiranaRequest::DirectionalLeap { leap_key, turn_delay_ms }` and the fallback-thread path for a closed queue.
- `run_directional_leap_request`: `sleep(PRE_TURN_SETTLE_MS) → alt_down → mouse_click → sleep(ALT_RELEASE_DELAY_MS) → alt_up → sleep(turn_delay_ms) → press_key(leap_key)`.
- `ability_is_ready(event, LEAP_ABILITY_NAME)` scanning slots 0..=5 and matching **by name**.
- `MiranaState::can_intercept_leap()` / `MiranaState::execute_directional_leap(...)`.
- `impl HeroScript`: `handle_gsi_event` caches the event, advances the danger detector, and runs the shared survivability checks (Snapfire-shaped); `handle_standalone_trigger` runs the same combo.

Register the module in `src/actions/heroes/mod.rs` and the script in `src/actions/dispatcher.rs`.

**Match by name, never by slot index.** `docs/heroes/slark.md` records why: GSI slot order is *ability* order, and shard/scepter/innate entries are inserted ahead of the ultimate, so deriving a slot from the key it is bound to reads the wrong ability. Slark's shard fallback shipped broken for exactly this reason.

Create `tests/fixtures/mirana_event.json` alongside it.

> **Fixture caveat:** unlike `slark_event.json` this one is **handcrafted**, not captured from a live game, so its slot ordering is a plausible guess rather than ground truth. That is safe for the tests it backs — they assert name-matching, which is deliberately independent of slot order — but see the verification note below about charges.

## Task 4: Wire the keyboard interception

- `MiranaKeyboardSnapshot { enabled, leap_key: Option<Key>, leap_char: char, turn_delay_ms, require_ability_ready }`, exactly parallel to `SlarkKeyboardSnapshot`.
- Add `mirana_enabled` + `mirana` to `KeyboardSnapshot`, and populate them in `from_runtime`, `impl Default`, and the `broodmother_test_snapshot` test helper.
- Add the intercept branch next to the Slark one; on a pass, block the original key and enqueue the combo, on a fail fall through so the key reaches Dota untouched.
- Test: `keyboard_snapshot_populates_mirana_fields_for_mirana`.

## Task 5: Wire the React UI

`MiranaConfig` interface + `heroes.mirana`, `"mirana"` in the `HeroId` union and a `HEROES` row, a `MiranaConfig.tsx` panel (Directional Leap card + Safety card, copied from the top of `SlarkConfig.tsx`), the lazy-import row, and the mock config.

## Task 6: Docs and verification

`docs/heroes/mirana.md` from the hero template, plus rows in `AGENTS.md`, `docs/reference/file-index.md`, `docs/reference/configuration.md`, and a decision-tree step in `docs/features/keyboard-interception.md`.

Verify with `cargo test`, `cargo build --release`, and `npm test` in `src-ui`.

---

## Verify in-game before trusting the readiness gate

Two Mirana-specific unknowns that a live payload settles, and that no amount of unit testing can:

1. **Leap has charges.** The readiness gate reads `level > 0 && can_cast`, which is exactly right for Pounce's single cooldown. How GSI reports `can_cast` for a charge-based ability with a charge banked but the charge-refresh timer running is not something the existing fixtures cover. If `can_cast` reads `false` while a charge is actually available, `require_ability_ready = true` silently disables the whole feature — the key just passes through. Symptom to watch for: `🌙 Mirana leap intercept skipped: Leap not ready` in the log while Leap is visibly castable. Workaround if it happens: set `require_ability_ready = false`.
2. **Slot layout.** Confirm which index `mirana_leap` actually lands on, and replace the handcrafted fixture with the captured payload.

## Out of scope

Deliberately leap-only, mirroring what Slark shipped first:

- No auto Moonlight Shadow escape. Slark's Shadow Dance escape is a self-preservation ultimate on a solo hero; Moonlight Shadow is a **team** ultimate whose value depends on where four other players are, which GSI does not expose.
- No Sacred Arrow assistance. The hook cannot see targeting mode, so it cannot tell an armed arrow from an idle one.

## Inherited limitations (document, do not try to fix)

Shared with every facing combo in this repo:

- The facing right-click is a **move order** — Mirana walks toward the cursor for the whole `turn_delay_ms` before Leap fires.
- Cursor parked on the **minimap** issues a cross-map move order; the hook cannot see cursor position.
- Cursor over a unit produces an attack order rather than a move order; facing still resolves toward it, so the leap direction is right.
- The leap key **bypasses Soul Ring** while Mirana is active — the intercept returns before the Soul Ring replay path, matching the Shadow Fiend, Magnus, OD, and Slark branches.
- Added latency of ~100ms plus `turn_delay_ms` in front of the cast.
