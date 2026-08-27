use crate::models::{GsiWebhookEvent, Hero};
use crate::observability::minimap_capture_state::MinimapCaptureStatusSnapshot;
use crate::observability::rune_alerts::RuneAlertSnapshot;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

/// Dota only POSTs when game state changes, plus a keepalive on the `heartbeat`
/// interval in the GSI `.cfg` (30s in the config this app ships). A window
/// shorter than that heartbeat reports "Disconnected" whenever the game sits
/// still — the menu, the draft, or a dead hero — even though GSI is fine.
const GSI_ACTIVITY_TIMEOUT: Duration = Duration::from_secs(35);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeroType {
    EarthSpirit,
    EmberSpirit,
    Huskar,
    Invoker,
    Largo,
    LegionCommander,
    Magnus,
    Meepo,
    Mirana,
    OutworldDestroyer,
    ShadowFiend,
    Slark,
    Snapfire,
    Tiny,
}

/// Represents the current state of the auto-update check
#[derive(Debug, Clone)]
pub enum UpdateCheckState {
    /// No update check has been performed
    Idle,
    /// Currently checking for updates
    Checking,
    /// An update is available
    Available {
        version: String,
        release_notes: Option<String>,
    },
    /// Currently downloading the update
    Downloading,
    /// Update check or download failed
    Error(String),
    /// Already running the latest version
    UpToDate,
}

impl HeroType {
    pub fn from_hero_name(name: &str) -> Option<Self> {
        match name {
            name if name == Hero::EarthSpirit.to_game_name() => Some(HeroType::EarthSpirit),
            name if name == Hero::EmberSpirit.to_game_name() => Some(HeroType::EmberSpirit),
            name if name == Hero::Huskar.to_game_name() => Some(HeroType::Huskar),
            name if name == Hero::Invoker.to_game_name() => Some(HeroType::Invoker),
            name if name == Hero::Largo.to_game_name() => Some(HeroType::Largo),
            name if name == Hero::LegionCommander.to_game_name() => Some(HeroType::LegionCommander),
            name if name == Hero::Magnataur.to_game_name() => Some(HeroType::Magnus),
            name if name == Hero::Meepo.to_game_name() => Some(HeroType::Meepo),
            name if name == Hero::Mirana.to_game_name() => Some(HeroType::Mirana),
            name if name == Hero::ObsidianDestroyer.to_game_name() => {
                Some(HeroType::OutworldDestroyer)
            }
            name if name == Hero::Nevermore.to_game_name() => Some(HeroType::ShadowFiend),
            name if name == Hero::Slark.to_game_name() => Some(HeroType::Slark),
            name if name == Hero::Snapfire.to_game_name() => Some(HeroType::Snapfire),
            name if name == Hero::Tiny.to_game_name() => Some(HeroType::Tiny),
            _ => None,
        }
    }

    pub fn to_display_name(&self) -> &'static str {
        match self {
            HeroType::EarthSpirit => "Earth Spirit",
            HeroType::EmberSpirit => "Ember Spirit",
            HeroType::Huskar => "Huskar",
            HeroType::Invoker => "Invoker",
            HeroType::Largo => "Largo",
            HeroType::LegionCommander => "Legion Commander",
            HeroType::Magnus => "Magnus",
            HeroType::Meepo => "Meepo",
            HeroType::Mirana => "Mirana",
            HeroType::OutworldDestroyer => "Outworld Destroyer",
            HeroType::ShadowFiend => "Shadow Fiend",
            HeroType::Slark => "Slark",
            HeroType::Snapfire => "Snapfire",
            HeroType::Tiny => "Tiny",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct QueueMetrics {
    pub events_processed: u64,
    pub events_dropped: u64,
    /// Payloads Dota sent that did not deserialize. Non-zero here means the
    /// schema in `src/models/gsi_event.rs` has drifted from what the game sends.
    pub events_rejected: u64,
    pub current_queue_depth: usize,
}

#[derive(Debug, Clone)]
pub struct InvokerComboProfileState {
    pub id: String,
    pub enabled: bool,
    pub mode: String,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub selected_hero: Option<HeroType>,
    pub gsi_enabled: bool,
    pub standalone_enabled: bool,
    pub last_event: Option<GsiWebhookEvent>,
    pub last_gsi_activity_at: Option<SystemTime>,
    pub metrics: QueueMetrics,
    pub trigger_key: Arc<Mutex<String>>,
    pub sf_enabled: Arc<Mutex<bool>>,
    pub od_enabled: Arc<Mutex<bool>>,
    pub update_state: Arc<Mutex<UpdateCheckState>>,
    pub invoker_active_combo_profile_id: Option<String>,
    pub rune_alerts: Option<RuneAlertSnapshot>,
    pub minimap_capture: Option<MinimapCaptureStatusSnapshot>,
    /// Live draft identification, published by the draft reader worker.
    pub draft: Option<crate::observability::draft_reader::DraftStatusSnapshot>,
    /// User corrections (slot index, actual hero) queued for the draft reader
    /// to harvest as labelled exemplars; drained every capture frame.
    pub draft_corrections: Vec<(usize, String)>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            selected_hero: None,
            gsi_enabled: true,
            standalone_enabled: true,
            last_event: None,
            last_gsi_activity_at: None,
            metrics: QueueMetrics::default(),
            trigger_key: Arc::new(Mutex::new("Home".to_string())),
            sf_enabled: Arc::new(Mutex::new(false)),
            od_enabled: Arc::new(Mutex::new(false)),
            update_state: Arc::new(Mutex::new(UpdateCheckState::Idle)),
            invoker_active_combo_profile_id: None,
            rune_alerts: None,
            minimap_capture: None,
            draft: None,
            draft_corrections: Vec::new(),
        }
    }
}

impl AppState {
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::default()))
    }

    /// Returns `true` when this event changed the active hero.
    ///
    /// Callers use that to rebuild caches keyed on the hero — notably the
    /// `KeyboardSnapshot`, whose per-hero intercept flags stay stale until
    /// something rebuilds it.
    pub fn update_from_gsi(&mut self, event: GsiWebhookEvent) -> bool {
        // Update hero selection based on the GSI event if it changed
        let hero_type = HeroType::from_hero_name(&event.hero.name);
        let hero_changed = self.selected_hero != hero_type;

        if hero_changed {
            self.selected_hero = hero_type;
            *self.sf_enabled.lock().unwrap() = hero_type == Some(HeroType::ShadowFiend);
            *self.od_enabled.lock().unwrap() = hero_type == Some(HeroType::OutworldDestroyer);
        }

        self.last_event = Some(event);
        self.last_gsi_activity_at = Some(SystemTime::now());
        self.metrics.events_processed += 1;

        hero_changed
    }

    pub fn has_recent_gsi_activity(&self) -> bool {
        self.last_gsi_activity_at
            .and_then(|last_seen| SystemTime::now().duration_since(last_seen).ok())
            .map(|elapsed| elapsed <= GSI_ACTIVITY_TIMEOUT)
            .unwrap_or(false)
    }

    pub fn repair_invoker_active_combo(
        &mut self,
        profiles: &[InvokerComboProfileState],
    ) -> Option<String> {
        let active_id = self
            .invoker_active_combo_profile_id
            .as_ref()
            .and_then(|id| {
                profiles.iter().find(|profile| {
                    profile.enabled && profile.mode == "combo" && profile.id == *id
                })
            })
            .map(|profile| profile.id.clone());

        let next_active_id = active_id.or_else(|| {
            profiles
                .iter()
                .find(|profile| profile.enabled && profile.mode == "combo")
                .map(|profile| profile.id.clone())
        });

        self.invoker_active_combo_profile_id = next_active_id.clone();
        next_active_id
    }
}

#[cfg(test)]
mod tests {
    use super::{AppState, HeroType, InvokerComboProfileState};
    use crate::models::{Hero, GsiWebhookEvent};

    fn event_for(hero_name: &str) -> GsiWebhookEvent {
        let mut event = GsiWebhookEvent::default();
        event.hero.name = hero_name.to_string();
        event
    }

    #[test]
    fn update_from_gsi_reports_the_first_hero_detection_as_a_change() {
        let mut state = AppState::default();

        // The keyboard snapshot is built before any GSI event arrives, so this
        // first transition is what makes per-hero intercepts go live.
        assert!(state.update_from_gsi(event_for(Hero::Magnataur.to_game_name())));
        assert_eq!(state.selected_hero, Some(HeroType::Magnus));
    }

    #[test]
    fn update_from_gsi_reports_no_change_while_the_hero_stays_the_same() {
        let mut state = AppState::default();
        state.update_from_gsi(event_for(Hero::Magnataur.to_game_name()));

        assert!(!state.update_from_gsi(event_for(Hero::Magnataur.to_game_name())));
    }

    #[test]
    fn update_from_gsi_reports_a_change_when_the_hero_swaps() {
        let mut state = AppState::default();
        state.update_from_gsi(event_for(Hero::Magnataur.to_game_name()));

        assert!(state.update_from_gsi(event_for(Hero::Nevermore.to_game_name())));
        assert_eq!(state.selected_hero, Some(HeroType::ShadowFiend));
        assert!(*state.sf_enabled.lock().unwrap());
    }

    #[test]
    fn meepo_maps_into_hero_type() {
        let game_name = Hero::Meepo.to_game_name();
        assert_eq!(HeroType::from_hero_name(game_name), Some(HeroType::Meepo));
        assert_eq!(HeroType::Meepo.to_display_name(), "Meepo");
    }

    #[test]
    fn snapfire_hero_type_maps_name_and_display() {
        assert_eq!(
            HeroType::from_hero_name(crate::models::Hero::Snapfire.to_game_name()),
            Some(HeroType::Snapfire)
        );
        assert_eq!(HeroType::Snapfire.to_display_name(), "Snapfire");
    }

    #[test]
    fn magnus_hero_type_maps_name_and_display() {
        assert_eq!(
            HeroType::from_hero_name(crate::models::Hero::Magnataur.to_game_name()),
            Some(HeroType::Magnus)
        );
        assert_eq!(HeroType::Magnus.to_display_name(), "Magnus");
    }

    #[test]
    fn mirana_hero_type_maps_name_and_display() {
        assert_eq!(
            HeroType::from_hero_name(crate::models::Hero::Mirana.to_game_name()),
            Some(HeroType::Mirana)
        );
        assert_eq!(HeroType::Mirana.to_display_name(), "Mirana");
    }

    #[test]
    fn ember_spirit_hero_type_maps_name_and_display() {
        assert_eq!(
            HeroType::from_hero_name(crate::models::Hero::EmberSpirit.to_game_name()),
            Some(HeroType::EmberSpirit)
        );
        assert_eq!(HeroType::EmberSpirit.to_display_name(), "Ember Spirit");
    }

    #[test]
    fn slark_hero_type_maps_name_and_display() {
        assert_eq!(
            HeroType::from_hero_name(crate::models::Hero::Slark.to_game_name()),
            Some(HeroType::Slark)
        );
        assert_eq!(HeroType::Slark.to_display_name(), "Slark");
    }

    #[test]
    fn invoker_round_trips_from_game_name() {
        assert_eq!(
            HeroType::from_hero_name(crate::models::Hero::Invoker.to_game_name()),
            Some(HeroType::Invoker)
        );
        assert_eq!(HeroType::Invoker.to_display_name(), "Invoker");
    }

    #[test]
    fn app_state_defaults_invoker_active_combo_profile_id_to_none() {
        let state = AppState::default();
        assert_eq!(state.invoker_active_combo_profile_id, None);
    }

    #[test]
    fn repair_invoker_active_combo_ignores_prep_profiles_and_uses_first_enabled_combo() {
        let mut state = AppState::default();
        state.invoker_active_combo_profile_id = Some("invalid".to_string());

        let profiles = vec![
            InvokerComboProfileState {
                id: "prep".to_string(),
                enabled: true,
                mode: "prep".to_string(),
            },
            InvokerComboProfileState {
                id: "combo-a".to_string(),
                enabled: true,
                mode: "combo".to_string(),
            },
            InvokerComboProfileState {
                id: "combo-b".to_string(),
                enabled: true,
                mode: "combo".to_string(),
            },
        ];

        assert_eq!(
            state.repair_invoker_active_combo(&profiles),
            Some("combo-a".to_string())
        );
        assert_eq!(
            state.invoker_active_combo_profile_id,
            Some("combo-a".to_string())
        );
    }
}
