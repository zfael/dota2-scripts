use std::collections::VecDeque;

/// HSV color value (hue 0–360, saturation 0–100, value 0–100).
#[derive(Debug, Clone, Copy)]
pub struct Hsv {
    pub h: f32,
    pub s: f32,
    pub v: f32,
}

/// Thresholds for red/green color detection and cluster filtering.
#[derive(Debug, Clone)]
pub struct ColorThresholds {
    pub red_hue_max: f32,
    pub red_hue_min_wrap: f32,
    pub red_min_saturation: f32,
    pub red_min_value: f32,
    pub green_hue_min: f32,
    pub green_hue_max: f32,
    pub green_min_saturation: f32,
    pub green_min_value: f32,
    pub min_cluster_size: usize,
    pub max_cluster_size: usize,
}

impl Default for ColorThresholds {
    fn default() -> Self {
        Self {
            red_hue_max: 15.0,
            red_hue_min_wrap: 340.0,
            red_min_saturation: 40.0,
            red_min_value: 30.0,
            green_hue_min: 80.0,
            green_hue_max: 160.0,
            green_min_saturation: 35.0,
            green_min_value: 25.0,
            min_cluster_size: 20,
            max_cluster_size: 200,
        }
    }
}

/// Convert RGB (0–255) to HSV (h: 0–360, s: 0–100, v: 0–100).
pub fn rgb_to_hsv(r: u8, g: u8, b: u8) -> Hsv {
    let rf = r as f32 / 255.0;
    let gf = g as f32 / 255.0;
    let bf = b as f32 / 255.0;
    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let delta = max - min;

    let v = max * 100.0;
    if max == 0.0 {
        return Hsv { h: 0.0, s: 0.0, v: 0.0 };
    }
    let s = (delta / max) * 100.0;
    if delta < 0.0001 {
        return Hsv { h: 0.0, s: 0.0, v };
    }

    let h = if (max - rf).abs() < 0.0001 {
        60.0 * (((gf - bf) / delta) % 6.0)
    } else if (max - gf).abs() < 0.0001 {
        60.0 * (((bf - rf) / delta) + 2.0)
    } else {
        60.0 * (((rf - gf) / delta) + 4.0)
    };
    let h = if h < 0.0 { h + 360.0 } else { h };

    Hsv { h, s, v }
}

/// Check if an RGB pixel falls within the red hue range.
pub fn is_red_pixel(r: u8, g: u8, b: u8, t: &ColorThresholds) -> bool {
    let hsv = rgb_to_hsv(r, g, b);
    hsv.s >= t.red_min_saturation
        && hsv.v >= t.red_min_value
        && (hsv.h <= t.red_hue_max || hsv.h >= t.red_hue_min_wrap)
}

/// Check if an RGB pixel falls within the green hue range.
pub fn is_green_pixel(r: u8, g: u8, b: u8, t: &ColorThresholds) -> bool {
    let hsv = rgb_to_hsv(r, g, b);
    hsv.s >= t.green_min_saturation
        && hsv.v >= t.green_min_value
        && hsv.h >= t.green_hue_min
        && hsv.h <= t.green_hue_max
}

/// Build boolean masks for red and green pixels from RGBA image data.
///
/// Returns `(red_mask, green_mask)` where each mask has one entry per pixel.
pub fn build_color_masks(
    pixels: &[u8],
    width: u32,
    height: u32,
    t: &ColorThresholds,
) -> (Vec<bool>, Vec<bool>) {
    let total = (width * height) as usize;
    let mut red_mask = vec![false; total];
    let mut green_mask = vec![false; total];
    for i in 0..total {
        let base = i * 4;
        if base + 3 >= pixels.len() {
            break;
        }
        let (r, g, b) = (pixels[base], pixels[base + 1], pixels[base + 2]);
        if is_red_pixel(r, g, b, t) {
            red_mask[i] = true;
        }
        if is_green_pixel(r, g, b, t) {
            green_mask[i] = true;
        }
    }
    (red_mask, green_mask)
}

/// A detected cluster of same-color pixels.
#[derive(Debug, Clone)]
pub struct DetectedCluster {
    pub center_x: u32,
    pub center_y: u32,
    pub size: usize,
}

/// Find connected clusters in a boolean mask using BFS flood fill.
///
/// Only returns clusters with `min_size <= size <= max_size`.
pub fn find_clusters(
    mask: &[bool],
    width: u32,
    height: u32,
    min_size: usize,
    max_size: usize,
) -> Vec<DetectedCluster> {
    let total = (width * height) as usize;
    let mut visited = vec![false; total];
    let mut clusters = Vec::new();

    for start in 0..total {
        if !mask[start] || visited[start] {
            continue;
        }
        let mut queue = VecDeque::new();
        queue.push_back(start);
        visited[start] = true;
        let mut sum_x: u64 = 0;
        let mut sum_y: u64 = 0;
        let mut count: usize = 0;

        while let Some(idx) = queue.pop_front() {
            let px = (idx % width as usize) as u64;
            let py = (idx / width as usize) as u64;
            sum_x += px;
            sum_y += py;
            count += 1;

            let ix = px as i32;
            let iy = py as i32;
            for (dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let nx = ix + dx;
                let ny = iy + dy;
                if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                    let ni = (ny as usize) * (width as usize) + (nx as usize);
                    if mask[ni] && !visited[ni] {
                        visited[ni] = true;
                        queue.push_back(ni);
                    }
                }
            }
        }

        if count >= min_size && count <= max_size {
            clusters.push(DetectedCluster {
                center_x: (sum_x / count as u64) as u32,
                center_y: (sum_y / count as u64) as u32,
                size: count,
            });
        }
    }

    clusters
}
