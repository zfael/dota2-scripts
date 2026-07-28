use crate::ipc_types::{OverlayBoundsDto, WaveOverlayStatusDto};
use crate::TauriAppState;
use dota2_scripts::config::Settings;
use dota2_scripts::observability::minimap_capture_backend::{
    detect_dota2_window_mode, find_dota2_client_screen_rect, Dota2WindowMode,
};
use dota2_scripts::observability::wave_overlay::{overlay_bounds, OverlayBounds};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};
use tracing::{info, warn};

pub const OVERLAY_LABEL: &str = "wave-overlay";

/// Guards the reposition loop so toggling repeatedly cannot stack up tasks.
static FOLLOW_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Published during Tauri setup so the hotkey thread can reach the app.
///
/// The keyboard listener and its hotkey handler are started before the Tauri
/// builder runs, so they cannot be handed an `AppHandle` directly.
static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

pub fn register_app_handle(app: AppHandle) {
    let _ = APP_HANDLE.set(app);
}

/// How often the overlay re-reads Dota's window position.
///
/// The Dota window can be moved, resized, or switched between monitors at any
/// time, and there is no notification for it from outside the process — so the
/// overlay polls rather than being told.
const FOLLOW_INTERVAL: Duration = Duration::from_millis(1000);

fn mode_label(mode: Dota2WindowMode) -> &'static str {
    match mode {
        Dota2WindowMode::NotFound => "NotFound",
        Dota2WindowMode::Windowed => "Windowed",
        Dota2WindowMode::Borderless => "Borderless",
    }
}

fn current_bounds(settings: &Settings) -> Option<OverlayBounds> {
    let client_rect = find_dota2_client_screen_rect()?;
    overlay_bounds(&client_rect, &settings.minimap_capture, &settings.wave_overlay)
}

fn snapshot_settings(state: &tauri::State<'_, TauriAppState>) -> Result<Settings, String> {
    state
        .settings
        .lock()
        .map(|settings| settings.clone())
        .map_err(|e| format!("Failed to lock settings: {}", e))
}

/// Create the overlay window if it does not exist yet.
///
/// The window is deliberately built hidden: it is shown only once it has been
/// positioned, so it never flashes at the wrong place on screen.
fn ensure_overlay_window(app: &AppHandle) -> Result<tauri::WebviewWindow, String> {
    if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
        return Ok(window);
    }

    let window = WebviewWindowBuilder::new(
        app,
        OVERLAY_LABEL,
        // Query parameter rather than a route: the app uses BrowserRouter, and a
        // param survives the static file load without needing a second entry point.
        WebviewUrl::App("index.html?overlay=1".into()),
    )
    .title("Wave Overlay")
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .shadow(false)
    .focused(false)
    .visible(false)
    .build()
    .map_err(|e| format!("Failed to create overlay window: {}", e))?;

    // Click-through. Without this the overlay swallows minimap clicks and breaks
    // click-to-move, which is the single most important property of the window.
    window
        .set_ignore_cursor_events(true)
        .map_err(|e| format!("Failed to make overlay click-through: {}", e))?;

    Ok(window)
}

fn apply_bounds(window: &tauri::WebviewWindow, bounds: OverlayBounds) -> Result<(), String> {
    window
        .set_position(PhysicalPosition::new(bounds.x, bounds.y))
        .map_err(|e| format!("Failed to position overlay: {}", e))?;
    window
        .set_size(PhysicalSize::new(bounds.width, bounds.height))
        .map_err(|e| format!("Failed to size overlay: {}", e))?;
    Ok(())
}

/// Keep the overlay glued to Dota's minimap while it is visible.
fn start_follow_loop(app: AppHandle, settings: Arc<std::sync::Mutex<Settings>>) {
    if FOLLOW_ACTIVE.swap(true, Ordering::SeqCst) {
        return;
    }

    tauri::async_runtime::spawn(async move {
        let mut last_applied: Option<OverlayBounds> = None;

        loop {
            tokio::time::sleep(FOLLOW_INTERVAL).await;

            let Some(window) = app.get_webview_window(OVERLAY_LABEL) else {
                break;
            };
            if !window.is_visible().unwrap_or(false) {
                break;
            }

            let snapshot = match settings.lock() {
                Ok(settings) => settings.clone(),
                Err(_) => continue,
            };

            match current_bounds(&snapshot) {
                Some(bounds) => {
                    // Only touch the window when something actually moved.
                    if last_applied != Some(bounds) {
                        if let Err(e) = apply_bounds(&window, bounds) {
                            warn!("wave overlay: {}", e);
                        } else {
                            last_applied = Some(bounds);
                        }
                    }
                }
                None => {
                    // Dota closed or the region is unusable — hide rather than
                    // leave a stale overlay floating over the desktop.
                    let _ = window.hide();
                    break;
                }
            }
        }

        FOLLOW_ACTIVE.store(false, Ordering::SeqCst);
    });
}

/// Show the overlay. No-op if Dota is not running.
///
/// Deliberately **not** `async`: this builds a window, and window creation has to
/// happen on the main thread. Unlike the polled read commands it runs once per
/// user action, so it costs the message pump nothing.
#[tauri::command]
pub fn show_wave_overlay(
    app: AppHandle,
    state: tauri::State<'_, TauriAppState>,
) -> Result<bool, String> {
    let settings = snapshot_settings(&state)?;

    let Some(bounds) = current_bounds(&settings) else {
        warn!("wave overlay: Dota 2 window not found or minimap region is empty");
        return Ok(false);
    };

    let window = ensure_overlay_window(&app)?;
    apply_bounds(&window, bounds)?;
    window
        .show()
        .map_err(|e| format!("Failed to show overlay: {}", e))?;

    start_follow_loop(app.clone(), state.settings.clone());
    info!("wave overlay shown at {:?}", bounds);
    Ok(true)
}

/// Hide the overlay, leaving the window built so re-showing is instant.
#[tauri::command]
pub fn hide_wave_overlay(app: AppHandle) -> Result<bool, String> {
    if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
        window
            .hide()
            .map_err(|e| format!("Failed to hide overlay: {}", e))?;
    }
    Ok(false)
}

/// Flip overlay visibility. Returns the new visible state.
#[tauri::command]
pub fn toggle_wave_overlay(
    app: AppHandle,
    state: tauri::State<'_, TauriAppState>,
) -> Result<bool, String> {
    let visible = app
        .get_webview_window(OVERLAY_LABEL)
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);

    if visible {
        hide_wave_overlay(app)
    } else {
        show_wave_overlay(app, state)
    }
}

/// Read-only, so it runs off the main thread. See `get_wave_snapshot`.
#[tauri::command]
pub async fn get_wave_overlay_status(
    app: AppHandle,
    state: tauri::State<'_, TauriAppState>,
) -> Result<WaveOverlayStatusDto, String> {
    let settings = snapshot_settings(&state)?;

    let visible = app
        .get_webview_window(OVERLAY_LABEL)
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);

    Ok(WaveOverlayStatusDto {
        enabled: settings.wave_overlay.enabled,
        visible,
        toggle_key: settings.wave_overlay.toggle_key.clone(),
        dota_window_mode: mode_label(detect_dota2_window_mode()).to_string(),
        bounds: current_bounds(&settings).map(|b| OverlayBoundsDto {
            x: b.x,
            y: b.y,
            width: b.width,
            height: b.height,
        }),
    })
}

/// Toggle driven by the global hotkey rather than the UI.
///
/// Silently does nothing until Tauri setup has published the app handle.
///
/// The keyboard hook runs on its own thread, and this can create a window, so the
/// work is marshalled onto the main thread rather than done inline.
pub fn toggle_from_hotkey() {
    let Some(app) = APP_HANDLE.get() else {
        warn!("wave overlay hotkey fired before the app was ready");
        return;
    };

    let handle = app.clone();
    let result = app.run_on_main_thread(move || {
        let state = handle.state::<TauriAppState>();
        match toggle_wave_overlay(handle.clone(), state) {
            Ok(visible) => info!(
                "wave overlay {} via hotkey",
                if visible { "shown" } else { "hidden" }
            ),
            Err(e) => warn!("wave overlay hotkey toggle failed: {}", e),
        }
    });

    if let Err(e) = result {
        warn!("wave overlay hotkey could not reach the main thread: {}", e);
    }
}
