# Lane Heat Analysis Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Classify minimap hero detections into zone activity levels (Quiet/Active/Fight) and detect events (fights, rotations, groupings) across a rolling window of frames.

**Architecture:** Three layers — a pure per-frame zone classifier, a stateful rolling-window tracker, and enhanced CLI output in the existing `minimap_analyze` example. All in one new file `src/observability/lane_heat.rs` plus modifications to the example and test file.

**Tech Stack:** Rust standard library only (VecDeque, HashMap). No new crate dependencies.

**Spec:** `docs/superpowers/specs/2026-03-31-lane-heat-analysis-design.md`

---

### Task 1: Add Hash to MapZone and create lane_heat module with Layer 1 types + classifier

**Files:**
- Modify: `src/observability/minimap_zones.rs:3` (add `Hash` to derive)
- Create: `src/observability/lane_heat.rs`
- Modify: `src/observability/mod.rs` (add `pub mod lane_heat;`)
- Modify: `tests/minimap_analysis_tests.rs` (append Layer 1 tests)

**Why:** MapZone needs Hash so the tracker (Task 2) can use it as a HashMap key. Layer 1 is the pure per-frame classifier — all types and the `classify_zone_activity` function.

- [ ] **Step 1: Write Layer 1 tests**

Append to `tests/minimap_analysis_tests.rs`:

```rust
use dota2_scripts::observability::lane_heat::{
    classify_zone_activity, ActivityLevel, TeamSide, ZoneSnapshot,
};
use dota2_scripts::observability::minimap_analysis::TeamColor;

#[test]
fn team_side_from_dire() {
    let side = TeamSide::from_team_name("dire");
    assert_eq!(side.ally_color, TeamColor::Red);
    assert_eq!(side.enemy_color, TeamColor::Green);
}

#[test]
fn team_side_from_radiant() {
    let side = TeamSide::from_team_name("radiant");
    assert_eq!(side.ally_color, TeamColor::Green);
    assert_eq!(side.enemy_color, TeamColor::Red);
}

#[test]
fn classify_empty_heroes_returns_empty() {
    let side = TeamSide::from_team_name("dire");
    let result = classify_zone_activity(&[], &side);
    assert!(result.is_empty());
}

#[test]
fn classify_single_ally_in_zone() {
    use dota2_scripts::observability::minimap_analysis::DetectedHero;
    let side = TeamSide::from_team_name("dire"); // ally=Red
    let heroes = vec![DetectedHero {
        x: 10,
        y: 10,
        zone: MapZone::TopLane,
        team_color: TeamColor::Red,
        cluster_size: 30,
    }];
    let result = classify_zone_activity(&heroes, &side);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].zone, MapZone::TopLane);
    assert_eq!(result[0].ally_count, 1);
    assert_eq!(result[0].enemy_count, 0);
    assert_eq!(result[0].activity, ActivityLevel::Active);
}

#[test]
fn classify_fight_both_teams() {
    use dota2_scripts::observability::minimap_analysis::DetectedHero;
    let side = TeamSide::from_team_name("dire"); // ally=Red, enemy=Green
    let heroes = vec![
        DetectedHero {
            x: 10, y: 10, zone: MapZone::TopLane,
            team_color: TeamColor::Red, cluster_size: 30,
        },
        DetectedHero {
            x: 15, y: 15, zone: MapZone::TopLane,
            team_color: TeamColor::Green, cluster_size: 25,
        },
    ];
    let result = classify_zone_activity(&heroes, &side);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].zone, MapZone::TopLane);
    assert_eq!(result[0].ally_count, 1);
    assert_eq!(result[0].enemy_count, 1);
    assert_eq!(result[0].activity, ActivityLevel::Fight);
}

#[test]
fn classify_multiple_zones() {
    use dota2_scripts::observability::minimap_analysis::DetectedHero;
    let side = TeamSide::from_team_name("radiant"); // ally=Green, enemy=Red
    let heroes = vec![
        DetectedHero {
            x: 10, y: 10, zone: MapZone::TopLane,
            team_color: TeamColor::Green, cluster_size: 30,
        },
        DetectedHero {
            x: 12, y: 12, zone: MapZone::TopLane,
            team_color: TeamColor::Green, cluster_size: 28,
        },
        DetectedHero {
            x: 200, y: 200, zone: MapZone::BotLane,
            team_color: TeamColor::Red, cluster_size: 35,
        },
    ];
    let result = classify_zone_activity(&heroes, &side);
    assert_eq!(result.len(), 2);

    let top = result.iter().find(|s| s.zone == MapZone::TopLane).unwrap();
    assert_eq!(top.ally_count, 2);
    assert_eq!(top.enemy_count, 0);
    assert_eq!(top.activity, ActivityLevel::Active);

    let bot = result.iter().find(|s| s.zone == MapZone::BotLane).unwrap();
    assert_eq!(bot.ally_count, 0);
    assert_eq!(bot.enemy_count, 1);
    assert_eq!(bot.activity, ActivityLevel::Active);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test lane_heat -- --nocapture 2>&1 | head -20`
Expected: Compilation errors — `lane_heat` module doesn't exist yet.

- [ ] **Step 3: Add Hash to MapZone derive**

In `src/observability/minimap_zones.rs`, change line 3:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapZone {
```

- [ ] **Step 4: Create lane_heat.rs with Layer 1 implementation**

Create `src/observability/lane_heat.rs`:

```rust
use std::collections::HashMap;
use crate::observability::minimap_analysis::{DetectedHero, TeamColor};
use crate::observability::minimap_zones::MapZone;

/// Activity level of a map zone based on hero presence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityLevel {
    Quiet,
    Active,
    Fight,
}

impl std::fmt::Display for ActivityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActivityLevel::Quiet => write!(f, "Quiet"),
            ActivityLevel::Active => write!(f, "Active"),
            ActivityLevel::Fight => write!(f, "FIGHT"),
        }
    }
}

/// Which team the player is on, mapping team colors to ally/enemy.
#[derive(Debug, Clone)]
pub struct TeamSide {
    pub ally_color: TeamColor,
    pub enemy_color: TeamColor,
}

impl TeamSide {
    /// Create a TeamSide from a team name string.
    /// "dire" → ally=Red, enemy=Green (standard Dota 2 minimap colors).
    /// "radiant" → ally=Green, enemy=Red.
    /// Defaults to dire for unrecognized input.
    pub fn from_team_name(team: &str) -> Self {
        match team.to_lowercase().as_str() {
            "radiant" => Self {
                ally_color: TeamColor::Green,
                enemy_color: TeamColor::Red,
            },
            _ => Self {
                ally_color: TeamColor::Red,
                enemy_color: TeamColor::Green,
            },
        }
    }
}

/// Per-zone activity snapshot for a single frame.
#[derive(Debug, Clone)]
pub struct ZoneSnapshot {
    pub zone: MapZone,
    pub ally_count: u32,
    pub enemy_count: u32,
    pub activity: ActivityLevel,
}

/// Classify detected heroes into per-zone activity snapshots.
///
/// Returns only zones that contain at least one hero. Empty zones are
/// implicitly Quiet and omitted from the result.
pub fn classify_zone_activity(
    heroes: &[DetectedHero],
    team: &TeamSide,
) -> Vec<ZoneSnapshot> {
    if heroes.is_empty() {
        return Vec::new();
    }

    let mut zone_counts: HashMap<MapZone, (u32, u32)> = HashMap::new();
    for hero in heroes {
        let entry = zone_counts.entry(hero.zone).or_insert((0, 0));
        if hero.team_color == team.ally_color {
            entry.0 += 1;
        } else {
            entry.1 += 1;
        }
    }

    let mut snapshots: Vec<ZoneSnapshot> = zone_counts
        .into_iter()
        .map(|(zone, (ally, enemy))| {
            let activity = if ally > 0 && enemy > 0 {
                ActivityLevel::Fight
            } else {
                ActivityLevel::Active
            };
            ZoneSnapshot {
                zone,
                ally_count: ally,
                enemy_count: enemy,
                activity,
            }
        })
        .collect();

    // Sort by zone for deterministic output
    snapshots.sort_by_key(|s| format!("{:?}", s.zone));
    snapshots
}
```

- [ ] **Step 5: Add module export**

In `src/observability/mod.rs`, add:

```rust
pub mod lane_heat;
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test lane_heat -- --nocapture`
Expected: All 6 new tests pass.

- [ ] **Step 7: Run full test suite**

Run: `cargo test --quiet`
Expected: All tests pass (no regressions).

- [ ] **Step 8: Commit**

```bash
git add src/observability/minimap_zones.rs src/observability/lane_heat.rs src/observability/mod.rs tests/minimap_analysis_tests.rs
git commit -m "feat: add zone activity classifier with TeamSide and ActivityLevel"
```

---

### Task 2: Lane Heat Tracker (Layer 2 — rolling window, summary, events)

**Files:**
- Modify: `src/observability/lane_heat.rs` (append tracker types and impl)
- Modify: `tests/minimap_analysis_tests.rs` (append Layer 2 tests)

**Why:** The tracker is the stateful layer that detects meaningful transitions across frames. It depends on Layer 1 types from Task 1.

- [ ] **Step 1: Write Layer 2 tests**

Append to `tests/minimap_analysis_tests.rs`:

```rust
use dota2_scripts::observability::lane_heat::{
    LaneEvent, LaneHeatTracker, ZoneSummary,
};

#[test]
fn tracker_empty_summary() {
    let tracker = LaneHeatTracker::new(5);
    assert!(tracker.summary().is_empty());
    assert!(tracker.events().is_empty());
}

#[test]
fn tracker_single_frame_summary() {
    let mut tracker = LaneHeatTracker::new(5);
    let snapshots = vec![ZoneSnapshot {
        zone: MapZone::TopLane,
        ally_count: 2,
        enemy_count: 0,
        activity: ActivityLevel::Active,
    }];
    tracker.push_frame(snapshots);
    let summary = tracker.summary();
    assert_eq!(summary.len(), 1);
    assert_eq!(summary[0].zone, MapZone::TopLane);
    assert!((summary[0].avg_ally_count - 2.0).abs() < 0.01);
    assert!((summary[0].avg_enemy_count - 0.0).abs() < 0.01);
    assert_eq!(summary[0].peak_activity, ActivityLevel::Active);
    assert_eq!(summary[0].current_activity, ActivityLevel::Active);
    assert_eq!(summary[0].frames_with_fight, 0);
}

#[test]
fn tracker_fight_detected_event() {
    let mut tracker = LaneHeatTracker::new(5);
    // Frame 1: only allies in top
    tracker.push_frame(vec![ZoneSnapshot {
        zone: MapZone::TopLane,
        ally_count: 1,
        enemy_count: 0,
        activity: ActivityLevel::Active,
    }]);
    // Frame 2: fight begins in top
    tracker.push_frame(vec![ZoneSnapshot {
        zone: MapZone::TopLane,
        ally_count: 1,
        enemy_count: 1,
        activity: ActivityLevel::Fight,
    }]);
    let events = tracker.events();
    assert!(events.iter().any(|e| matches!(e, LaneEvent::FightDetected { zone } if *zone == MapZone::TopLane)));
}

#[test]
fn tracker_fight_ongoing_event() {
    let mut tracker = LaneHeatTracker::new(5);
    // Two consecutive fight frames
    tracker.push_frame(vec![ZoneSnapshot {
        zone: MapZone::MidLane,
        ally_count: 2,
        enemy_count: 2,
        activity: ActivityLevel::Fight,
    }]);
    tracker.push_frame(vec![ZoneSnapshot {
        zone: MapZone::MidLane,
        ally_count: 2,
        enemy_count: 2,
        activity: ActivityLevel::Fight,
    }]);
    let events = tracker.events();
    assert!(events.iter().any(|e| matches!(e, LaneEvent::FightOngoing { zone } if *zone == MapZone::MidLane)));
}

#[test]
fn tracker_enemy_rotation_event() {
    let mut tracker = LaneHeatTracker::new(5);
    // 3 frames with 0 enemies in bot
    for _ in 0..3 {
        tracker.push_frame(vec![ZoneSnapshot {
            zone: MapZone::BotLane,
            ally_count: 1,
            enemy_count: 0,
            activity: ActivityLevel::Active,
        }]);
    }
    // Frame 4: 3 enemies appear (increase of 3 vs avg 0 → rotation)
    tracker.push_frame(vec![ZoneSnapshot {
        zone: MapZone::BotLane,
        ally_count: 1,
        enemy_count: 3,
        activity: ActivityLevel::Fight,
    }]);
    let events = tracker.events();
    assert!(events.iter().any(|e| matches!(e, LaneEvent::EnemyRotation { zone } if *zone == MapZone::BotLane)));
}

#[test]
fn tracker_enemy_grouping_event() {
    let mut tracker = LaneHeatTracker::new(5);
    tracker.push_frame(vec![ZoneSnapshot {
        zone: MapZone::MidLane,
        ally_count: 0,
        enemy_count: 3,
        activity: ActivityLevel::Active,
    }]);
    let events = tracker.events();
    assert!(events.iter().any(|e| matches!(e, LaneEvent::EnemyGrouping { zone, count } if *zone == MapZone::MidLane && *count == 3)));
}

#[test]
fn tracker_window_evicts_old_frames() {
    let mut tracker = LaneHeatTracker::new(3);
    // Push 5 frames — only last 3 should be in window
    for i in 0..5 {
        tracker.push_frame(vec![ZoneSnapshot {
            zone: MapZone::TopLane,
            ally_count: i + 1,
            enemy_count: 0,
            activity: ActivityLevel::Active,
        }]);
    }
    let summary = tracker.summary();
    let top = summary.iter().find(|s| s.zone == MapZone::TopLane).unwrap();
    // Frames 3,4,5 (ally counts 3,4,5) → avg = 4.0
    assert!((top.avg_ally_count - 4.0).abs() < 0.01);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test tracker_ -- --nocapture 2>&1 | head -20`
Expected: Compilation errors — `LaneHeatTracker`, `LaneEvent`, `ZoneSummary` don't exist yet.

- [ ] **Step 3: Implement Layer 2 types and LaneHeatTracker**

Append to `src/observability/lane_heat.rs`:

```rust
use std::collections::VecDeque;

/// A detected lane event based on rolling-window analysis.
#[derive(Debug, Clone)]
pub enum LaneEvent {
    FightDetected { zone: MapZone },
    FightOngoing { zone: MapZone },
    EnemyRotation { zone: MapZone },
    EnemyGrouping { zone: MapZone, count: u32 },
}

impl std::fmt::Display for LaneEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LaneEvent::FightDetected { zone } => write!(f, "FIGHT DETECTED in {}", zone),
            LaneEvent::FightOngoing { zone } => write!(f, "FIGHT ONGOING in {}", zone),
            LaneEvent::EnemyRotation { zone } => write!(f, "ENEMY ROTATION to {}", zone),
            LaneEvent::EnemyGrouping { zone, count } => {
                write!(f, "ENEMY GROUPING in {} ({} enemies)", zone, count)
            }
        }
    }
}

/// Aggregated zone summary across the rolling window.
#[derive(Debug, Clone)]
pub struct ZoneSummary {
    pub zone: MapZone,
    pub avg_ally_count: f32,
    pub avg_enemy_count: f32,
    pub peak_activity: ActivityLevel,
    pub current_activity: ActivityLevel,
    pub frames_with_fight: u32,
}

/// Stateful rolling-window tracker for lane heat analysis.
pub struct LaneHeatTracker {
    window_size: usize,
    frames: VecDeque<Vec<ZoneSnapshot>>,
}

impl LaneHeatTracker {
    pub fn new(window_size: usize) -> Self {
        Self {
            window_size: window_size.max(1),
            frames: VecDeque::new(),
        }
    }

    pub fn push_frame(&mut self, snapshots: Vec<ZoneSnapshot>) {
        self.frames.push_back(snapshots);
        while self.frames.len() > self.window_size {
            self.frames.pop_front();
        }
    }

    pub fn summary(&self) -> Vec<ZoneSummary> {
        if self.frames.is_empty() {
            return Vec::new();
        }

        let mut zone_data: HashMap<MapZone, Vec<(u32, u32, ActivityLevel)>> = HashMap::new();

        for frame in &self.frames {
            // Track which zones appear in this frame
            let mut seen_zones: std::collections::HashSet<MapZone> = std::collections::HashSet::new();
            for snap in frame {
                seen_zones.insert(snap.zone);
                zone_data
                    .entry(snap.zone)
                    .or_default()
                    .push((snap.ally_count, snap.enemy_count, snap.activity));
            }
            // For zones NOT in this frame, record zeros (Quiet)
            for zone in zone_data.keys().cloned().collect::<Vec<_>>() {
                if !seen_zones.contains(&zone) {
                    zone_data
                        .entry(zone)
                        .or_default()
                        .push((0, 0, ActivityLevel::Quiet));
                }
            }
        }

        let frame_count = self.frames.len() as f32;
        let mut summaries: Vec<ZoneSummary> = zone_data
            .into_iter()
            .map(|(zone, data)| {
                let total_ally: u32 = data.iter().map(|(a, _, _)| a).sum();
                let total_enemy: u32 = data.iter().map(|(_, e, _)| e).sum();
                let peak_activity = data
                    .iter()
                    .map(|(_, _, act)| *act)
                    .max_by_key(|a| match a {
                        ActivityLevel::Quiet => 0,
                        ActivityLevel::Active => 1,
                        ActivityLevel::Fight => 2,
                    })
                    .unwrap_or(ActivityLevel::Quiet);

                let current_activity = self
                    .frames
                    .back()
                    .and_then(|last| last.iter().find(|s| s.zone == zone))
                    .map(|s| s.activity)
                    .unwrap_or(ActivityLevel::Quiet);

                let fights = data.iter().filter(|(_, _, act)| *act == ActivityLevel::Fight).count() as u32;

                ZoneSummary {
                    zone,
                    avg_ally_count: total_ally as f32 / frame_count,
                    avg_enemy_count: total_enemy as f32 / frame_count,
                    peak_activity,
                    current_activity,
                    frames_with_fight: fights,
                }
            })
            .collect();

        summaries.sort_by_key(|s| format!("{:?}", s.zone));
        summaries
    }

    pub fn events(&self) -> Vec<LaneEvent> {
        if self.frames.is_empty() {
            return Vec::new();
        }

        let latest = self.frames.back().unwrap();
        let previous = if self.frames.len() >= 2 {
            Some(&self.frames[self.frames.len() - 2])
        } else {
            None
        };

        let mut events = Vec::new();

        for snap in latest {
            // FightDetected: Fight in latest, wasn't fight in previous
            if snap.activity == ActivityLevel::Fight {
                let was_fight = previous
                    .and_then(|prev| prev.iter().find(|s| s.zone == snap.zone))
                    .map(|s| s.activity == ActivityLevel::Fight)
                    .unwrap_or(false);

                if was_fight {
                    events.push(LaneEvent::FightOngoing { zone: snap.zone });
                } else {
                    events.push(LaneEvent::FightDetected { zone: snap.zone });
                }
            }

            // EnemyGrouping: 3+ enemies in a single zone
            if snap.enemy_count >= 3 {
                events.push(LaneEvent::EnemyGrouping {
                    zone: snap.zone,
                    count: snap.enemy_count,
                });
            }

            // EnemyRotation: enemy count increased by 2+ vs window average
            if self.frames.len() >= 2 {
                let history_len = self.frames.len() - 1;
                let avg_enemy: f32 = self.frames
                    .iter()
                    .take(history_len) // exclude latest frame
                    .map(|frame| {
                        frame
                            .iter()
                            .find(|s| s.zone == snap.zone)
                            .map(|s| s.enemy_count)
                            .unwrap_or(0) as f32
                    })
                    .sum::<f32>()
                    / history_len as f32;

                if (snap.enemy_count as f32 - avg_enemy) >= 2.0 {
                    events.push(LaneEvent::EnemyRotation { zone: snap.zone });
                }
            }
        }

        events
    }
}
```

Note: The `VecDeque` import and `HashMap` import are already at the top of the file from Task 1. The implementer should place the `VecDeque` import alongside the existing `HashMap` import. Also add a `frame_count()` accessor:

```rust
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test lane_heat -- --nocapture && cargo test tracker_ -- --nocapture`
Expected: All 13 lane heat tests pass (6 from Task 1 + 7 from Task 2).

- [ ] **Step 5: Run full test suite**

Run: `cargo test --quiet`
Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/observability/lane_heat.rs tests/minimap_analysis_tests.rs
git commit -m "feat: add LaneHeatTracker with rolling window, summary, and event detection"
```

---

### Task 3: Enhanced standalone output (Layer 3 — CLI flags + zone activity + rolling summary)

**Files:**
- Modify: `examples/minimap_analyze.rs` (add `--team`, `--window` flags; zone activity + rolling summary output)

**Why:** This makes the lane heat analysis visible in the standalone tool so it can be tested on real captures.

- [ ] **Step 1: Add new imports to minimap_analyze.rs**

Add these imports at the top of `examples/minimap_analyze.rs`:

```rust
use dota2_scripts::observability::lane_heat::{
    classify_zone_activity, ActivityLevel, LaneHeatTracker, TeamSide,
};
```

- [ ] **Step 2: Add `--team` and `--window` fields to Args struct**

Update the `Args` struct and its `parse()` method:

```rust
struct Args {
    dir: String,
    baseline_frames: u32,
    min_cluster: usize,
    max_cluster: usize,
    team: String,
    window: usize,
}

impl Args {
    fn parse() -> Self {
        let mut args = Self {
            dir: "logs/minimap_capture".to_string(),
            baseline_frames: 5,
            min_cluster: 20,
            max_cluster: 200,
            team: "dire".to_string(),
            window: 5,
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
                "--team" => {
                    i += 1;
                    if i >= raw.len() {
                        eprintln!("Error: --team requires a value");
                        std::process::exit(1);
                    }
                    let val = raw[i].to_lowercase();
                    if val != "dire" && val != "radiant" {
                        eprintln!("Error: --team must be 'dire' or 'radiant'");
                        std::process::exit(1);
                    }
                    args.team = val;
                }
                "--window" => {
                    i += 1;
                    args.window = parse_usize(&raw, i, "--window");
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
```

- [ ] **Step 3: Update print_usage to include new flags**

```rust
fn print_usage() {
    println!("Usage: cargo run --example minimap_analyze [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --dir <PATH>            PNG directory (default: logs/minimap_capture)");
    println!("  --baseline-frames <N>   Frames for baseline (default: 5)");
    println!("  --min-cluster <N>       Min cluster size (default: 20)");
    println!("  --max-cluster <N>       Max cluster size (default: 200)");
    println!("  --team <dire|radiant>   Player's team (default: dire)");
    println!("  --window <N>            Rolling window size (default: 5)");
    println!("  --help, -h              Show this help");
}
```

- [ ] **Step 4: Add lane heat output to main analysis loop**

In the `main()` function, after `let thresholds = ...` and before the analysis loop, add tracker initialization:

```rust
    let team_side = TeamSide::from_team_name(&args.team);
    let mut tracker = LaneHeatTracker::new(args.window);
```

Update the startup banner to include new settings:

```rust
    println!("  Team: {} (ally={}, enemy={})", args.team, team_side.ally_color, team_side.enemy_color);
    println!("  Rolling window: {} frames", args.window);
```

Replace the per-frame analysis section (from `for (name, pixels, w, h) in &frames {` through the end of the loop) with:

```rust
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

        // Zone activity classification
        let snapshots = classify_zone_activity(&heroes, &team_side);
        if !snapshots.is_empty() {
            println!("  Zone Activity:");
            for snap in &snapshots {
                let detail = match snap.activity {
                    ActivityLevel::Fight => format!(
                        "{} ({} ally, {} enemy)",
                        snap.activity, snap.ally_count, snap.enemy_count
                    ),
                    _ => {
                        if snap.ally_count > 0 && snap.enemy_count > 0 {
                            format!(
                                "{} ({} ally, {} enemy)",
                                snap.activity, snap.ally_count, snap.enemy_count
                            )
                        } else if snap.ally_count > 0 {
                            format!("{} ({} ally)", snap.activity, snap.ally_count)
                        } else {
                            format!("{} ({} enemy)", snap.activity, snap.enemy_count)
                        }
                    }
                };
                println!("    {}: {}", snap.zone, detail);
            }
        }

        // Rolling window tracker
        tracker.push_frame(snapshots);
        let summary = tracker.summary();
        if !summary.is_empty() {
            println!("  Rolling Summary (last {} frames):", args.window);
            for s in &summary {
                let mut line = format!(
                    "    {}: avg {:.1} ally, {:.1} enemy | peak: {}",
                    s.zone, s.avg_ally_count, s.avg_enemy_count, s.peak_activity
                );
                if s.frames_with_fight > 0 {
                    line.push_str(&format!(
                        " | fight in {}/{} frames",
                        s.frames_with_fight,
                        tracker.frame_count()
                    ));
                }
                println!("{}", line);
            }
        }

        let events = tracker.events();
        if !events.is_empty() {
            println!("  ⚠ Events:");
            for event in &events {
                println!("    {}", event);
            }
        }

        println!();
    }
```

Use `tracker.frame_count()` (the method added in Task 2) for the fight-frames denominator in the rolling summary.

- [ ] **Step 5: Build the example to verify it compiles**

Run: `cargo build --example minimap_analyze`
Expected: Build succeeds.

- [ ] **Step 6: Run full test suite**

Run: `cargo test --quiet`
Expected: All tests pass.

- [ ] **Step 7: Commit**

```bash
git add examples/minimap_analyze.rs src/observability/lane_heat.rs
git commit -m "feat: enhance minimap_analyze with zone activity and rolling lane heat output"
```

---

### Task 4: Documentation and final verification

**Files:**
- Modify: `docs/reference/file-index.md` (add `lane_heat.rs` entry)
- Modify: `docs/reference/configuration.md` (no new config for this phase, but mention lane heat in minimap section)

- [ ] **Step 1: Add lane_heat.rs to file-index.md**

Add a row to the `src/observability/` section in `docs/reference/file-index.md`:

```markdown
| `src/observability/lane_heat.rs` | Zone activity classifier, rolling lane heat tracker, and event detection | `docs/superpowers/specs/2026-03-31-lane-heat-analysis-design.md` |
```

- [ ] **Step 2: Run full test suite**

Run: `cargo test --quiet`
Expected: All tests pass.

- [ ] **Step 3: Run release build**

Run: `cargo build --release --quiet`
Expected: Build succeeds.

- [ ] **Step 4: Commit**

```bash
git add docs/reference/file-index.md
git commit -m "docs: add lane heat module to file-index"
```
