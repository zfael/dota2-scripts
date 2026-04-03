use crate::TauriAppState;
use dota2_scripts::config::Settings;
use tracing::info;

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

    validate_settings(&new_settings)?;
    new_settings
        .save()
        .map_err(|e| format!("Failed to write config: {}", e))?;

    *settings = new_settings;
    info!("Hero config '{}' updated and persisted", hero);

    Ok(())
}
