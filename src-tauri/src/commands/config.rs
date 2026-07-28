use crate::TauriAppState;
use dota2_scripts::config::Settings;
use dota2_scripts::input::keyboard::KeyboardSnapshot;
use tauri::{AppHandle, Emitter};
use tracing::{info, warn};

/// Emitted after a successful config write so open windows re-render against the
/// new settings instead of the copy they loaded at startup.
pub const CONFIG_UPDATED_EVENT: &str = "config_updated";

/// Push the persisted settings to every window.
///
/// Broadcast rather than addressed to the non-editing windows: the JS `listen()`
/// helper registers as `EventTarget::Any`, so a label-based filter would match no
/// one. The window that made the edit already holds the same values and drops its
/// own echo while it still has debounced writes outstanding — see `configStore.ts`.
fn broadcast_config(app: &AppHandle, settings: &Settings) {
    if let Err(e) = app.emit(CONFIG_UPDATED_EVENT, settings) {
        warn!("Failed to broadcast config update: {}", e);
    }
}

fn validate_settings(settings: &Settings) -> Result<(), String> {
    if settings.server.port == 0 {
        return Err("Server port must be greater than 0".to_string());
    }

    let dd = &settings.danger_detection;
    if dd.hp_threshold_percent > 100 {
        return Err("Danger HP threshold must be 0-100".to_string());
    }
    if dd.satanic_hp_threshold > 100 {
        return Err("Satanic HP threshold must be 0-100".to_string());
    }

    if settings.common.survivability_hp_threshold > 100 {
        return Err("Survivability HP threshold must be 0-100".to_string());
    }

    let sr = &settings.soul_ring;
    if sr.min_mana_percent > 100 {
        return Err("Soul Ring min mana must be 0-100".to_string());
    }
    if sr.min_health_percent > 100 {
        return Err("Soul Ring min health must be 0-100".to_string());
    }

    let meepo = &settings.heroes.meepo;
    if meepo.dig_hp_threshold_percent > 100 {
        return Err("Meepo dig HP threshold must be 0-100".to_string());
    }
    if meepo.megameepo_hp_threshold_percent > 100 {
        return Err("Meepo MegaMeepo HP threshold must be 0-100".to_string());
    }

    Ok(())
}

/// Returns the full config as JSON (snake_case keys matching config.toml)
#[tauri::command]
pub fn get_config(state: tauri::State<'_, TauriAppState>) -> Result<Settings, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|e| format!("Failed to lock settings: {}", e))?;
    Ok(settings.clone())
}

/// Updates a config section and persists to config.toml
#[tauri::command]
pub fn update_config(
    section: String,
    updates: serde_json::Value,
    app_handle: AppHandle,
    state: tauri::State<'_, TauriAppState>,
) -> Result<(), String> {
    let mut settings = state
        .settings
        .lock()
        .map_err(|e| format!("Failed to lock settings: {}", e))?;

    let mut config_value =
        serde_json::to_value(&*settings).map_err(|e| format!("Serialize error: {}", e))?;

    if let Some(section_value) = config_value.get_mut(&section) {
        if let (Some(existing_obj), Some(update_obj)) =
            (section_value.as_object_mut(), updates.as_object())
        {
            for (key, value) in update_obj {
                existing_obj.insert(key.clone(), value.clone());
            }
        }
    } else {
        return Err(format!("Unknown config section: {}", section));
    }

    let new_settings: Settings =
        serde_json::from_value(config_value).map_err(|e| format!("Deserialize error: {}", e))?;

    validate_settings(&new_settings)?;
    new_settings
        .save()
        .map_err(|e| format!("Failed to write config: {}", e))?;

    *settings = new_settings;
    let app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;
    let snapshot = KeyboardSnapshot::from_runtime(&settings, &app);
    drop(app);
    let mut keyboard_snapshot = state
        .keyboard_snapshot
        .write()
        .map_err(|e| format!("Failed to lock keyboard snapshot: {}", e))?;
    *keyboard_snapshot = snapshot;
    drop(keyboard_snapshot);

    broadcast_config(&app_handle, &settings);
    info!("Config section '{}' updated and persisted", section);

    Ok(())
}

/// Repairs the active Invoker combo profile after hero config edits.
/// Only affects Invoker edits; no-op for other heroes.
fn repair_invoker_active_combo_after_hero_edit(
    hero: &str,
    state: &TauriAppState,
) -> Result<(), String> {
    if hero != "invoker" {
        return Ok(());
    }

    let settings = state
        .settings
        .lock()
        .map_err(|e| format!("Failed to lock settings: {}", e))?;

    let profiles: Vec<dota2_scripts::state::app_state::InvokerComboProfileState> = settings
        .heroes
        .invoker
        .profiles
        .iter()
        .map(|p| dota2_scripts::state::app_state::InvokerComboProfileState {
            id: p.id.clone(),
            enabled: p.enabled,
            mode: p.mode.as_str().to_string(),
        })
        .collect();

    drop(settings);

    let mut app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;

    app.repair_invoker_active_combo(&profiles);

    Ok(())
}

/// Updates a hero-specific config section
#[tauri::command]
pub fn update_hero_config(
    hero: String,
    updates: serde_json::Value,
    app_handle: AppHandle,
    state: tauri::State<'_, TauriAppState>,
) -> Result<(), String> {
    let mut settings = state
        .settings
        .lock()
        .map_err(|e| format!("Failed to lock settings: {}", e))?;

    let mut config_value =
        serde_json::to_value(&*settings).map_err(|e| format!("Serialize error: {}", e))?;

    let heroes_section = config_value
        .get_mut("heroes")
        .and_then(|h| h.as_object_mut())
        .ok_or("Missing heroes section")?;

    if let Some(hero_section) = heroes_section.get_mut(&hero) {
        if let (Some(existing_obj), Some(update_obj)) =
            (hero_section.as_object_mut(), updates.as_object())
        {
            for (key, value) in update_obj {
                existing_obj.insert(key.clone(), value.clone());
            }
        }
    } else {
        return Err(format!("Unknown hero: {}", hero));
    }

    let new_settings: Settings =
        serde_json::from_value(config_value).map_err(|e| format!("Deserialize error: {}", e))?;

    validate_settings(&new_settings)?;
    new_settings
        .save()
        .map_err(|e| format!("Failed to write config: {}", e))?;

    *settings = new_settings;
    drop(settings);

    repair_invoker_active_combo_after_hero_edit(&hero, &state)?;

    let settings = state
        .settings
        .lock()
        .map_err(|e| format!("Failed to lock settings: {}", e))?;
    let app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;
    let snapshot = KeyboardSnapshot::from_runtime(&settings, &app);
    let broadcast = settings.clone();
    drop(settings);
    drop(app);
    let mut keyboard_snapshot = state
        .keyboard_snapshot
        .write()
        .map_err(|e| format!("Failed to lock keyboard snapshot: {}", e))?;
    *keyboard_snapshot = snapshot;
    drop(keyboard_snapshot);

    broadcast_config(&app_handle, &broadcast);
    info!("Hero config '{}' updated and persisted", hero);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dota2_scripts::actions::executor::ExecutorMetrics;
    use dota2_scripts::config::settings::{
        InvokerProfile, InvokerProfileExecutionStyle, InvokerProfileMode,
    };
    use dota2_scripts::state::app_state::AppState;
    use std::sync::{Arc, Mutex, RwLock};

    fn make_test_profile(id: &str, enabled: bool, mode: InvokerProfileMode) -> InvokerProfile {
        InvokerProfile {
            id: id.to_string(),
            name: format!("{} Profile", id),
            enabled,
            hotkey: "F1".to_string(),
            mode,
            execution_style: InvokerProfileExecutionStyle::Automatic,
            build_tag: String::new(),
            steps: vec![],
        }
    }

    fn make_test_app_state() -> TauriAppState {
        TauriAppState {
            app_state: Arc::new(Mutex::new(AppState::default())),
            settings: Arc::new(Mutex::new(Settings::default())),
            keyboard_snapshot: Arc::new(RwLock::new(KeyboardSnapshot::default())),
            executor_metrics: ExecutorMetrics::new(),
        }
    }

    #[test]
    fn repair_invoker_active_combo_after_edit_valid_enabled_active_combo_remains_unchanged() {
        let state = make_test_app_state();
        {
            let mut app = state.app_state.lock().unwrap();
            app.invoker_active_combo_profile_id = Some("combo-a".to_string());
        }

        let mut settings = state.settings.lock().unwrap();
        settings.heroes.invoker.profiles = vec![
            make_test_profile("combo-a", true, InvokerProfileMode::Combo),
            make_test_profile("combo-b", true, InvokerProfileMode::Combo),
        ];
        drop(settings);

        repair_invoker_active_combo_after_hero_edit("invoker", &state).unwrap();

        let app = state.app_state.lock().unwrap();
        assert_eq!(
            app.invoker_active_combo_profile_id,
            Some("combo-a".to_string())
        );
    }

    #[test]
    fn repair_invoker_active_combo_after_edit_invalid_disabled_active_combo_falls_back_to_first_enabled(
    ) {
        let state = make_test_app_state();
        {
            let mut app = state.app_state.lock().unwrap();
            app.invoker_active_combo_profile_id = Some("combo-a".to_string());
        }

        let mut settings = state.settings.lock().unwrap();
        settings.heroes.invoker.profiles = vec![
            make_test_profile("combo-a", false, InvokerProfileMode::Combo),
            make_test_profile("combo-b", true, InvokerProfileMode::Combo),
        ];
        drop(settings);

        repair_invoker_active_combo_after_hero_edit("invoker", &state).unwrap();

        let app = state.app_state.lock().unwrap();
        assert_eq!(
            app.invoker_active_combo_profile_id,
            Some("combo-b".to_string())
        );
    }

    #[test]
    fn repair_invoker_active_combo_after_edit_non_invoker_hero_edits_do_not_mutate_active_combo_state(
    ) {
        let state = make_test_app_state();
        {
            let mut app = state.app_state.lock().unwrap();
            app.invoker_active_combo_profile_id = Some("original".to_string());
        }

        repair_invoker_active_combo_after_hero_edit("meepo", &state).unwrap();

        let app = state.app_state.lock().unwrap();
        assert_eq!(
            app.invoker_active_combo_profile_id,
            Some("original".to_string())
        );
    }
}
