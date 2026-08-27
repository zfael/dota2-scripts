//! Draft advice commands: dataset status, token capture, and pick ranking.
//!
//! The token never travels back to the UI. `get_stratz_status` reports only
//! *whether* one is set; `get_config` blanks it (see `config::redacted`).

use crate::ipc_types::{DraftAdviceDto, MatchupDetailDto, StratzStatusDto, SuggestionDto};
use crate::TauriAppState;
use dota2_scripts::stratz::advice::advise;
use dota2_scripts::stratz::advisor::MatchupDetail;
use dota2_scripts::stratz::client::{StratzClient, StratzError};
use tauri::{AppHandle, Emitter};
use tracing::warn;

/// Dataset and token state behind the advice panel.
#[tauri::command]
pub fn get_stratz_status(state: tauri::State<'_, TauriAppState>) -> Result<StratzStatusDto, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|e| format!("Failed to lock settings: {}", e))?;
    // A token may come from the environment instead of config, so ask the
    // same resolver the worker uses rather than checking the config field.
    let has_token = !StratzClient::resolve_token(&settings.stratz.api_token).is_empty();
    let enabled = settings.stratz.enabled;
    let position = settings.stratz.position;
    let meta_only = settings.stratz.meta_only;
    drop(settings);

    let app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;

    let snapshot = app.stratz_status.clone().unwrap_or_default();
    Ok(StratzStatusDto {
        enabled,
        has_token,
        position,
        meta_only,
        ready: snapshot.ready,
        refreshing: snapshot.refreshing,
        progress: snapshot.progress,
        hero_count: snapshot.hero_count,
        incomplete_heroes: snapshot.incomplete_heroes,
        bracket: snapshot.bracket,
        built_at: snapshot.built_at,
        last_error: snapshot.last_error,
    })
}

/// Store a STRATZ API token and enable advice.
///
/// Validated against the API before saving, so a typo is reported here rather
/// than surfacing a minute later as a failed background refresh.
#[tauri::command]
pub fn set_stratz_token(
    token: String,
    app_handle: AppHandle,
    state: tauri::State<'_, TauriAppState>,
) -> Result<(), String> {
    let token = token.trim().to_string();
    if token.is_empty() {
        return Err("Token is empty".to_string());
    }

    // One cheap query proves the token works before it is written to disk.
    //
    // Only an outright rejection blocks the save. STRATZ goes through spells
    // of failing most requests with 503, and refusing a perfectly good token
    // because their service is down would make setup impossible exactly when
    // the user is trying to do it. Anything that is not a rejection is
    // accepted, and the background worker retries the refresh.
    let mut client = StratzClient::new(token.clone());
    match client.query("query { constants { heroes { id } } }", serde_json::Value::Null) {
        Ok(_) => {}
        Err(e @ StratzError::Unauthorized(_)) => return Err(e.to_string()),
        Err(e) => warn!("STRATZ: could not verify token now ({e}); saving it anyway"),
    }

    let mut settings = state
        .settings
        .lock()
        .map_err(|e| format!("Failed to lock settings: {}", e))?;
    settings.stratz.api_token = token;
    settings.stratz.enabled = true;
    settings
        .save()
        .map_err(|e| format!("Failed to write config: {}", e))?;

    // Broadcast the redacted copy so open windows re-render without ever
    // seeing the credential.
    let safe = crate::commands::config::redacted(&settings);
    drop(settings);
    let _ = app_handle.emit(crate::commands::config::CONFIG_UPDATED_EVENT, safe);
    Ok(())
}

/// Forget the stored token and turn advice off.
#[tauri::command]
pub fn clear_stratz_token(
    app_handle: AppHandle,
    state: tauri::State<'_, TauriAppState>,
) -> Result<(), String> {
    let mut settings = state
        .settings
        .lock()
        .map_err(|e| format!("Failed to lock settings: {}", e))?;
    settings.stratz.api_token = String::new();
    settings.stratz.enabled = false;
    settings
        .save()
        .map_err(|e| format!("Failed to write config: {}", e))?;

    let safe = crate::commands::config::redacted(&settings);
    drop(settings);
    let _ = app_handle.emit(crate::commands::config::CONFIG_UPDATED_EVENT, safe);
    Ok(())
}

/// Ask the background worker to rebuild the dataset now.
///
/// The refresh itself stays with the worker rather than running here: it is a
/// minute of throttled requests, and two of them racing would have both
/// writing the same cache file. This only raises the flag the worker takes on
/// its next pass, which is within a second.
#[tauri::command]
pub fn refresh_stratz_dataset(state: tauri::State<'_, TauriAppState>) -> Result<(), String> {
    let has_token = {
        let settings = state
            .settings
            .lock()
            .map_err(|e| format!("Failed to lock settings: {}", e))?;
        !StratzClient::resolve_token(&settings.stratz.api_token).is_empty()
    };
    if !has_token {
        return Err("Connect a STRATZ token first".to_string());
    }

    let mut app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;
    // Clicking twice must not queue a second rebuild behind the first.
    if app.stratz_status.as_ref().is_some_and(|s| s.refreshing) {
        return Err("A refresh is already running".to_string());
    }
    app.stratz_refresh_requested = true;
    Ok(())
}

/// Rank picks for the draft currently on screen.
///
/// Reads the live lineup from the draft reader, so the UI does not have to
/// send it back — the advice always matches what the reader believes.
#[tauri::command]
pub fn get_draft_advice(state: tauri::State<'_, TauriAppState>) -> Result<DraftAdviceDto, String> {
    let config = {
        let settings = state
            .settings
            .lock()
            .map_err(|e| format!("Failed to lock settings: {}", e))?;
        settings.stratz.clone()
    };

    let (dataset, slots) = {
        let app = state
            .app_state
            .lock()
            .map_err(|e| format!("Failed to lock app state: {}", e))?;
        // Cloning the Arc, not the matrices: ranking then runs without
        // holding the AppState lock across ~16k reads, so the draft reader
        // is never blocked by the advice panel.
        let dataset = app.stratz_dataset.clone();
        let slots = app.draft.as_ref().map(|d| d.slots.clone()).unwrap_or_default();
        (dataset, slots)
    };

    let Some(dataset) = dataset else {
        return Ok(DraftAdviceDto::default());
    };

    let advice = advise(&dataset, &slots, &config);
    Ok(DraftAdviceDto {
        suggestions: advice
            .suggestions
            .iter()
            .map(|s| SuggestionDto {
                slug: s.slug.clone(),
                display_name: s.display_name.clone(),
                score: s.score,
                counter: s.counter,
                synergy: s.synergy,
                position_win_rate: s.position_win_rate,
                pick_rate: s.pick_rate,
                best_against: s.best_against.as_ref().map(|(name, _)| name.clone()),
                vs_enemies: s.vs_enemies.iter().map(detail).collect(),
                with_allies: s.with_allies.iter().map(detail).collect(),
                counter_samples: s.counter_samples,
            })
            .collect(),
        unresolved: advice.unresolved,
        allies_used: advice.allies_used,
        enemies_used: advice.enemies_used,
    })
}

fn detail(d: &MatchupDetail) -> MatchupDetailDto {
    MatchupDetailDto {
        slug: d.slug.clone(),
        display_name: d.display_name.clone(),
        offset: d.offset,
        matches: d.matches,
        contribution: d.contribution,
    }
}
