use crate::models::{GsiWebhookEvent, Hero};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeroType {
    Huskar,
    Largo,
    LegionCommander,
    ShadowFiend,
    Tiny,
}

impl HeroType {
    pub fn from_hero_name(name: &str) -> Option<Self> {
        match name {
            name if name == Hero::Huskar.to_game_name() => Some(HeroType::Huskar),
            name if name == Hero::Largo.to_game_name() => Some(HeroType::Largo),
            name if name == Hero::LegionCommander.to_game_name() => Some(HeroType::LegionCommander),
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
            // Update sf_enabled flag when hero changes via GSI
            // Must be explicitly set to false when not SF to avoid stale state
            *self.sf_enabled.lock().unwrap() = hero_type == Some(HeroType::ShadowFiend);
        }

        self.last_event = Some(event);
        self.metrics.events_processed += 1;
    }
}
