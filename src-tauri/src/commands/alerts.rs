use crate::ipc_types::AlertCountdownDto;
use crate::TauriAppState;
use dota2_scripts::observability::alerts::{self, AlertEvent};

fn event_from_key(key: &str) -> Option<AlertEvent> {
    AlertEvent::ALL.into_iter().find(|e| e.key() == key)
}

/// Countdowns for every alert event, for the alerts page.
#[tauri::command]
pub fn get_alert_countdowns() -> Vec<AlertCountdownDto> {
    alerts::latest_countdowns()
        .into_iter()
        .map(|countdown| AlertCountdownDto {
            event: countdown.event.key().to_string(),
            display_name: countdown.event.display_name().to_string(),
            enabled: countdown.enabled,
            next_occurrence_seconds: countdown.next_occurrence_seconds,
            seconds_until: countdown.seconds_until,
        })
        .collect()
}

/// Voice packs available under `assets/voice/`.
#[tauri::command]
pub fn list_voice_packs() -> Vec<String> {
    use dota2_scripts::audio::voice_pack::{list_packs, VOICE_PACK_DIR};
    list_packs(std::path::Path::new(VOICE_PACK_DIR))
}

/// Play one event's cue immediately, so the operator can hear what they are
/// configuring without waiting for the objective to come round.
#[tauri::command]
pub fn test_play_alert(
    event: String,
    state: tauri::State<'_, TauriAppState>,
) -> Result<(), String> {
    let alert_event =
        event_from_key(&event).ok_or_else(|| format!("Unknown alert event: {}", event))?;

    let config = {
        let settings = state
            .settings
            .lock()
            .map_err(|e| format!("Failed to lock settings: {}", e))?;
        settings.alerts.clone()
    };

    alerts::play_alert(alert_event, &config);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_event_key_round_trips() {
        for event in AlertEvent::ALL {
            assert_eq!(event_from_key(event.key()), Some(event));
        }
    }

    #[test]
    fn an_unknown_key_resolves_to_nothing() {
        assert_eq!(event_from_key("roshan"), None);
        assert_eq!(event_from_key(""), None);
    }
}
