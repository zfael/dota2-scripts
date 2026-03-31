# Minimap Hero Detection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Detect hero icon positions on captured Dota 2 minimap screenshots using HSV color segmentation, static baseline subtraction, BFS clustering, and zone mapping.

**Architecture:** RGBA pixels from the existing capture pipeline are analyzed per-frame: HSV color thresholding isolates red/green pixels, a static baseline mask (built from N initial frames) filters out map fixtures (towers, buildings), BFS flood fill finds connected pixel clusters that represent hero icons, and each cluster centroid is classified into a map zone (Top Lane, Mid Lane, Bot Lane, Dire Jungle, Radiant Jungle, Roshan). All analysis is pure functions with no OS or runtime dependencies. The standalone example loads PNGs from disk for offline testing.

**Tech Stack:** Rust standard library (no new crate dependencies). Pure functions for all analysis logic. `image` crate (already a dependency) for the standalone example's PNG loading.

---

## File Structure

| File | Action | Purpose |
|---|---|---|
| `src/observability/minimap_zones.rs` | Create | `MapZone` enum, zone boundary definitions, `classify_zone()` |
| `src/observability/minimap_analysis.rs` | Create | HSV conversion, color thresholding, BFS clustering, `detect_heroes()` |
| `src/observability/minimap_baseline.rs` | Create | `BaselineMask` — N-frame static pixel accumulator |
| `src/observability/mod.rs` | Modify | Export 3 new modules |
| `src/config/settings.rs` | Modify | Add `MinimapAnalysisConfig` struct |
| `config/config.toml` | Modify | Add `[minimap_analysis]` section |
| `src/models/gsi_event.rs` | Modify | Add `Player` struct with `team_name` |
| `examples/minimap_analyze.rs` | Create | Standalone analysis CLI that loads PNGs and prints detection results |
| `tests/minimap_analysis_tests.rs` | Create | Unit tests for zones, color, clustering, baseline, pipeline |
| `docs/reference/file-index.md` | Modify | Add new files |
| `docs/reference/configuration.md` | Modify | Add analysis config section |

---

### Task 1: Zone Definitions

**Files:**
- Create: `src/observability/minimap_zones.rs`
- Create: `tests/minimap_analysis_tests.rs`
- Modify: `src/observability/mod.rs`

- [ ] **Step 1: Write zone classification tests**

Create `tests/minimap_analysis_tests.rs`:

```rust
use dota2_scripts::observability::minimap_zones::{classify_zone, MapZone};

#[test]
fn classify_zone_top_lane() {
    // nx=10/240=0.042, ny=10/245=0.041 → TopLane [0.00,0.25]×[0.00,0.55]
    assert_eq!(classify_zone(10, 10, 240, 245), MapZone::TopLane);
}

#[test]
fn classify_zone_bot_lane() {
    // nx=220/240=0.917, ny=230/245=0.939 → BotLane [0.75,1.00]×[0.45,1.00]
    assert_eq!(classify_zone(220, 230, 240, 245), MapZone::BotLane);
}

#[test]
fn classify_zone_mid_lane() {
    // nx=120/240=0.500, ny=122/245=0.498 → MidLane [0.25,0.75]×[0.25,0.75]
    assert_eq!(classify_zone(120, 122, 240, 245), MapZone::MidLane);
}

#[test]
fn classify_zone_roshan() {
    // nx=90/240=0.375, ny=90/245=0.367 → Roshan [0.30,0.45]×[0.32,0.45]
    assert_eq!(classify_zone(90, 90, 240, 245), MapZone::Roshan);
}

#[test]
fn classify_zone_dire_jungle() {
    // nx=200/240=0.833, ny=50/245=0.204 → DireJungle [0.45,1.00]×[0.00,0.45]
    assert_eq!(classify_zone(200, 50, 240, 245), MapZone::DireJungle);
}

#[test]
fn classify_zone_radiant_jungle() {
    // nx=50/240=0.208, ny=180/245=0.735 → RadiantJungle [0.00,0.55]×[0.55,1.00]
    assert_eq!(classify_zone(50, 180, 240, 245), MapZone::RadiantJungle);
}

#[test]
fn classify_zone_zero_dimensions_returns_other() {
    assert_eq!(classify_zone(10, 10, 0, 0), MapZone::Other);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test classify_zone --quiet`
Expected: compilation error — module `minimap_zones` doesn't exist yet.

- [ ] **Step 3: Create `minimap_zones.rs`**

Create `src/observability/minimap_zones.rs`:

```rust
/// A region of the Dota 2 minimap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapZone {
    TopLane,
    MidLane,
    BotLane,
    DireJungle,
    RadiantJungle,
    Roshan,
    Other,
}

impl std::fmt::Display for MapZone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MapZone::TopLane => write!(f, "Top Lane"),
            MapZone::MidLane => write!(f, "Mid Lane"),
            MapZone::BotLane => write!(f, "Bot Lane"),
            MapZone::DireJungle => write!(f, "Dire Jungle"),
            MapZone::RadiantJungle => write!(f, "Radiant Jungle"),
            MapZone::Roshan => write!(f, "Roshan"),
            MapZone::Other => write!(f, "Other"),
        }
    }
}

struct ZoneBounds {
    zone: MapZone,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
}

// Roshan is checked first because its bounds overlap with MidLane.
const ZONE_DEFS: [ZoneBounds; 6] = [
    ZoneBounds { zone: MapZone::Roshan,        x1: 0.30, y1: 0.32, x2: 0.45, y2: 0.45 },
    ZoneBounds { zone: MapZone::TopLane,        x1: 0.00, y1: 0.00, x2: 0.25, y2: 0.55 },
    ZoneBounds { zone: MapZone::BotLane,        x1: 0.75, y1: 0.45, x2: 1.00, y2: 1.00 },
    ZoneBounds { zone: MapZone::DireJungle,     x1: 0.45, y1: 0.00, x2: 1.00, y2: 0.45 },
    ZoneBounds { zone: MapZone::RadiantJungle,  x1: 0.00, y1: 0.55, x2: 0.55, y2: 1.00 },
    ZoneBounds { zone: MapZone::MidLane,        x1: 0.25, y1: 0.25, x2: 0.75, y2: 0.75 },
];

/// Classify a pixel coordinate into a map zone.
///
/// Coordinates are pixel positions within the capture region (0-indexed).
/// The position is normalized to `[0.0, 1.0]` and checked against predefined
/// zone rectangles. Returns `MapZone::Other` if the point doesn't match any zone.
pub fn classify_zone(x: u32, y: u32, image_width: u32, image_height: u32) -> MapZone {
    if image_width == 0 || image_height == 0 {
        return MapZone::Other;
    }
    let nx = x as f32 / image_width as f32;
    let ny = y as f32 / image_height as f32;
    for def in &ZONE_DEFS {
        if nx >= def.x1 && nx <= def.x2 && ny >= def.y1 && ny <= def.y2 {
            return def.zone;
        }
    }
    MapZone::Other
}
```

- [ ] **Step 4: Export module in `mod.rs`**

Add to `src/observability/mod.rs`:

```rust
pub mod minimap_zones;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test classify_zone --quiet`
Expected: 7 tests pass.

- [ ] **Step 6: Commit**

```
git add src/observability/minimap_zones.rs src/observability/mod.rs tests/minimap_analysis_tests.rs
git commit -m "feat: add minimap zone definitions and classify function"
```

---

### Task 2: Color Analysis and Clustering

**Files:**
- Create: `src/observability/minimap_analysis.rs`
- Modify: `src/observability/mod.rs`
- Modify: `tests/minimap_analysis_tests.rs`

This task creates the color segmentation and BFS clustering functions. The `detect_heroes()` orchestrator is added in Task 4 after the baseline module exists.

- [ ] **Step 1: Add color and clustering tests**

Append to `tests/minimap_analysis_tests.rs`:

```rust
use dota2_scripts::observability::minimap_analysis::{
    build_color_masks, find_clusters, is_green_pixel, is_red_pixel, rgb_to_hsv, ColorThresholds,
};

#[test]
fn hsv_pure_red() {
    let hsv = rgb_to_hsv(255, 0, 0);
    assert!((hsv.h - 0.0).abs() < 1.0);
    assert!((hsv.s - 100.0).abs() < 1.0);
    assert!((hsv.v - 100.0).abs() < 1.0);
}

#[test]
fn hsv_pure_green() {
    let hsv = rgb_to_hsv(0, 255, 0);
    assert!((hsv.h - 120.0).abs() < 1.0);
    assert!((hsv.s - 100.0).abs() < 1.0);
    assert!((hsv.v - 100.0).abs() < 1.0);
}

#[test]
fn hsv_pure_blue() {
    let hsv = rgb_to_hsv(0, 0, 255);
    assert!((hsv.h - 240.0).abs() < 1.0);
    assert!((hsv.s - 100.0).abs() < 1.0);
    assert!((hsv.v - 100.0).abs() < 1.0);
}

#[test]
fn hsv_black_has_zero_value() {
    let hsv = rgb_to_hsv(0, 0, 0);
    assert!(hsv.v.abs() < 0.01);
}

#[test]
fn is_red_detects_dota_hero_red() {
    let t = ColorThresholds::default();
    assert!(is_red_pixel(200, 40, 40, &t));
    assert!(is_red_pixel(255, 0, 0, &t));
}

#[test]
fn is_red_rejects_non_red() {
    let t = ColorThresholds::default();
    assert!(!is_red_pixel(40, 200, 40, &t)); // green
    assert!(!is_red_pixel(30, 10, 10, &t));  // too dark
    assert!(!is_red_pixel(128, 128, 128, &t)); // gray (low saturation)
}

#[test]
fn is_green_detects_dota_hero_green() {
    let t = ColorThresholds::default();
    assert!(is_green_pixel(40, 200, 40, &t));
    assert!(is_green_pixel(0, 255, 0, &t));
}

#[test]
fn is_green_rejects_non_green() {
    let t = ColorThresholds::default();
    assert!(!is_green_pixel(200, 40, 40, &t)); // red
    assert!(!is_green_pixel(10, 30, 10, &t));  // too dark
}

#[test]
fn build_color_masks_separates_red_and_green() {
    let t = ColorThresholds::default();
    // 3x1 image: [red, black, green] in RGBA
    let pixels: Vec<u8> = vec![
        200, 40, 40, 255, // red
        0, 0, 0, 255,     // black
        40, 200, 40, 255, // green
    ];
    let (red_mask, green_mask) = build_color_masks(&pixels, 3, 1, &t);
    assert_eq!(red_mask, vec![true, false, false]);
    assert_eq!(green_mask, vec![false, false, true]);
}

#[test]
fn find_clusters_detects_two_separate_groups() {
    // 6x6 mask with two 2×2 clusters separated by gap
    let width = 6u32;
    let height = 6u32;
    let mut mask = vec![false; 36];
    // Cluster A at (0,0)-(1,1): indices 0,1,6,7
    mask[0] = true;
    mask[1] = true;
    mask[6] = true;
    mask[7] = true;
    // Cluster B at (4,4)-(5,5): indices 28,29,34,35
    mask[28] = true;
    mask[29] = true;
    mask[34] = true;
    mask[35] = true;

    let clusters = find_clusters(&mask, width, height, 3, 100);
    assert_eq!(clusters.len(), 2);
    assert_eq!(clusters[0].size, 4);
    assert_eq!(clusters[1].size, 4);
}

#[test]
fn find_clusters_computes_centroid() {
    // 4x4 mask with L-shaped cluster: (0,0),(1,0),(0,1)
    let width = 4u32;
    let height = 4u32;
    let mut mask = vec![false; 16];
    mask[0] = true; // (0,0)
    mask[1] = true; // (1,0)
    mask[4] = true; // (0,1)

    let clusters = find_clusters(&mask, width, height, 1, 100);
    assert_eq!(clusters.len(), 1);
    // Centroid: x=(0+1+0)/3=0, y=(0+0+1)/3=0 (integer division)
    assert_eq!(clusters[0].center_x, 0);
    assert_eq!(clusters[0].center_y, 0);
    assert_eq!(clusters[0].size, 3);
}

#[test]
fn find_clusters_filters_below_min_size() {
    let width = 4u32;
    let height = 1u32;
    // Two isolated pixels, each cluster size=1
    let mask = vec![true, false, false, true];
    let clusters = find_clusters(&mask, width, height, 3, 100);
    assert_eq!(clusters.len(), 0);
}

#[test]
fn find_clusters_filters_above_max_size() {
    let width = 3u32;
    let height = 3u32;
    let mask = vec![true; 9]; // one 3×3 cluster = 9 pixels
    let clusters = find_clusters(&mask, width, height, 1, 5);
    assert_eq!(clusters.len(), 0); // 9 > max_size 5
}

#[test]
fn find_clusters_empty_mask_returns_empty() {
    let mask = vec![false; 25];
    let clusters = find_clusters(&mask, 5, 5, 1, 100);
    assert!(clusters.is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test minimap_analysis --quiet`
Expected: compilation error — module `minimap_analysis` doesn't exist.

- [ ] **Step 3: Create `minimap_analysis.rs`**

Create `src/observability/minimap_analysis.rs`:

```rust
use std::collections::VecDeque;

/// HSV color value (hue 0–360, saturation 0–100, value 0–100).
#[derive(Debug, Clone, Copy)]
pub struct Hsv {
    pub h: f32,
    pub s: f32,
    pub v: f32,
}

/// Thresholds for red/green color detection and cluster filtering.
#[derive(Debug, Clone)]
pub struct ColorThresholds {
    pub red_hue_max: f32,
    pub red_hue_min_wrap: f32,
    pub red_min_saturation: f32,
    pub red_min_value: f32,
    pub green_hue_min: f32,
    pub green_hue_max: f32,
    pub green_min_saturation: f32,
    pub green_min_value: f32,
    pub min_cluster_size: usize,
    pub max_cluster_size: usize,
}

impl Default for ColorThresholds {
    fn default() -> Self {
        Self {
            red_hue_max: 15.0,
            red_hue_min_wrap: 340.0,
            red_min_saturation: 40.0,
            red_min_value: 30.0,
            green_hue_min: 80.0,
            green_hue_max: 160.0,
            green_min_saturation: 35.0,
            green_min_value: 25.0,
            min_cluster_size: 20,
            max_cluster_size: 200,
        }
    }
}

/// Convert RGB (0–255) to HSV (h: 0–360, s: 0–100, v: 0–100).
pub fn rgb_to_hsv(r: u8, g: u8, b: u8) -> Hsv {
    let rf = r as f32 / 255.0;
    let gf = g as f32 / 255.0;
    let bf = b as f32 / 255.0;
    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let delta = max - min;

    let v = max * 100.0;
    if max == 0.0 {
        return Hsv { h: 0.0, s: 0.0, v: 0.0 };
    }
    let s = (delta / max) * 100.0;
    if delta < 0.0001 {
        return Hsv { h: 0.0, s: 0.0, v };
    }

    let h = if (max - rf).abs() < 0.0001 {
        60.0 * (((gf - bf) / delta) % 6.0)
    } else if (max - gf).abs() < 0.0001 {
        60.0 * (((bf - rf) / delta) + 2.0)
    } else {
        60.0 * (((rf - gf) / delta) + 4.0)
    };
    let h = if h < 0.0 { h + 360.0 } else { h };

    Hsv { h, s, v }
}

/// Check if an RGB pixel falls within the red hue range.
pub fn is_red_pixel(r: u8, g: u8, b: u8, t: &ColorThresholds) -> bool {
    let hsv = rgb_to_hsv(r, g, b);
    hsv.s >= t.red_min_saturation
        && hsv.v >= t.red_min_value
        && (hsv.h <= t.red_hue_max || hsv.h >= t.red_hue_min_wrap)
}

/// Check if an RGB pixel falls within the green hue range.
pub fn is_green_pixel(r: u8, g: u8, b: u8, t: &ColorThresholds) -> bool {
    let hsv = rgb_to_hsv(r, g, b);
    hsv.s >= t.green_min_saturation
        && hsv.v >= t.green_min_value
        && hsv.h >= t.green_hue_min
        && hsv.h <= t.green_hue_max
}

/// Build boolean masks for red and green pixels from RGBA image data.
///
/// Returns `(red_mask, green_mask)` where each mask has one entry per pixel.
pub fn build_color_masks(
    pixels: &[u8],
    width: u32,
    height: u32,
    t: &ColorThresholds,
) -> (Vec<bool>, Vec<bool>) {
    let total = (width * height) as usize;
    let mut red_mask = vec![false; total];
    let mut green_mask = vec![false; total];
    for i in 0..total {
        let base = i * 4;
        if base + 3 >= pixels.len() {
            break;
        }
        let (r, g, b) = (pixels[base], pixels[base + 1], pixels[base + 2]);
        if is_red_pixel(r, g, b, t) {
            red_mask[i] = true;
        }
        if is_green_pixel(r, g, b, t) {
            green_mask[i] = true;
        }
    }
    (red_mask, green_mask)
}

/// A detected cluster of same-color pixels.
#[derive(Debug, Clone)]
pub struct DetectedCluster {
    pub center_x: u32,
    pub center_y: u32,
    pub size: usize,
}

/// Find connected clusters in a boolean mask using BFS flood fill.
///
/// Only returns clusters with `min_size <= size <= max_size`.
pub fn find_clusters(
    mask: &[bool],
    width: u32,
    height: u32,
    min_size: usize,
    max_size: usize,
) -> Vec<DetectedCluster> {
    let total = (width * height) as usize;
    let mut visited = vec![false; total];
    let mut clusters = Vec::new();

    for start in 0..total {
        if !mask[start] || visited[start] {
            continue;
        }
        let mut queue = VecDeque::new();
        queue.push_back(start);
        visited[start] = true;
        let mut sum_x: u64 = 0;
        let mut sum_y: u64 = 0;
        let mut count: usize = 0;

        while let Some(idx) = queue.pop_front() {
            let px = (idx % width as usize) as u64;
            let py = (idx / width as usize) as u64;
            sum_x += px;
            sum_y += py;
            count += 1;

            let ix = px as i32;
            let iy = py as i32;
            for (dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let nx = ix + dx;
                let ny = iy + dy;
                if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                    let ni = (ny as usize) * (width as usize) + (nx as usize);
                    if mask[ni] && !visited[ni] {
                        visited[ni] = true;
                        queue.push_back(ni);
                    }
                }
            }
        }

        if count >= min_size && count <= max_size {
            clusters.push(DetectedCluster {
                center_x: (sum_x / count as u64) as u32,
                center_y: (sum_y / count as u64) as u32,
                size: count,
            });
        }
    }

    clusters
}
```

- [ ] **Step 4: Export module in `mod.rs`**

Add to `src/observability/mod.rs`:

```rust
pub mod minimap_analysis;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test minimap_analysis --quiet`
Expected: all color/clustering tests pass (16 tests including zone tests from Task 1).

- [ ] **Step 6: Commit**

```
git add src/observability/minimap_analysis.rs src/observability/mod.rs tests/minimap_analysis_tests.rs
git commit -m "feat: add HSV color segmentation and BFS clustering"
```

---

### Task 3: Baseline Mask

**Files:**
- Create: `src/observability/minimap_baseline.rs`
- Modify: `src/observability/mod.rs`
- Modify: `tests/minimap_analysis_tests.rs`

- [ ] **Step 1: Add baseline tests**

Append to `tests/minimap_analysis_tests.rs`:

```rust
use dota2_scripts::observability::minimap_baseline::BaselineMask;

#[test]
fn baseline_marks_consistent_red_as_static() {
    let mut bl = BaselineMask::new(3, 3, 0.8);
    // 10 frames: pixel 0 is always red
    for _ in 0..10 {
        let red = vec![true, false, false, false, false, false, false, false, false];
        let green = vec![false; 9];
        bl.accumulate_frame(&red, &green);
    }
    bl.build();
    assert!(bl.is_built());
    assert!(bl.is_static_red(0));  // 10/10 = 100% > 80%
    assert!(!bl.is_static_red(1)); // 0/10 = 0%
}

#[test]
fn baseline_marks_consistent_green_as_static() {
    let mut bl = BaselineMask::new(2, 2, 0.8);
    for _ in 0..10 {
        let red = vec![false; 4];
        let green = vec![false, false, false, true]; // pixel 3 always green
        bl.accumulate_frame(&red, &green);
    }
    bl.build();
    assert!(bl.is_static_green(3));
    assert!(!bl.is_static_green(0));
}

#[test]
fn baseline_excludes_infrequent_pixels() {
    let mut bl = BaselineMask::new(3, 3, 0.8);
    // Only 3/10 frames have pixel 4 (center) as red → 30% < 80%
    for i in 0..10 {
        let mut red = vec![false; 9];
        if i < 3 {
            red[4] = true;
        }
        let green = vec![false; 9];
        bl.accumulate_frame(&red, &green);
    }
    bl.build();
    assert!(!bl.is_static_red(4));
}

#[test]
fn baseline_not_built_returns_false() {
    let bl = BaselineMask::new(2, 2, 0.8);
    assert!(!bl.is_built());
    assert!(!bl.is_static_red(0));
    assert!(!bl.is_static_green(0));
}

#[test]
fn baseline_frame_count_tracks_accumulation() {
    let mut bl = BaselineMask::new(2, 2, 0.8);
    assert_eq!(bl.frame_count(), 0);
    bl.accumulate_frame(&[false; 4], &[false; 4]);
    assert_eq!(bl.frame_count(), 1);
    bl.accumulate_frame(&[false; 4], &[false; 4]);
    assert_eq!(bl.frame_count(), 2);
}

#[test]
fn baseline_out_of_bounds_index_returns_false() {
    let mut bl = BaselineMask::new(2, 2, 0.8);
    for _ in 0..5 {
        bl.accumulate_frame(&[true; 4], &[false; 4]);
    }
    bl.build();
    assert!(!bl.is_static_red(99)); // out of bounds
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test baseline --quiet`
Expected: compilation error — module `minimap_baseline` doesn't exist.

- [ ] **Step 3: Create `minimap_baseline.rs`**

Create `src/observability/minimap_baseline.rs`:

```rust
/// Accumulates color masks from N frames and identifies consistently-present
/// pixels as static map fixtures (towers, buildings, camp markers).
pub struct BaselineMask {
    width: u32,
    height: u32,
    red_counts: Vec<u32>,
    green_counts: Vec<u32>,
    frames: u32,
    threshold: f32,
    built: bool,
    static_red: Vec<bool>,
    static_green: Vec<bool>,
}

impl BaselineMask {
    /// Create a new baseline accumulator for the given image dimensions.
    ///
    /// `threshold` is the fraction of frames a pixel must appear in to be
    /// marked as static (e.g., 0.8 = must appear in ≥80% of frames).
    pub fn new(width: u32, height: u32, threshold: f32) -> Self {
        let total = (width * height) as usize;
        Self {
            width,
            height,
            red_counts: vec![0; total],
            green_counts: vec![0; total],
            frames: 0,
            threshold,
            built: false,
            static_red: Vec::new(),
            static_green: Vec::new(),
        }
    }

    /// Feed one frame's red and green boolean masks into the accumulator.
    pub fn accumulate_frame(&mut self, red_mask: &[bool], green_mask: &[bool]) {
        let total = (self.width * self.height) as usize;
        for i in 0..total.min(red_mask.len()) {
            if red_mask[i] {
                self.red_counts[i] += 1;
            }
        }
        for i in 0..total.min(green_mask.len()) {
            if green_mask[i] {
                self.green_counts[i] += 1;
            }
        }
        self.frames += 1;
    }

    /// Finalize the baseline after all frames have been accumulated.
    ///
    /// Pixels appearing in ≥ `threshold` fraction of frames are marked static.
    pub fn build(&mut self) {
        if self.frames == 0 {
            self.built = true;
            return;
        }
        let cutoff = (self.frames as f32 * self.threshold) as u32;
        let total = (self.width * self.height) as usize;
        self.static_red = self.red_counts.iter().take(total).map(|&c| c >= cutoff).collect();
        self.static_green = self.green_counts.iter().take(total).map(|&c| c >= cutoff).collect();
        self.built = true;
    }

    /// Whether `build()` has been called.
    pub fn is_built(&self) -> bool {
        self.built
    }

    /// Whether pixel at `idx` is a static red element.
    pub fn is_static_red(&self, idx: usize) -> bool {
        self.static_red.get(idx).copied().unwrap_or(false)
    }

    /// Whether pixel at `idx` is a static green element.
    pub fn is_static_green(&self, idx: usize) -> bool {
        self.static_green.get(idx).copied().unwrap_or(false)
    }

    /// How many frames have been accumulated so far.
    pub fn frame_count(&self) -> u32 {
        self.frames
    }
}
```

- [ ] **Step 4: Export module in `mod.rs`**

Add to `src/observability/mod.rs`:

```rust
pub mod minimap_baseline;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test baseline --quiet`
Expected: 6 baseline tests pass.

- [ ] **Step 6: Commit**

```
git add src/observability/minimap_baseline.rs src/observability/mod.rs tests/minimap_analysis_tests.rs
git commit -m "feat: add baseline mask accumulator for static element filtering"
```

---

### Task 4: Hero Detection Pipeline

**Files:**
- Modify: `src/observability/minimap_analysis.rs`
- Modify: `tests/minimap_analysis_tests.rs`

This task adds the `detect_heroes()` orchestrator function that ties together color masks, baseline subtraction, clustering, and zone classification.

- [ ] **Step 1: Add detection pipeline tests**

Append to `tests/minimap_analysis_tests.rs`:

```rust
use dota2_scripts::observability::minimap_analysis::{detect_heroes, TeamColor};

#[test]
fn detect_heroes_finds_red_and_green_clusters() {
    // 20×20 RGBA image with a red cluster and a green cluster
    let width = 20u32;
    let height = 20u32;
    let mut pixels = vec![0u8; (width * height * 4) as usize];

    // Red 2×3 block at (2,2)-(3,4) → 6 pixels, center ≈ (2,3)
    for y in 2..=4 {
        for x in 2..=3 {
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx] = 200;
            pixels[idx + 1] = 40;
            pixels[idx + 2] = 40;
            pixels[idx + 3] = 255;
        }
    }

    // Green 2×3 block at (17,17)-(18,19) → 6 pixels, center ≈ (17,18)
    for y in 17..=19 {
        for x in 17..=18 {
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx] = 40;
            pixels[idx + 1] = 200;
            pixels[idx + 2] = 40;
            pixels[idx + 3] = 255;
        }
    }

    let thresholds = ColorThresholds {
        min_cluster_size: 3,
        max_cluster_size: 50,
        ..ColorThresholds::default()
    };

    let heroes = detect_heroes(&pixels, width, height, None, &thresholds);
    assert_eq!(heroes.len(), 2);

    let reds: Vec<_> = heroes.iter().filter(|h| h.team_color == TeamColor::Red).collect();
    let greens: Vec<_> = heroes
        .iter()
        .filter(|h| h.team_color == TeamColor::Green)
        .collect();
    assert_eq!(reds.len(), 1);
    assert_eq!(greens.len(), 1);
    assert_eq!(reds[0].cluster_size, 6);
    assert_eq!(greens[0].cluster_size, 6);
}

#[test]
fn detect_heroes_subtracts_baseline_static_elements() {
    // 10×10 image: red cluster at (0,0)-(1,1) is static (in baseline)
    let width = 10u32;
    let height = 10u32;
    let mut pixels = vec![0u8; (width * height * 4) as usize];

    // Red 2×2 block at (0,0)-(1,1)
    for y in 0..=1 {
        for x in 0..=1 {
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx] = 200;
            pixels[idx + 1] = 40;
            pixels[idx + 2] = 40;
            pixels[idx + 3] = 255;
        }
    }

    let thresholds = ColorThresholds {
        min_cluster_size: 3,
        max_cluster_size: 50,
        ..ColorThresholds::default()
    };

    // Without baseline: cluster is detected
    let heroes_no_bl = detect_heroes(&pixels, width, height, None, &thresholds);
    assert_eq!(heroes_no_bl.len(), 1);

    // Build a baseline that marks (0,0)-(1,1) as static red
    let mut bl = BaselineMask::new(width, height, 0.8);
    for _ in 0..10 {
        let (red, green) = build_color_masks(&pixels, width, height, &thresholds);
        bl.accumulate_frame(&red, &green);
    }
    bl.build();

    // With baseline: static cluster is subtracted
    let heroes_with_bl = detect_heroes(&pixels, width, height, Some(&bl), &thresholds);
    assert_eq!(heroes_with_bl.len(), 0);
}

#[test]
fn detect_heroes_maps_to_zones() {
    // 240×245 image (real minimap dimensions) with one cluster in Roshan area
    let width = 240u32;
    let height = 245u32;
    let mut pixels = vec![0u8; (width * height * 4) as usize];

    // 5×5 green block centered at (90,90): Roshan area
    for y in 88..=92 {
        for x in 88..=92 {
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx] = 40;
            pixels[idx + 1] = 200;
            pixels[idx + 2] = 40;
            pixels[idx + 3] = 255;
        }
    }

    let thresholds = ColorThresholds {
        min_cluster_size: 5,
        max_cluster_size: 200,
        ..ColorThresholds::default()
    };

    let heroes = detect_heroes(&pixels, width, height, None, &thresholds);
    assert_eq!(heroes.len(), 1);
    assert_eq!(heroes[0].zone, MapZone::Roshan);
    assert_eq!(heroes[0].team_color, TeamColor::Green);
}

#[test]
fn detect_heroes_empty_image_returns_empty() {
    let pixels = vec![0u8; 100 * 100 * 4]; // all black
    let heroes = detect_heroes(&pixels, 100, 100, None, &ColorThresholds::default());
    assert!(heroes.is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test detect_heroes --quiet`
Expected: compilation error — `detect_heroes` function doesn't exist.

- [ ] **Step 3: Add `detect_heroes()` to `minimap_analysis.rs`**

Add these `use` statements at the **top** of `src/observability/minimap_analysis.rs` (alongside the existing `use std::collections::VecDeque;`):

```rust
use crate::observability::minimap_baseline::BaselineMask;
use crate::observability::minimap_zones::{classify_zone, MapZone};
```

Then append the following types and function to the **bottom** of the file:
/// Which color team was detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeamColor {
    Red,
    Green,
}

impl std::fmt::Display for TeamColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TeamColor::Red => write!(f, "Red"),
            TeamColor::Green => write!(f, "Green"),
        }
    }
}

/// A detected hero position on the minimap.
#[derive(Debug, Clone)]
pub struct DetectedHero {
    pub x: u32,
    pub y: u32,
    pub zone: MapZone,
    pub team_color: TeamColor,
    pub cluster_size: usize,
}

/// Run the full hero detection pipeline on a captured RGBA frame.
///
/// 1. Build red/green color masks from pixel data.
/// 2. Subtract static baseline positions (if provided).
/// 3. Find connected clusters in the remaining masks.
/// 4. Map each cluster centroid to a map zone.
pub fn detect_heroes(
    pixels: &[u8],
    width: u32,
    height: u32,
    baseline: Option<&BaselineMask>,
    thresholds: &ColorThresholds,
) -> Vec<DetectedHero> {
    let (mut red_mask, mut green_mask) = build_color_masks(pixels, width, height, thresholds);

    if let Some(bl) = baseline {
        for i in 0..red_mask.len() {
            if bl.is_static_red(i) {
                red_mask[i] = false;
            }
            if bl.is_static_green(i) {
                green_mask[i] = false;
            }
        }
    }

    let red_clusters = find_clusters(
        &red_mask,
        width,
        height,
        thresholds.min_cluster_size,
        thresholds.max_cluster_size,
    );
    let green_clusters = find_clusters(
        &green_mask,
        width,
        height,
        thresholds.min_cluster_size,
        thresholds.max_cluster_size,
    );

    let mut heroes = Vec::new();
    for c in red_clusters {
        heroes.push(DetectedHero {
            x: c.center_x,
            y: c.center_y,
            zone: classify_zone(c.center_x, c.center_y, width, height),
            team_color: TeamColor::Red,
            cluster_size: c.size,
        });
    }
    for c in green_clusters {
        heroes.push(DetectedHero {
            x: c.center_x,
            y: c.center_y,
            zone: classify_zone(c.center_x, c.center_y, width, height),
            team_color: TeamColor::Green,
            cluster_size: c.size,
        });
    }
    heroes
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test detect_heroes --quiet`
Expected: 4 detection pipeline tests pass.

- [ ] **Step 5: Run the full test suite**

Run: `cargo test --quiet`
Expected: all tests pass (existing + 27 new analysis tests).

- [ ] **Step 6: Commit**

```
git add src/observability/minimap_analysis.rs tests/minimap_analysis_tests.rs
git commit -m "feat: add detect_heroes pipeline with baseline subtraction and zone mapping"
```

---

### Task 5: Config and GSI Enhancement

**Files:**
- Modify: `src/config/settings.rs`
- Modify: `config/config.toml`
- Modify: `src/models/gsi_event.rs`
- Modify: `tests/minimap_analysis_tests.rs`

- [ ] **Step 1: Add config and GSI tests**

Append to `tests/minimap_analysis_tests.rs`:

```rust
use dota2_scripts::config::MinimapAnalysisConfig;
use dota2_scripts::models::gsi_event::Player;

#[test]
fn minimap_analysis_config_defaults() {
    let config = MinimapAnalysisConfig::default();
    assert!(!config.enabled);
    assert_eq!(config.baseline_frames, 10);
    assert!((config.baseline_threshold - 0.8).abs() < 0.01);
    assert_eq!(config.min_cluster_size, 20);
    assert_eq!(config.max_cluster_size, 200);
    assert!((config.red_hue_max - 15.0).abs() < 0.01);
    assert!((config.green_hue_min - 80.0).abs() < 0.01);
}

#[test]
fn minimap_analysis_config_to_color_thresholds() {
    let config = MinimapAnalysisConfig::default();
    let t = config.to_color_thresholds();
    assert!((t.red_hue_max - 15.0).abs() < 0.01);
    assert!((t.red_hue_min_wrap - 340.0).abs() < 0.01);
    assert!((t.green_hue_min - 80.0).abs() < 0.01);
    assert!((t.green_hue_max - 160.0).abs() < 0.01);
    assert_eq!(t.min_cluster_size, 20);
    assert_eq!(t.max_cluster_size, 200);
}

#[test]
fn gsi_event_deserializes_without_player() {
    // Existing fixtures don't have a "player" field — deserialization must still work
    let json = std::fs::read_to_string("tests/fixtures/huskar_event.json").unwrap();
    let event: dota2_scripts::models::gsi_event::GsiWebhookEvent =
        serde_json::from_str(&json).unwrap();
    assert!(event.player.is_none());
}

#[test]
fn gsi_event_deserializes_with_player_team() {
    let json = r#"{
        "hero": {
            "aghanims_scepter": false, "aghanims_shard": false, "alive": true,
            "attributes_level": 0, "break": false, "buyback_cooldown": 0,
            "buyback_cost": 0, "disarmed": false, "facet": 0, "has_debuff": false,
            "health": 1000, "health_percent": 100, "hexed": false, "id": 1,
            "level": 1, "magicimmune": false, "mana": 500, "mana_percent": 100,
            "max_health": 1000, "max_mana": 500, "muted": false,
            "name": "npc_dota_hero_huskar", "respawn_seconds": 0, "silenced": false,
            "smoked": false, "stunned": false,
            "talent_1": false, "talent_2": false, "talent_3": false, "talent_4": false,
            "talent_5": false, "talent_6": false, "talent_7": false, "talent_8": false,
            "xp": 0, "xpos": 0, "ypos": 0
        },
        "abilities": {
            "ability0": {"ability_active":true,"can_cast":true,"cooldown":0,"level":1,"name":"huskar_inner_fire","passive":false,"ultimate":false},
            "ability1": {"ability_active":true,"can_cast":true,"cooldown":0,"level":0,"name":"huskar_burning_spear","passive":false,"ultimate":false},
            "ability2": {"ability_active":true,"can_cast":true,"cooldown":0,"level":0,"name":"huskar_berserkers_blood","passive":true,"ultimate":false},
            "ability3": {"ability_active":true,"can_cast":true,"cooldown":0,"level":0,"name":"huskar_inner_vitality","passive":false,"ultimate":false},
            "ability4": {"ability_active":true,"can_cast":true,"cooldown":0,"level":0,"name":"huskar_life_break","passive":false,"ultimate":true},
            "ability5": {"ability_active":true,"can_cast":true,"cooldown":0,"level":0,"name":"empty","passive":false,"ultimate":false}
        },
        "items": {
            "neutral0":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "slot0":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "slot1":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "slot2":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "slot3":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "slot4":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "slot5":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "slot6":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "slot7":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "slot8":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "stash0":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "stash1":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "stash2":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "stash3":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "stash4":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "stash5":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "teleport0":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null}
        },
        "map": {"clock_time": 0},
        "player": {"team_name": "dire"}
    }"#;

    let event: dota2_scripts::models::gsi_event::GsiWebhookEvent =
        serde_json::from_str(json).unwrap();
    let player = event.player.unwrap();
    assert_eq!(player.team_name.as_deref(), Some("dire"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test minimap_analysis_config --quiet && cargo test gsi_event_deserializes_with --quiet`
Expected: compilation errors — `MinimapAnalysisConfig` and `Player` don't exist.

- [ ] **Step 3: Add `Player` struct to `gsi_event.rs`**

Add before `GsiWebhookEvent` in `src/models/gsi_event.rs`:

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Player {
    pub team_name: Option<String>,
}
```

Add `player` field to `GsiWebhookEvent`:

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GsiWebhookEvent {
    pub hero: Hero,
    pub abilities: Abilities,
    pub items: Items,
    pub map: Map,
    #[serde(default)]
    pub player: Option<Player>,
}
```

- [ ] **Step 4: Add `MinimapAnalysisConfig` to `settings.rs`**

Add to `src/config/settings.rs`:

```rust
fn default_minimap_analysis_enabled() -> bool {
    false
}
fn default_baseline_frames() -> u32 {
    10
}
fn default_baseline_threshold() -> f32 {
    0.8
}
fn default_analysis_min_cluster_size() -> usize {
    20
}
fn default_analysis_max_cluster_size() -> usize {
    200
}
fn default_red_hue_max() -> f32 {
    15.0
}
fn default_red_hue_min_wrap() -> f32 {
    340.0
}
fn default_red_min_saturation() -> f32 {
    40.0
}
fn default_red_min_value() -> f32 {
    30.0
}
fn default_green_hue_min() -> f32 {
    80.0
}
fn default_green_hue_max() -> f32 {
    160.0
}
fn default_green_min_saturation() -> f32 {
    35.0
}
fn default_green_min_value() -> f32 {
    25.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinimapAnalysisConfig {
    #[serde(default = "default_minimap_analysis_enabled")]
    pub enabled: bool,
    #[serde(default = "default_baseline_frames")]
    pub baseline_frames: u32,
    #[serde(default = "default_baseline_threshold")]
    pub baseline_threshold: f32,
    #[serde(default = "default_analysis_min_cluster_size")]
    pub min_cluster_size: usize,
    #[serde(default = "default_analysis_max_cluster_size")]
    pub max_cluster_size: usize,
    #[serde(default = "default_red_hue_max")]
    pub red_hue_max: f32,
    #[serde(default = "default_red_hue_min_wrap")]
    pub red_hue_min_wrap: f32,
    #[serde(default = "default_red_min_saturation")]
    pub red_min_saturation: f32,
    #[serde(default = "default_red_min_value")]
    pub red_min_value: f32,
    #[serde(default = "default_green_hue_min")]
    pub green_hue_min: f32,
    #[serde(default = "default_green_hue_max")]
    pub green_hue_max: f32,
    #[serde(default = "default_green_min_saturation")]
    pub green_min_saturation: f32,
    #[serde(default = "default_green_min_value")]
    pub green_min_value: f32,
}

impl Default for MinimapAnalysisConfig {
    fn default() -> Self {
        Self {
            enabled: default_minimap_analysis_enabled(),
            baseline_frames: default_baseline_frames(),
            baseline_threshold: default_baseline_threshold(),
            min_cluster_size: default_analysis_min_cluster_size(),
            max_cluster_size: default_analysis_max_cluster_size(),
            red_hue_max: default_red_hue_max(),
            red_hue_min_wrap: default_red_hue_min_wrap(),
            red_min_saturation: default_red_min_saturation(),
            red_min_value: default_red_min_value(),
            green_hue_min: default_green_hue_min(),
            green_hue_max: default_green_hue_max(),
            green_min_saturation: default_green_min_saturation(),
            green_min_value: default_green_min_value(),
        }
    }
}

impl MinimapAnalysisConfig {
    /// Convert config values into the `ColorThresholds` used by the analysis engine.
    pub fn to_color_thresholds(&self) -> crate::observability::minimap_analysis::ColorThresholds {
        crate::observability::minimap_analysis::ColorThresholds {
            red_hue_max: self.red_hue_max,
            red_hue_min_wrap: self.red_hue_min_wrap,
            red_min_saturation: self.red_min_saturation,
            red_min_value: self.red_min_value,
            green_hue_min: self.green_hue_min,
            green_hue_max: self.green_hue_max,
            green_min_saturation: self.green_min_saturation,
            green_min_value: self.green_min_value,
            min_cluster_size: self.min_cluster_size,
            max_cluster_size: self.max_cluster_size,
        }
    }
}
```

Add the field to the `Settings` struct:

```rust
#[serde(default)]
pub minimap_analysis: MinimapAnalysisConfig,
```

- [ ] **Step 5: Export from `config/mod.rs`**

In `src/config/mod.rs`, add `MinimapAnalysisConfig` to the existing `pub use` list:

```rust
pub use settings::{
    AutoAbilityConfig, DangerDetectionConfig, MinimapAnalysisConfig, MinimapCaptureConfig,
    OutworldDestroyerConfig, RuneAlertConfig, Settings,
};
```

- [ ] **Step 6: Add `[minimap_analysis]` section to `config/config.toml`**

Append to `config/config.toml`:

```toml
[minimap_analysis]
enabled = false
baseline_frames = 10
baseline_threshold = 0.8
min_cluster_size = 20
max_cluster_size = 200
red_hue_max = 15.0
red_hue_min_wrap = 340.0
red_min_saturation = 40.0
red_min_value = 30.0
green_hue_min = 80.0
green_hue_max = 160.0
green_min_saturation = 35.0
green_min_value = 25.0
```

- [ ] **Step 7: Run tests to verify they pass**

Run: `cargo test minimap_analysis_config --quiet && cargo test gsi_event_deserializes --quiet`
Expected: all 4 config/GSI tests pass.

- [ ] **Step 8: Run the full test suite**

Run: `cargo test --quiet`
Expected: all tests pass (existing fixtures still deserialize with `player: None`).

- [ ] **Step 9: Commit**

```
git add src/config/settings.rs src/config/mod.rs config/config.toml src/models/gsi_event.rs tests/minimap_analysis_tests.rs
git commit -m "feat: add minimap analysis config and GSI player.team_name"
```

---

### Task 6: Standalone Analysis Example

**Files:**
- Create: `examples/minimap_analyze.rs`

- [ ] **Step 1: Create the standalone analysis example**

Create `examples/minimap_analyze.rs`:

```rust
//! Standalone minimap analysis utility.
//!
//! Loads PNG captures from a directory, optionally builds a baseline mask,
//! then runs hero detection on each frame and prints results.
//!
//! Usage:
//!   cargo run --example minimap_analyze -- --dir logs/minimap_capture
//!   cargo run --example minimap_analyze -- --dir logs/minimap_capture --baseline-frames 5
//!   cargo run --example minimap_analyze -- --dir logs/minimap_capture --min-cluster 10 --max-cluster 150

use dota2_scripts::observability::minimap_analysis::{
    build_color_masks, detect_heroes, ColorThresholds, TeamColor,
};
use dota2_scripts::observability::minimap_baseline::BaselineMask;
use std::env;
use std::path::Path;
use std::time::Instant;

fn main() {
    let args = Args::parse();

    println!("Minimap Analysis Utility");
    println!("  Directory: {}", args.dir);
    println!("  Baseline frames: {}", args.baseline_frames);
    println!(
        "  Cluster size: {}-{}",
        args.min_cluster, args.max_cluster
    );
    println!();

    // Collect PNG files sorted by name
    let mut png_files: Vec<_> = std::fs::read_dir(&args.dir)
        .unwrap_or_else(|e| {
            eprintln!("Error reading directory '{}': {}", args.dir, e);
            std::process::exit(1);
        })
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("png") {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    png_files.sort();

    if png_files.is_empty() {
        eprintln!("No PNG files found in '{}'", args.dir);
        std::process::exit(1);
    }
    println!("Found {} PNG files", png_files.len());

    let thresholds = ColorThresholds {
        min_cluster_size: args.min_cluster,
        max_cluster_size: args.max_cluster,
        ..ColorThresholds::default()
    };

    // Load all frames
    let mut frames: Vec<(String, Vec<u8>, u32, u32)> = Vec::new();
    for path in &png_files {
        let img = image::open(path).unwrap_or_else(|e| {
            eprintln!("Error loading {}: {}", path.display(), e);
            std::process::exit(1);
        });
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        frames.push((name, rgba.into_raw(), w, h));
    }

    // Build baseline from first N frames
    let baseline_count = (args.baseline_frames as usize).min(frames.len());
    let mut baseline: Option<BaselineMask> = None;
    if baseline_count > 0 && !frames.is_empty() {
        let (_, _, w, h) = &frames[0];
        let mut bl = BaselineMask::new(*w, *h, 0.8);
        println!("\nBuilding baseline from {} frames...", baseline_count);
        for (name, pixels, w, h) in frames.iter().take(baseline_count) {
            let (red, green) = build_color_masks(pixels, *w, *h, &thresholds);
            bl.accumulate_frame(&red, &green);
            println!("  Accumulated: {}", name);
        }
        bl.build();
        println!("Baseline built.\n");
        baseline = Some(bl);
    }

    // Analyze each frame
    println!("{:-<70}", "");
    for (name, pixels, w, h) in &frames {
        let start = Instant::now();
        let heroes = detect_heroes(pixels, *w, *h, baseline.as_ref(), &thresholds);
        let elapsed = start.elapsed();

        let red_count = heroes.iter().filter(|h| h.team_color == TeamColor::Red).count();
        let green_count = heroes
            .iter()
            .filter(|h| h.team_color == TeamColor::Green)
            .count();

        println!(
            "{}: {} heroes detected ({} red, {} green) [{:.1}ms]",
            name,
            heroes.len(),
            red_count,
            green_count,
            elapsed.as_secs_f64() * 1000.0
        );

        for hero in &heroes {
            println!(
                "  {} at ({},{}) → {} [{}px]",
                hero.team_color, hero.x, hero.y, hero.zone, hero.cluster_size
            );
        }
    }
    println!("{:-<70}", "");
}

struct Args {
    dir: String,
    baseline_frames: u32,
    min_cluster: usize,
    max_cluster: usize,
}

impl Args {
    fn parse() -> Self {
        let mut args = Self {
            dir: "logs/minimap_capture".to_string(),
            baseline_frames: 5,
            min_cluster: 20,
            max_cluster: 200,
        };
        let raw: Vec<String> = env::args().collect();
        let mut i = 1;
        while i < raw.len() {
            match raw[i].as_str() {
                "--dir" => {
                    i += 1;
                    if i >= raw.len() {
                        eprintln!("Error: --dir requires a value");
                        std::process::exit(1);
                    }
                    args.dir = raw[i].clone();
                }
                "--baseline-frames" => {
                    i += 1;
                    args.baseline_frames = parse_u32(&raw, i, "--baseline-frames");
                }
                "--min-cluster" => {
                    i += 1;
                    args.min_cluster = parse_usize(&raw, i, "--min-cluster");
                }
                "--max-cluster" => {
                    i += 1;
                    args.max_cluster = parse_usize(&raw, i, "--max-cluster");
                }
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                other => {
                    eprintln!("Unknown argument: {}", other);
                    print_usage();
                    std::process::exit(1);
                }
            }
            i += 1;
        }
        args
    }
}

fn parse_u32(raw: &[String], i: usize, flag: &str) -> u32 {
    if i >= raw.len() {
        eprintln!("Error: {} requires a value", flag);
        std::process::exit(1);
    }
    raw[i].parse().unwrap_or_else(|_| {
        eprintln!("Error: {} value '{}' is not a valid number", flag, raw[i]);
        std::process::exit(1);
    })
}

fn parse_usize(raw: &[String], i: usize, flag: &str) -> usize {
    if i >= raw.len() {
        eprintln!("Error: {} requires a value", flag);
        std::process::exit(1);
    }
    raw[i].parse().unwrap_or_else(|_| {
        eprintln!("Error: {} value '{}' is not a valid number", flag, raw[i]);
        std::process::exit(1);
    })
}

fn print_usage() {
    println!("Usage: cargo run --example minimap_analyze [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --dir <PATH>            PNG directory (default: logs/minimap_capture)");
    println!("  --baseline-frames <N>   Frames for baseline (default: 5)");
    println!("  --min-cluster <N>       Min cluster size (default: 20)");
    println!("  --max-cluster <N>       Max cluster size (default: 200)");
    println!("  --help, -h              Show this help");
}
```

- [ ] **Step 2: Verify the example compiles**

Run: `cargo build --example minimap_analyze --quiet`
Expected: compiles with no errors.

- [ ] **Step 3: Commit**

```
git add examples/minimap_analyze.rs
git commit -m "feat: add standalone minimap analysis example utility"
```

---

### Task 7: Documentation and Verification

**Files:**
- Modify: `docs/reference/file-index.md`
- Modify: `docs/reference/configuration.md`

- [ ] **Step 1: Update file-index.md**

Add entries for the new files in `docs/reference/file-index.md`:

```markdown
| `src/observability/minimap_zones.rs` | Map zone definitions and point-to-zone classification | `docs/superpowers/specs/2026-03-31-minimap-hero-detection-design.md` |
| `src/observability/minimap_analysis.rs` | HSV color segmentation, BFS clustering, hero detection pipeline | `docs/superpowers/specs/2026-03-31-minimap-hero-detection-design.md` |
| `src/observability/minimap_baseline.rs` | Static baseline mask accumulator for filtering map fixtures | `docs/superpowers/specs/2026-03-31-minimap-hero-detection-design.md` |
| `examples/minimap_analyze.rs` | Standalone CLI for running hero detection on PNG captures | `docs/superpowers/specs/2026-03-31-minimap-hero-detection-design.md` |
| `tests/minimap_analysis_tests.rs` | Tests for zone mapping, color analysis, clustering, baseline, detection | `docs/superpowers/specs/2026-03-31-minimap-hero-detection-design.md` |
```

- [ ] **Step 2: Update configuration.md**

Add a `Minimap Analysis` section to `docs/reference/configuration.md`:

```markdown
## Minimap Analysis

| Key | Type | Default | Description |
|---|---|---|---|
| `minimap_analysis.enabled` | bool | `false` | Enable hero detection analysis on captured frames |
| `minimap_analysis.baseline_frames` | u32 | `10` | Number of initial frames used to build the static baseline mask |
| `minimap_analysis.baseline_threshold` | f32 | `0.8` | Fraction of frames a pixel must appear in to be considered static |
| `minimap_analysis.min_cluster_size` | usize | `20` | Minimum pixel count for a cluster to be considered a hero icon |
| `minimap_analysis.max_cluster_size` | usize | `200` | Maximum pixel count (larger clusters are not hero icons) |
| `minimap_analysis.red_hue_max` | f32 | `15.0` | Upper bound of red hue range (0–15) |
| `minimap_analysis.red_hue_min_wrap` | f32 | `340.0` | Lower bound of red hue wrap range (340–360) |
| `minimap_analysis.red_min_saturation` | f32 | `40.0` | Minimum HSV saturation for red detection |
| `minimap_analysis.red_min_value` | f32 | `30.0` | Minimum HSV brightness for red detection |
| `minimap_analysis.green_hue_min` | f32 | `80.0` | Lower bound of green hue range |
| `minimap_analysis.green_hue_max` | f32 | `160.0` | Upper bound of green hue range |
| `minimap_analysis.green_min_saturation` | f32 | `35.0` | Minimum HSV saturation for green detection |
| `minimap_analysis.green_min_value` | f32 | `25.0` | Minimum HSV brightness for green detection |
```

- [ ] **Step 3: Run the full test suite**

Run: `cargo test --quiet`
Expected: all tests pass.

- [ ] **Step 4: Run a release build**

Run: `cargo build --release --quiet`
Expected: builds successfully.

- [ ] **Step 5: Commit**

```
git add docs/reference/file-index.md docs/reference/configuration.md
git commit -m "docs: add minimap analysis files to file-index and configuration reference"
```
