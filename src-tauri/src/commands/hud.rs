//! Calibration for points on Dota's own HUD.
//!
//! See `dota2_scripts::observability::hud_anchors` for why an anchor is needed
//! at all and why it is stored as a fraction of Dota's client rect.

use crate::commands::config::CONFIG_UPDATED_EVENT;
use crate::TauriAppState;
use dota2_scripts::config::Settings;
use dota2_scripts::observability::hud_anchors::{apply_portrait_capture, resolve_portrait_point};
use serde::Serialize;
use std::sync::{Arc, Mutex, OnceLock};
use tauri::{AppHandle, Emitter};
use tracing::{info, warn};

/// Published during Tauri setup so the hotkey thread can reach the app.
///
/// Same reason as `commands::overlay`: the keyboard listener and its hotkey
/// handler start before the Tauri builder runs, so they cannot be handed an
/// `AppHandle` directly.
static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

pub fn register_app_handle(app: AppHandle) {
    let _ = APP_HANDLE.set(app);
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HudPortraitDto {
    pub x_fraction: f32,
    pub y_fraction: f32,
}

/// Push the settings to every window so a capture shows up without a restart.
fn broadcast(settings: &Settings) {
    let Some(app) = APP_HANDLE.get() else {
        return;
    };

    if let Err(e) = app.emit(CONFIG_UPDATED_EVENT, settings) {
        warn!("Failed to broadcast config after a HUD capture: {}", e);
    }
}

/// Capture the anchor and tell the UI, from wherever the request came.
fn capture_and_broadcast(settings: &Arc<Mutex<Settings>>) -> Result<HudPortraitDto, String> {
    let (x_fraction, y_fraction) =
        apply_portrait_capture(settings).map_err(|e| e.user_message().to_string())?;

    let snapshot = settings.lock().unwrap().clone();
    broadcast(&snapshot);

    Ok(HudPortraitDto {
        x_fraction,
        y_fraction,
    })
}

/// Record the cursor's current position as the hero portrait.
#[tauri::command]
pub fn capture_hud_portrait(
    state: tauri::State<'_, TauriAppState>,
) -> Result<HudPortraitDto, String> {
    capture_and_broadcast(&state.settings)
}

/// Same thing, driven by the global hotkey rather than a button.
pub fn capture_from_hotkey(settings: &Arc<Mutex<Settings>>) {
    match capture_and_broadcast(settings) {
        Ok(portrait) => info!(
            "HUD portrait anchor captured at ({:.4}, {:.4})",
            portrait.x_fraction, portrait.y_fraction
        ),
        Err(message) => warn!("HUD portrait capture failed: {}", message),
    }
}

/// Park the cursor on the stored anchor so the user can see where it landed.
///
/// Deliberately does **not** click. This runs while Dota is focused and a stray
/// click there is a move order.
#[tauri::command]
pub fn test_hud_portrait(state: tauri::State<'_, TauriAppState>) -> Result<(), String> {
    let hud = {
        let settings = state
            .settings
            .lock()
            .map_err(|e| format!("Failed to lock settings: {}", e))?;
        settings.hud.clone()
    };

    let point = resolve_portrait_point(&hud).ok_or_else(|| {
        if hud.portrait_calibrated {
            "Dota 2 is not running.".to_string()
        } else {
            "The portrait anchor has not been calibrated yet.".to_string()
        }
    })?;

    info!(
        "Testing the HUD portrait anchor at ({}, {})",
        point.x, point.y
    );
    dota2_scripts::input::simulation::move_cursor_to(point.x, point.y);

    Ok(())
}
