use crate::state::{AppState, HeroType, UpdateCheckState};
use crate::config::Settings;
use crate::update::{apply_update, restart_application, ApplyUpdateResult};
use eframe::egui;
use std::sync::{Arc, Mutex};

#[derive(PartialEq)]
enum Tab {
    Main,
    DangerDetection,
    Settings,
}

pub struct Dota2ScriptApp {
    app_state: Arc<Mutex<AppState>>,
    settings: Arc<Mutex<Settings>>,
    selected_tab: Tab,
    update_dismissed: bool,
}

impl Dota2ScriptApp {
    pub fn new(app_state: Arc<Mutex<AppState>>, settings: Arc<Mutex<Settings>>) -> Self {
        Self { 
            app_state,
            settings,
            selected_tab: Tab::Main,
            update_dismissed: false,
        }
    }
}

impl eframe::App for Dota2ScriptApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Request continuous repaints for real-time updates
        ctx.request_repaint();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Dota 2 Script Automation");
            
            // Update notification banner
            self.render_update_banner(ui);
            
            ui.separator();

            // Tab selection
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.selected_tab, Tab::Main, "Main");
                ui.selectable_value(&mut self.selected_tab, Tab::DangerDetection, "Danger Detection");
                ui.selectable_value(&mut self.selected_tab, Tab::Settings, "Settings");
            });
            
            ui.separator();
            
            // Add scroll area for all content
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    match self.selected_tab {
                        Tab::Main => self.render_main_tab(ui),
                        Tab::DangerDetection => self.render_danger_detection_tab(ui),
                        Tab::Settings => self.render_settings_tab(ui),
                    }
                });
        });
    }
}

impl Dota2ScriptApp {
    fn render_main_tab(&mut self, ui: &mut egui::Ui) {
        let settings = self.settings.lock().unwrap();

            // Hero Selection Section
            ui.heading("Hero Selection");
            {
                let mut state = self.app_state.lock().unwrap();
                let prev_hero = state.selected_hero;
                
                // Show current hero prominently
                ui.horizontal(|ui| {
                    ui.label("Active Hero:");
                    let hero_display = match state.selected_hero {
                        Some(hero) => hero.to_display_name(),
                        None => "None (waiting for GSI)",
                    };
                    ui.strong(hero_display);
                });
                
                // Collapsible manual override section
                ui.collapsing("Manual Override", |ui| {
                    ui.label("Hero is auto-detected from game. Override if needed:");
                    ui.horizontal_wrapped(|ui| {
                        if ui.selectable_label(state.selected_hero.is_none(), "None").clicked() {
                            state.selected_hero = None;
                        }
                        if ui.selectable_label(state.selected_hero == Some(HeroType::Huskar), "Huskar").clicked() {
                            state.selected_hero = Some(HeroType::Huskar);
                        }
                        if ui.selectable_label(state.selected_hero == Some(HeroType::Largo), "Largo").clicked() {
                            state.selected_hero = Some(HeroType::Largo);
                        }
                        if ui.selectable_label(state.selected_hero == Some(HeroType::LegionCommander), "Legion Commander").clicked() {
                            state.selected_hero = Some(HeroType::LegionCommander);
                        }
                        if ui.selectable_label(state.selected_hero == Some(HeroType::ShadowFiend), "Shadow Fiend").clicked() {
                            state.selected_hero = Some(HeroType::ShadowFiend);
                        }
                        if ui.selectable_label(state.selected_hero == Some(HeroType::Tiny), "Tiny").clicked() {
                            state.selected_hero = Some(HeroType::Tiny);
                        }
                    });
                });
                
                // Update trigger key when hero changes
                if state.selected_hero != prev_hero {
                    // Update SF enabled flag for keyboard listener
                    *state.sf_enabled.lock().unwrap() = state.selected_hero == Some(HeroType::ShadowFiend);
                    
                    if let Some(hero_type) = state.selected_hero {
                        let _hero_name = match hero_type {
                            HeroType::Huskar => "huskar",
                            HeroType::Largo => "largo",
                            HeroType::LegionCommander => "legion_commander",
                            HeroType::ShadowFiend => "shadow_fiend",
                            HeroType::Tiny => "tiny",
                        };
                        let new_key = settings.get_standalone_key(
                            match hero_type {
                                HeroType::Huskar => "huskar",
                                HeroType::Largo => "largo",
                                HeroType::LegionCommander => "legion_commander",
                                HeroType::ShadowFiend => "shadow_fiend",
                                HeroType::Tiny => "tiny",
                            }
                        );
                        *state.trigger_key.lock().unwrap() = new_key;
                    }
                }
            }
            
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
                    
                    // Danger indicator
                    if crate::actions::danger_detector::is_in_danger() {
                        ui.colored_label(egui::Color32::RED, "⚠️ IN DANGER");
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
    }
    
    fn render_danger_detection_tab(&mut self, ui: &mut egui::Ui) {
        let mut settings = self.settings.lock().unwrap();
        let config = &mut settings.danger_detection;
        
        ui.heading("Danger Detection Settings");
        ui.separator();
        
        ui.checkbox(&mut config.enabled, "Enable Danger Detection");
        ui.label("Detects when hero takes rapid damage or HP drops below threshold");
        
        ui.add_space(10.0);
        
        // Detection Thresholds
        ui.heading("Detection Thresholds");
        
        ui.horizontal(|ui| {
            ui.label("HP Threshold:");
            ui.add(egui::Slider::new(&mut config.hp_threshold_percent, 30..=90).suffix("%"));
        });
        ui.label("Danger detected when HP drops below this percentage");
        
        ui.horizontal(|ui| {
            ui.label("Rapid Loss Threshold:");
            ui.add(egui::Slider::new(&mut config.rapid_loss_hp, 50..=300).suffix(" HP"));
        });
        ui.label("HP loss amount to trigger danger detection");
        
        ui.horizontal(|ui| {
            ui.label("Time Window:");
            ui.add(egui::Slider::new(&mut config.time_window_ms, 200..=1000).suffix(" ms"));
        });
        ui.label("Time window to measure HP loss rate");
        
        ui.horizontal(|ui| {
            ui.label("Clear Delay:");
            ui.add(egui::Slider::new(&mut config.clear_delay_seconds, 1..=10).suffix(" s"));
        });
        ui.label("Time before danger state clears after HP stabilizes");
        
        ui.add_space(10.0);
        ui.separator();
        
        // Healing Configuration
        ui.heading("Healing in Danger");
        
        ui.horizontal(|ui| {
            ui.label("HP Threshold (in danger):");
            ui.add(egui::Slider::new(&mut config.healing_threshold_in_danger, 30..=80).suffix("%"));
        });
        ui.label("Use healing items when HP below this % (when in danger)");
        
        ui.horizontal(|ui| {
            ui.label("Max Healing Items:");
            ui.add(egui::Slider::new(&mut config.max_healing_items_per_danger, 1..=5));
        });
        ui.label("Maximum number of healing items to use per danger event");
        
        ui.add_space(10.0);
        ui.separator();
        
        // Defensive Items
        ui.heading("Auto-Trigger Defensive Items");
        ui.label("Automatically use defensive items when danger is detected");
        
        ui.add_space(5.0);
        
        egui::CollapsingHeader::new("Defensive Item Configuration")
            .default_open(true)
            .show(ui, |ui| {
                ui.checkbox(&mut config.auto_bkb, "Black King Bar (BKB)");
                ui.label("  Grants magic immunity");
                
                ui.checkbox(&mut config.auto_satanic, "Satanic");
                ui.label("  Active lifesteal for healing");
                if config.auto_satanic {
                    ui.add(egui::Slider::new(&mut config.satanic_hp_threshold, 10..=80)
                        .text("Satanic HP Threshold %"));
                    ui.label("  Use Satanic when HP drops below this percentage");
                }
                
                ui.checkbox(&mut config.auto_blade_mail, "Blade Mail");
                ui.label("  Reflects damage back to attackers");
                
                ui.checkbox(&mut config.auto_glimmer_cape, "Glimmer Cape");
                ui.label("  Magic resistance and invisibility");
                
                ui.checkbox(&mut config.auto_ghost_scepter, "Ghost Scepter");
                ui.label("  Physical immunity (ethereal form)");
                
                ui.checkbox(&mut config.auto_shivas_guard, "Shiva's Guard");
                ui.label("  AoE damage and armor");
            });
        
        ui.add_space(10.0);
        ui.separator();
        
        // Save button
        if ui.button("Save Configuration").clicked() {
            if let Err(e) = settings.save() {
                tracing::error!("Failed to save settings: {}", e);
            } else {
                tracing::info!("Danger detection settings saved");
            }
        }
    }

    fn render_update_banner(&mut self, ui: &mut egui::Ui) {
        let update_state = self.app_state.lock().unwrap().update_state.clone();
        let state = update_state.lock().unwrap().clone();

        match state {
            UpdateCheckState::Checking => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Checking for updates...");
                });
            }
            UpdateCheckState::Available { ref version, ref release_notes } if !self.update_dismissed => {
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(50, 80, 50))
                    .inner_margin(8.0)
                    .rounding(4.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::GREEN, format!("✨ Update available: v{}", version));
                            
                            if ui.button("Update Now").clicked() {
                                self.start_update();
                            }
                            if ui.button("Later").clicked() {
                                self.update_dismissed = true;
                            }
                        });
                        
                        if let Some(ref notes) = release_notes {
                            if !notes.is_empty() {
                                ui.collapsing("Release Notes", |ui| {
                                    // Clean up markdown for display
                                    let cleaned = Self::clean_release_notes(notes);
                                    
                                    egui::ScrollArea::vertical()
                                        .max_height(150.0)
                                        .show(ui, |ui| {
                                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                                            ui.label(egui::RichText::new(cleaned).size(12.0));
                                        });
                                });
                            }
                        }
                    });
            }
            UpdateCheckState::Downloading => {
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(50, 60, 80))
                    .inner_margin(8.0)
                    .rounding(4.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Downloading update...");
                        });
                    });
            }
            UpdateCheckState::Error(ref msg) => {
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(80, 50, 50))
                    .inner_margin(8.0)
                    .rounding(4.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::RED, format!("❌ Update error: {}", msg));
                            if ui.button("Retry").clicked() {
                                self.retry_update_check();
                            }
                            if ui.button("Dismiss").clicked() {
                                let update_state = self.app_state.lock().unwrap().update_state.clone();
                                *update_state.lock().unwrap() = UpdateCheckState::Idle;
                            }
                        });
                    });
            }
            _ => {} // Idle, UpToDate - don't show anything
        }
    }

    fn start_update(&self) {
        let update_state = self.app_state.lock().unwrap().update_state.clone();
        *update_state.lock().unwrap() = UpdateCheckState::Downloading;

        let update_state_clone = update_state.clone();
        std::thread::spawn(move || {
            match apply_update() {
                ApplyUpdateResult::Success { new_version } => {
                    tracing::info!("Update to v{} successful, restarting...", new_version);
                    if let Err(e) = restart_application() {
                        *update_state_clone.lock().unwrap() = UpdateCheckState::Error(
                            format!("Update applied but restart failed: {}. Please restart manually.", e)
                        );
                    }
                }
                ApplyUpdateResult::UpToDate => {
                    *update_state_clone.lock().unwrap() = UpdateCheckState::UpToDate;
                }
                ApplyUpdateResult::Error(msg) => {
                    *update_state_clone.lock().unwrap() = UpdateCheckState::Error(msg);
                }
            }
        });
    }

    /// Clean up markdown release notes for plain text display
    fn clean_release_notes(notes: &str) -> String {
        notes
            .lines()
            .map(|line| {
                let trimmed = line.trim();
                // Convert markdown headers to plain text with emoji indicators
                if trimmed.starts_with("####") {
                    format!("  {}", trimmed.trim_start_matches('#').trim())
                } else if trimmed.starts_with("###") {
                    format!("\n{}", trimmed.trim_start_matches('#').trim())
                } else if trimmed.starts_with("##") {
                    trimmed.trim_start_matches('#').trim().to_string()
                } else if trimmed.starts_with('#') {
                    trimmed.trim_start_matches('#').trim().to_string()
                } else if trimmed.starts_with("- ") {
                    // Keep list items but clean them up
                    format!("  • {}", trimmed.trim_start_matches("- "))
                } else if trimmed.starts_with("* ") {
                    format!("  • {}", trimmed.trim_start_matches("* "))
                } else if trimmed.starts_with("**") && trimmed.ends_with("**") {
                    // Bold text - just remove the markers
                    trimmed.trim_matches('*').to_string()
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
            // Remove excessive blank lines
            .replace("\n\n\n", "\n\n")
            // Truncate if too long
            .chars()
            .take(1000)
            .collect()
    }

    fn retry_update_check(&self) {
        let update_state = self.app_state.lock().unwrap().update_state.clone();
        let include_prereleases = self.settings.lock().unwrap().updates.include_prereleases;
        
        *update_state.lock().unwrap() = UpdateCheckState::Checking;

        let update_state_clone = update_state.clone();
        std::thread::spawn(move || {
            match crate::update::check_for_update(include_prereleases) {
                crate::update::UpdateCheckResult::Available(info) => {
                    *update_state_clone.lock().unwrap() = UpdateCheckState::Available {
                        version: info.version,
                        release_notes: info.release_notes,
                    };
                }
                crate::update::UpdateCheckResult::UpToDate => {
                    *update_state_clone.lock().unwrap() = UpdateCheckState::UpToDate;
                }
                crate::update::UpdateCheckResult::Error(msg) => {
                    *update_state_clone.lock().unwrap() = UpdateCheckState::Error(msg);
                }
            }
        });
    }

    fn render_settings_tab(&mut self, ui: &mut egui::Ui) {
        let mut settings = self.settings.lock().unwrap();
        
        ui.heading("Application Settings");
        ui.separator();
        
        // Update Settings
        ui.heading("Auto-Update");
        ui.add_space(5.0);
        
        ui.checkbox(&mut settings.updates.check_on_startup, "Check for updates on startup");
        ui.label("Automatically check for new versions when the application starts");
        
        ui.add_space(5.0);
        
        ui.checkbox(&mut settings.updates.include_prereleases, "Include pre-releases");
        ui.label("Include release candidates (RC), alpha, and beta versions");
        
        ui.add_space(10.0);
        
        // Current version info
        ui.horizontal(|ui| {
            ui.label("Current version:");
            ui.strong(format!("v{}", env!("CARGO_PKG_VERSION")));
        });
        
        // Manual update check button
        ui.add_space(5.0);
        if ui.button("Check for Updates Now").clicked() {
            self.retry_update_check();
        }
        
        // Show current update state
        let update_state = self.app_state.lock().unwrap().update_state.clone();
        let state = update_state.lock().unwrap().clone();
        match state {
            UpdateCheckState::UpToDate => {
                ui.colored_label(egui::Color32::GREEN, "✅ You're running the latest version");
            }
            UpdateCheckState::Available { version, .. } => {
                ui.colored_label(egui::Color32::YELLOW, format!("✨ Update available: v{}", version));
            }
            _ => {}
        }
        
        ui.add_space(20.0);
        ui.separator();
        
        // Save button
        if ui.button("Save Settings").clicked() {
            if let Err(e) = settings.save() {
                tracing::error!("Failed to save settings: {}", e);
            } else {
                tracing::info!("Settings saved");
            }
        }
    }
}
