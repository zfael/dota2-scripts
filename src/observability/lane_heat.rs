use std::collections::{HashMap, VecDeque};
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

/// Lane events detected from frame transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaneEvent {
    FightDetected { zone: MapZone },
    FightOngoing { zone: MapZone },
    EnemyRotation { zone: MapZone },
    EnemyGrouping { zone: MapZone, count: u32 },
}

impl std::fmt::Display for LaneEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LaneEvent::FightDetected { zone } => write!(f, "Fight detected in {:?}", zone),
            LaneEvent::FightOngoing { zone } => write!(f, "Fight ongoing in {:?}", zone),
            LaneEvent::EnemyRotation { zone } => write!(f, "Enemy rotation to {:?}", zone),
            LaneEvent::EnemyGrouping { zone, count } => {
                write!(f, "Enemy grouping in {:?} ({} heroes)", zone, count)
            }
        }
    }
}

/// Aggregated statistics for a zone across the rolling window.
#[derive(Debug, Clone)]
pub struct ZoneSummary {
    pub zone: MapZone,
    pub avg_ally_count: f32,
    pub avg_enemy_count: f32,
    pub peak_activity: ActivityLevel,
    pub current_activity: ActivityLevel,
    pub frames_with_fight: u32,
}

/// Stateful tracker that maintains a rolling window of zone snapshots
/// and detects meaningful transitions.
pub struct LaneHeatTracker {
    window_size: usize,
    frames: VecDeque<Vec<ZoneSnapshot>>,
}

impl LaneHeatTracker {
    /// Create a new tracker with the given window size (clamped to minimum 1).
    pub fn new(window_size: usize) -> Self {
        Self {
            window_size: window_size.max(1),
            frames: VecDeque::new(),
        }
    }

    /// Add a new frame of zone snapshots and trim the window to size.
    pub fn push_frame(&mut self, snapshots: Vec<ZoneSnapshot>) {
        self.frames.push_back(snapshots);
        while self.frames.len() > self.window_size {
            self.frames.pop_front();
        }
    }

    /// Get the number of frames currently stored.
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Generate summary statistics across the rolling window.
    ///
    /// Returns a summary for each zone that appears in at least one frame.
    /// Zones missing from some frames are treated as having 0 allies/enemies
    /// for those frames when computing averages.
    pub fn summary(&self) -> Vec<ZoneSummary> {
        if self.frames.is_empty() {
            return Vec::new();
        }

        // Collect all zones that appear in any frame
        let mut zone_data: HashMap<MapZone, Vec<(u32, u32, ActivityLevel)>> = HashMap::new();
        
        for frame in &self.frames {
            // Track which zones appear in this frame
            let mut zones_in_frame = HashMap::new();
            for snapshot in frame {
                zones_in_frame.insert(snapshot.zone, (snapshot.ally_count, snapshot.enemy_count, snapshot.activity));
            }

            // For all zones we've seen, add data for this frame (0s if missing)
            let existing_zones: Vec<MapZone> = zone_data.keys().copied().collect();
            for zone in existing_zones {
                if let Some(&(ally, enemy, activity)) = zones_in_frame.get(&zone) {
                    zone_data.get_mut(&zone).unwrap().push((ally, enemy, activity));
                } else {
                    zone_data.get_mut(&zone).unwrap().push((0, 0, ActivityLevel::Quiet));
                }
            }

            // Add new zones from this frame
            for (&zone, &(ally, enemy, activity)) in &zones_in_frame {
                zone_data.entry(zone).or_insert_with(|| {
                    // Backfill zeros for previous frames
                    let mut data = vec![(0, 0, ActivityLevel::Quiet); self.frames.len() - 1];
                    data.push((ally, enemy, activity));
                    data
                });
            }
        }

        let frame_count = self.frames.len();
        let latest_frame = self.frames.back().unwrap();

        let mut summaries: Vec<ZoneSummary> = zone_data
            .into_iter()
            .map(|(zone, data)| {
                let total_ally: u32 = data.iter().map(|(a, _, _)| a).sum();
                let total_enemy: u32 = data.iter().map(|(_, e, _)| e).sum();
                let avg_ally_count = total_ally as f32 / frame_count as f32;
                let avg_enemy_count = total_enemy as f32 / frame_count as f32;

                // Find peak activity (Fight > Active > Quiet)
                let peak_activity = data.iter()
                    .map(|(_, _, act)| act)
                    .max_by_key(|act| match act {
                        ActivityLevel::Fight => 2,
                        ActivityLevel::Active => 1,
                        ActivityLevel::Quiet => 0,
                    })
                    .copied()
                    .unwrap_or(ActivityLevel::Quiet);

                // Current activity from latest frame
                let current_activity = latest_frame
                    .iter()
                    .find(|s| s.zone == zone)
                    .map(|s| s.activity)
                    .unwrap_or(ActivityLevel::Quiet);

                // Count frames with fight
                let frames_with_fight = data.iter()
                    .filter(|(_, _, act)| matches!(act, ActivityLevel::Fight))
                    .count() as u32;

                ZoneSummary {
                    zone,
                    avg_ally_count,
                    avg_enemy_count,
                    peak_activity,
                    current_activity,
                    frames_with_fight,
                }
            })
            .collect();

        // Sort for deterministic output
        summaries.sort_by_key(|s| format!("{:?}", s.zone));
        summaries
    }

    /// Detect events by comparing the latest frame with previous frames.
    pub fn events(&self) -> Vec<LaneEvent> {
        if self.frames.is_empty() {
            return Vec::new();
        }

        let latest_frame = self.frames.back().unwrap();
        let mut events = Vec::new();

        // Build map for previous frame
        let prev_map: HashMap<MapZone, &ZoneSnapshot> = if self.frames.len() > 1 {
            self.frames[self.frames.len() - 2]
                .iter()
                .map(|s| (s.zone, s))
                .collect()
        } else {
            HashMap::new()
        };

        for snapshot in latest_frame {
            // FightDetected: Fight in latest, wasn't Fight in previous frame
            if snapshot.activity == ActivityLevel::Fight {
                let was_fight_before = prev_map
                    .get(&snapshot.zone)
                    .map(|s| s.activity == ActivityLevel::Fight)
                    .unwrap_or(false);

                if was_fight_before {
                    events.push(LaneEvent::FightOngoing { zone: snapshot.zone });
                } else {
                    events.push(LaneEvent::FightDetected { zone: snapshot.zone });
                }
            }

            // EnemyGrouping: 3+ enemies in a zone
            if snapshot.enemy_count >= 3 {
                events.push(LaneEvent::EnemyGrouping {
                    zone: snapshot.zone,
                    count: snapshot.enemy_count,
                });
            }

            // EnemyRotation: enemy_count increased by 2+ vs average of prior frames
            if self.frames.len() > 1 {
                let prior_frames = self.frames.iter().take(self.frames.len() - 1);
                let mut prior_enemy_counts = Vec::new();
                
                for frame in prior_frames {
                    if let Some(s) = frame.iter().find(|s| s.zone == snapshot.zone) {
                        prior_enemy_counts.push(s.enemy_count);
                    }
                }

                if !prior_enemy_counts.is_empty() {
                    let avg_prior_enemies = prior_enemy_counts.iter().sum::<u32>() as f32
                        / prior_enemy_counts.len() as f32;
                    
                    if snapshot.enemy_count as f32 >= avg_prior_enemies + 2.0 {
                        events.push(LaneEvent::EnemyRotation { zone: snapshot.zone });
                    }
                }
            }
        }

        events
    }
}
