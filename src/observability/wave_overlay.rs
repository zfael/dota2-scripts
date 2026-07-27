//! Geometry for the click-through wave overlay.
//!
//! Kept separate from the Tauri layer so the placement maths is unit-testable
//! without a window system.

use crate::config::{MinimapCaptureConfig, WaveOverlayConfig};
use crate::observability::minimap_capture_backend::WindowRect;

/// Screen-space bounds for the overlay window, in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlayBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Place the overlay over Dota's minimap.
///
/// The minimap region is stored client-relative (it is what the capture worker
/// BitBlts), so it is translated by the client area's screen origin. `offset_x` /
/// `offset_y` are a manual nudge on top of that for fine alignment.
///
/// Returns `None` when the region has no area — a zero-sized window would be
/// invisible and, on Windows, can fail to create at all.
pub fn overlay_bounds(
    client_rect: &WindowRect,
    capture: &MinimapCaptureConfig,
    overlay: &WaveOverlayConfig,
) -> Option<OverlayBounds> {
    if capture.minimap_width == 0 || capture.minimap_height == 0 {
        return None;
    }

    Some(OverlayBounds {
        x: client_rect.x + capture.minimap_x as i32 + overlay.offset_x,
        y: client_rect.y + capture.minimap_y as i32 + overlay.offset_y,
        width: capture.minimap_width,
        height: capture.minimap_height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client_rect(x: i32, y: i32) -> WindowRect {
        WindowRect {
            x,
            y,
            width: 2560,
            height: 1440,
        }
    }

    fn capture() -> MinimapCaptureConfig {
        MinimapCaptureConfig {
            minimap_x: 2,
            minimap_y: 835,
            minimap_width: 240,
            minimap_height: 245,
            ..Default::default()
        }
    }

    #[test]
    fn bounds_translate_the_minimap_region_by_the_client_origin() {
        let bounds = overlay_bounds(&client_rect(100, 50), &capture(), &Default::default()).unwrap();

        assert_eq!(bounds.x, 102);
        assert_eq!(bounds.y, 885);
        assert_eq!(bounds.width, 240);
        assert_eq!(bounds.height, 245);
    }

    #[test]
    fn bounds_follow_the_dota_window_when_it_moves() {
        let capture = capture();
        let first = overlay_bounds(&client_rect(0, 0), &capture, &Default::default()).unwrap();
        let moved = overlay_bounds(&client_rect(400, 300), &capture, &Default::default()).unwrap();

        assert_eq!(moved.x - first.x, 400);
        assert_eq!(moved.y - first.y, 300);
    }

    #[test]
    fn manual_offsets_nudge_the_placement() {
        let overlay = WaveOverlayConfig {
            offset_x: -6,
            offset_y: 12,
            ..Default::default()
        };
        let bounds = overlay_bounds(&client_rect(100, 50), &capture(), &overlay).unwrap();

        assert_eq!(bounds.x, 96);
        assert_eq!(bounds.y, 897);
    }

    #[test]
    fn negative_placement_is_allowed_for_multi_monitor_layouts() {
        // A monitor left of the primary has negative screen coordinates; the
        // overlay must follow rather than clamp to zero.
        let bounds =
            overlay_bounds(&client_rect(-1920, 0), &capture(), &Default::default()).unwrap();

        assert_eq!(bounds.x, -1918);
    }

    #[test]
    fn a_zero_sized_region_yields_no_bounds() {
        let mut capture = capture();
        capture.minimap_width = 0;

        assert!(overlay_bounds(&client_rect(0, 0), &capture, &Default::default()).is_none());
    }
}
