use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_port")]
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingsConfig {
    #[serde(default = "default_slot0")]
    pub slot0: char,
    #[serde(default = "default_slot1")]
    pub slot1: char,
    #[serde(default = "default_slot2")]
    pub slot2: char,
    #[serde(default = "default_slot3")]
    pub slot3: char,
    #[serde(default = "default_slot4")]
    pub slot4: char,
    #[serde(default = "default_slot5")]
    pub slot5: char,
    #[serde(default = "default_neutral")]
    pub neutral0: char,
    #[serde(default = "default_hotkey")]
    pub combo_trigger: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonConfig {
    #[serde(default = "default_survivability_threshold")]
    pub survivability_hp_threshold: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HuskarConfig {
    #[serde(default = "default_armlet_threshold")]
    pub armlet_toggle_threshold: u32,
    #[serde(default = "default_armlet_offset")]
    pub armlet_predictive_offset: u32,
    #[serde(default = "default_armlet_cooldown")]
    pub armlet_toggle_cooldown_ms: u64,
    #[serde(default = "default_berserker_blood_key")]
    pub berserker_blood_key: char,
    #[serde(default = "default_berserker_blood_delay")]
    pub berserker_blood_delay_ms: u64,
    #[serde(default = "default_standalone_key")]
    pub standalone_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegionCommanderConfig {
    #[serde(default = "default_standalone_key")]
    pub standalone_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowFiendConfig {
    #[serde(default = "default_sf_raze_enabled")]
    pub raze_intercept_enabled: bool,
    #[serde(default = "default_raze_delay")]
    pub raze_delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TinyConfig {
    #[serde(default = "default_standalone_key")]
    pub standalone_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LargoConfig {
    #[serde(default = "default_amphibian_enabled")]
    pub amphibian_rhapsody_enabled: bool,
    #[serde(default = "default_auto_toggle_on_danger")]
    pub auto_toggle_on_danger: bool,
    #[serde(default = "default_largo_mana_threshold")]
    pub mana_threshold_percent: u32,
    #[serde(default = "default_largo_heal_threshold")]
    pub heal_hp_threshold: u32,
    #[serde(default = "default_beat_interval_ms")]
    pub beat_interval_ms: u32,
    #[serde(default = "default_beat_correction_ms")]
    pub beat_correction_ms: i32,  // Correction to apply (can be negative)
    #[serde(default = "default_beat_correction_every_n_beats")]
    pub beat_correction_every_n_beats: u32,  // Apply correction every N beats (0 = disabled)
    #[serde(default = "default_largo_q_key")]
    pub q_ability_key: char,
    #[serde(default = "default_largo_w_key")]
    pub w_ability_key: char,
    #[serde(default = "default_largo_e_key")]
    pub e_ability_key: char,
    #[serde(default = "default_largo_r_key")]
    pub r_ability_key: char,
    #[serde(default = "default_standalone_key")]
    pub standalone_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroesConfig {
    #[serde(default)]
    pub huskar: HuskarConfig,
    #[serde(default)]
    pub legion_commander: LegionCommanderConfig,
    #[serde(default)]
    pub shadow_fiend: ShadowFiendConfig,
    #[serde(default)]
    pub tiny: TinyConfig,
    #[serde(default)]
    pub largo: LargoConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DangerDetectionConfig {
    #[serde(default = "default_danger_enabled")]
    pub enabled: bool,
    #[serde(default = "default_danger_hp_threshold")]
    pub hp_threshold_percent: u32,
    #[serde(default = "default_rapid_loss_hp")]
    pub rapid_loss_hp: u32,
    #[serde(default = "default_time_window_ms")]
    pub time_window_ms: u64,
    #[serde(default = "default_clear_delay_seconds")]
    pub clear_delay_seconds: u64,
    #[serde(default = "default_healing_threshold_in_danger")]
    pub healing_threshold_in_danger: u32,
    #[serde(default = "default_max_healing_items")]
    pub max_healing_items_per_danger: u32,
    #[serde(default = "default_auto_bkb")]
    pub auto_bkb: bool,
    #[serde(default = "default_auto_satanic")]
    pub auto_satanic: bool,
    #[serde(default = "default_satanic_hp_threshold")]
    pub satanic_hp_threshold: u32,
    #[serde(default = "default_auto_blade_mail")]
    pub auto_blade_mail: bool,
    #[serde(default = "default_auto_glimmer_cape")]
    pub auto_glimmer_cape: bool,
    #[serde(default = "default_auto_ghost_scepter")]
    pub auto_ghost_scepter: bool,
    #[serde(default = "default_auto_shivas_guard")]
    pub auto_shivas_guard: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeutralItemConfig {
    #[serde(default = "default_neutral_items_enabled")]
    pub enabled: bool,
    #[serde(default = "default_self_cast_key")]
    pub self_cast_key: char,
    #[serde(default = "default_log_discoveries")]
    pub log_discoveries: bool,
    #[serde(default = "default_use_in_danger")]
    pub use_in_danger: bool,
    #[serde(default = "default_neutral_hp_threshold")]
    pub hp_threshold: u32,
    #[serde(default = "default_allowed_items")]
    pub allowed_items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoulRingConfig {
    #[serde(default = "default_soul_ring_enabled")]
    pub enabled: bool,
    #[serde(default = "default_soul_ring_min_mana_percent")]
    pub min_mana_percent: u32,
    #[serde(default = "default_soul_ring_min_health_percent")]
    pub min_health_percent: u32,
    #[serde(default = "default_soul_ring_delay_ms")]
    pub delay_before_ability_ms: u64,
    #[serde(default = "default_soul_ring_cooldown_ms")]
    pub trigger_cooldown_ms: u64,
    #[serde(default = "default_soul_ring_ability_keys")]
    pub ability_keys: Vec<String>,
    #[serde(default = "default_soul_ring_intercept_items")]
    pub intercept_item_keys: bool,
}

impl Default for SoulRingConfig {
    fn default() -> Self {
        Self {
            enabled: default_soul_ring_enabled(),
            min_mana_percent: default_soul_ring_min_mana_percent(),
            min_health_percent: default_soul_ring_min_health_percent(),
            delay_before_ability_ms: default_soul_ring_delay_ms(),
            trigger_cooldown_ms: default_soul_ring_cooldown_ms(),
            ability_keys: default_soul_ring_ability_keys(),
            intercept_item_keys: default_soul_ring_intercept_items(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerTreadsConfig {
    #[serde(default = "default_power_treads_enabled")]
    pub enabled: bool,
    #[serde(default = "default_power_treads_toggle_for_abilities")]
    pub toggle_for_abilities: bool,
    #[serde(default = "default_power_treads_toggle_for_items")]
    pub toggle_for_items: bool,
    #[serde(default = "default_power_treads_ability_keys")]
    pub ability_keys: Vec<String>,
    #[serde(default = "default_power_treads_delay_after_switch_ms")]
    pub delay_after_switch_ms: u64,
    #[serde(default = "default_power_treads_switch_back_delay_ms")]
    pub switch_back_delay_ms: u64,
    #[serde(default = "default_power_treads_toggle_cooldown_ms")]
    pub toggle_cooldown_ms: u64,
}

impl Default for PowerTreadsConfig {
    fn default() -> Self {
        Self {
            enabled: default_power_treads_enabled(),
            toggle_for_abilities: default_power_treads_toggle_for_abilities(),
            toggle_for_items: default_power_treads_toggle_for_items(),
            ability_keys: default_power_treads_ability_keys(),
            delay_after_switch_ms: default_power_treads_delay_after_switch_ms(),
            switch_back_delay_ms: default_power_treads_switch_back_delay_ms(),
            toggle_cooldown_ms: default_power_treads_toggle_cooldown_ms(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GsiLoggingConfig {
    #[serde(default = "default_gsi_logging_enabled")]
    pub enabled: bool,
    #[serde(default = "default_gsi_logging_dir")]
    pub output_dir: String,
}

impl Default for GsiLoggingConfig {
    fn default() -> Self {
        Self {
            enabled: default_gsi_logging_enabled(),
            output_dir: default_gsi_logging_dir(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub keybindings: KeybindingsConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub common: CommonConfig,
    #[serde(default)]
    pub heroes: HeroesConfig,
    #[serde(default)]
    pub danger_detection: DangerDetectionConfig,
    #[serde(default)]
    pub neutral_items: NeutralItemConfig,
    #[serde(default)]
    pub soul_ring: SoulRingConfig,
    #[serde(default)]
    pub power_treads: PowerTreadsConfig,
    #[serde(default)]
    pub gsi_logging: GsiLoggingConfig,
}

// Default functions
fn default_port() -> u16 {
    3000
}

fn default_slot0() -> char {
    'z'
}
fn default_slot1() -> char {
    'x'
}
fn default_slot2() -> char {
    'c'
}
fn default_slot3() -> char {
    'v'
}
fn default_slot4() -> char {
    'b'
}
fn default_slot5() -> char {
    'n'
}
fn default_neutral() -> char {
    '0'
}
fn default_hotkey() -> String {
    "Home".to_string()
}
fn default_log_level() -> String {
    "info".to_string()
}
fn default_survivability_threshold() -> u32 {
    30
}
fn default_armlet_threshold() -> u32 {
    320
}
fn default_armlet_offset() -> u32 {
    30
}
fn default_armlet_cooldown() -> u64 {
    250
}
fn default_berserker_blood_key() -> char {
    'e'
}
fn default_berserker_blood_delay() -> u64 {
    300
}
fn default_standalone_key() -> String {
    "Home".to_string()
}
fn default_sf_raze_enabled() -> bool {
    true
}
fn default_raze_delay() -> u64 {
    100
}

fn default_amphibian_enabled() -> bool {
    true
}
fn default_auto_toggle_on_danger() -> bool {
    true
}
fn default_largo_mana_threshold() -> u32 {
    20
}
fn default_largo_heal_threshold() -> u32 {
    50
}
fn default_beat_interval_ms() -> u32 {
    995
}
fn default_beat_correction_ms() -> i32 {
    -10  // Subtract 10ms every N beats (speeds up to compensate for delay)
}
fn default_beat_correction_every_n_beats() -> u32 {
    5  // Apply correction every 5 beats
}
fn default_largo_q_key() -> char {
    'q'
}
fn default_largo_w_key() -> char {
    'w'
}
fn default_largo_e_key() -> char {
    'e'
}
fn default_largo_r_key() -> char {
    'r'
}

fn default_danger_enabled() -> bool {
    true
}
fn default_danger_hp_threshold() -> u32 {
    70
}
fn default_rapid_loss_hp() -> u32 {
    100
}
fn default_time_window_ms() -> u64 {
    500
}
fn default_clear_delay_seconds() -> u64 {
    3
}
fn default_healing_threshold_in_danger() -> u32 {
    50
}
fn default_max_healing_items() -> u32 {
    3
}
fn default_auto_bkb() -> bool {
    false
}
fn default_auto_satanic() -> bool {
    true
}
fn default_satanic_hp_threshold() -> u32 {
    40
}
fn default_auto_blade_mail() -> bool {
    true
}
fn default_auto_glimmer_cape() -> bool {
    true
}
fn default_auto_ghost_scepter() -> bool {
    true
}
fn default_auto_shivas_guard() -> bool {
    true
}

fn default_neutral_items_enabled() -> bool {
    false
}
fn default_self_cast_key() -> char {
    ' '
}
fn default_log_discoveries() -> bool {
    true
}
fn default_use_in_danger() -> bool {
    true
}
fn default_neutral_hp_threshold() -> u32 {
    50
}
fn default_allowed_items() -> Vec<String> {
    Vec::new()
}
fn default_gsi_logging_enabled() -> bool {
    false
}
fn default_gsi_logging_dir() -> String {
    "logs/gsi_events".to_string()
}

// Soul Ring defaults
fn default_soul_ring_enabled() -> bool {
    true
}
fn default_soul_ring_min_mana_percent() -> u32 {
    90
}
fn default_soul_ring_min_health_percent() -> u32 {
    20
}
fn default_soul_ring_delay_ms() -> u64 {
    30
}
fn default_soul_ring_cooldown_ms() -> u64 {
    500
}
fn default_soul_ring_ability_keys() -> Vec<String> {
    vec!["q".to_string(), "w".to_string(), "e".to_string(), "r".to_string(), "d".to_string(), "f".to_string()]
}
fn default_soul_ring_intercept_items() -> bool {
    true
}

// Power Treads defaults
fn default_power_treads_enabled() -> bool {
    true
}
fn default_power_treads_toggle_for_abilities() -> bool {
    true
}
fn default_power_treads_toggle_for_items() -> bool {
    true
}
fn default_power_treads_ability_keys() -> Vec<String> {
    vec!["q".to_string(), "w".to_string(), "e".to_string(), "r".to_string(), "d".to_string(), "f".to_string()]
}
fn default_power_treads_delay_after_switch_ms() -> u64 {
    35
}
fn default_power_treads_switch_back_delay_ms() -> u64 {
    50
}
fn default_power_treads_toggle_cooldown_ms() -> u64 {
    100
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
        }
    }
}

impl Default for KeybindingsConfig {
    fn default() -> Self {
        Self {
            slot0: default_slot0(),
            slot1: default_slot1(),
            slot2: default_slot2(),
            slot3: default_slot3(),
            slot4: default_slot4(),
            slot5: default_slot5(),
            neutral0: default_neutral(),
            combo_trigger: default_hotkey(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

impl Default for CommonConfig {
    fn default() -> Self {
        Self {
            survivability_hp_threshold: default_survivability_threshold(),
        }
    }
}

impl Default for HuskarConfig {
    fn default() -> Self {
        Self {
            armlet_toggle_threshold: default_armlet_threshold(),
            armlet_predictive_offset: default_armlet_offset(),
            armlet_toggle_cooldown_ms: default_armlet_cooldown(),
            berserker_blood_key: default_berserker_blood_key(),
            berserker_blood_delay_ms: default_berserker_blood_delay(),
            standalone_key: default_standalone_key(),
        }
    }
}

impl Default for LegionCommanderConfig {
    fn default() -> Self {
        Self {
            standalone_key: default_standalone_key(),
        }
    }
}

impl Default for ShadowFiendConfig {
    fn default() -> Self {
        Self {
            raze_intercept_enabled: default_sf_raze_enabled(),
            raze_delay_ms: default_raze_delay(),
        }
    }
}

impl Default for TinyConfig {
    fn default() -> Self {
        Self {
            standalone_key: default_standalone_key(),
        }
    }
}

impl Default for LargoConfig {
    fn default() -> Self {
        Self {
            amphibian_rhapsody_enabled: default_amphibian_enabled(),
            auto_toggle_on_danger: default_auto_toggle_on_danger(),
            mana_threshold_percent: default_largo_mana_threshold(),
            heal_hp_threshold: default_largo_heal_threshold(),
            beat_interval_ms: default_beat_interval_ms(),
            beat_correction_ms: default_beat_correction_ms(),
            beat_correction_every_n_beats: default_beat_correction_every_n_beats(),
            q_ability_key: default_largo_q_key(),
            w_ability_key: default_largo_w_key(),
            e_ability_key: default_largo_e_key(),
            r_ability_key: default_largo_r_key(),
            standalone_key: default_standalone_key(),
        }
    }
}

impl Default for HeroesConfig {
    fn default() -> Self {
        Self {
            huskar: HuskarConfig::default(),
            legion_commander: LegionCommanderConfig::default(),
            shadow_fiend: ShadowFiendConfig::default(),
            tiny: TinyConfig::default(),
            largo: LargoConfig::default(),
        }
    }
}

impl Default for DangerDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: default_danger_enabled(),
            hp_threshold_percent: default_danger_hp_threshold(),
            rapid_loss_hp: default_rapid_loss_hp(),
            time_window_ms: default_time_window_ms(),
            clear_delay_seconds: default_clear_delay_seconds(),
            healing_threshold_in_danger: default_healing_threshold_in_danger(),
            max_healing_items_per_danger: default_max_healing_items(),
            auto_bkb: default_auto_bkb(),
            auto_satanic: default_auto_satanic(),
            satanic_hp_threshold: default_satanic_hp_threshold(),
            auto_blade_mail: default_auto_blade_mail(),
            auto_glimmer_cape: default_auto_glimmer_cape(),
            auto_ghost_scepter: default_auto_ghost_scepter(),
            auto_shivas_guard: default_auto_shivas_guard(),
        }
    }
}

impl Default for NeutralItemConfig {
    fn default() -> Self {
        Self {
            enabled: default_neutral_items_enabled(),
            self_cast_key: default_self_cast_key(),
            log_discoveries: default_log_discoveries(),
            use_in_danger: default_use_in_danger(),
            hp_threshold: default_neutral_hp_threshold(),
            allowed_items: default_allowed_items(),
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            keybindings: KeybindingsConfig::default(),
            logging: LoggingConfig::default(),
            common: CommonConfig::default(),
            heroes: HeroesConfig::default(),
            danger_detection: DangerDetectionConfig::default(),
            neutral_items: NeutralItemConfig::default(),
            soul_ring: SoulRingConfig::default(),
            power_treads: PowerTreadsConfig::default(),
            gsi_logging: GsiLoggingConfig::default(),
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let config_path = "config/config.toml";

        match fs::read_to_string(config_path) {
            Ok(contents) => match toml::from_str(&contents) {
                Ok(settings) => {
                    info!("Loaded configuration from {}", config_path);
                    let settings: Settings = settings;
                    settings.validate_keybindings();
                    settings
                }
                Err(e) => {
                    warn!(
                        "Failed to parse {}: {}. Using default settings.",
                        config_path, e
                    );
                    Settings::default()
                }
            },
            Err(_) => {
                info!(
                    "Configuration file {} not found. Using default settings.",
                    config_path
                );
                Settings::default()
            }
        }
    }

    fn validate_keybindings(&self) {
        let mut key_map: HashMap<char, Vec<&str>> = HashMap::new();

        key_map
            .entry(self.keybindings.slot0)
            .or_insert_with(Vec::new)
            .push("slot0");
        key_map
            .entry(self.keybindings.slot1)
            .or_insert_with(Vec::new)
            .push("slot1");
        key_map
            .entry(self.keybindings.slot2)
            .or_insert_with(Vec::new)
            .push("slot2");
        key_map
            .entry(self.keybindings.slot3)
            .or_insert_with(Vec::new)
            .push("slot3");
        key_map
            .entry(self.keybindings.slot4)
            .or_insert_with(Vec::new)
            .push("slot4");
        key_map
            .entry(self.keybindings.slot5)
            .or_insert_with(Vec::new)
            .push("slot5");
        key_map
            .entry(self.keybindings.neutral0)
            .or_insert_with(Vec::new)
            .push("neutral0");

        for (key, slots) in key_map.iter() {
            if slots.len() > 1 {
                warn!(
                    "Keybinding conflict: Key '{}' is assigned to multiple slots: {:?}",
                    key, slots
                );
            }
        }
    }

    pub fn get_key_for_slot(&self, slot: &str) -> Option<char> {
        match slot {
            "slot0" => Some(self.keybindings.slot0),
            "slot1" => Some(self.keybindings.slot1),
            "slot2" => Some(self.keybindings.slot2),
            "slot3" => Some(self.keybindings.slot3),
            "slot4" => Some(self.keybindings.slot4),
            "slot5" => Some(self.keybindings.slot5),
            "neutral0" => Some(self.keybindings.neutral0),
            _ => None,
        }
    }

    pub fn get_standalone_key(&self, hero: &str) -> String {
        match hero {
            "huskar" => self.heroes.huskar.standalone_key.clone(),
            "legion_commander" => self.heroes.legion_commander.standalone_key.clone(),
            "shadow_fiend" => "q".to_string(), // SF uses Q/W/E interception
            "tiny" => self.heroes.tiny.standalone_key.clone(),
            _ => default_standalone_key(),
        }
    }
    
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = "config/config.toml";
        let toml_string = toml::to_string_pretty(self)?;
        fs::write(config_path, toml_string)?;
        info!("Settings saved to {}", config_path);
        Ok(())
    }
}
