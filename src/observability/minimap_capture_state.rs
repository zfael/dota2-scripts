#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MinimapCaptureHealth {
    Idle,
    Healthy,
    Unhealthy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinimapCaptureStatusSnapshot {
    pub enabled: bool,
    pub capture_interval_ms: u64,
    pub last_success_at: Option<String>,
    pub last_failure_at: Option<String>,
    pub consecutive_failures: u32,
    pub last_capture_duration_ms: Option<u64>,
    pub last_artifact_path: Option<String>,
    pub sampling_mode: String,
    pub window_binding_status: String,
    pub health: MinimapCaptureHealth,
}
