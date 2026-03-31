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

pub fn build_artifact_metadata(
    capture_timestamp: String,
    window_binding_status: String,
    minimap_x: u32,
    minimap_y: u32,
    minimap_width: u32,
    minimap_height: u32,
    image_width: u32,
    image_height: u32,
    capture_duration_ms: u64,
    capture_result: String,
    failure_reason: Option<String>,
) -> MinimapArtifactMetadata {
    MinimapArtifactMetadata {
        capture_timestamp,
        window_binding_status,
        minimap_x,
        minimap_y,
        minimap_width,
        minimap_height,
        image_width,
        image_height,
        capture_duration_ms,
        capture_result,
        failure_reason,
    }
}

pub fn artifact_metadata_path(base_dir: &str, file_stem: &str) -> String {
    format!("{}/{}.json", base_dir, file_stem)
}
