# Testing and Debugging

**Purpose**: Use this page for the repo's normal verification loop: tests, release builds, logging, fixtures, and live runtime checks.

---

## Quick commands

| Command | Use it for |
|---|---|
| `cargo test` | Run the current test suite |
| `cargo test test_huskar_armlet_detection` | Run one named test by filter |
| `cargo test armlet --lib` | Run the shared armlet regression + replay-model tests |
| `cargo test print_armlet_tuning_matrix_for_burst_scenarios --lib -- --ignored --nocapture` | Print the built-in armlet threshold/cooldown comparison matrix |
| `cargo build --release` | Verify the optimized build still succeeds |
| ``$env:RUST_LOG="debug"; cargo run --release`` | Maximum practical runtime visibility while debugging in PowerShell |
| ``$env:RUST_LOG="info"; cargo run --release`` | Normal operator-level logs in PowerShell |

If you only changed a fixture-backed GSI behavior, still run both `cargo test` and `cargo build --release` before calling the work done.

---

## What the current test surface covers

| Path | Coverage today |
|---|---|
| `tests/gsi_handler_tests.rs` | Schema / fixture smoke tests for `GsiWebhookEvent` deserialization and representative field access |
| `src/actions/soul_ring.rs` | Unit test covering the default `SoulRingState` shape |
| `tests/fixtures/huskar_event.json` | Huskar payload with Armlet and Magic Wand examples |
| `tests/fixtures/tiny_event.json` | Tiny payload with Aghanim's Scepter example |

Important: despite the filename, `tests/gsi_handler_tests.rs` is currently focused on fixture deserialization and field assertions, not full HTTP handler or dispatcher integration. The in-file test in `src/actions/soul_ring.rs` is a narrow unit test, not end-to-end runtime coverage.

---

## Normal verification loop

1. **Run the test suite**
   - `cargo test`
2. **Run the release build**
   - `cargo build --release`
3. **If behavior is runtime-only, run with logs**
   - ``$env:RUST_LOG="debug"; cargo run --release``
4. **Exercise the feature in game or with fixtures**
5. **Confirm docs still match the code/config**

For docs-only work, the same commands are still the safest repo-wide smoke check.

---

## Fixture-based verification

Use fixtures when the behavior depends on GSI payload shape or on a specific hero/item state.

### Current pattern

`tests/gsi_handler_tests.rs` uses:

1. `fs::read_to_string("tests/fixtures/<file>.json")`
2. `serde_json::from_str::<GsiWebhookEvent>(&json_data)`
3. direct assertions on `event.hero`, `event.items`, or `event.abilities`

### When to add or update a fixture

Add or adjust a fixture when:

- a new GSI field becomes required
- an item/ability naming assumption changes
- a hero script depends on a payload shape not represented today
- you need a regression test for a bug that came from malformed or surprising GSI data

### What a useful fixture should show

Try to include the exact fields your logic reads:

- `hero.name`
- `hero.health` / `hero.health_percent`
- `hero.mana_percent`
- `items.slot*` / `items.neutral0`
- `abilities.ability*`

See `docs/reference/gsi-schema-and-usage.md` for the current runtime consumers.

---

## Logging model

### Startup and global logging

- `src/main.rs` initializes tracing using:
  - `RUST_LOG` if present
  - otherwise `[logging].level` from `config/config.toml`

### Practical log levels

| Level | What you usually get |
|---|---|
| `info` | Startup, major action decisions, hero action summaries, update flow |
| `debug` | Gating decisions, input interception details, GSI routing details, item/ability readiness checks |

### Useful focused filters

```powershell
$env:RUST_LOG="dota2_scripts::gsi=debug,dota2_scripts::actions=debug"; cargo run --release
$env:RUST_LOG="dota2_scripts::input=debug"; cargo run --release
$env:RUST_LOG="dota2_scripts::actions::heroes=debug"; cargo run --release
$env:RUST_LOG="dota2_scripts::actions::armlet=debug,dota2_scripts::input::simulation=debug"; cargo run --release
```

### Armlet-specific diagnostics

For armlet tuning, the most useful pair is:

1. `cargo test armlet --lib`
2. `cargo test print_armlet_tuning_matrix_for_burst_scenarios --lib -- --ignored --nocapture`

The first command runs the deterministic replay tests that lock down:

- threshold sensitivity
- cooldown tradeoffs under repeated burst windows
- critical-retry behavior

The ignored matrix test prints a small built-in comparison table across several threshold / cooldown combinations so you can see how many toggle attempts, cooldown blocks, and critical retries each config would produce on the sample damage timelines.

For live runtime confirmation, run:

```powershell
$env:RUST_LOG="dota2_scripts::actions::armlet=debug,dota2_scripts::input::simulation=debug"; cargo run --release
```

That filter shows:

- resolved armlet trigger decisions and cooldown blocks from `src/actions/armlet.rs`
- the executed synthetic input sequence from `src/input/simulation.rs`

So you can confirm whether a live toggle emitted `slot-key -> modifier down -> slot-key -> modifier up`, and roughly how many milliseconds each step took on the synthetic-input lane.

The current armlet implementation uses one dedicated worker-side chord command rather than four separately queued jobs, so the logs should show the whole armlet sequence inside a single synthetic-input worker operation with one short guard window after the chord completes.

### GSI payload logging

If you need raw event capture over time, enable:

```toml
[gsi_logging]
enabled = true
output_dir = "logs/gsi_events"
```

That makes `src/gsi/handler.rs` write one JSON event per line.

---

## Live runtime checks

The main UI in `src/ui/app.rs` already exposes the fastest sanity checks:

| UI field / area | What it tells you |
|---|---|
| `Current Hero` | The raw `event.hero.name` from the latest GSI payload |
| `Active Hero` | The current `AppState.selected_hero` / manual-override target for standalone flows |
| `No GSI events received yet` vs hero data | Whether the app is receiving payloads at all |
| `Events Processed` | Whether GSI traffic is moving |
| `Queue Depth` | Whether the bounded queue may be backing up |
| HP / Mana / Status labels | Whether the current payload values look sane |

If you are debugging interception or combos, pair the UI with:

- `docs/features/keyboard-interception.md`
- the hero doc in `docs/heroes/`

---

## Fast paths by problem type

| Problem type | Best first move |
|---|---|
| GSI schema or field mismatch | add or edit a fixture + `cargo test` |
| Hero script branch not firing | ``$env:RUST_LOG="debug"; cargo run --release`` + inspect the matching hero doc |
| Key interception feels wrong | debug logging + `docs/features/keyboard-interception.md` |
| Config tuning change | release run with logs + `docs/reference/configuration.md` |

---

## Related docs

- `docs/workflows/troubleshooting.md`
- `docs/reference/gsi-schema-and-usage.md`
- `docs/reference/configuration.md`
- `docs/architecture/runtime-flow.md`
