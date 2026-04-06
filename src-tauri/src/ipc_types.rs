use serde::Serialize;
use std::cmp::PartialEq;

/// Matches frontend GameState in src-ui/src/types/game.ts
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GameStateDto {
    pub hero_name: Option<String>,
    pub hero_level: u32,
    pub hp_percent: u32,
    pub mana_percent: u32,
    pub in_danger: bool,
    pub connected: bool,
    pub alive: bool,
    pub stunned: bool,
    pub silenced: bool,
    pub respawn_timer: Option<u32>,
    pub rune_timer: Option<i32>,
    pub game_time: i32,
}

/// Matches frontend AppState-related fields
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppStateDto {
    pub selected_hero: Option<String>,
    pub gsi_enabled: bool,
    pub standalone_enabled: bool,
    pub armlet_roshan_armed: bool,
    pub app_version: String,
}

/// Matches frontend QueueMetrics in src-ui/src/types/game.ts
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueMetricsDto {
    pub events_processed: u64,
    pub events_dropped: u64,
    pub current_queue_depth: usize,
    pub max_queue_depth: usize,
}

/// Matches frontend syntheticInput in DiagnosticsState
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyntheticInputDto {
    pub queue_depth: usize,
    pub total_queued: u64,
    pub peak_depth: usize,
    pub completions: u64,
    pub drops: u64,
}

/// Matches frontend DiagnosticsState in src-ui/src/types/game.ts
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsDto {
    pub gsi_connected: bool,
    pub keyboard_hook_active: bool,
    pub queue_metrics: QueueMetricsDto,
    pub synthetic_input: SyntheticInputDto,
    pub soul_ring_state: String,
    pub blocked_keys: Vec<String>,
}

/// Matches frontend UpdateCheckState in src-ui/src/types/game.ts
/// Uses internally-tagged enum: { "kind": "idle" }, { "kind": "available", "version": "..." }
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
pub enum UpdateStateDto {
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "checking")]
    Checking,
    #[serde(rename = "available", rename_all = "camelCase")]
    Available {
        version: String,
        release_notes: Option<String>,
    },
    #[serde(rename = "downloading")]
    Downloading,
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "upToDate")]
    UpToDate,
}

/// Activity entry emitted to frontend
#[derive(Debug, Clone, Serialize)]
pub struct ActivityEntryDto {
    pub id: String,
    pub timestamp: String,
    pub category: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

/// Meepo hero-specific observed runtime state
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeepoStateDto {
    pub health_percent: u32,
    pub mana_percent: u32,
    pub in_danger: bool,
    pub alive: bool,
    pub stunned: bool,
    pub silenced: bool,
    pub poof_ready: bool,
    pub dig_ready: bool,
    pub megameepo_ready: bool,
    pub has_shard: bool,
    pub has_scepter: bool,
    pub blink_available: bool,
    pub combo_items: Vec<String>,
}

/// Minimap capture status for frontend display
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MinimapStatusDto {
    pub enabled: bool,
    pub health: String,
    pub capture_interval_ms: u64,
    pub window_binding_status: String,
    pub consecutive_failures: u32,
    pub last_capture_duration_ms: Option<u64>,
    pub sampling_mode: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_state_dto_serializes_camel_case() {
        let dto = GameStateDto {
            hero_name: Some("Shadow Fiend".to_string()),
            hero_level: 25,
            hp_percent: 85,
            mana_percent: 70,
            in_danger: false,
            connected: true,
            alive: true,
            stunned: false,
            silenced: false,
            respawn_timer: None,
            rune_timer: Some(45),
            game_time: 1234,
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["heroName"], "Shadow Fiend");
        assert_eq!(json["hpPercent"], 85);
        assert_eq!(json["inDanger"], false);
        assert_eq!(json["runeTimer"], 45);
        assert!(json.get("hero_name").is_none());
    }

    #[test]
    fn update_state_dto_tags_correctly() {
        let idle = UpdateStateDto::Idle;
        let json = serde_json::to_value(&idle).unwrap();
        assert_eq!(json["kind"], "idle");

        let available = UpdateStateDto::Available {
            version: "1.2.0".to_string(),
            release_notes: Some("Bug fixes".to_string()),
        };
        let json = serde_json::to_value(&available).unwrap();
        assert_eq!(json["kind"], "available");
        assert_eq!(json["version"], "1.2.0");
        assert_eq!(json["releaseNotes"], "Bug fixes");

        let up_to_date = UpdateStateDto::UpToDate;
        let json = serde_json::to_value(&up_to_date).unwrap();
        assert_eq!(json["kind"], "upToDate");
    }

    #[test]
    fn minimap_status_dto_serializes_camel_case() {
        let dto = MinimapStatusDto {
            enabled: true,
            health: "healthy".to_string(),
            capture_interval_ms: 1000,
            window_binding_status: "bound".to_string(),
            consecutive_failures: 0,
            last_capture_duration_ms: Some(42),
            sampling_mode: "every-5".to_string(),
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["enabled"], true);
        assert_eq!(json["captureIntervalMs"], 1000);
        assert_eq!(json["windowBindingStatus"], "bound");
        assert_eq!(json["lastCaptureDurationMs"], 42);
        assert!(json.get("capture_interval_ms").is_none());
    }

    #[test]
    fn diagnostics_dto_serializes_nested() {
        let dto = DiagnosticsDto {
            gsi_connected: true,
            keyboard_hook_active: true,
            queue_metrics: QueueMetricsDto {
                events_processed: 100,
                events_dropped: 2,
                current_queue_depth: 3,
                max_queue_depth: 10,
            },
            synthetic_input: SyntheticInputDto {
                queue_depth: 0,
                total_queued: 50,
                peak_depth: 5,
                completions: 48,
                drops: 2,
            },
            soul_ring_state: "ready".to_string(),
            blocked_keys: vec!["q".to_string(), "w".to_string()],
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["gsiConnected"], true);
        assert_eq!(json["queueMetrics"]["eventsProcessed"], 100);
        assert_eq!(json["syntheticInput"]["peakDepth"], 5);
    }
}
