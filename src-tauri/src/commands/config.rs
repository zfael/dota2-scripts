use crate::TauriAppState;
use dota2_scripts::config::Settings;
use std::fs;
use tracing::info;

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

    let toml_str =
        toml::to_string_pretty(&new_settings).map_err(|e| format!("TOML error: {}", e))?;
    fs::write("config/config.toml", &toml_str)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    *settings = new_settings;
    info!("Config section '{}' updated and persisted", section);

    Ok(())
}

/// Updates a hero-specific config section
#[tauri::command]
pub fn update_hero_config(
    hero: String,
    updates: serde_json::Value,
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

    let toml_str =
        toml::to_string_pretty(&new_settings).map_err(|e| format!("TOML error: {}", e))?;
    fs::write("config/config.toml", &toml_str)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    *settings = new_settings;
    info!("Hero config '{}' updated and persisted", hero);

    Ok(())
}
