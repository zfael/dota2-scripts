# Minimap Hero Detection — Design Spec

## Goal

Detect hero icon positions on the Dota 2 minimap from captured screenshots, enabling lane heat and activity awareness without relying on GSI (which doesn't expose other players' positions).

## Context

- Minimap captures are 240×245 RGBA PNGs from the `PrintWindow` pipeline.
- Hero icons have colored borders: red for one team, green for the other.
- Which color = allies depends on the player's side (Dire = red allies, Radiant = green allies).
- Static map elements (towers, buildings, camp markers) also appear as red/green.
- From cross-frame analysis of 17 real game samples, we identified:
  - 6 stable red positions (Dire structures) appearing in >80% of frames.
  - 0 stable green positions (enemy buildings destroyed or different shade).
  - Dynamic clusters (1–7/17 frames) correspond to hero movements.

## Architecture

### Phase 1: Detection Engine (this spec)

1. **Static baseline mask** — On startup or when calibrating, capture N frames and mark pixel positions that appear consistently as "static". These are map fixtures (towers, buildings, camp markers).

2. **Color segmentation** — For each new capture:
   - Convert RGBA pixels to HSV.
   - Threshold for red-ish and green-ish pixels.
   - Subtract the static baseline mask.
   - Find connected clusters in the remaining pixels (BFS flood fill).
   - Filter by size: hero icons are typically 30–120 pixels in a 240×245 capture.

3. **Zone mapping** — Map detected cluster centroids to predefined map zones:
   - Top Lane, Mid Lane, Bot Lane
   - Dire Jungle, Radiant Jungle
   - Roshan pit

4. **Team color resolution** — Determine which color = allies:
   - Primary: Parse `player.team_name` from GSI payload (add to model).
   - Fallback: Config field `player_team = "dire"` or `"radiant"`.

### Phase 2: Lane Heat (future, not this spec)

- Aggregate hero positions over a sliding time window.
- Classify lane activity: quiet / active / fight.
- Surface via UI panel or audio alert.

## Detection Parameters (tunable via config)

| Parameter | Default | Description |
|---|---|---|
| `baseline_frames` | 10 | Frames used to build static mask |
| `baseline_threshold` | 0.8 | Fraction of frames a pixel must appear in to be "static" |
| `red_hue_range` | `[0, 15] ∪ [340, 360]` | HSV hue range for red detection |
| `red_min_saturation` | 40 | Minimum saturation for red |
| `red_min_value` | 30 | Minimum brightness for red |
| `green_hue_range` | `[80, 160]` | HSV hue range for green detection |
| `green_min_saturation` | 35 | Minimum saturation for green |
| `green_min_value` | 25 | Minimum brightness for green |
| `min_cluster_size` | 20 | Minimum pixels to count as a cluster |
| `max_cluster_size` | 200 | Maximum pixels (larger = not a hero icon) |

## Zone Definitions

Map zones as percentage rectangles of the capture area:

```
Top Lane:     x=[0.00, 0.25] y=[0.00, 0.55]
Mid Lane:     x=[0.25, 0.75] y=[0.25, 0.75]
Bot Lane:     x=[0.75, 1.00] y=[0.45, 1.00]
Dire Jungle:  x=[0.45, 1.00] y=[0.00, 0.45]
Rad Jungle:   x=[0.00, 0.55] y=[0.55, 1.00]
Roshan:       x=[0.30, 0.45] y=[0.32, 0.45]
```

## GSI Enhancement (Small)

Add `player` section to the GSI model:

```rust
pub struct Player {
    pub team_name: Option<String>, // "dire" or "radiant"
}
```

Update `GsiWebhookEvent`:
```rust
pub struct GsiWebhookEvent {
    pub hero: Hero,
    pub abilities: Abilities,
    pub items: Items,
    pub map: Map,
    pub player: Option<Player>,
}
```

Update the Dota 2 GSI config file to request `"player" "1"`.

## File Structure

| File | Purpose |
|---|---|
| `src/observability/minimap_analysis.rs` | Color segmentation, clustering, zone mapping |
| `src/observability/minimap_baseline.rs` | Static mask builder from N-frame accumulation |
| `src/observability/minimap_zones.rs` | Zone definitions and point-to-zone mapping |
| `src/models/gsi_event.rs` | Add `Player` struct with `team_name` |
| `src/config/settings.rs` | Detection parameter config |
| `examples/minimap_analyze.rs` | Standalone analysis tool for testing |
| `tests/minimap_analysis_tests.rs` | Unit tests for detection logic |

## Testing Strategy

- **Unit tests**: HSV conversion, color thresholding, clustering, zone mapping — all pure functions.
- **Integration test with fixtures**: Load a real capture PNG, run detection, assert expected cluster count/zones.
- **Standalone example**: `cargo run --example minimap_analyze` loads captures from `logs/minimap_capture/` and prints detected hero positions + zone heat.

## Constraints

- Detection is read-only — no gameplay actions.
- All parameters are tunable via config so thresholds can be adjusted without recompiling.
- The baseline mask is rebuilt when capture coordinates change.
- Performance target: <5ms per frame analysis (240×245 = ~59K pixels).

## Open Questions

1. Should the baseline be persisted to disk or rebuilt each session?
2. Should detection run on every capture frame or at a lower rate?
