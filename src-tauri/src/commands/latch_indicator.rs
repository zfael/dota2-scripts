//! Phantom Lancer's latch indicator: a click-through box over Dota, green
//! while the Ctrl latch is on, red while it is off.
//!
//! Unlike the wave overlay it has no toggle of its own. A worker shows it
//! while `latch_indicator::indicator_visible` says so and hides it otherwise,
//! so it is there whenever Phantom Lancer is played and gone the rest of the
//! time. The window is built on first need, never at startup.
//!
//! Geometry and the show/hide rule live in
//! `dota2_scripts::observability::latch_indicator`, where they are tested.

use crate::commands::overlay::{apply_bounds, apply_click_through};
use crate::TauriAppState;
use dota2_scripts::actions::heroes::phantom_lancer;
use dota2_scripts::config::Settings;
use dota2_scripts::observability::latch_indicator::{indicator_bounds, indicator_visible};
use dota2_scripts::observability::minimap_capture_backend::{
    find_dota2_client_screen_rect, is_dota2_foreground,
};
use dota2_scripts::observability::wave_overlay::OverlayBounds;
use dota2_scripts::state::{AppState, HeroType};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tracing::{info, warn};

/// Must be listed in `capabilities/default.json`. A window missing from it
/// gets no IPC at all: its listener and its commands are refused, and the
/// box sits on its first paint, "LATCH OFF", forever.
pub const LATCH_INDICATOR_LABEL: &str = "pl-latch-indicator";

/// Carries the latch state, `true` for on. Broadcast; only the indicator
/// window listens for it.
const LATCH_EVENT: &str = "pl_latch_update";

/// How often the latch is read. Fast enough that the box changes with the cue.
const LATCH_POLL: Duration = Duration::from_millis(30);

/// Re-place the window every this many latch polls (~300ms): finding Dota's
/// window costs more than reading the latch, and it moves far less often.
const PLACE_EVERY: u32 = 10;

/// Start the worker. Called once, from Tauri setup.
pub fn start(app: AppHandle) {
    let spawned = thread::Builder::new()
        .name("pl-latch-indicator".into())
        .spawn(move || run(app));
    if let Err(e) = spawned {
        warn!("latch indicator: failed to start its worker: {}", e);
    }
}

/// A plain thread rather than an async task: building the window blocks
/// until WebView2 is up, and it must not run on the main thread (see
/// `commands::overlay::ensure_overlay_window`).
fn run(app: AppHandle) {
    let state = app.state::<TauriAppState>();
    let settings = state.settings.clone();
    let app_state = state.app_state.clone();

    let mut tick: u32 = 0;
    // Where the window is while it is on screen; `None` while hidden.
    let mut shown: Option<OverlayBounds> = None;
    let mut last_sent: Option<bool> = None;
    let mut failing = false;

    loop {
        if tick % PLACE_EVERY == 0 {
            let target = target_bounds(&settings, &app_state);
            match (target, shown) {
                (Some(bounds), current) if current != Some(bounds) => {
                    match show_at(&app, bounds) {
                        Ok(()) => {
                            if current.is_none() {
                                // Re-send on every show: the view may have
                                // missed changes while hidden.
                                last_sent = None;
                            }
                            shown = Some(bounds);
                            failing = false;
                        }
                        Err(e) => {
                            if !std::mem::replace(&mut failing, true) {
                                warn!("latch indicator: {}", e);
                            }
                        }
                    }
                }
                (None, Some(_)) => {
                    if let Some(window) = app.get_webview_window(LATCH_INDICATOR_LABEL) {
                        let _ = window.hide();
                    }
                    shown = None;
                }
                _ => {}
            }
        }

        if shown.is_some() {
            let latched = phantom_lancer::is_latched();
            if last_sent != Some(latched) {
                // Broadcast, the path the wave overlay's `config_updated`
                // already takes to the view's global `listen`.
                match app.emit(LATCH_EVENT, latched) {
                    Ok(()) => last_sent = Some(latched),
                    Err(e) => warn!("latch indicator: failed to send the latch state: {}", e),
                }
            }
        }

        tick = tick.wrapping_add(1);
        thread::sleep(LATCH_POLL);
    }
}

/// Where the box belongs right now, or `None` when it should be hidden.
fn target_bounds(
    settings: &Arc<Mutex<Settings>>,
    app_state: &Arc<Mutex<AppState>>,
) -> Option<OverlayBounds> {
    let config = settings.lock().ok()?.heroes.phantom_lancer.clone();
    let selected = app_state.lock().ok()?.selected_hero == Some(HeroType::PhantomLancer);
    let client = find_dota2_client_screen_rect();

    if !indicator_visible(&config, selected, client.is_some(), is_dota2_foreground()) {
        return None;
    }
    Some(indicator_bounds(
        &client?,
        config.latch_indicator_offset_x,
        config.latch_indicator_offset_y,
    ))
}

fn show_at(app: &AppHandle, bounds: OverlayBounds) -> Result<(), String> {
    let window = ensure_window(app)?;
    apply_bounds(&window, bounds)?;
    if !window.is_visible().unwrap_or(false) {
        window
            .show()
            .map_err(|e| format!("Failed to show the latch indicator: {}", e))?;
        info!("latch indicator shown at {:?}", bounds);
    }
    Ok(())
}

/// Built hidden and only shown once placed, like the wave overlay, so it never
/// flashes at the wrong spot.
fn ensure_window(app: &AppHandle) -> Result<tauri::WebviewWindow, String> {
    if let Some(window) = app.get_webview_window(LATCH_INDICATOR_LABEL) {
        return Ok(window);
    }

    let window = WebviewWindowBuilder::new(
        app,
        LATCH_INDICATOR_LABEL,
        WebviewUrl::App("index.html?overlay=latch".into()),
    )
    .title("Latch Indicator")
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .shadow(false)
    .focused(false)
    .visible(false)
    .build()
    .map_err(|e| format!("Failed to create the latch indicator window: {}", e))?;

    // It sits in the middle of the screen: it must never take a click meant
    // for the game.
    apply_click_through(&window);
    Ok(window)
}

/// The view's first read, before any event reaches it. Events sent while its
/// page was still loading are lost, so it asks once it is listening.
#[tauri::command]
pub async fn get_pl_latch_state() -> bool {
    phantom_lancer::is_latched()
}

#[cfg(test)]
mod tests {
    use super::LATCH_INDICATOR_LABEL;
    use crate::commands::overlay::OVERLAY_LABEL;

    /// The indicator first shipped without this and never left "LATCH OFF".
    #[test]
    fn every_overlay_window_is_granted_ipc() {
        let capability: serde_json::Value =
            serde_json::from_str(include_str!("../../capabilities/default.json"))
                .expect("capabilities/default.json should parse");
        let windows: Vec<&str> = capability["windows"]
            .as_array()
            .expect("the capability should list its windows")
            .iter()
            .filter_map(|window| window.as_str())
            .collect();

        for label in [OVERLAY_LABEL, LATCH_INDICATOR_LABEL] {
            assert!(
                windows.contains(&label),
                "window `{label}` has no capability, so its IPC is refused"
            );
        }
    }
}
