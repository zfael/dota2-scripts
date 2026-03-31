# Lane Heat Analysis — Design Spec

**Status:** Draft  
**Depends on:** Minimap hero detection (Phase 1, complete)  
**Scope:** Pure analysis logic + enhanced standalone example output. No audio alerts, no live integration yet.

---

## Goal

Give the player visibility into lane activity across the map by analyzing hero positions detected on minimap captures. The standalone `minimap_analyze` tool prints per-frame zone breakdowns and a rolling summary that flags fights, rotations, and unusual groupings.

---

## Architecture

Three layers, all pure Rust with no new crate dependencies:

### Layer 1: Zone Activity Classifier (per-frame, pure function)

Takes `Vec<DetectedHero>` from `detect_heroes()` and produces a per-zone activity snapshot.

**Input:** `&[DetectedHero]`, team context (which color is ally)  
**Output:** `Vec<ZoneSnapshot>` — one entry per zone that has activity

```
ZoneSnapshot {
    zone: MapZone,
    ally_count: u32,
    enemy_count: u32,
    activity: ActivityLevel,
}
```

**ActivityLevel classification:**

| Condition | Level |
|---|---|
| No heroes detected in zone | `Quiet` |
| 1+ heroes from one team only | `Active` |
| Heroes from both teams present | `Fight` |

The classifier needs to know which `TeamColor` is the ally. This is provided as a `TeamSide` enum:

```
TeamSide {
    ally_color: TeamColor,
    enemy_color: TeamColor,
}
```

For the standalone tool, this is derived from the `--team` CLI flag:
- `--team dire` → Dire is conventionally red on the minimap, so ally=Red, enemy=Green
- `--team radiant` → ally=Green, enemy=Red
- (The live app will derive this from `Player.team_name` in the GSI event)

**Function signature:**
```rust
pub fn classify_zone_activity(
    heroes: &[DetectedHero],
    team: &TeamSide,
) -> Vec<ZoneSnapshot>
```

Returns only zones with at least one hero (empty zones are omitted, implicitly Quiet).

### Layer 2: Lane Heat Tracker (stateful, rolling window)

Maintains a sliding window of the last N frames of `ZoneSnapshot` data and detects meaningful events.

**State:**
```
LaneHeatTracker {
    window_size: usize,         // default: 5
    frames: VecDeque<Vec<ZoneSnapshot>>,  // rolling window
}
```

**Public API:**
- `new(window_size: usize) -> Self`
- `push_frame(&mut self, snapshots: Vec<ZoneSnapshot>)` — adds a frame, trims to window
- `summary(&self) -> Vec<ZoneSummary>` — aggregated view across the window
- `events(&self) -> Vec<LaneEvent>` — detected events in the latest frame vs. recent history

**ZoneSummary** (aggregated across window):
```
ZoneSummary {
    zone: MapZone,
    avg_ally_count: f32,
    avg_enemy_count: f32,
    peak_activity: ActivityLevel,    // highest seen in window
    current_activity: ActivityLevel, // latest frame
    frames_with_fight: u32,          // how many window frames had Fight
}
```

**LaneEvent** detection rules:

| Event | Condition |
|---|---|
| `FightDetected { zone }` | Both teams present in zone in latest frame, AND this is a new fight (wasn't Fight in previous frame) |
| `FightOngoing { zone }` | Both teams present for 2+ consecutive frames |
| `EnemyRotation { zone }` | Enemy count in zone increased by 2+ compared to the window average |
| `EnemyGrouping { zone, count }` | 3+ enemies in a single zone |

Events are computed by comparing the latest frame against the rolling window history.

### Layer 3: Enhanced Standalone Output

The existing `minimap_analyze` example gets two new sections in its output:

**Per-frame output** (already shows hero positions, now adds zone activity):
```
minimap_001.png: 4 heroes detected (2 red, 2 green) [1.2ms]
  Red at (50,30) → Top Lane [45px]
  Red at (120,120) → Mid Lane [38px]
  Green at (55,35) → Top Lane [42px]
  Green at (200,200) → Bot Lane [35px]
  Zone Activity:
    Top Lane: FIGHT (1 ally, 1 enemy)
    Mid Lane: Active (1 ally)
    Bot Lane: Active (1 enemy)
```

**Rolling summary** (printed after each frame, shows tracker state):
```
  Rolling Summary (last 5 frames):
    Top Lane: avg 1.2 ally, 0.8 enemy | peak: Fight | fight in 3/5 frames
    Mid Lane: avg 1.0 ally, 0.0 enemy | peak: Active
    Bot Lane: avg 0.4 ally, 1.2 enemy | peak: Active
  ⚠ Events:
    FIGHT DETECTED in Top Lane
    ENEMY GROUPING in Bot Lane (3 enemies)
```

**New CLI flags:**
- `--team <dire|radiant>` — which team the player is on (required for ally/enemy labeling; defaults to `dire`)
- `--window <N>` — rolling window size (default: 5)

---

## File Structure

| File | Action | Purpose |
|---|---|---|
| `src/observability/lane_heat.rs` | Create | `TeamSide`, `ZoneSnapshot`, `ActivityLevel`, `classify_zone_activity()`, `LaneHeatTracker`, `ZoneSummary`, `LaneEvent` |
| `src/observability/mod.rs` | Modify | Export `lane_heat` module |
| `examples/minimap_analyze.rs` | Modify | Add zone activity + rolling summary output, `--team` and `--window` flags |
| `tests/minimap_analysis_tests.rs` | Modify | Append lane heat tests |

---

## Types Reference

```rust
// --- Layer 1: Per-frame classification ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityLevel {
    Quiet,
    Active,
    Fight,
}

impl std::fmt::Display for ActivityLevel { ... }

#[derive(Debug, Clone)]
pub struct TeamSide {
    pub ally_color: TeamColor,
    pub enemy_color: TeamColor,
}

impl TeamSide {
    pub fn from_team_name(team: &str) -> Self { ... }
    // "dire" → ally=Red, enemy=Green
    // "radiant" → ally=Green, enemy=Red
}

#[derive(Debug, Clone)]
pub struct ZoneSnapshot {
    pub zone: MapZone,
    pub ally_count: u32,
    pub enemy_count: u32,
    pub activity: ActivityLevel,
}

pub fn classify_zone_activity(
    heroes: &[DetectedHero],
    team: &TeamSide,
) -> Vec<ZoneSnapshot>;

// --- Layer 2: Rolling tracker ---

#[derive(Debug, Clone)]
pub enum LaneEvent {
    FightDetected { zone: MapZone },
    FightOngoing { zone: MapZone },
    EnemyRotation { zone: MapZone },
    EnemyGrouping { zone: MapZone, count: u32 },
}

impl std::fmt::Display for LaneEvent { ... }

#[derive(Debug, Clone)]
pub struct ZoneSummary {
    pub zone: MapZone,
    pub avg_ally_count: f32,
    pub avg_enemy_count: f32,
    pub peak_activity: ActivityLevel,
    pub current_activity: ActivityLevel,
    pub frames_with_fight: u32,
}

pub struct LaneHeatTracker {
    window_size: usize,
    frames: VecDeque<Vec<ZoneSnapshot>>,
}

impl LaneHeatTracker {
    pub fn new(window_size: usize) -> Self;
    pub fn push_frame(&mut self, snapshots: Vec<ZoneSnapshot>);
    pub fn summary(&self) -> Vec<ZoneSummary>;
    pub fn events(&self) -> Vec<LaneEvent>;
}
```

---

## Test Plan

**Layer 1 tests** (zone classification):
- `classify_empty_heroes_returns_empty` — no heroes → no snapshots
- `classify_single_ally_in_zone` — 1 ally in Top → Active
- `classify_fight_both_teams` — ally + enemy in same zone → Fight
- `classify_multiple_zones` — heroes spread across zones → correct per-zone counts
- `team_side_from_dire` — "dire" → ally=Red, enemy=Green
- `team_side_from_radiant` — "radiant" → ally=Green, enemy=Red

**Layer 2 tests** (rolling tracker):
- `tracker_empty_summary` — no frames → empty summary
- `tracker_single_frame_summary` — one frame → summary matches frame
- `tracker_fight_detected_event` — fight appears in new frame → FightDetected event
- `tracker_fight_ongoing_event` — fight persists 2+ frames → FightOngoing
- `tracker_enemy_rotation_event` — enemy count jumps → EnemyRotation
- `tracker_enemy_grouping_event` — 3+ enemies in zone → EnemyGrouping
- `tracker_window_evicts_old_frames` — window size respected

---

## Design Decisions

1. **No audio alerts yet.** This phase is about validating detection quality on real captures. Audio integration follows once we're confident the data is reliable.

2. **Dire=Red, Radiant=Green convention.** This is the standard Dota 2 minimap coloring. If a user's game uses custom colors, they can't use this tool correctly — acceptable for now.

3. **Events compare latest frame vs. history.** This means the first N frames (while the window fills) will have sparse events. That's expected — the tool is for evaluating multi-frame sequences, not single-frame accuracy.

4. **Only zones with heroes are returned.** Quiet zones are implicit (not in the list). This keeps the output clean and avoids 7 entries per frame when most zones are empty.

5. **`Other` zone is included in classification** but unlikely to trigger events (hero icons at map edges or in unclassified areas). No special handling needed.

6. **EnemyRotation threshold is 2+ increase** over average. A single hero appearing could be noise; 2+ is a meaningful signal.

---

## Out of Scope

- Audio alerts (future phase)
- Live integration with capture worker (future phase)
- UI panel display (future phase)
- Camp detection / farming routes (separate feature)
- Persisting lane heat data to disk
