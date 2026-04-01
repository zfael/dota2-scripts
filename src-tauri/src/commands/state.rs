use crate::ipc_types::AppStateDto;
use crate::TauriAppState;
use dota2_scripts::state::HeroType;

/// Returns current app state (selected hero, enabled flags)
#[tauri::command]
pub fn get_app_state(state: tauri::State<'_, TauriAppState>) -> Result<AppStateDto, String> {
    let app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;

    Ok(AppStateDto {
        selected_hero: app.selected_hero.map(|h| h.to_display_name().to_string()),
        gsi_enabled: app.gsi_enabled,
        standalone_enabled: app.standalone_enabled,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Toggles GSI automation on/off
#[tauri::command]
pub fn set_gsi_enabled(
    enabled: bool,
    state: tauri::State<'_, TauriAppState>,
) -> Result<(), String> {
    let mut app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;
    app.gsi_enabled = enabled;
    Ok(())
}

/// Toggles standalone script on/off
#[tauri::command]
pub fn set_standalone_enabled(
    enabled: bool,
    state: tauri::State<'_, TauriAppState>,
) -> Result<(), String> {
    let mut app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;
    app.standalone_enabled = enabled;
    Ok(())
}

/// Manually selects a hero (or clears selection with null)
#[tauri::command]
pub fn select_hero(
    hero: Option<String>,
    state: tauri::State<'_, TauriAppState>,
) -> Result<(), String> {
    let mut app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;

    let hero_type = match hero {
        Some(name) => {
            let game_name = match name.as_str() {
                "Broodmother" => "npc_dota_hero_broodmother",
                "Huskar" => "npc_dota_hero_huskar",
                "Largo" => "npc_dota_hero_largo",
                "Legion Commander" => "npc_dota_hero_legion_commander",
                "Meepo" => "npc_dota_hero_meepo",
                "Outworld Destroyer" => "npc_dota_hero_obsidian_destroyer",
                "Shadow Fiend" => "npc_dota_hero_nevermore",
                "Tiny" => "npc_dota_hero_tiny",
                _ => return Err(format!("Unknown hero: {}", name)),
            };
            HeroType::from_hero_name(game_name)
        }
        None => None,
    };

    app.selected_hero = hero_type;

    if let Some(ht) = hero_type {
        *app.sf_enabled.lock().unwrap() = ht == HeroType::ShadowFiend;
        *app.od_enabled.lock().unwrap() = ht == HeroType::OutworldDestroyer;
    } else {
        *app.sf_enabled.lock().unwrap() = false;
        *app.od_enabled.lock().unwrap() = false;
    }

    Ok(())
}
