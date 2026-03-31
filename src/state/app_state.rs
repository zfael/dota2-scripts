use crate::models::{GsiWebhookEvent, Hero};
use crate::observability::minimap_capture_state::MinimapCaptureStatusSnapshot;
use crate::observability::rune_alerts::RuneAlertSnapshot;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeroType {
    Huskar,
    Largo,
    LegionCommander,
    Meepo,
    OutworldDestroyer,
    ShadowFiend,
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
            name if name == Hero::Huskar.to_game_name() => Some(HeroType::Huskar),
            name if name == Hero::Largo.to_game_name() => Some(HeroType::Largo),
            name if name == Hero::LegionCommander.to_game_name() => Some(HeroType::LegionCommander),
            name if name == Hero::Meepo.to_game_name() => Some(HeroType::Meepo),
            name if name == Hero::ObsidianDestroyer.to_game_name() => {
                Some(HeroType::OutworldDestroyer)
            }
            name if name == Hero::Nevermore.to_game_name() => Some(HeroType::ShadowFiend),
            name if name == Hero::Tiny.to_game_name() => Some(HeroType::Tiny),
            _ => None,
        }
    }

    pub fn to_display_name(&self) -> &'static str {
        match self {
            HeroType::Huskar => "Huskar",
            HeroType::Largo => "Largo",
            HeroType::LegionCommander => "Legion Commander",
            HeroType::Meepo => "Meepo",
            HeroType::OutworldDestroyer => "Outworld Destroyer",
            HeroType::ShadowFiend => "Shadow Fiend",
            HeroType::Tiny => "Tiny",
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueueMetrics {
    pub events_processed: u64,
    pub events_dropped: u64,
    pub current_queue_depth: usize,
}

impl Default for QueueMetrics {
    fn default() -> Self {
        Self {
            events_processed: 0,
            events_dropped: 0,
            current_queue_depth: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub selected_hero: Option<HeroType>,
    pub gsi_enabled: bool,
    pub standalone_enabled: bool,
    pub last_event: Option<GsiWebhookEvent>,
    pub metrics: QueueMetrics,
    pub trigger_key: Arc<Mutex<String>>,
    pub sf_enabled: Arc<Mutex<bool>>,
    pub od_enabled: Arc<Mutex<bool>>,
    pub update_state: Arc<Mutex<UpdateCheckState>>,
    pub rune_alerts: Option<RuneAlertSnapshot>,
    pub minimap_capture: Option<MinimapCaptureStatusSnapshot>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            selected_hero: None,
            gsi_enabled: true,
            standalone_enabled: true,
            last_event: None,
            metrics: QueueMetrics::default(),
            trigger_key: Arc::new(Mutex::new("Home".to_string())),
            sf_enabled: Arc::new(Mutex::new(false)),
            od_enabled: Arc::new(Mutex::new(false)),
            update_state: Arc::new(Mutex::new(UpdateCheckState::Idle)),
            rune_alerts: None,
            minimap_capture: None,
        }
    }
}

impl AppState {
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::default()))
    }

    pub fn update_from_gsi(&mut self, event: GsiWebhookEvent) {
        // Update hero selection based on the GSI event if it changed
        let hero_type = HeroType::from_hero_name(&event.hero.name);

        if self.selected_hero != hero_type {
            self.selected_hero = hero_type;
            *self.sf_enabled.lock().unwrap() = hero_type == Some(HeroType::ShadowFiend);
            *self.od_enabled.lock().unwrap() = hero_type == Some(HeroType::OutworldDestroyer);
        }

        self.last_event = Some(event);
        self.metrics.events_processed += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::{HeroType};
    use crate::models::Hero;

    #[test]
    fn meepo_maps_into_hero_type() {
        let game_name = Hero::Meepo.to_game_name();
        assert_eq!(HeroType::from_hero_name(game_name), Some(HeroType::Meepo));
        assert_eq!(HeroType::Meepo.to_display_name(), "Meepo");
    }
}
