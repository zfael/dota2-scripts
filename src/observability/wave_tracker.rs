//! Clock-driven creep wave prediction.
//!
//! Creep waves spawn on a fixed 30-second cadence, so their approximate position
//! can be derived from the game clock alone with no game-state access. This module
//! is pure arithmetic over `clock_time` — it performs no I/O and holds no state.
//!
//! # Accuracy
//!
//! Positional precision is an explicit non-goal. The model assumes both waves in a
//! lane are alive and undisrupted, which holds during laning and breaks down once a
//! wave is killed, eaten by a tower, or the lane settles off-centre. Spawn countdowns
//! stay exact regardless (pure clock arithmetic); dot positions degrade, which is what
//! [`Confidence`] communicates to the renderer.
//!
//! # Coordinate space
//!
//! Map points are normalised to `[0.0, 1.0]` with the origin at the **bottom-left**
//! (Radiant corner); `(1.0, 1.0)` is the top-right (Dire corner). Renderers flip the
//! y-axis as needed for screen space.
//!
//! Lane progress is also normalised: `0.0` is the Radiant barracks end of the lane,
//! `1.0` the Dire barracks end, regardless of which team's wave is being tracked.

use crate::config::WaveTrackerConfig;

/// Seconds between creep wave spawns.
pub const SPAWN_INTERVAL_SECONDS: i32 = 30;

/// A point in normalised map space. Origin is bottom-left (Radiant corner).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapPoint {
    pub x: f32,
    pub y: f32,
}

/// The three creep lanes.
///
/// Named by map position rather than by role, since role depends on team:
/// `Bottom` is the Radiant safelane and the Dire offlane, and vice versa for `Top`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lane {
    Top,
    Mid,
    Bottom,
}

impl Lane {
    pub const ALL: [Lane; 3] = [Lane::Top, Lane::Mid, Lane::Bottom];

    /// Waypoints running from the Radiant barracks end to the Dire barracks end.
    ///
    /// Approximations fitted to in-game tower positions rather than derived from
    /// game geometry. Tuning them is expected — but tune them in *map* space. How
    /// map space lands on Dota's minimap panel is a separate, per-resolution
    /// concern that belongs in `WaveOverlayConfig::map_offset_*` / `map_scale_*`;
    /// bending these to compensate for a placement error would make the in-app
    /// panel wrong to make the overlay right.
    ///
    /// Public so renderers can draw the lanes from the same geometry the model
    /// interpolates along, keeping the two from drifting apart.
    pub fn waypoints(self) -> &'static [MapPoint] {
        match self {
            Lane::Top => &TOP_LANE_PATH,
            Lane::Mid => &MID_LANE_PATH,
            Lane::Bottom => &BOTTOM_LANE_PATH,
        }
    }
}

impl std::fmt::Display for Lane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Lane::Top => write!(f, "Top"),
            Lane::Mid => write!(f, "Mid"),
            Lane::Bottom => write!(f, "Bottom"),
        }
    }
}

/// Which team a wave belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Team {
    Radiant,
    Dire,
}

impl Team {
    pub const ALL: [Team; 2] = [Team::Radiant, Team::Dire];
}

impl std::fmt::Display for Team {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Team::Radiant => write!(f, "Radiant"),
            Team::Dire => write!(f, "Dire"),
        }
    }
}

/// How much the renderer should trust a predicted position.
///
/// Decays with game time: the undisrupted-wave assumption holds well in laning and
/// progressively less well afterwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
    High,
    Degrading,
    Low,
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Confidence::High => write!(f, "High"),
            Confidence::Degrading => write!(f, "Degrading"),
            Confidence::Low => write!(f, "Low"),
        }
    }
}

/// A single wave's predicted position at one instant.
#[derive(Debug, Clone, PartialEq)]
pub struct WavePosition {
    pub lane: Lane,
    pub team: Team,
    /// Normalised position along the lane path: 0.0 = Radiant barracks, 1.0 = Dire barracks.
    pub progress: f32,
    /// `progress` resolved to normalised map coordinates.
    pub point: MapPoint,
    /// True once the wave has reached the clash point and stopped advancing.
    pub has_clashed: bool,
}

/// Where and when the current wave pair meets in a lane.
#[derive(Debug, Clone, PartialEq)]
pub struct LaneClash {
    pub lane: Lane,
    /// Normalised position along the lane path where the waves meet.
    pub progress: f32,
    pub point: MapPoint,
    /// Seconds until the current wave pair meets; 0 once they already have.
    pub seconds_until_clash: i32,
}

/// Complete wave picture for one instant.
#[derive(Debug, Clone, PartialEq)]
pub struct WaveSnapshot {
    /// The clock value this snapshot was built from; may be fractional.
    pub clock_time_seconds: f32,
    /// Game-clock time of the next spawn.
    pub next_spawn_time_seconds: i32,
    pub seconds_until_next_spawn: i32,
    /// Seconds since the wave currently in flight spawned. `None` before the first spawn.
    pub current_wave_age_seconds: Option<f32>,
    pub confidence: Confidence,
    /// One entry per lane per team (6 total), empty before the first spawn.
    pub waves: Vec<WavePosition>,
    /// One entry per lane, empty before the first spawn.
    pub clashes: Vec<LaneClash>,
}

// Lane paths, Radiant end first. See module docs for the coordinate space.
//
// Fitted to tower positions measured off an in-game minimap: towers sit on the
// lane, so their centres are the only landmarks precise enough to calibrate
// against. The ring they trace runs at 0.12 / 0.88 on each axis.
//
// The map is symmetric under a 180° rotation, which swaps the two teams and maps
// the top lane onto the bottom lane. `BOTTOM_LANE_PATH` is therefore the reverse
// complement of `TOP_LANE_PATH` rather than an independent fit — see
// `side_lanes_are_rotations_of_one_another`, which is what stops the two drifting
// apart the next time these are retuned.

const TOP_LANE_PATH: [MapPoint; 8] = [
    MapPoint { x: 0.150, y: 0.150 },
    MapPoint { x: 0.120, y: 0.300 },
    MapPoint { x: 0.120, y: 0.560 },
    MapPoint { x: 0.150, y: 0.720 },
    MapPoint { x: 0.230, y: 0.830 },
    MapPoint { x: 0.400, y: 0.877 },
    MapPoint { x: 0.640, y: 0.880 },
    MapPoint { x: 0.850, y: 0.850 },
];

const MID_LANE_PATH: [MapPoint; 5] = [
    MapPoint { x: 0.150, y: 0.150 },
    MapPoint { x: 0.325, y: 0.325 },
    MapPoint { x: 0.500, y: 0.500 },
    MapPoint { x: 0.675, y: 0.675 },
    MapPoint { x: 0.850, y: 0.850 },
];

const BOTTOM_LANE_PATH: [MapPoint; 8] = [
    MapPoint { x: 0.150, y: 0.150 },
    MapPoint { x: 0.360, y: 0.120 },
    MapPoint { x: 0.600, y: 0.123 },
    MapPoint { x: 0.770, y: 0.170 },
    MapPoint { x: 0.850, y: 0.280 },
    MapPoint { x: 0.880, y: 0.440 },
    MapPoint { x: 0.880, y: 0.700 },
    MapPoint { x: 0.850, y: 0.850 },
];

/// Game-clock time of the next creep spawn.
///
/// Before the horn (`clock_time < 0`) the next spawn is the first one, at 0:00.
/// Exactly on a spawn tick this returns the *following* spawn, matching the
/// "strictly next" convention used by `rune_alerts`.
pub fn next_spawn_time_seconds(clock_time_seconds: i32) -> i32 {
    if clock_time_seconds < 0 {
        return 0;
    }
    clock_time_seconds - clock_time_seconds.rem_euclid(SPAWN_INTERVAL_SECONDS) + SPAWN_INTERVAL_SECONDS
}

/// Seconds remaining until the next creep spawn.
pub fn seconds_until_next_spawn(clock_time_seconds: i32) -> i32 {
    next_spawn_time_seconds(clock_time_seconds) - clock_time_seconds
}

/// Age of the wave currently in flight, or `None` before the first spawn.
///
/// Fractional, so callers passing an interpolated clock get smooth motion.
fn current_wave_age_seconds(clock_time_seconds: f32) -> Option<f32> {
    if clock_time_seconds < 0.0 {
        None
    } else {
        Some(clock_time_seconds.rem_euclid(SPAWN_INTERVAL_SECONDS as f32))
    }
}

/// Resolve a normalised lane progress to normalised map coordinates.
///
/// Walks the lane polyline by cumulative segment length. `progress` is clamped to
/// `[0.0, 1.0]`.
pub fn point_at(lane: Lane, progress: f32) -> MapPoint {
    let waypoints = lane.waypoints();
    let progress = progress.clamp(0.0, 1.0);

    let segment_lengths: Vec<f32> = waypoints
        .windows(2)
        .map(|pair| {
            let dx = pair[1].x - pair[0].x;
            let dy = pair[1].y - pair[0].y;
            (dx * dx + dy * dy).sqrt()
        })
        .collect();

    let total: f32 = segment_lengths.iter().sum();
    if total <= f32::EPSILON {
        return waypoints[0];
    }

    let mut remaining = progress * total;
    for (index, &length) in segment_lengths.iter().enumerate() {
        if remaining <= length || index == segment_lengths.len() - 1 {
            let fraction = if length <= f32::EPSILON {
                0.0
            } else {
                (remaining / length).clamp(0.0, 1.0)
            };
            let start = waypoints[index];
            let end = waypoints[index + 1];
            return MapPoint {
                x: start.x + (end.x - start.x) * fraction,
                y: start.y + (end.y - start.y) * fraction,
            };
        }
        remaining -= length;
    }

    waypoints[waypoints.len() - 1]
}

/// Per-lane calibration pulled from config.
struct LaneTiming {
    /// Seconds after spawn at which the two waves meet.
    meet_seconds: f32,
    /// Normalised lane position where they meet. 0.5 is the exact midpoint;
    /// side lanes are biased toward the offlane tower.
    meet_progress: f32,
}

fn lane_timing(lane: Lane, config: &WaveTrackerConfig) -> LaneTiming {
    match lane {
        Lane::Mid => LaneTiming {
            meet_seconds: config.mid_meet_seconds,
            meet_progress: config.mid_meet_progress,
        },
        Lane::Top => LaneTiming {
            meet_seconds: config.side_meet_seconds,
            meet_progress: config.top_meet_progress,
        },
        Lane::Bottom => LaneTiming {
            meet_seconds: config.side_meet_seconds,
            meet_progress: config.bottom_meet_progress,
        },
    }
}

/// Predicted lane progress for one team's wave at a given age.
///
/// Both waves advance linearly from their own barracks and stop at the clash point,
/// which is why the two teams use different rates when `meet_progress != 0.5`: they
/// cover different distances in the same time.
fn wave_progress(team: Team, age_seconds: f32, timing: &LaneTiming) -> (f32, bool) {
    let meet_progress = timing.meet_progress.clamp(0.0, 1.0);

    if timing.meet_seconds <= 0.0 {
        return (meet_progress, true);
    }

    let travel_fraction = (age_seconds / timing.meet_seconds).clamp(0.0, 1.0);
    let has_clashed = age_seconds >= timing.meet_seconds;

    let progress = match team {
        Team::Radiant => meet_progress * travel_fraction,
        Team::Dire => 1.0 - (1.0 - meet_progress) * travel_fraction,
    };

    (progress, has_clashed)
}

fn confidence_at(clock_time_seconds: f32, config: &WaveTrackerConfig) -> Confidence {
    if clock_time_seconds < config.confidence_high_seconds as f32 {
        Confidence::High
    } else if clock_time_seconds < config.confidence_degrading_seconds as f32 {
        Confidence::Degrading
    } else {
        Confidence::Low
    }
}

/// Build the complete wave picture for a game-clock instant.
///
/// `clock_time_seconds` may be fractional: pass an interpolated clock for smooth
/// animation, or the raw GSI value for exact readouts. Countdown fields are always
/// whole seconds, derived from the floored clock.
///
/// `config.enabled` suppresses the *predictions* — positions and clashes — because
/// those are the part that can be wrong. The spawn countdown is plain clock
/// arithmetic with nothing to disbelieve, so it keeps reporting either way.
pub fn wave_snapshot(clock_time_seconds: f32, config: &WaveTrackerConfig) -> WaveSnapshot {
    let whole_clock = clock_time_seconds.floor() as i32;
    let age = current_wave_age_seconds(clock_time_seconds);
    let confidence = confidence_at(clock_time_seconds, config);

    let mut waves = Vec::new();
    let mut clashes = Vec::new();

    if let Some(age_seconds) = age.filter(|_| config.enabled) {
        for lane in Lane::ALL {
            let timing = lane_timing(lane, config);

            for team in Team::ALL {
                let (progress, has_clashed) = wave_progress(team, age_seconds, &timing);
                waves.push(WavePosition {
                    lane,
                    team,
                    progress,
                    point: point_at(lane, progress),
                    has_clashed,
                });
            }

            let meet_progress = timing.meet_progress.clamp(0.0, 1.0);
            // Ceil so the countdown never reads zero before the waves actually meet.
            let seconds_until_clash = (timing.meet_seconds - age_seconds).max(0.0).ceil() as i32;
            clashes.push(LaneClash {
                lane,
                progress: meet_progress,
                point: point_at(lane, meet_progress),
                seconds_until_clash,
            });
        }
    }

    WaveSnapshot {
        clock_time_seconds,
        next_spawn_time_seconds: next_spawn_time_seconds(whole_clock),
        seconds_until_next_spawn: seconds_until_next_spawn(whole_clock),
        current_wave_age_seconds: age,
        confidence,
        waves,
        clashes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> WaveTrackerConfig {
        WaveTrackerConfig::default()
    }

    fn assert_close(actual: f32, expected: f32, label: &str) {
        assert!(
            (actual - expected).abs() < 0.001,
            "{label}: expected {expected}, got {actual}"
        );
    }

    fn wave(snapshot: &WaveSnapshot, lane: Lane, team: Team) -> WavePosition {
        snapshot
            .waves
            .iter()
            .find(|w| w.lane == lane && w.team == team)
            .unwrap_or_else(|| panic!("no wave for {lane}/{team}"))
            .clone()
    }

    #[test]
    fn disabling_tracking_drops_predictions_but_keeps_the_spawn_countdown() {
        let disabled = WaveTrackerConfig {
            enabled: false,
            ..config()
        };
        let snapshot = wave_snapshot(100.0, &disabled);

        assert!(snapshot.waves.is_empty(), "predicted positions are opinions");
        assert!(snapshot.clashes.is_empty());
        // The cadence is arithmetic, not prediction — it stays useful either way.
        assert_eq!(snapshot.next_spawn_time_seconds, 120);
        assert_eq!(snapshot.seconds_until_next_spawn, 20);
    }

    #[test]
    fn next_spawn_lands_on_thirty_second_boundaries() {
        assert_eq!(next_spawn_time_seconds(0), 30);
        assert_eq!(next_spawn_time_seconds(1), 30);
        assert_eq!(next_spawn_time_seconds(29), 30);
        assert_eq!(next_spawn_time_seconds(30), 60);
        assert_eq!(next_spawn_time_seconds(31), 60);
        assert_eq!(next_spawn_time_seconds(599), 600);
    }

    #[test]
    fn next_spawn_before_the_horn_is_the_first_wave() {
        assert_eq!(next_spawn_time_seconds(-90), 0);
        assert_eq!(seconds_until_next_spawn(-90), 90);
    }

    #[test]
    fn seconds_until_next_spawn_counts_down_within_the_cycle() {
        assert_eq!(seconds_until_next_spawn(0), 30);
        assert_eq!(seconds_until_next_spawn(10), 20);
        assert_eq!(seconds_until_next_spawn(29), 1);
        assert_eq!(seconds_until_next_spawn(30), 30);
    }

    #[test]
    fn no_waves_exist_before_the_horn() {
        let snapshot = wave_snapshot(-30.0, &config());
        assert!(snapshot.waves.is_empty());
        assert!(snapshot.clashes.is_empty());
        assert_eq!(snapshot.current_wave_age_seconds, None);
    }

    #[test]
    fn waves_start_at_their_own_barracks_on_spawn() {
        let snapshot = wave_snapshot(0.0, &config());
        assert_eq!(snapshot.waves.len(), 6);

        for lane in Lane::ALL {
            assert_close(wave(&snapshot, lane, Team::Radiant).progress, 0.0, "radiant start");
            assert_close(wave(&snapshot, lane, Team::Dire).progress, 1.0, "dire start");
        }
    }

    #[test]
    fn both_waves_converge_on_the_clash_point() {
        let config = config();
        // Mid meets at 17s by default; sample past it so both are clamped.
        let snapshot = wave_snapshot(20.0, &config);

        let radiant = wave(&snapshot, Lane::Mid, Team::Radiant);
        let dire = wave(&snapshot, Lane::Mid, Team::Dire);

        assert_close(radiant.progress, config.mid_meet_progress, "radiant at clash");
        assert_close(dire.progress, config.mid_meet_progress, "dire at clash");
        assert!(radiant.has_clashed);
        assert!(dire.has_clashed);
    }

    #[test]
    fn waves_have_not_clashed_before_the_meet_time() {
        let snapshot = wave_snapshot(10.0, &config());

        let radiant = wave(&snapshot, Lane::Mid, Team::Radiant);
        let dire = wave(&snapshot, Lane::Mid, Team::Dire);

        assert!(!radiant.has_clashed);
        assert!(!dire.has_clashed);
        assert!(radiant.progress < dire.progress, "waves should not have crossed");
    }

    #[test]
    fn side_lanes_clash_later_than_mid() {
        let config = config();
        // At 20s mid has clashed but the side lanes (28s) have not.
        let snapshot = wave_snapshot(20.0, &config);

        assert!(wave(&snapshot, Lane::Mid, Team::Radiant).has_clashed);
        assert!(!wave(&snapshot, Lane::Top, Team::Radiant).has_clashed);
        assert!(!wave(&snapshot, Lane::Bottom, Team::Radiant).has_clashed);
    }

    #[test]
    fn side_lane_clashes_are_biased_toward_opposite_offlanes() {
        let config = config();
        let snapshot = wave_snapshot(5.0, &config);

        let top = snapshot.clashes.iter().find(|c| c.lane == Lane::Top).unwrap();
        let bottom = snapshot.clashes.iter().find(|c| c.lane == Lane::Bottom).unwrap();
        let mid = snapshot.clashes.iter().find(|c| c.lane == Lane::Mid).unwrap();

        // Top is the Radiant offlane, so the clash sits on the Radiant half.
        assert!(top.progress < 0.5, "top clash should favour the Radiant side");
        // Bottom is the Dire offlane, so it mirrors.
        assert!(bottom.progress > 0.5, "bottom clash should favour the Dire side");
        assert_close(mid.progress, 0.5, "mid clash is centred");
        // The two side lanes should be mirror images of one another.
        assert_close(top.progress, 1.0 - bottom.progress, "side lanes mirror");
    }

    #[test]
    fn clash_countdown_reaches_zero_and_stays_there() {
        let config = config();

        let early = wave_snapshot(0.0, &config);
        let mid_clash = early.clashes.iter().find(|c| c.lane == Lane::Mid).unwrap();
        assert_eq!(mid_clash.seconds_until_clash, 17);

        let late = wave_snapshot(25.0, &config);
        let mid_clash = late.clashes.iter().find(|c| c.lane == Lane::Mid).unwrap();
        assert_eq!(mid_clash.seconds_until_clash, 0);
    }

    #[test]
    fn wave_cycle_repeats_every_thirty_seconds() {
        let config = config();
        let first = wave_snapshot(10.0, &config);
        let second = wave_snapshot(40.0, &config);
        let third = wave_snapshot(310.0, &config);

        for lane in Lane::ALL {
            for team in Team::ALL {
                let a = wave(&first, lane, team).progress;
                let b = wave(&second, lane, team).progress;
                let c = wave(&third, lane, team).progress;
                assert_close(a, b, "cycle repeats at +30s");
                assert_close(a, c, "cycle repeats at +300s");
            }
        }
    }

    #[test]
    fn progress_advances_monotonically_until_the_clash() {
        let config = config();
        let mut previous = -1.0;

        for age in 0..=17 {
            let snapshot = wave_snapshot(age as f32, &config);
            let progress = wave(&snapshot, Lane::Mid, Team::Radiant).progress;
            assert!(
                progress >= previous,
                "radiant mid progress went backwards at {age}s: {progress} < {previous}"
            );
            previous = progress;
        }
    }

    #[test]
    fn path_endpoints_match_the_lane_waypoints() {
        for lane in Lane::ALL {
            let waypoints = lane.waypoints();
            let start = point_at(lane, 0.0);
            let end = point_at(lane, 1.0);

            assert_close(start.x, waypoints[0].x, "path start x");
            assert_close(start.y, waypoints[0].y, "path start y");
            assert_close(end.x, waypoints[waypoints.len() - 1].x, "path end x");
            assert_close(end.y, waypoints[waypoints.len() - 1].y, "path end y");
        }
    }

    #[test]
    fn path_progress_is_clamped_to_the_lane() {
        let below = point_at(Lane::Mid, -5.0);
        let above = point_at(Lane::Mid, 5.0);

        assert_close(below.x, MID_LANE_PATH[0].x, "clamped below");
        assert_close(above.x, MID_LANE_PATH[MID_LANE_PATH.len() - 1].x, "clamped above");
    }

    /// A 180° rotation about the map centre swaps the teams and carries the top
    /// lane onto the bottom lane, so one path determines the other. Asserting it
    /// keeps a retune of one side from silently skewing the map.
    #[test]
    fn side_lanes_are_rotations_of_one_another() {
        let top = Lane::Top.waypoints();
        let bottom = Lane::Bottom.waypoints();
        assert_eq!(top.len(), bottom.len());

        for (index, point) in top.iter().enumerate() {
            let mirror = bottom[bottom.len() - 1 - index];
            assert_close(mirror.x, 1.0 - point.x, "bottom lane mirror x");
            assert_close(mirror.y, 1.0 - point.y, "bottom lane mirror y");
        }
    }

    /// Mid runs corner to corner through the centre, so it is its own rotation.
    #[test]
    fn mid_lane_is_symmetric_about_the_map_centre() {
        let mid = Lane::Mid.waypoints();

        for (index, point) in mid.iter().enumerate() {
            let mirror = mid[mid.len() - 1 - index];
            assert_close(mirror.x, 1.0 - point.x, "mid mirror x");
            assert_close(mirror.y, 1.0 - point.y, "mid mirror y");
        }
    }

    /// All three lanes start and end at the two bases, so the wave dots converge
    /// on the same points rather than fanning out at the corners.
    #[test]
    fn every_lane_spans_the_same_two_bases() {
        for lane in Lane::ALL {
            let start = point_at(lane, 0.0);
            let end = point_at(lane, 1.0);

            assert_close(start.x, 0.150, &format!("{lane} radiant end x"));
            assert_close(start.y, 0.150, &format!("{lane} radiant end y"));
            assert_close(end.x, 0.850, &format!("{lane} dire end x"));
            assert_close(end.y, 0.850, &format!("{lane} dire end y"));
        }
    }

    #[test]
    fn path_points_stay_within_normalised_map_space() {
        for lane in Lane::ALL {
            for step in 0..=100 {
                let point = point_at(lane, step as f32 / 100.0);
                assert!(
                    (0.0..=1.0).contains(&point.x) && (0.0..=1.0).contains(&point.y),
                    "{lane} left normalised space at {step}%: {point:?}"
                );
            }
        }
    }

    #[test]
    fn confidence_decays_with_game_time() {
        let config = config();
        assert_eq!(wave_snapshot(0.0, &config).confidence, Confidence::High);
        assert_eq!(wave_snapshot(599.0, &config).confidence, Confidence::High);
        assert_eq!(wave_snapshot(600.0, &config).confidence, Confidence::Degrading);
        assert_eq!(wave_snapshot(899.0, &config).confidence, Confidence::Degrading);
        assert_eq!(wave_snapshot(900.0, &config).confidence, Confidence::Low);
    }

    #[test]
    fn fractional_clock_yields_intermediate_positions() {
        let config = config();

        let at_eight = wave_snapshot(8.0, &config);
        let at_eight_half = wave_snapshot(8.5, &config);
        let at_nine = wave_snapshot(9.0, &config);

        let a = wave(&at_eight, Lane::Mid, Team::Radiant).progress;
        let b = wave(&at_eight_half, Lane::Mid, Team::Radiant).progress;
        let c = wave(&at_nine, Lane::Mid, Team::Radiant).progress;

        assert!(a < b && b < c, "fractional clock should interpolate: {a} < {b} < {c}");
        // Countdowns stay whole-second across the fractional step.
        assert_eq!(at_eight.seconds_until_next_spawn, 22);
        assert_eq!(at_eight_half.seconds_until_next_spawn, 22);
    }

    #[test]
    fn zero_meet_seconds_does_not_divide_by_zero() {
        let mut config = config();
        config.mid_meet_seconds = 0.0;

        let snapshot = wave_snapshot(10.0, &config);
        let radiant = wave(&snapshot, Lane::Mid, Team::Radiant);

        assert!(radiant.has_clashed);
        assert_close(radiant.progress, config.mid_meet_progress, "degenerate timing");
    }
}
