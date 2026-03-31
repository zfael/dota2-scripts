#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinimapArtifactMetadata {
    pub capture_timestamp: String,
    pub window_binding_status: String,
    pub minimap_x: u32,
    pub minimap_y: u32,
    pub minimap_width: u32,
    pub minimap_height: u32,
    pub image_width: u32,
    pub image_height: u32,
    pub capture_duration_ms: u64,
    pub capture_result: String,
    pub failure_reason: Option<String>,
}

pub fn should_persist_sample(success_index: u64, sample_every_n: u32) -> bool {
    if sample_every_n == 0 {
        return false;
    }

    success_index % sample_every_n as u64 == 0
}
