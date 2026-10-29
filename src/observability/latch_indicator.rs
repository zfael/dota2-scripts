//! Geometry and visibility for Phantom Lancer's latch indicator: a small
//! click-through box over Dota, green while the Ctrl latch is on, red while
//! it is off.
//!
//! Kept separate from the Tauri layer, like `wave_overlay`, so placement and
//! the show/hide decision are unit-testable without a window system.

use crate::config::settings::PhantomLancerConfig;
use crate::observability::minimap_capture_backend::WindowRect;
use crate::observability::wave_overlay::OverlayBounds;

/// Box height as a fraction of Dota's client height: 43px at 1440p, 32px at
/// 1080p. Tied to the window rather than fixed so it reads the same at any
/// resolution.
const HEIGHT_FRACTION: f32 = 0.03;

/// Wide enough for "LATCH OFF" at the font the view sizes from the height.
const ASPECT: f32 = 3.6;

/// Centre the box in Dota's client area, shifted by the configured offsets,
/// and kept inside the client area however far the offsets push it.
pub fn indicator_bounds(client: &WindowRect, offset_x: i32, offset_y: i32) -> OverlayBounds {
    let height =
        ((client.height as f32 * HEIGHT_FRACTION).round() as u32).clamp(1, client.height.max(1));
    let width = ((height as f32 * ASPECT).round() as u32).clamp(1, client.width.max(1));

    let centred_x = client.x + (client.width as i32 - width as i32) / 2;
    let centred_y = client.y + (client.height as i32 - height as i32) / 2;
    let max_x = client.x + client.width as i32 - width as i32;
    let max_y = client.y + client.height as i32 - height as i32;

    OverlayBounds {
        x: (centred_x + offset_x).clamp(client.x, max_x.max(client.x)),
        y: (centred_y + offset_y).clamp(client.y, max_y.max(client.y)),
        width,
        height,
    }
}

/// Whether the box should be on screen.
///
/// It needs Dota's window to be placed at all. Normally it shows only while
/// Phantom Lancer is the hero and Dota is in front: an always-on-top box left
/// in the middle of the desktop after an alt-tab would sit over whatever the
/// player switched to. Preview drops both conditions, so the box stays put
/// while the player moves it from the app.
pub fn indicator_visible(
    config: &PhantomLancerConfig,
    phantom_lancer_selected: bool,
    dota_found: bool,
    dota_foreground: bool,
) -> bool {
    if !config.ctrl_latch_enabled || !config.latch_indicator_enabled || !dota_found {
        return false;
    }
    config.latch_indicator_preview || (phantom_lancer_selected && dota_foreground)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client(x: i32, y: i32, width: u32, height: u32) -> WindowRect {
        WindowRect {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn the_box_is_centred_in_the_client_area() {
        let bounds = indicator_bounds(&client(0, 0, 2560, 1440), 0, 0);

        assert_eq!(bounds.height, 43);
        assert_eq!(bounds.width, 155);
        assert_eq!(bounds.x, (2560 - 155) / 2);
        assert_eq!(bounds.y, (1440 - 43) / 2);
    }

    #[test]
    fn the_box_scales_with_the_window() {
        let bounds = indicator_bounds(&client(0, 0, 1920, 1080), 0, 0);

        assert_eq!(bounds.height, 32);
        assert_eq!(bounds.width, 115);
    }

    #[test]
    fn offsets_shift_it_from_the_centre() {
        let centred = indicator_bounds(&client(0, 0, 2560, 1440), 0, 0);
        let moved = indicator_bounds(&client(0, 0, 2560, 1440), 40, -300);

        assert_eq!(moved.x - centred.x, 40);
        assert_eq!(moved.y - centred.y, -300);
    }

    #[test]
    fn it_follows_the_window_onto_another_monitor() {
        let bounds = indicator_bounds(&client(-1920, 0, 1920, 1080), 0, 0);

        assert_eq!(bounds.x, -1920 + (1920 - 115) / 2);
    }

    #[test]
    fn offsets_cannot_push_it_out_of_the_window() {
        let window = client(100, 50, 1920, 1080);
        let far = indicator_bounds(&window, 10_000, 10_000);
        let near = indicator_bounds(&window, -10_000, -10_000);

        assert_eq!(far.x + far.width as i32, 100 + 1920);
        assert_eq!(far.y + far.height as i32, 50 + 1080);
        assert_eq!((near.x, near.y), (100, 50));
    }

    #[test]
    fn a_degenerate_window_still_yields_a_box() {
        let bounds = indicator_bounds(&client(0, 0, 0, 0), 0, 0);

        assert_eq!((bounds.width, bounds.height), (1, 1));
    }

    #[test]
    fn it_shows_for_phantom_lancer_with_dota_in_front() {
        let config = PhantomLancerConfig::default();

        assert!(indicator_visible(&config, true, true, true));
        assert!(!indicator_visible(&config, true, true, false), "alt-tabbed");
        assert!(
            !indicator_visible(&config, false, true, true),
            "another hero"
        );
        assert!(
            !indicator_visible(&config, true, false, true),
            "no Dota window"
        );
    }

    #[test]
    fn preview_shows_it_for_any_hero_and_any_window_in_front() {
        let config = PhantomLancerConfig {
            latch_indicator_preview: true,
            ..Default::default()
        };

        assert!(indicator_visible(&config, false, true, false));
        assert!(
            !indicator_visible(&config, false, false, false),
            "nothing to centre on without Dota"
        );
    }

    #[test]
    fn it_hides_when_the_indicator_or_the_latch_is_off() {
        let indicator_off = PhantomLancerConfig {
            latch_indicator_enabled: false,
            latch_indicator_preview: true,
            ..Default::default()
        };
        let latch_off = PhantomLancerConfig {
            ctrl_latch_enabled: false,
            latch_indicator_preview: true,
            ..Default::default()
        };

        assert!(!indicator_visible(&indicator_off, true, true, true));
        assert!(!indicator_visible(&latch_off, true, true, true));
    }
}
