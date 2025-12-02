use crate::state::{AppState, HeroType};
use crate::config::Settings;
use eframe::egui;
use std::sync::{Arc, Mutex};

pub struct Dota2ScriptApp {
    app_state: Arc<Mutex<AppState>>,
    settings: Settings,
}

impl Dota2ScriptApp {
    pub fn new(app_state: Arc<Mutex<AppState>>, settings: Settings) -> Self {
        Self { app_state, settings }
    }
}

impl eframe::App for Dota2ScriptApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Request continuous repaints for real-time updates
        ctx.request_repaint();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Dota 2 Script Automation");
            ui.separator();

            // Hero Selection Section
            ui.heading("Hero Selection");
            ui.horizontal(|ui| {
                let mut state = self.app_state.lock().unwrap();
                
                let prev_hero = state.selected_hero;
                
                if ui
                    .radio(state.selected_hero.is_none(), "None")
                    .clicked()
                {
                    state.selected_hero = None;
                }
                
                if ui
                    .radio(
                        state.selected_hero == Some(HeroType::Huskar),
                        "Huskar",
                    )
                    .clicked()
                {
                    state.selected_hero = Some(HeroType::Huskar);
                }
                
                if ui
                    .radio(
                        state.selected_hero == Some(HeroType::LegionCommander),
                        "Legion Commander",
                    )
                    .clicked()
                {
                    state.selected_hero = Some(HeroType::LegionCommander);
                }
                
                if ui
                    .radio(
                        state.selected_hero == Some(HeroType::ShadowFiend),
                        "Shadow Fiend",
                    )
                    .clicked()
                {
                    state.selected_hero = Some(HeroType::ShadowFiend);
                }
                
                if ui
                    .radio(
                        state.selected_hero == Some(HeroType::Tiny),
                        "Tiny",
                    )
                    .clicked()
                {
                    state.selected_hero = Some(HeroType::Tiny);
                }
                
                // Update trigger key when hero changes
                if state.selected_hero != prev_hero {
                    // Update SF enabled flag
                    *state.sf_enabled.lock().unwrap() = state.selected_hero == Some(HeroType::ShadowFiend);
                    
                    if let Some(hero_type) = state.selected_hero {
                        let _hero_name = match hero_type {
                            HeroType::Huskar => "huskar",
                            HeroType::LegionCommander => "legion_commander",
                            HeroType::ShadowFiend => "shadow_fiend",
                            HeroType::Tiny => "tiny",
                        };
                        let new_key = self.settings.get_standalone_key(
                            match hero_type {
                                HeroType::Huskar => "huskar",
                                HeroType::LegionCommander => "legion_commander",
                                HeroType::ShadowFiend => "shadow_fiend",
                                HeroType::Tiny => "tiny",
                            }
                        );
                        *state.trigger_key.lock().unwrap() = new_key;
                    }
                }
            });
            
            ui.add_space(10.0);
            
            // Keybinding Section
            ui.heading("Keybindings");
            {
                let state = self.app_state.lock().unwrap();
                
                if let Some(hero_type) = state.selected_hero {
                    match hero_type {
                        HeroType::ShadowFiend => {
                            ui.label("Shadow Fiend uses Q/W/E interception:");
                            ui.label("  Q: Right-click + L (close raze)");
                            ui.label("  W: Right-click + K (medium raze)");
                            ui.label("  E: Right-click + J (far raze)");
                            ui.label("Edit in config/config.toml under [heroes.shadow_fiend]");
                        }
                        _ => {
                            let current_key = state.trigger_key.lock().unwrap().clone();
                            ui.horizontal(|ui| {
                                ui.label(format!("Standalone Combo Key ({}): ", hero_type.to_display_name()));
                                ui.label(&current_key);
                            });
                            ui.label("Edit in config/config.toml under [heroes.<hero>.standalone_key]");
                        }
                    }
                } else {
                    ui.label("Select a hero to view keybindings");
                }
            }
            
            ui.add_space(10.0);

            // Automation Controls Section
            ui.heading("Automation Controls");
            {
                let mut state = self.app_state.lock().unwrap();
                
                ui.checkbox(&mut state.gsi_enabled, "Enable GSI Automation");
                
                let current_key = state.trigger_key.lock().unwrap().clone();
                ui.checkbox(&mut state.standalone_enabled, 
                    format!("Enable Standalone Script ({} key)", current_key));
            }
            
            ui.add_space(10.0);
            ui.separator();

            // Status Panel Section
            ui.heading("Status");
            {
                let state = self.app_state.lock().unwrap();
                
                if let Some(event) = &state.last_event {
                    ui.label(format!("Current Hero: {}", event.hero.name));
                    ui.label(format!("Level: {}", event.hero.level));
                    
                    // HP Bar
                    ui.horizontal(|ui| {
                        ui.label("HP:");
                        let hp_fraction = event.hero.health as f32 / event.hero.max_health.max(1) as f32;
                        let hp_bar = egui::ProgressBar::new(hp_fraction)
                            .text(format!("{}/{}", event.hero.health, event.hero.max_health))
                            .fill(egui::Color32::from_rgb(0, 255, 0));
                        ui.add(hp_bar);
                    });
                    
                    // Mana Bar
                    ui.horizontal(|ui| {
                        ui.label("Mana:");
                        let mana_fraction = event.hero.mana as f32 / event.hero.max_mana.max(1) as f32;
                        let mana_bar = egui::ProgressBar::new(mana_fraction)
                            .text(format!("{}/{}", event.hero.mana, event.hero.max_mana))
                            .fill(egui::Color32::from_rgb(100, 150, 255));
                        ui.add(mana_bar);
                    });
                    
                    // Status Effects
                    if event.hero.alive {
                        ui.label("Status: Alive");
                    } else {
                        ui.colored_label(egui::Color32::RED, format!("Respawning in: {}s", event.hero.respawn_seconds));
                    }
                    
                    if event.hero.stunned {
                        ui.colored_label(egui::Color32::YELLOW, "⚡ Stunned");
                    }
                    if event.hero.silenced {
                        ui.colored_label(egui::Color32::YELLOW, "🔇 Silenced");
                    }
                } else {
                    ui.label("No GSI events received yet");
                }
            }
            
            ui.add_space(10.0);
            ui.separator();

            // Debug Metrics Section
            ui.heading("Debug Metrics");
            {
                let state = self.app_state.lock().unwrap();
                ui.label(format!("Events Processed: {}", state.metrics.events_processed));
                ui.label(format!("Events Dropped: {}", state.metrics.events_dropped));
                ui.label(format!("Queue Depth: {}", state.metrics.current_queue_depth));
            }
            
            ui.add_space(10.0);
            ui.separator();
            
            // Instructions
            ui.heading("Instructions");
            ui.label("• GSI events will automatically trigger survivability actions");
            ui.label("• Hero-specific actions are based on the detected hero");
            ui.label("• Press HOME key to trigger standalone combo (if enabled)");
            ui.label("• Auto-selection: Hero is selected automatically from GSI events");
        });
    }
}
