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
    pub invoker_active_combo_profile_id: Option<String>,
    pub app_version: String,
}

/// A point in normalised map space. Origin is bottom-left (Radiant corner).
///
/// Matches frontend `MapPoint` in src-ui/src/types/waves.ts
#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MapPointDto {
    pub x: f32,
    pub y: f32,
}

/// Static lane geometry, sent once so the renderer draws the same polyline the
/// model interpolates along.
///
/// Matches frontend `LanePath` in src-ui/src/types/waves.ts
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LanePathDto {
    pub lane: String,
    pub points: Vec<MapPointDto>,
}

/// Matches frontend `WavePosition` in src-ui/src/types/waves.ts
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WavePositionDto {
    pub lane: String,
    pub team: String,
    pub progress: f32,
    pub point: MapPointDto,
    pub has_clashed: bool,
}

/// Matches frontend `LaneClash` in src-ui/src/types/waves.ts
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LaneClashDto {
    pub lane: String,
    pub progress: f32,
    pub point: MapPointDto,
    pub seconds_until_clash: i32,
}

/// Matches frontend `WaveSnapshot` in src-ui/src/types/waves.ts
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WaveSnapshotDto {
    pub enabled: bool,
    pub clock_time_seconds: f32,
    pub next_spawn_time_seconds: i32,
    pub seconds_until_next_spawn: i32,
    pub current_wave_age_seconds: Option<f32>,
    pub confidence: String,
    pub waves: Vec<WavePositionDto>,
    pub clashes: Vec<LaneClashDto>,
}

/// Matches frontend `AlertCountdown` in src-ui/src/types/alerts.ts
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AlertCountdownDto {
    /// Stable key, e.g. "power_rune".
    pub event: String,
    pub display_name: String,
    pub enabled: bool,
    pub next_occurrence_seconds: Option<i32>,
    pub seconds_until: Option<i32>,
}

/// Matches frontend `OverlayBounds` in src-ui/src/types/waves.ts
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OverlayBoundsDto {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Matches frontend `WaveOverlayStatus` in src-ui/src/types/waves.ts
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WaveOverlayStatusDto {
    pub enabled: bool,
    pub visible: bool,
    pub toggle_key: String,
    /// "NotFound" | "Windowed" | "Borderless".
    pub dota_window_mode: String,
    /// Where the overlay is (or would be) placed; `None` if Dota is not running
    /// or the configured minimap region has no area.
    pub bounds: Option<OverlayBoundsDto>,
}

/// Matches frontend QueueMetrics in src-ui/src/types/game.ts
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueMetricsDto {
    pub events_processed: u64,
    pub events_dropped: u64,
    pub events_rejected: u64,
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

/// STRATZ dataset and token state behind the advice panel.
///
/// Deliberately carries no token — only whether one is set. The value never
/// leaves the backend.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StratzStatusDto {
    pub enabled: bool,
    pub has_token: bool,
    /// Position 1-5 being queued for; 0 means no filter.
    pub position: u8,
    pub ready: bool,
    pub refreshing: bool,
    pub progress: u8,
    pub hero_count: usize,
    /// Heroes whose matchups the refresh could not fetch. They still appear
    /// as suggestions but carry no counter or synergy signal.
    pub incomplete_heroes: usize,
    pub bracket: String,
    pub built_at: u64,
    pub last_error: Option<String>,
}

/// One ranked pick.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestionDto {
    pub slug: String,
    pub display_name: String,
    pub score: f32,
    pub counter: f32,
    pub synergy: f32,
    pub position_win_rate: Option<f32>,
    pub best_against: Option<String>,
    pub counter_samples: u32,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftAdviceDto {
    pub suggestions: Vec<SuggestionDto>,
    /// Identified heroes the dataset did not recognise — a cache older than
    /// the current patch. Surfaced so the UI can say the advice is partial.
    pub unresolved: Vec<String>,
    pub allies_used: usize,
    pub enemies_used: usize,
}

/// One draft slot as identified so far.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftSlotDto {
    /// 0-9 in strip order: left team first, each side inner-to-outer as drawn.
    pub index: usize,
    pub is_ally: bool,
    /// Present only when the read is trustworthy; `None` + `unknown` means the
    /// slot is occupied by a portrait we cannot match (someone's arcana).
    pub hero: Option<String>,
    pub unknown: bool,
    pub agreement: f32,
    pub best_score: f32,
}

/// Live draft identification status for the Draft page.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftStatusDto {
    pub enabled: bool,
    pub active: bool,
    pub game_state: String,
    /// Identity of the current draft; changes on every new one. The UI resets
    /// its per-slot verdicts when this changes — `matchid` cannot serve here,
    /// bot matches report it as `"0"` for every game.
    pub session_id: String,
    pub matchid: String,
    pub team_name: String,
    pub own_hero: String,
    pub frames: u32,
    pub slots: Vec<DraftSlotDto>,
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
    fn draft_status_dto_serializes_camel_case() {
        let dto = DraftStatusDto {
            enabled: true,
            active: true,
            game_state: "DOTA_GAMERULES_STATE_HERO_SELECTION".to_string(),
            session_id: "1787782500_0".to_string(),
            matchid: "812345".to_string(),
            team_name: "dire".to_string(),
            own_hero: "npc_dota_hero_nevermore".to_string(),
            frames: 12,
            slots: vec![DraftSlotDto {
                index: 0,
                is_ally: false,
                hero: Some("skeleton_king".to_string()),
                unknown: false,
                agreement: 0.75,
                // Exactly representable in f32, so the JSON number compares
                // cleanly against an f64 literal.
                best_score: 0.5,
            }],
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["gameState"], "DOTA_GAMERULES_STATE_HERO_SELECTION");
        // The UI resets its per-slot verdicts when this changes, so the
        // camelCase spelling is load-bearing: a mismatch here reads as "the
        // draft never changed" and the previous game's votes stick.
        assert_eq!(json["sessionId"], "1787782500_0");
        assert_eq!(json["ownHero"], "npc_dota_hero_nevermore");
        assert_eq!(json["teamName"], "dire");
        assert_eq!(json["slots"][0]["isAlly"], false);
        assert_eq!(json["slots"][0]["bestScore"], 0.5);
        assert_eq!(json["slots"][0]["hero"], "skeleton_king");
    }

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
                events_rejected: 1,
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

    #[test]
    fn app_state_dto_serializes_invoker_active_combo_profile_id_in_camel_case() {
        let dto = AppStateDto {
            selected_hero: Some("Invoker".to_string()),
            gsi_enabled: true,
            standalone_enabled: true,
            armlet_roshan_armed: false,
            invoker_active_combo_profile_id: Some("qe-burst".to_string()),
            app_version: "0.15.0".to_string(),
        };

        let json = serde_json::to_value(&dto).unwrap();

        assert_eq!(json["selectedHero"], "Invoker");
        assert_eq!(json["invokerActiveComboProfileId"], "qe-burst");
        assert!(json.get("invoker_active_combo_profile_id").is_none());
    }
}
