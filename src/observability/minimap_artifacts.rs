use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

/// Save RGBA pixel data as a PNG file.
/// Creates the output directory if it does not exist.
/// Returns the full path of the saved PNG file.
pub fn save_capture_artifact(
    output_dir: &str,
    file_stem: &str,
    pixels: &[u8],
    width: u32,
    height: u32,
) -> Result<String, String> {
    let dir = std::path::Path::new(output_dir);
    std::fs::create_dir_all(dir).map_err(|e| format!("failed to create output dir: {}", e))?;

    let file_path = dir.join(format!("{}.png", file_stem));
    let file_path_str = file_path.to_string_lossy().to_string();

    let img = image::RgbaImage::from_raw(width, height, pixels.to_vec())
        .ok_or_else(|| "failed to create image from pixel data".to_string())?;

    img.save(&file_path)
        .map_err(|e| format!("failed to save PNG: {}", e))?;

    Ok(file_path_str)
}

/// Save artifact metadata as a JSON sidecar file.
/// Creates the output directory if it does not exist.
pub fn save_metadata_json(
    output_dir: &str,
    file_stem: &str,
    metadata: &MinimapArtifactMetadata,
) -> Result<(), String> {
    let dir = std::path::Path::new(output_dir);
    std::fs::create_dir_all(dir).map_err(|e| format!("failed to create output dir: {}", e))?;

    let file_path = dir.join(format!("{}.json", file_stem));
    let json = serde_json::to_string_pretty(metadata)
        .map_err(|e| format!("failed to serialize metadata: {}", e))?;

    std::fs::write(&file_path, json).map_err(|e| format!("failed to write metadata: {}", e))?;

    Ok(())
}
