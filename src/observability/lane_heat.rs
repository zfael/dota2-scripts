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
