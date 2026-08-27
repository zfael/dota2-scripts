//! Draft page commands: live identification status, and the feedback loop
//! that turns the user's own sanity checks into labelled training data.

use crate::ipc_types::{DraftSlotDto, DraftStatusDto};
use crate::TauriAppState;
use std::io::Write;

/// Current draft identification state, as published by the reader worker.
#[tauri::command]
pub fn get_draft_status(state: tauri::State<'_, TauriAppState>) -> Result<DraftStatusDto, String> {
    let app = state
        .app_state
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;

    let dto = match &app.draft {
        Some(d) => DraftStatusDto {
            enabled: d.enabled,
            active: d.active,
            game_state: d.game_state.clone(),
            session_id: d.session_id.clone(),
            matchid: d.matchid.clone(),
            team_name: d.team_name.clone(),
            own_hero: d.own_hero.clone(),
            frames: d.frames,
            slots: d
                .slots
                .iter()
                .map(|s| DraftSlotDto {
                    index: s.index,
                    is_ally: s.is_ally,
                    hero: s.hero.clone(),
                    unknown: s.unknown,
                    agreement: s.agreement,
                    best_score: s.best_score,
                })
                .collect(),
        },
        None => DraftStatusDto {
            enabled: false,
            active: false,
            game_state: String::new(),
            session_id: String::new(),
            matchid: String::new(),
            team_name: String::new(),
            own_hero: String::new(),
            frames: 0,
            slots: Vec::new(),
        },
    };
    Ok(dto)
}

/// Record the user's verdict on one identified slot.
///
/// Feedback lands in the current session's telemetry directory as
/// `feedback.jsonl`, next to the frames it judges — a wrong read plus the
/// user-supplied correction is a labelled training example, and a confirmed
/// read is a regression anchor. The session dir outlives the draft ending, so
/// voting from the strategy screen or loading screen still lands correctly.
#[tauri::command]
pub fn submit_draft_feedback(
    slot_index: usize,
    correct: bool,
    actual_hero: Option<String>,
    state: tauri::State<'_, TauriAppState>,
) -> Result<(), String> {
    let (session_dir, snapshot_slot) = {
        let mut app = state
            .app_state
            .lock()
            .map_err(|e| format!("Failed to lock app state: {}", e))?;
        // A correction names what is on screen right now: queue it for the
        // reader worker, which harvests that slot's crop as a labelled
        // exemplar (guarded there — the name must be a known hero and the
        // slot must actually hold a portrait).
        if let (false, Some(hero)) = (correct, actual_hero.as_deref()) {
            let hero = hero.trim().to_lowercase().replace(' ', "_");
            if !hero.is_empty() {
                app.draft_corrections.push((slot_index, hero));
            }
        }
        let draft = app.draft.as_ref().ok_or("No draft session to judge")?;
        let dir = draft
            .session_dir
            .clone()
            .ok_or("Draft telemetry is disabled — nowhere to record feedback")?;
        let slot = draft.slots.get(slot_index).cloned();
        (dir, slot)
    };

    let line = serde_json::json!({
        "ts": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        "slot_index": slot_index,
        "correct": correct,
        // What the matcher claimed at the moment of judgement, so the verdict
        // stays meaningful even if the lineup changes afterwards.
        "identified": snapshot_slot.as_ref().and_then(|s| s.hero.clone()),
        "unknown": snapshot_slot.as_ref().map(|s| s.unknown),
        "actual_hero": actual_hero,
    });

    let path = std::path::Path::new(&session_dir).join("feedback.jsonl");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("Cannot open {}: {e}", path.display()))?;
    writeln!(file, "{line}").map_err(|e| format!("Cannot write feedback: {e}"))?;
    Ok(())
}
