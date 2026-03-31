# Minimap Capture Example — Design Spec

## Goal

A standalone Cargo example that captures the Dota 2 minimap region once, saves a PNG + JSON metadata, and exits. Enables fast iteration on capture coordinates and pipeline health without running the full app.

## Usage

```
cargo run --example minimap_capture
cargo run --example minimap_capture -- --x 15 --y 820 --width 280 --height 280
cargo run --example minimap_capture -- --output captures/test1
```

## Behavior

1. Parse optional CLI args: `--x`, `--y`, `--width`, `--height`, `--output`. Defaults: 10, 815, 260, 260, `logs/minimap_capture`.
2. Print the configured coordinates.
3. Call `find_dota2_window_rect()`. On success, print window dimensions. On failure, print a clear error and exit with code 1.
4. Call `capture_window_region(x, y, width, height)`. On failure, print the error and exit with code 1.
5. Generate a timestamped file stem (`minimap_{unix_seconds}`).
6. Call `save_capture_artifact()` to write the PNG.
7. Call `save_metadata_json()` to write the JSON sidecar (build metadata via `build_artifact_metadata()`).
8. Print both output paths and the capture duration, then exit with code 0.

## CLI Arg Parsing

Use `std::env::args()` with simple `--key value` pair matching. No new dependencies. Unrecognized args print a short usage message and exit.

## File

- `examples/minimap_capture.rs` — single file, no modules.

## Dependencies

All functionality comes from the existing `dota2_scripts` library crate:

- `dota2_scripts::observability::minimap_capture_backend::{find_dota2_window_rect, capture_window_region, CaptureBackendResult}`
- `dota2_scripts::observability::minimap_artifacts::{save_capture_artifact, save_metadata_json, build_artifact_metadata}`
- `std::time::{Instant, SystemTime, UNIX_EPOCH}` for timing and timestamps
- `std::env` for arg parsing

## Output Format

```
Minimap Capture Utility
  Region: x=10, y=815, 260x260
  Output: logs/minimap_capture

Finding Dota 2 window... Found (1920x1080)
Capturing minimap region... OK (12ms)
Saved PNG:  logs/minimap_capture/minimap_1711883198.png
Saved JSON: logs/minimap_capture/minimap_1711883198.json
```

On failure:
```
Finding Dota 2 window... NOT FOUND
Error: Dota 2 window not found. Make sure the game is running.
```

## Testing

This is a manual-run utility. No automated tests — correctness is verified by inspecting the output PNG. The underlying functions are already tested in `tests/minimap_capture_tests.rs`.
