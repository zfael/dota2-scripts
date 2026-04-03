use crate::ipc_types::UpdateStateDto;
use crate::TauriAppState;
use dota2_scripts::state::UpdateCheckState;
use dota2_scripts::update::{ApplyUpdateResult, UpdateCheckResult};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Returns current update check state
#[tauri::command]
pub fn get_update_state(state: tauri::State<'_, TauriAppState>) -> Result<UpdateStateDto, String> {
    let app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;

    let update_state = app
        .update_state
        .lock()
        .map_err(|e| format!("Failed to lock update state: {}", e))?;

    let dto = match &*update_state {
        UpdateCheckState::Idle => UpdateStateDto::Idle,
        UpdateCheckState::Checking => UpdateStateDto::Checking,
        UpdateCheckState::Available {
            version,
            release_notes,
        } => UpdateStateDto::Available {
            version: version.clone(),
            release_notes: release_notes.clone(),
        },
        UpdateCheckState::Downloading => UpdateStateDto::Downloading,
        UpdateCheckState::Error(msg) => UpdateStateDto::Error {
            message: msg.clone(),
        },
        UpdateCheckState::UpToDate => UpdateStateDto::UpToDate,
    };

    Ok(dto)
}

/// Triggers an update check
#[tauri::command]
pub async fn check_for_updates(
    state: tauri::State<'_, TauriAppState>,
) -> Result<UpdateStateDto, String> {
    let update_state_arc: Arc<Mutex<UpdateCheckState>> = {
        let app = state
            .app_state
            .lock()
            .map_err(|e| format!("Failed to lock app state: {}", e))?;
        app.update_state.clone()
    };
    let include_prereleases = state
        .settings
        .lock()
        .map_err(|e| format!("Failed to lock settings: {}", e))?
        .updates
        .include_prereleases;

    {
        let mut us = update_state_arc.lock().unwrap();
        *us = UpdateCheckState::Checking;
    }

    let update_state_clone = update_state_arc.clone();
    let result = tokio::task::spawn_blocking(move || {
        let check_result = dota2_scripts::update::check_for_update(include_prereleases);
        let mut us = update_state_clone.lock().unwrap();
        match check_result {
            UpdateCheckResult::Available(info) => {
                let dto = UpdateStateDto::Available {
                    version: info.version.clone(),
                    release_notes: info.release_notes.clone(),
                };
                *us = UpdateCheckState::Available {
                    version: info.version,
                    release_notes: info.release_notes,
                };
                dto
            }
            UpdateCheckResult::UpToDate => {
                *us = UpdateCheckState::UpToDate;
                UpdateStateDto::UpToDate
            }
            UpdateCheckResult::Error(msg) => {
                let dto = UpdateStateDto::Error {
                    message: msg.clone(),
                };
                *us = UpdateCheckState::Error(msg);
                dto
            }
        }
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;

    Ok(result)
}

/// Downloads and applies the available update
#[tauri::command]
pub async fn apply_update(
    state: tauri::State<'_, TauriAppState>,
) -> Result<UpdateStateDto, String> {
    let include_prereleases = state
        .settings
        .lock()
        .map_err(|e| format!("Failed to lock settings: {}", e))?
        .updates
        .include_prereleases;
    let update_state_arc: Arc<Mutex<UpdateCheckState>> = {
        let app = state
            .app_state
            .lock()
            .map_err(|e| format!("Failed to lock app state: {}", e))?;
        app.update_state.clone()
    };

    {
        let mut us = update_state_arc.lock().unwrap();
        *us = UpdateCheckState::Downloading;
    }

    let update_state_clone = update_state_arc.clone();
    let (result, should_exit) = tokio::task::spawn_blocking(move || {
        let apply_result = dota2_scripts::update::apply_update(include_prereleases);
        let mut us = update_state_clone.lock().unwrap();
        match apply_result {
            ApplyUpdateResult::Success { new_version: _ } => {
                *us = UpdateCheckState::UpToDate;
                (UpdateStateDto::UpToDate, true)
            }
            ApplyUpdateResult::UpToDate => {
                *us = UpdateCheckState::UpToDate;
                (UpdateStateDto::UpToDate, false)
            }
            ApplyUpdateResult::Error(msg) => {
                let dto = UpdateStateDto::Error {
                    message: msg.clone(),
                };
                *us = UpdateCheckState::Error(msg);
                (dto, false)
            }
        }
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;

    if should_exit {
        std::thread::spawn(|| {
            std::thread::sleep(Duration::from_millis(500));
            std::process::exit(0);
        });
    }

    Ok(result)
}

/// Dismisses update banner (resets to Idle)
#[tauri::command]
pub fn dismiss_update(state: tauri::State<'_, TauriAppState>) -> Result<(), String> {
    let app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;
    *app.update_state.lock().unwrap() = UpdateCheckState::Idle;
    Ok(())
}
