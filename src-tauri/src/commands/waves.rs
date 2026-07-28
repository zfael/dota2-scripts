use crate::ipc_types::{
    LaneClashDto, LanePathDto, MapPointDto, WavePositionDto, WaveSnapshotDto,
};
use crate::TauriAppState;
use dota2_scripts::observability::wave_tracker::{self, Lane, MapPoint};

fn to_point_dto(point: MapPoint) -> MapPointDto {
    MapPointDto {
        x: point.x,
        y: point.y,
    }
}

/// The static lane polylines in normalised map space.
///
/// Split out from the command so tests can call it without an async runtime.
fn lane_paths() -> Vec<LanePathDto> {
    Lane::ALL
        .iter()
        .map(|&lane| LanePathDto {
            lane: lane.to_string(),
            points: lane.waypoints().iter().copied().map(to_point_dto).collect(),
        })
        .collect()
}

/// Returns the static lane polylines in normalised map space.
///
/// Called once by the renderer; the geometry does not change at runtime.
///
/// `async` so Tauri runs it on the async runtime. Commands declared without it
/// execute on the **main thread**, where they block the window's message pump.
#[tauri::command]
pub async fn get_wave_lane_paths() -> Vec<LanePathDto> {
    lane_paths()
}

/// Returns predicted wave positions for a game-clock instant.
///
/// `clock_time_seconds` may be fractional — callers pass an interpolated clock so
/// dots animate smoothly between GSI packets.
///
/// **Must stay `async`.** The renderer polls this ~15 times a second; as a sync
/// command Tauri would run every one of those on the main thread, starving the
/// window's message pump until the app stops responding entirely.
#[tauri::command]
pub async fn get_wave_snapshot(
    clock_time_seconds: f32,
    state: tauri::State<'_, TauriAppState>,
) -> Result<WaveSnapshotDto, String> {
    let config = {
        let settings = state
            .settings
            .lock()
            .map_err(|e| format!("Failed to lock settings: {}", e))?;
        settings.wave_tracker.clone()
    };

    let snapshot = wave_tracker::wave_snapshot(clock_time_seconds, &config);

    Ok(WaveSnapshotDto {
        enabled: config.enabled,
        clock_time_seconds: snapshot.clock_time_seconds,
        next_spawn_time_seconds: snapshot.next_spawn_time_seconds,
        seconds_until_next_spawn: snapshot.seconds_until_next_spawn,
        current_wave_age_seconds: snapshot.current_wave_age_seconds,
        confidence: snapshot.confidence.to_string(),
        waves: snapshot
            .waves
            .into_iter()
            .map(|wave| WavePositionDto {
                lane: wave.lane.to_string(),
                team: wave.team.to_string(),
                progress: wave.progress,
                point: to_point_dto(wave.point),
                has_clashed: wave.has_clashed,
            })
            .collect(),
        clashes: snapshot
            .clashes
            .into_iter()
            .map(|clash| LaneClashDto {
                lane: clash.lane.to_string(),
                progress: clash.progress,
                point: to_point_dto(clash.point),
                seconds_until_clash: clash.seconds_until_clash,
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lane_paths_cover_every_lane_with_usable_geometry() {
        let paths = lane_paths();

        assert_eq!(paths.len(), 3);
        let lanes: Vec<&str> = paths.iter().map(|p| p.lane.as_str()).collect();
        assert!(lanes.contains(&"Top"));
        assert!(lanes.contains(&"Mid"));
        assert!(lanes.contains(&"Bottom"));

        for path in &paths {
            assert!(
                path.points.len() >= 2,
                "{} needs at least two points to draw",
                path.lane
            );
            for point in &path.points {
                assert!(
                    (0.0..=1.0).contains(&point.x) && (0.0..=1.0).contains(&point.y),
                    "{} has a point outside normalised space: {:?}",
                    path.lane,
                    point
                );
            }
        }
    }
}
