# Dota 2 Script Automation

Windows-focused Rust automation tooling for Dota 2.

The app combines:

- GSI-driven automation from live game state
- global keyboard and mouse interception for combo helpers
- an `egui` desktop UI for status, settings, and updates

## Supported heroes

- Broodmother
- Huskar
- Largo
- Legion Commander
- Outworld Destroyer
- Shadow Fiend
- Tiny

Hero-specific behavior lives under `src/actions/heroes/`, with matching docs in `docs/heroes/`.

## Current feature set

- Shared survivability automation for healing, defensive items, neutral items, and silence dispels
- Danger detection from HP-loss heuristics
- Soul Ring pre-cast automation for eligible ability/item keys
- Hero-specific combo or interception flows:
  - Huskar armlet + Berserker Blood handling
  - Largo Amphibian Rhapsody beat timing and song control
  - Legion Commander standalone combo
  - Outworld Destroyer danger-triggered barrier, ultimate interception, self-Astral helper, and standalone combo
  - Shadow Fiend raze/ultimate interception plus standalone combo
  - Tiny standalone combo
  - Broodmother spider micro and Space+right-click auto-items/abilities
- Startup update checks via GitHub Releases

## Requirements

- Windows 10/11
- Rust toolchain
- Dota 2 with Game State Integration enabled
- Administrator privileges recommended for reliable global input interception

## Quick start

1. Clone the repo and build:

```powershell
git clone https://github.com/yourusername/dota2-scripts.git
cd dota2-scripts
cargo build --release
```

2. Create a Dota 2 GSI config file at:

```text
steamapps/common/dota 2 beta/game/dota/cfg/gamestate_integration/gamestate_integration_dota2scripts.cfg
```

3. Point Dota at the local server. The default checked-in port is `3000`, but if you change `[server].port` in `config/config.toml`, update the URI to match:

```text
"Dota 2 Integration Configuration"
{
    "uri"               "http://127.0.0.1:3000/"
    "timeout"           "5.0"
    "buffer"            "0.1"
    "throttle"          "0.1"
    "heartbeat"         "30.0"
    "data"
    {
        "provider"      "1"
        "map"           "1"
        "player"        "1"
        "hero"          "1"
        "abilities"     "1"
        "items"         "1"
    }
}
```

4. Run the app:

```powershell
cargo run --release
```

For verbose logs in PowerShell:

```powershell
$env:RUST_LOG="debug"; cargo run --release
```

## Configuration

Main runtime settings live in `config/config.toml`.

The current checked-in config surface includes:

- `[server]`
- `[updates]`
- `[keybindings]`
- `[logging]`
- `[common]`
- `[danger_detection]`
- `[neutral_items]`
- `[gsi_logging]`
- `[soul_ring]`
- `[heroes.huskar]`
- `[heroes.legion_commander]`
- `[heroes.shadow_fiend]`
- `[heroes.tiny]`
- `[heroes.outworld_destroyer]`
- `[heroes.largo]`
- `[heroes.broodmother]`

For exact fields, checked-in values, fallback defaults, and known runtime drift, use `docs/reference/configuration.md`.

## Using the UI

The app exposes:

- current hero/event status from the latest GSI payload
- manual hero override for `HeroType`-based heroes
- standalone trigger display for the currently selected hero
- GSI and standalone enable toggles
- update banner and settings
- queue/event metrics for debugging

## Development and verification

Normal repo checks:

```powershell
cargo test
cargo build --release
$env:RUST_LOG="debug"; cargo run --release
```

Current tests include:

- fixture-backed deserialization checks in `tests/gsi_handler_tests.rs`
- JSON fixtures in `tests/fixtures/`
- a small unit test in `src/actions/soul_ring.rs`

## Documentation map

- `AGENTS.md` — canonical navigation hub for contributors and AI agents
- `docs/architecture/overview.md` — system map and entry points
- `docs/reference/configuration.md` — config reference and fallback defaults
- `docs/reference/gsi-schema-and-usage.md` — consumed GSI fields and where they flow
- `docs/workflows/testing-and-debugging.md` — tests, logs, fixtures, runtime checks
- `docs/workflows/adding-a-hero.md` — end-to-end hero addition checklist
- `docs/reference/file-index.md` — file-to-purpose index for the repo

## License

MIT

## Disclaimer

This tool is for educational purposes. Use it responsibly and ensure compliance with Dota 2 and Steam terms of service.
