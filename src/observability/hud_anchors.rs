//! Points on Dota's own HUD that automation needs to click.
//!
//! Some abilities cannot be self-cast — Dota resolves them at the cursor, and
//! double-tap or an ALT modifier does nothing. Clicking the **hero portrait** is
//! the only way to land one of those on your own hero, because Dota treats a
//! click on the portrait as a click on the hero.
//!
//! That means the app needs a screen coordinate, and there is no way to derive
//! one: where the portrait sits inside Dota's window depends on resolution, UI
//! scale, and HUD skin. So it is measured once by the user and stored.
//!
//! # Coordinate basis
//!
//! Anchors are fractions of Dota's **client rect**, not of the display. A
//! fraction survives moving the window, changing resolution, and a second
//! monitor; a screen pixel survives none of those. Resolution back to pixels
//! happens at use time against the live window rect.
//!
//! # Failing safe
//!
//! [`resolve_portrait_point`] returns `None` when the anchor has never been
//! calibrated or Dota is not running, and callers must treat that as "do not
//! click". A stray click in Dota is a move order — the automation must never be
//! able to issue one from a coordinate nobody measured.

use std::sync::{Arc, Mutex};

use tracing::{info, warn};

use crate::config::settings::HudConfig;
use crate::config::Settings;
use crate::observability::minimap_capture_backend::{find_dota2_client_screen_rect, WindowRect};

/// A point in physical screen pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenPoint {
    pub x: i32,
    pub y: i32,
}

/// Why a portrait capture could not be completed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HudAnchorError {
    /// Dota 2 is not running, or its window could not be found.
    DotaNotFound,
    /// The OS would not tell us where the cursor is.
    CursorUnavailable,
    /// Dota's window has no area, so no fraction is meaningful.
    EmptyClientRect,
    /// The cursor was somewhere other than Dota's window.
    CursorOutsideDota,
}

impl HudAnchorError {
    /// A message suitable for showing to the user as-is.
    pub fn user_message(&self) -> &'static str {
        match self {
            HudAnchorError::DotaNotFound => "Dota 2 is not running.",
            HudAnchorError::CursorUnavailable => "Could not read the cursor position.",
            HudAnchorError::EmptyClientRect => "Dota 2's window has no size yet.",
            HudAnchorError::CursorOutsideDota => {
                "Point at Dota's hero portrait, not the desktop, then try again."
            }
        }
    }
}

/// Where the mouse is, in physical screen pixels.
pub fn cursor_screen_position() -> Option<ScreenPoint> {
    #[cfg(windows)]
    {
        cursor_screen_position_win32()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[cfg(windows)]
fn cursor_screen_position_win32() -> Option<ScreenPoint> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let mut point = POINT { x: 0, y: 0 };
    unsafe { GetCursorPos(&mut point) }.ok()?;

    Some(ScreenPoint {
        x: point.x,
        y: point.y,
    })
}

/// Express a screen point as a fraction of a window rect.
///
/// Returns `None` for a zero-sized rect (division by zero) or a point outside
/// it — both mean the measurement is not usable, and silently clamping would
/// store a plausible-looking anchor that points at the wrong thing.
pub fn fraction_from_point(point: ScreenPoint, rect: &WindowRect) -> Option<(f32, f32)> {
    if rect.width == 0 || rect.height == 0 {
        return None;
    }

    let offset_x = point.x - rect.x;
    let offset_y = point.y - rect.y;

    if offset_x < 0
        || offset_y < 0
        || offset_x >= rect.width as i32
        || offset_y >= rect.height as i32
    {
        return None;
    }

    Some((
        offset_x as f32 / rect.width as f32,
        offset_y as f32 / rect.height as f32,
    ))
}

/// Resolve a fraction of a window rect back to a screen point.
///
/// The fraction is clamped to `[0, 1]` so a corrupt config cannot produce a
/// click outside Dota's window.
pub fn point_from_fraction(fraction: (f32, f32), rect: &WindowRect) -> ScreenPoint {
    let clamped_x = fraction.0.clamp(0.0, 1.0);
    let clamped_y = fraction.1.clamp(0.0, 1.0);

    ScreenPoint {
        x: rect.x + (clamped_x * rect.width as f32) as i32,
        y: rect.y + (clamped_y * rect.height as f32) as i32,
    }
}

/// Read the cursor's current position as a portrait anchor.
///
/// The caller is expected to be hovering Dota's hero portrait when this runs.
pub fn capture_portrait_fraction() -> Result<(f32, f32), HudAnchorError> {
    let rect = find_dota2_client_screen_rect().ok_or(HudAnchorError::DotaNotFound)?;
    if rect.width == 0 || rect.height == 0 {
        return Err(HudAnchorError::EmptyClientRect);
    }

    let cursor = cursor_screen_position().ok_or(HudAnchorError::CursorUnavailable)?;

    fraction_from_point(cursor, &rect).ok_or(HudAnchorError::CursorOutsideDota)
}

/// Where the hero portrait is right now, in screen pixels.
///
/// `None` means "do not click": either the anchor was never calibrated, or Dota
/// is not on screen to click at.
pub fn resolve_portrait_point(config: &HudConfig) -> Option<ScreenPoint> {
    if !config.portrait_calibrated {
        return None;
    }

    let rect = find_dota2_client_screen_rect()?;
    if rect.width == 0 || rect.height == 0 {
        return None;
    }

    Some(point_from_fraction(
        (config.portrait_x_fraction, config.portrait_y_fraction),
        &rect,
    ))
}

/// Capture the portrait anchor and persist it.
///
/// Shared by the hotkey handlers in both binaries and the Tauri command, so the
/// capture rules live in exactly one place.
pub fn apply_portrait_capture(
    settings: &Arc<Mutex<Settings>>,
) -> Result<(f32, f32), HudAnchorError> {
    let fraction = capture_portrait_fraction()?;

    let mut settings = settings.lock().unwrap();
    settings.hud.portrait_x_fraction = fraction.0;
    settings.hud.portrait_y_fraction = fraction.1;
    settings.hud.portrait_calibrated = true;

    if let Err(e) = settings.save() {
        // The in-memory anchor is still good for this session, so this is a
        // warning rather than a failure — it just will not survive a restart.
        warn!("Captured the HUD portrait anchor but could not persist it: {e}");
    }

    info!(
        "🎯 HUD portrait anchor captured at ({:.4}, {:.4}) of Dota's client area",
        fraction.0, fraction.1
    );

    Ok(fraction)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> WindowRect {
        WindowRect {
            x: 100,
            y: 50,
            width: 1920,
            height: 1080,
        }
    }

    #[test]
    fn fraction_is_measured_from_the_window_origin_not_the_screen() {
        // Dead centre of a window that starts at (100, 50).
        let point = ScreenPoint { x: 1060, y: 590 };
        let (x, y) = fraction_from_point(point, &rect()).expect("centre is inside the window");

        assert!((x - 0.5).abs() < 0.001, "x was {x}");
        assert!((y - 0.5).abs() < 0.001, "y was {y}");
    }

    #[test]
    fn fraction_and_point_round_trip() {
        let rect = rect();
        let original = ScreenPoint { x: 946, y: 1023 };

        let fraction = fraction_from_point(original, &rect).expect("inside");
        let resolved = point_from_fraction(fraction, &rect);

        // Integer pixels through f32 and back, so allow a pixel of rounding.
        assert!((resolved.x - original.x).abs() <= 1);
        assert!((resolved.y - original.y).abs() <= 1);
    }

    #[test]
    fn a_cursor_outside_the_window_has_no_fraction() {
        // Above and left of the window origin.
        assert_eq!(fraction_from_point(ScreenPoint { x: 10, y: 10 }, &rect()), None);
        // Past the far edge.
        assert_eq!(
            fraction_from_point(ScreenPoint { x: 3000, y: 600 }, &rect()),
            None
        );
    }

    #[test]
    fn a_zero_sized_window_has_no_fraction() {
        let empty = WindowRect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        };

        assert_eq!(fraction_from_point(ScreenPoint { x: 0, y: 0 }, &empty), None);
    }

    #[test]
    fn resolving_clamps_a_corrupt_fraction_into_the_window() {
        let rect = rect();
        let resolved = point_from_fraction((5.0, -2.0), &rect);

        assert_eq!(resolved.x, rect.x + rect.width as i32);
        assert_eq!(resolved.y, rect.y);
    }

    #[test]
    fn an_uncalibrated_anchor_never_resolves() {
        let config = HudConfig::default();
        assert!(!config.portrait_calibrated);
        // No Dota window in tests either, but the calibration gate is checked
        // first so this holds regardless of the environment.
        assert_eq!(resolve_portrait_point(&config), None);
    }
}
