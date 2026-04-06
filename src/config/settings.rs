use crate::config::storage::{
    bootstrap_live_config, persist_live_config, ConfigPaths, EMBEDDED_CONFIG_TEMPLATE,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    #[serde(default = "default_check_on_startup")]
    pub check_on_startup: bool,
    #[serde(default = "default_include_prereleases")]
    pub include_prereleases: bool,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            check_on_startup: default_check_on_startup(),
            include_prereleases: default_include_prereleases(),
        }
    }
}

fn default_check_on_startup() -> bool {
    true
}

fn default_include_prereleases() -> bool {
    false
}

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
    #[serde(default = "default_lane_phase_duration_seconds")]
    pub lane_phase_duration_seconds: u64,
    #[serde(default = "default_lane_phase_healing_threshold")]
    pub lane_phase_healing_threshold: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArmletRoshanConfig {
    #[serde(default = "default_armlet_roshan_enabled")]
    pub enabled: bool,
    #[serde(default = "default_armlet_roshan_toggle_key")]
    pub toggle_key: String,
    #[serde(default = "default_armlet_roshan_emergency_margin_hp")]
    pub emergency_margin_hp: u32,
    #[serde(default = "default_armlet_roshan_learning_window_ms")]
    pub learning_window_ms: u64,
    #[serde(default = "default_armlet_roshan_min_confidence_hits")]
    pub min_confidence_hits: usize,
    #[serde(default = "default_armlet_roshan_min_sample_damage")]
    pub min_sample_damage: u32,
    #[serde(default = "default_armlet_roshan_stale_reset_ms")]
    pub stale_reset_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArmletAutomationConfig {
    #[serde(default = "default_armlet_enabled")]
    pub enabled: bool,
    #[serde(default = "default_armlet_cast_modifier")]
    pub cast_modifier: String,
    #[serde(default = "default_armlet_threshold")]
    pub toggle_threshold: u32,
    #[serde(default = "default_armlet_offset")]
    pub predictive_offset: u32,
    #[serde(default = "default_armlet_cooldown")]
    pub toggle_cooldown_ms: u64,
    #[serde(default)]
    pub roshan: ArmletRoshanConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HeroArmletOverrideConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub toggle_threshold: Option<u32>,
    #[serde(default)]
    pub predictive_offset: Option<u32>,
    #[serde(default)]
    pub toggle_cooldown_ms: Option<u64>,
}

impl HeroArmletOverrideConfig {
    pub fn is_empty(&self) -> bool {
        self.enabled.is_none()
            && self.toggle_threshold.is_none()
            && self.predictive_offset.is_none()
            && self.toggle_cooldown_ms.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveArmletConfig {
    pub enabled: bool,
    pub cast_modifier: String,
    pub toggle_threshold: u32,
    pub predictive_offset: u32,
    pub toggle_cooldown_ms: u64,
    pub roshan: ArmletRoshanConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HuskarRoshanSpearsConfig {
    #[serde(default = "default_huskar_roshan_spears_enabled")]
    pub enabled: bool,
    #[serde(default = "default_huskar_burning_spear_key")]
    pub burning_spear_key: char,
    #[serde(default = "default_huskar_roshan_spears_disable_buffer_hp")]
    pub disable_buffer_hp: u32,
    #[serde(default = "default_huskar_roshan_spears_reenable_buffer_hp")]
    pub reenable_buffer_hp: u32,
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
    #[serde(default)]
    pub armlet: HeroArmletOverrideConfig,
    #[serde(default)]
    pub roshan_spears: HuskarRoshanSpearsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegionCommanderConfig {
    #[serde(default = "default_standalone_key")]
    pub standalone_key: String,
    #[serde(default)]
    pub armlet: HeroArmletOverrideConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowFiendConfig {
    #[serde(default = "default_sf_raze_enabled")]
    pub raze_intercept_enabled: bool,
    #[serde(default = "default_raze_delay")]
    pub raze_delay_ms: u64,
    /// Automatically use BKB before ultimate (Requiem of Souls)
    #[serde(default = "default_sf_auto_bkb_on_ultimate")]
    pub auto_bkb_on_ultimate: bool,
    /// Automatically press D (Aghanim's ability) before ultimate
    #[serde(default = "default_sf_auto_d_on_ultimate")]
    pub auto_d_on_ultimate: bool,
    /// Standalone combo trigger key (Blink + Ultimate combo)
    #[serde(default = "default_standalone_key")]
    pub standalone_key: String,
    #[serde(default)]
    pub armlet: HeroArmletOverrideConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TinyConfig {
    #[serde(default = "default_standalone_key")]
    pub standalone_key: String,
    #[serde(default)]
    pub armlet: HeroArmletOverrideConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutworldDestroyerConfig {
    #[serde(default = "default_standalone_key")]
    pub standalone_key: String,
    #[serde(default = "default_od_objurgation_key")]
    pub objurgation_key: char,
    #[serde(default = "default_od_arcane_orb_key")]
    pub arcane_orb_key: char,
    #[serde(default = "default_od_astral_imprisonment_key")]
    pub astral_imprisonment_key: char,
    #[serde(default = "default_od_auto_objurgation_on_danger")]
    pub auto_objurgation_on_danger: bool,
    #[serde(default = "default_od_objurgation_hp_threshold_percent")]
    pub objurgation_hp_threshold_percent: u32,
    #[serde(default = "default_od_objurgation_min_mana_percent")]
    pub objurgation_min_mana_percent: u32,
    #[serde(default = "default_od_objurgation_trigger_cooldown_ms")]
    pub objurgation_trigger_cooldown_ms: u64,
    #[serde(default = "default_od_ultimate_intercept_enabled")]
    pub ultimate_intercept_enabled: bool,
    #[serde(default = "default_od_auto_bkb_on_ultimate")]
    pub auto_bkb_on_ultimate: bool,
    #[serde(default = "default_od_auto_objurgation_on_ultimate")]
    pub auto_objurgation_on_ultimate: bool,
    #[serde(default = "default_od_post_bkb_delay_ms")]
    pub post_bkb_delay_ms: u64,
    #[serde(default = "default_od_post_blink_delay_ms")]
    pub post_blink_delay_ms: u64,
    #[serde(default = "default_od_astral_self_cast_enabled")]
    pub astral_self_cast_enabled: bool,
    #[serde(default = "default_od_astral_self_cast_key")]
    pub astral_self_cast_key: String,
    #[serde(default = "default_od_combo_items")]
    pub combo_items: Vec<String>,
    #[serde(default = "default_od_combo_item_spam_count")]
    pub combo_item_spam_count: u32,
    #[serde(default = "default_od_combo_item_delay_ms")]
    pub combo_item_delay_ms: u64,
    #[serde(default = "default_od_post_ultimate_arcane_orb_presses")]
    pub post_ultimate_arcane_orb_presses: u32,
    #[serde(default = "default_od_arcane_orb_press_interval_ms")]
    pub arcane_orb_press_interval_ms: u64,
    #[serde(default)]
    pub armlet: HeroArmletOverrideConfig,
}

/// Configuration for auto-casting an ability during Space+Right-click combo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoAbilityConfig {
    /// Ability slot index (0-5, corresponds to ability0-ability5 in GSI)
    pub index: u8,
    /// Key to press for this ability ('q', 'w', 'e', 'r', 'd', 'f')
    pub key: char,
    /// Optional HP threshold - only cast if HP% is below this value
    /// If None/null, always cast when off cooldown
    #[serde(default)]
    pub hp_threshold: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroodmotherConfig {
    #[serde(default = "default_broodmother_enabled")]
    pub spider_micro_enabled: bool,
    #[serde(default = "default_broodmother_spider_control_group")]
    pub spider_control_group_key: String,
    #[serde(default = "default_broodmother_reselect_hero_key")]
    pub reselect_hero_key: String,
    #[serde(default = "default_broodmother_attack_key")]
    pub attack_key: char,
    #[serde(default = "default_auto_items_enabled")]
    pub auto_items_enabled: bool,
    #[serde(default = "default_auto_items_modifier")]
    pub auto_items_modifier: String,
    #[serde(default = "default_auto_items")]
    pub auto_items: Vec<String>,
    #[serde(default = "default_auto_abilities")]
    pub auto_abilities: Vec<AutoAbilityConfig>,
    #[serde(default = "default_auto_abilities_first")]
    pub auto_abilities_first: bool,
    #[serde(default)]
    pub armlet: HeroArmletOverrideConfig,
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
    pub beat_correction_ms: i32, // Correction to apply (can be negative)
    #[serde(default = "default_beat_correction_every_n_beats")]
    pub beat_correction_every_n_beats: u32, // Apply correction every N beats (0 = disabled)
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
    #[serde(default)]
    pub armlet: HeroArmletOverrideConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeepoFarmAssistConfig {
    #[serde(default = "default_meepo_farm_assist_enabled")]
    pub enabled: bool,
    #[serde(default = "default_meepo_farm_assist_toggle_key")]
    pub toggle_key: String,
    #[serde(default = "default_meepo_farm_assist_pulse_interval_ms")]
    pub pulse_interval_ms: u64,
    #[serde(default = "default_meepo_farm_assist_minimum_mana_percent")]
    pub minimum_mana_percent: u32,
    #[serde(default = "default_meepo_farm_assist_minimum_health_percent")]
    pub minimum_health_percent: u32,
    #[serde(default = "default_meepo_farm_assist_right_click_after_poof")]
    pub right_click_after_poof: bool,
    #[serde(default = "default_meepo_farm_assist_suspend_on_danger")]
    pub suspend_on_danger: bool,
    #[serde(default = "default_meepo_farm_assist_suspend_after_manual_combo_ms")]
    pub suspend_after_manual_combo_ms: u64,
    #[serde(default = "default_meepo_farm_assist_poof_press_count")]
    pub poof_press_count: u32,
    #[serde(default = "default_meepo_farm_assist_poof_press_interval_ms")]
    pub poof_press_interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeepoConfig {
    #[serde(default = "default_standalone_key")]
    pub standalone_key: String,
    #[serde(default = "default_meepo_earthbind_key")]
    pub earthbind_key: char,
    #[serde(default = "default_meepo_poof_key")]
    pub poof_key: char,
    #[serde(default = "default_meepo_dig_key")]
    pub dig_key: char,
    #[serde(default = "default_meepo_megameepo_key")]
    pub megameepo_key: char,
    #[serde(default = "default_meepo_post_blink_delay_ms")]
    pub post_blink_delay_ms: u64,
    #[serde(default = "default_meepo_combo_items")]
    pub combo_items: Vec<String>,
    #[serde(default = "default_meepo_combo_item_spam_count")]
    pub combo_item_spam_count: u32,
    #[serde(default = "default_meepo_combo_item_delay_ms")]
    pub combo_item_delay_ms: u64,
    #[serde(default = "default_meepo_earthbind_press_count")]
    pub earthbind_press_count: u32,
    #[serde(default = "default_meepo_earthbind_press_interval_ms")]
    pub earthbind_press_interval_ms: u64,
    #[serde(default = "default_meepo_poof_press_count")]
    pub poof_press_count: u32,
    #[serde(default = "default_meepo_poof_press_interval_ms")]
    pub poof_press_interval_ms: u64,
    #[serde(default = "default_meepo_auto_dig_on_danger")]
    pub auto_dig_on_danger: bool,
    #[serde(default = "default_meepo_dig_hp_threshold_percent")]
    pub dig_hp_threshold_percent: u32,
    #[serde(default = "default_meepo_auto_megameepo_on_danger")]
    pub auto_megameepo_on_danger: bool,
    #[serde(default = "default_meepo_megameepo_hp_threshold_percent")]
    pub megameepo_hp_threshold_percent: u32,
    #[serde(default = "default_meepo_defensive_trigger_cooldown_ms")]
    pub defensive_trigger_cooldown_ms: u64,
    #[serde(default)]
    pub farm_assist: MeepoFarmAssistConfig,
    #[serde(default)]
    pub armlet: HeroArmletOverrideConfig,
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
    pub outworld_destroyer: OutworldDestroyerConfig,
    #[serde(default)]
    pub largo: LargoConfig,
    #[serde(default)]
    pub broodmother: BroodmotherConfig,
    #[serde(default)]
    pub meepo: MeepoConfig,
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
    #[serde(default = "default_auto_manta_on_silence")]
    pub auto_manta_on_silence: bool,
    #[serde(default = "default_auto_lotus_on_silence")]
    pub auto_lotus_on_silence: bool,
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
pub struct ManaAutomationConfig {
    #[serde(default = "default_mana_automation_enabled")]
    pub enabled: bool,
    #[serde(default = "default_mana_threshold_percent")]
    pub mana_threshold_percent: u32,
    #[serde(default = "default_mana_automation_excluded_heroes")]
    pub excluded_heroes: Vec<String>,
    #[serde(default = "default_mana_automation_allowed_items")]
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
pub struct GsiLoggingConfig {
    #[serde(default = "default_gsi_logging_enabled")]
    pub enabled: bool,
    #[serde(default = "default_gsi_logging_dir")]
    pub output_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuneAlertConfig {
    #[serde(default = "default_rune_alerts_enabled")]
    pub enabled: bool,
    #[serde(default = "default_rune_alert_lead_seconds")]
    pub alert_lead_seconds: i32,
    #[serde(default = "default_rune_alert_interval_seconds")]
    pub interval_seconds: i32,
    #[serde(default = "default_rune_alert_audio_enabled")]
    pub audio_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinimapCaptureConfig {
    #[serde(default = "default_minimap_capture_enabled")]
    pub enabled: bool,
    #[serde(default = "default_minimap_x")]
    pub minimap_x: u32,
    #[serde(default = "default_minimap_y")]
    pub minimap_y: u32,
    #[serde(default = "default_minimap_width")]
    pub minimap_width: u32,
    #[serde(default = "default_minimap_height")]
    pub minimap_height: u32,
    #[serde(default = "default_minimap_capture_interval_ms")]
    pub capture_interval_ms: u64,
    #[serde(default = "default_minimap_capture_sample_every_n")]
    pub sample_every_n: u32,
    #[serde(default = "default_minimap_capture_output_dir")]
    pub artifact_output_dir: String,
}

impl Default for MinimapCaptureConfig {
    fn default() -> Self {
        Self {
            enabled: default_minimap_capture_enabled(),
            minimap_x: default_minimap_x(),
            minimap_y: default_minimap_y(),
            minimap_width: default_minimap_width(),
            minimap_height: default_minimap_height(),
            capture_interval_ms: default_minimap_capture_interval_ms(),
            sample_every_n: default_minimap_capture_sample_every_n(),
            artifact_output_dir: default_minimap_capture_output_dir(),
        }
    }
}

impl Default for RuneAlertConfig {
    fn default() -> Self {
        Self {
            enabled: default_rune_alerts_enabled(),
            alert_lead_seconds: default_rune_alert_lead_seconds(),
            interval_seconds: default_rune_alert_interval_seconds(),
            audio_enabled: default_rune_alert_audio_enabled(),
        }
    }
}

impl Default for GsiLoggingConfig {
    fn default() -> Self {
        Self {
            enabled: default_gsi_logging_enabled(),
            output_dir: default_gsi_logging_dir(),
        }
    }
}

fn default_minimap_analysis_enabled() -> bool {
    false
}
fn default_baseline_frames() -> u32 {
    10
}
fn default_baseline_threshold() -> f32 {
    0.8
}
fn default_analysis_min_cluster_size() -> usize {
    40
}
fn default_analysis_max_cluster_size() -> usize {
    200
}
fn default_red_hue_max() -> f32 {
    15.0
}
fn default_red_hue_min_wrap() -> f32 {
    340.0
}
fn default_red_min_saturation() -> f32 {
    40.0
}
fn default_red_min_value() -> f32 {
    30.0
}
fn default_green_hue_min() -> f32 {
    80.0
}
fn default_green_hue_max() -> f32 {
    160.0
}
fn default_green_min_saturation() -> f32 {
    35.0
}
fn default_green_min_value() -> f32 {
    25.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinimapAnalysisConfig {
    #[serde(default = "default_minimap_analysis_enabled")]
    pub enabled: bool,
    #[serde(default = "default_baseline_frames")]
    pub baseline_frames: u32,
    #[serde(default = "default_baseline_threshold")]
    pub baseline_threshold: f32,
    #[serde(default = "default_analysis_min_cluster_size")]
    pub min_cluster_size: usize,
    #[serde(default = "default_analysis_max_cluster_size")]
    pub max_cluster_size: usize,
    #[serde(default = "default_red_hue_max")]
    pub red_hue_max: f32,
    #[serde(default = "default_red_hue_min_wrap")]
    pub red_hue_min_wrap: f32,
    #[serde(default = "default_red_min_saturation")]
    pub red_min_saturation: f32,
    #[serde(default = "default_red_min_value")]
    pub red_min_value: f32,
    #[serde(default = "default_green_hue_min")]
    pub green_hue_min: f32,
    #[serde(default = "default_green_hue_max")]
    pub green_hue_max: f32,
    #[serde(default = "default_green_min_saturation")]
    pub green_min_saturation: f32,
    #[serde(default = "default_green_min_value")]
    pub green_min_value: f32,
}

impl Default for MinimapAnalysisConfig {
    fn default() -> Self {
        Self {
            enabled: default_minimap_analysis_enabled(),
            baseline_frames: default_baseline_frames(),
            baseline_threshold: default_baseline_threshold(),
            min_cluster_size: default_analysis_min_cluster_size(),
            max_cluster_size: default_analysis_max_cluster_size(),
            red_hue_max: default_red_hue_max(),
            red_hue_min_wrap: default_red_hue_min_wrap(),
            red_min_saturation: default_red_min_saturation(),
            red_min_value: default_red_min_value(),
            green_hue_min: default_green_hue_min(),
            green_hue_max: default_green_hue_max(),
            green_min_saturation: default_green_min_saturation(),
            green_min_value: default_green_min_value(),
        }
    }
}

impl MinimapAnalysisConfig {
    /// Convert config values into the `ColorThresholds` used by the analysis engine.
    pub fn to_color_thresholds(&self) -> crate::observability::minimap_analysis::ColorThresholds {
        crate::observability::minimap_analysis::ColorThresholds {
            red_hue_max: self.red_hue_max,
            red_hue_min_wrap: self.red_hue_min_wrap,
            red_min_saturation: self.red_min_saturation,
            red_min_value: self.red_min_value,
            green_hue_min: self.green_hue_min,
            green_hue_max: self.green_hue_max,
            green_min_saturation: self.green_min_saturation,
            green_min_value: self.green_min_value,
            min_cluster_size: self.min_cluster_size,
            max_cluster_size: self.max_cluster_size,
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
    pub armlet: ArmletAutomationConfig,
    #[serde(default)]
    pub heroes: HeroesConfig,
    #[serde(default)]
    pub danger_detection: DangerDetectionConfig,
    #[serde(default)]
    pub neutral_items: NeutralItemConfig,
    #[serde(default)]
    pub mana_automation: ManaAutomationConfig,
    #[serde(default)]
    pub soul_ring: SoulRingConfig,
    #[serde(default)]
    pub gsi_logging: GsiLoggingConfig,
    #[serde(default)]
    pub updates: UpdateConfig,
    #[serde(default)]
    pub rune_alerts: RuneAlertConfig,
    #[serde(default)]
    pub minimap_capture: MinimapCaptureConfig,
    #[serde(default)]
    pub minimap_analysis: MinimapAnalysisConfig,
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
fn default_lane_phase_duration_seconds() -> u64 {
    480
}
fn default_lane_phase_healing_threshold() -> u32 {
    12
}
fn default_armlet_enabled() -> bool {
    true
}
fn default_armlet_cast_modifier() -> String {
    "Alt".to_string()
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
fn default_armlet_roshan_enabled() -> bool {
    false
}
fn default_armlet_roshan_toggle_key() -> String {
    "Insert".to_string()
}
fn default_armlet_roshan_emergency_margin_hp() -> u32 {
    60
}
fn default_armlet_roshan_learning_window_ms() -> u64 {
    5_000
}
fn default_armlet_roshan_min_confidence_hits() -> usize {
    2
}
fn default_armlet_roshan_min_sample_damage() -> u32 {
    80
}
fn default_armlet_roshan_stale_reset_ms() -> u64 {
    6_000
}
fn default_berserker_blood_key() -> char {
    'e'
}
fn default_berserker_blood_delay() -> u64 {
    300
}
fn default_huskar_roshan_spears_enabled() -> bool {
    false
}
fn default_huskar_burning_spear_key() -> char {
    'w'
}
fn default_huskar_roshan_spears_disable_buffer_hp() -> u32 {
    60
}
fn default_huskar_roshan_spears_reenable_buffer_hp() -> u32 {
    100
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
fn default_sf_auto_bkb_on_ultimate() -> bool {
    false
}
fn default_sf_auto_d_on_ultimate() -> bool {
    false
}
fn default_od_objurgation_key() -> char {
    'e'
}
fn default_od_arcane_orb_key() -> char {
    'q'
}
fn default_od_astral_imprisonment_key() -> char {
    'w'
}
fn default_od_auto_objurgation_on_danger() -> bool {
    true
}
fn default_od_objurgation_hp_threshold_percent() -> u32 {
    55
}
fn default_od_objurgation_min_mana_percent() -> u32 {
    25
}
fn default_od_objurgation_trigger_cooldown_ms() -> u64 {
    1500
}
fn default_od_ultimate_intercept_enabled() -> bool {
    true
}
fn default_od_auto_bkb_on_ultimate() -> bool {
    true
}
fn default_od_auto_objurgation_on_ultimate() -> bool {
    true
}
fn default_od_post_bkb_delay_ms() -> u64 {
    50
}
fn default_od_post_blink_delay_ms() -> u64 {
    100
}
fn default_od_astral_self_cast_enabled() -> bool {
    false
}
fn default_od_astral_self_cast_key() -> String {
    "F5".to_string()
}
fn default_od_combo_items() -> Vec<String> {
    vec![]
}
fn default_od_combo_item_spam_count() -> u32 {
    1
}
fn default_od_combo_item_delay_ms() -> u64 {
    50
}
fn default_od_post_ultimate_arcane_orb_presses() -> u32 {
    0
}
fn default_od_arcane_orb_press_interval_ms() -> u64 {
    30
}

fn default_broodmother_enabled() -> bool {
    true
}
fn default_broodmother_spider_control_group() -> String {
    "F2".to_string()
}
fn default_broodmother_reselect_hero_key() -> String {
    "F1".to_string()
}
fn default_broodmother_attack_key() -> char {
    'a'
}

fn default_auto_items_enabled() -> bool {
    false
}
fn default_auto_items_modifier() -> String {
    "Space".to_string()
}
fn default_auto_items() -> Vec<String> {
    vec![]
}
fn default_auto_abilities() -> Vec<AutoAbilityConfig> {
    vec![]
}
fn default_auto_abilities_first() -> bool {
    false // Items first by default
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
    -10 // Subtract 10ms every N beats (speeds up to compensate for delay)
}
fn default_beat_correction_every_n_beats() -> u32 {
    5 // Apply correction every 5 beats
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

fn default_meepo_earthbind_key() -> char {
    'q'
}
fn default_meepo_poof_key() -> char {
    'w'
}
fn default_meepo_dig_key() -> char {
    'd'
}
fn default_meepo_megameepo_key() -> char {
    'f'
}
fn default_meepo_post_blink_delay_ms() -> u64 {
    80
}
fn default_meepo_combo_items() -> Vec<String> {
    vec!["sheepstick".to_string(), "disperser".to_string()]
}
fn default_meepo_combo_item_spam_count() -> u32 {
    1
}
fn default_meepo_combo_item_delay_ms() -> u64 {
    40
}
fn default_meepo_earthbind_press_count() -> u32 {
    2
}
fn default_meepo_earthbind_press_interval_ms() -> u64 {
    30
}
fn default_meepo_poof_press_count() -> u32 {
    3
}
fn default_meepo_poof_press_interval_ms() -> u64 {
    35
}
fn default_meepo_auto_dig_on_danger() -> bool {
    true
}
fn default_meepo_dig_hp_threshold_percent() -> u32 {
    32
}
fn default_meepo_auto_megameepo_on_danger() -> bool {
    true
}
fn default_meepo_megameepo_hp_threshold_percent() -> u32 {
    45
}
fn default_meepo_defensive_trigger_cooldown_ms() -> u64 {
    1500
}
fn default_meepo_farm_assist_enabled() -> bool {
    true
}
fn default_meepo_farm_assist_toggle_key() -> String {
    "End".to_string()
}
fn default_meepo_farm_assist_pulse_interval_ms() -> u64 {
    700
}
fn default_meepo_farm_assist_minimum_mana_percent() -> u32 {
    35
}
fn default_meepo_farm_assist_minimum_health_percent() -> u32 {
    45
}
fn default_meepo_farm_assist_right_click_after_poof() -> bool {
    true
}
fn default_meepo_farm_assist_suspend_on_danger() -> bool {
    true
}
fn default_meepo_farm_assist_suspend_after_manual_combo_ms() -> u64 {
    2500
}
fn default_meepo_farm_assist_poof_press_count() -> u32 {
    1
}
fn default_meepo_farm_assist_poof_press_interval_ms() -> u64 {
    35
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
fn default_auto_manta_on_silence() -> bool {
    true
}
fn default_auto_lotus_on_silence() -> bool {
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
fn default_mana_automation_enabled() -> bool {
    true
}
fn default_mana_threshold_percent() -> u32 {
    25
}
fn default_mana_automation_excluded_heroes() -> Vec<String> {
    vec!["npc_dota_hero_huskar".to_string()]
}
fn default_mana_automation_allowed_items() -> Vec<String> {
    vec![
        "item_arcane_boots".to_string(),
        "item_mana_draught".to_string(),
    ]
}
fn default_gsi_logging_enabled() -> bool {
    false
}
fn default_gsi_logging_dir() -> String {
    "logs/gsi_events".to_string()
}

fn default_rune_alerts_enabled() -> bool {
    true
}
fn default_rune_alert_lead_seconds() -> i32 {
    10
}
fn default_rune_alert_interval_seconds() -> i32 {
    120
}
fn default_rune_alert_audio_enabled() -> bool {
    true
}

fn default_minimap_capture_enabled() -> bool {
    false
}
fn default_minimap_capture_interval_ms() -> u64 {
    1000
}
fn default_minimap_capture_sample_every_n() -> u32 {
    30
}
fn default_minimap_capture_output_dir() -> String {
    "logs/minimap_capture".to_string()
}
fn default_minimap_x() -> u32 {
    2
}
fn default_minimap_y() -> u32 {
    835
}
fn default_minimap_width() -> u32 {
    240
}
fn default_minimap_height() -> u32 {
    245
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
    vec![
        "q".to_string(),
        "w".to_string(),
        "e".to_string(),
        "r".to_string(),
        "d".to_string(),
        "f".to_string(),
    ]
}
fn default_soul_ring_intercept_items() -> bool {
    true
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
            lane_phase_duration_seconds: default_lane_phase_duration_seconds(),
            lane_phase_healing_threshold: default_lane_phase_healing_threshold(),
        }
    }
}

impl Default for ArmletAutomationConfig {
    fn default() -> Self {
        Self {
            enabled: default_armlet_enabled(),
            cast_modifier: default_armlet_cast_modifier(),
            toggle_threshold: default_armlet_threshold(),
            predictive_offset: default_armlet_offset(),
            toggle_cooldown_ms: default_armlet_cooldown(),
            roshan: ArmletRoshanConfig::default(),
        }
    }
}

impl Default for ArmletRoshanConfig {
    fn default() -> Self {
        Self {
            enabled: default_armlet_roshan_enabled(),
            toggle_key: default_armlet_roshan_toggle_key(),
            emergency_margin_hp: default_armlet_roshan_emergency_margin_hp(),
            learning_window_ms: default_armlet_roshan_learning_window_ms(),
            min_confidence_hits: default_armlet_roshan_min_confidence_hits(),
            min_sample_damage: default_armlet_roshan_min_sample_damage(),
            stale_reset_ms: default_armlet_roshan_stale_reset_ms(),
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
            armlet: HeroArmletOverrideConfig::default(),
            roshan_spears: HuskarRoshanSpearsConfig::default(),
        }
    }
}

impl Default for HuskarRoshanSpearsConfig {
    fn default() -> Self {
        Self {
            enabled: default_huskar_roshan_spears_enabled(),
            burning_spear_key: default_huskar_burning_spear_key(),
            disable_buffer_hp: default_huskar_roshan_spears_disable_buffer_hp(),
            reenable_buffer_hp: default_huskar_roshan_spears_reenable_buffer_hp(),
        }
    }
}

impl Default for LegionCommanderConfig {
    fn default() -> Self {
        Self {
            standalone_key: default_standalone_key(),
            armlet: HeroArmletOverrideConfig::default(),
        }
    }
}

impl Default for ShadowFiendConfig {
    fn default() -> Self {
        Self {
            raze_intercept_enabled: default_sf_raze_enabled(),
            raze_delay_ms: default_raze_delay(),
            auto_bkb_on_ultimate: default_sf_auto_bkb_on_ultimate(),
            auto_d_on_ultimate: default_sf_auto_d_on_ultimate(),
            standalone_key: default_standalone_key(),
            armlet: HeroArmletOverrideConfig::default(),
        }
    }
}

impl Default for OutworldDestroyerConfig {
    fn default() -> Self {
        Self {
            standalone_key: default_standalone_key(),
            objurgation_key: default_od_objurgation_key(),
            arcane_orb_key: default_od_arcane_orb_key(),
            astral_imprisonment_key: default_od_astral_imprisonment_key(),
            auto_objurgation_on_danger: default_od_auto_objurgation_on_danger(),
            objurgation_hp_threshold_percent: default_od_objurgation_hp_threshold_percent(),
            objurgation_min_mana_percent: default_od_objurgation_min_mana_percent(),
            objurgation_trigger_cooldown_ms: default_od_objurgation_trigger_cooldown_ms(),
            ultimate_intercept_enabled: default_od_ultimate_intercept_enabled(),
            auto_bkb_on_ultimate: default_od_auto_bkb_on_ultimate(),
            auto_objurgation_on_ultimate: default_od_auto_objurgation_on_ultimate(),
            post_bkb_delay_ms: default_od_post_bkb_delay_ms(),
            post_blink_delay_ms: default_od_post_blink_delay_ms(),
            astral_self_cast_enabled: default_od_astral_self_cast_enabled(),
            astral_self_cast_key: default_od_astral_self_cast_key(),
            combo_items: default_od_combo_items(),
            combo_item_spam_count: default_od_combo_item_spam_count(),
            combo_item_delay_ms: default_od_combo_item_delay_ms(),
            post_ultimate_arcane_orb_presses: default_od_post_ultimate_arcane_orb_presses(),
            arcane_orb_press_interval_ms: default_od_arcane_orb_press_interval_ms(),
            armlet: HeroArmletOverrideConfig::default(),
        }
    }
}

impl Default for BroodmotherConfig {
    fn default() -> Self {
        Self {
            spider_micro_enabled: default_broodmother_enabled(),
            spider_control_group_key: default_broodmother_spider_control_group(),
            reselect_hero_key: default_broodmother_reselect_hero_key(),
            attack_key: default_broodmother_attack_key(),
            auto_items_enabled: default_auto_items_enabled(),
            auto_items_modifier: default_auto_items_modifier(),
            auto_items: default_auto_items(),
            auto_abilities: default_auto_abilities(),
            auto_abilities_first: default_auto_abilities_first(),
            armlet: HeroArmletOverrideConfig::default(),
        }
    }
}

impl Default for TinyConfig {
    fn default() -> Self {
        Self {
            standalone_key: default_standalone_key(),
            armlet: HeroArmletOverrideConfig::default(),
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
            armlet: HeroArmletOverrideConfig::default(),
        }
    }
}

impl Default for MeepoFarmAssistConfig {
    fn default() -> Self {
        Self {
            enabled: default_meepo_farm_assist_enabled(),
            toggle_key: default_meepo_farm_assist_toggle_key(),
            pulse_interval_ms: default_meepo_farm_assist_pulse_interval_ms(),
            minimum_mana_percent: default_meepo_farm_assist_minimum_mana_percent(),
            minimum_health_percent: default_meepo_farm_assist_minimum_health_percent(),
            right_click_after_poof: default_meepo_farm_assist_right_click_after_poof(),
            suspend_on_danger: default_meepo_farm_assist_suspend_on_danger(),
            suspend_after_manual_combo_ms: default_meepo_farm_assist_suspend_after_manual_combo_ms(),
            poof_press_count: default_meepo_farm_assist_poof_press_count(),
            poof_press_interval_ms: default_meepo_farm_assist_poof_press_interval_ms(),
        }
    }
}

impl Default for MeepoConfig {
    fn default() -> Self {
        Self {
            standalone_key: default_standalone_key(),
            earthbind_key: default_meepo_earthbind_key(),
            poof_key: default_meepo_poof_key(),
            dig_key: default_meepo_dig_key(),
            megameepo_key: default_meepo_megameepo_key(),
            post_blink_delay_ms: default_meepo_post_blink_delay_ms(),
            combo_items: default_meepo_combo_items(),
            combo_item_spam_count: default_meepo_combo_item_spam_count(),
            combo_item_delay_ms: default_meepo_combo_item_delay_ms(),
            earthbind_press_count: default_meepo_earthbind_press_count(),
            earthbind_press_interval_ms: default_meepo_earthbind_press_interval_ms(),
            poof_press_count: default_meepo_poof_press_count(),
            poof_press_interval_ms: default_meepo_poof_press_interval_ms(),
            auto_dig_on_danger: default_meepo_auto_dig_on_danger(),
            dig_hp_threshold_percent: default_meepo_dig_hp_threshold_percent(),
            auto_megameepo_on_danger: default_meepo_auto_megameepo_on_danger(),
            megameepo_hp_threshold_percent: default_meepo_megameepo_hp_threshold_percent(),
            defensive_trigger_cooldown_ms: default_meepo_defensive_trigger_cooldown_ms(),
            farm_assist: MeepoFarmAssistConfig::default(),
            armlet: HeroArmletOverrideConfig::default(),
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
            outworld_destroyer: OutworldDestroyerConfig::default(),
            largo: LargoConfig::default(),
            broodmother: BroodmotherConfig::default(),
            meepo: MeepoConfig::default(),
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
            auto_manta_on_silence: default_auto_manta_on_silence(),
            auto_lotus_on_silence: default_auto_lotus_on_silence(),
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

impl Default for ManaAutomationConfig {
    fn default() -> Self {
        Self {
            enabled: default_mana_automation_enabled(),
            mana_threshold_percent: default_mana_threshold_percent(),
            excluded_heroes: default_mana_automation_excluded_heroes(),
            allowed_items: default_mana_automation_allowed_items(),
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
            armlet: ArmletAutomationConfig::default(),
            heroes: HeroesConfig::default(),
            danger_detection: DangerDetectionConfig::default(),
            neutral_items: NeutralItemConfig::default(),
            mana_automation: ManaAutomationConfig::default(),
            soul_ring: SoulRingConfig::default(),
            gsi_logging: GsiLoggingConfig::default(),
            updates: UpdateConfig::default(),
            rune_alerts: RuneAlertConfig::default(),
            minimap_capture: MinimapCaptureConfig::default(),
            minimap_analysis: MinimapAnalysisConfig::default(),
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let paths = match ConfigPaths::detect() {
            Ok(paths) => paths,
            Err(e) => {
                warn!("Failed to resolve config paths: {}. Using default settings.", e);
                return Settings::default();
            }
        };

        let config_path = match bootstrap_live_config(&paths, EMBEDDED_CONFIG_TEMPLATE) {
            Ok(path) => path,
            Err(e) => {
                warn!("Failed to bootstrap live config: {}. Using default settings.", e);
                return Settings::default();
            }
        };

        match fs::read_to_string(&config_path) {
            Ok(contents) => match toml::from_str(&contents) {
                Ok(settings) => {
                    info!("Loaded configuration from {}", config_path.display());
                    let settings: Settings = settings;
                    settings.validate_keybindings();
                    settings
                }
                Err(e) => {
                    warn!(
                        "Failed to parse {}: {}. Using default settings.",
                        config_path.display(),
                        e
                    );
                    Settings::default()
                }
            },
            Err(e) => {
                info!(
                    "Configuration file {} could not be read ({}). Using default settings.",
                    config_path.display(),
                    e
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

    fn huskar_armlet_override(&self) -> HeroArmletOverrideConfig {
        if !self.heroes.huskar.armlet.is_empty() {
            return self.heroes.huskar.armlet.clone();
        }

        HeroArmletOverrideConfig {
            enabled: None,
            toggle_threshold: Some(self.heroes.huskar.armlet_toggle_threshold),
            predictive_offset: Some(self.heroes.huskar.armlet_predictive_offset),
            toggle_cooldown_ms: Some(self.heroes.huskar.armlet_toggle_cooldown_ms),
        }
    }

    fn hero_armlet_override(&self, hero_name: &str) -> Option<HeroArmletOverrideConfig> {
        match hero_name {
            "npc_dota_hero_huskar" => Some(self.huskar_armlet_override()),
            "npc_dota_hero_legion_commander" => Some(self.heroes.legion_commander.armlet.clone()),
            "npc_dota_hero_nevermore" => Some(self.heroes.shadow_fiend.armlet.clone()),
            "npc_dota_hero_tiny" => Some(self.heroes.tiny.armlet.clone()),
            "npc_dota_hero_obsidian_destroyer" => {
                Some(self.heroes.outworld_destroyer.armlet.clone())
            }
            "npc_dota_hero_largo" => Some(self.heroes.largo.armlet.clone()),
            "npc_dota_hero_broodmother" => Some(self.heroes.broodmother.armlet.clone()),
            "npc_dota_hero_meepo" => Some(self.heroes.meepo.armlet.clone()),
            _ => None,
        }
    }

    pub fn resolve_armlet_config(&self, hero_name: &str) -> EffectiveArmletConfig {
        let mut resolved = EffectiveArmletConfig {
            enabled: self.armlet.enabled,
            cast_modifier: self.armlet.cast_modifier.clone(),
            toggle_threshold: self.armlet.toggle_threshold,
            predictive_offset: self.armlet.predictive_offset,
            toggle_cooldown_ms: self.armlet.toggle_cooldown_ms,
            roshan: self.armlet.roshan.clone(),
        };

        if let Some(hero_override) = self.hero_armlet_override(hero_name) {
            if let Some(enabled) = hero_override.enabled {
                resolved.enabled = enabled;
            }
            if let Some(toggle_threshold) = hero_override.toggle_threshold {
                resolved.toggle_threshold = toggle_threshold;
            }
            if let Some(predictive_offset) = hero_override.predictive_offset {
                resolved.predictive_offset = predictive_offset;
            }
            if let Some(toggle_cooldown_ms) = hero_override.toggle_cooldown_ms {
                resolved.toggle_cooldown_ms = toggle_cooldown_ms;
            }
        }

        resolved
    }

    pub fn get_standalone_key(&self, hero: &str) -> String {
        match hero {
            "huskar" => self.heroes.huskar.standalone_key.clone(),
            "legion_commander" => self.heroes.legion_commander.standalone_key.clone(),
            "shadow_fiend" => "q".to_string(), // SF uses Q/W/E interception
            "tiny" => self.heroes.tiny.standalone_key.clone(),
            "outworld_destroyer" => self.heroes.outworld_destroyer.standalone_key.clone(),
            "meepo" => self.heroes.meepo.standalone_key.clone(),
            _ => default_standalone_key(),
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let paths = ConfigPaths::detect().map_err(std::io::Error::other)?;
        let desired_contents = toml::to_string_pretty(self)?;
        let config_path =
            persist_live_config(&paths, &desired_contents, EMBEDDED_CONFIG_TEMPLATE)
                .map_err(std::io::Error::other)?;
        info!("Settings saved to {}", config_path.display());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn huskar_roshan_spears_defaults_are_exposed_through_settings() {
        let settings = Settings::default();

        assert!(!settings.heroes.huskar.roshan_spears.enabled);
        assert_eq!(settings.heroes.huskar.roshan_spears.burning_spear_key, 'w');
        assert_eq!(settings.heroes.huskar.roshan_spears.disable_buffer_hp, 60);
        assert_eq!(settings.heroes.huskar.roshan_spears.reenable_buffer_hp, 100);
    }

    #[test]
    fn meepo_defaults_are_exposed_through_settings() {
        let settings = Settings::default();

        // Verify all Meepo defaults match the spec
        assert_eq!(settings.heroes.meepo.standalone_key, "Home");
        assert_eq!(settings.heroes.meepo.earthbind_key, 'q');
        assert_eq!(settings.heroes.meepo.poof_key, 'w');
        assert_eq!(settings.heroes.meepo.dig_key, 'd');
        assert_eq!(settings.heroes.meepo.megameepo_key, 'f');
        assert_eq!(settings.heroes.meepo.post_blink_delay_ms, 80);
        assert_eq!(
            settings.heroes.meepo.combo_items,
            vec!["sheepstick", "disperser"]
        );
        assert_eq!(settings.heroes.meepo.combo_item_spam_count, 1);
        assert_eq!(settings.heroes.meepo.combo_item_delay_ms, 40);
        assert_eq!(settings.heroes.meepo.earthbind_press_count, 2);
        assert_eq!(settings.heroes.meepo.earthbind_press_interval_ms, 30);
        assert_eq!(settings.heroes.meepo.poof_press_count, 3);
        assert_eq!(settings.heroes.meepo.poof_press_interval_ms, 35);
        assert_eq!(settings.heroes.meepo.auto_dig_on_danger, true);
        assert_eq!(settings.heroes.meepo.dig_hp_threshold_percent, 32);
        assert_eq!(settings.heroes.meepo.auto_megameepo_on_danger, true);
        assert_eq!(settings.heroes.meepo.megameepo_hp_threshold_percent, 45);
        assert_eq!(settings.heroes.meepo.defensive_trigger_cooldown_ms, 1500);
        assert!(settings.heroes.meepo.farm_assist.enabled);
        assert_eq!(settings.heroes.meepo.farm_assist.toggle_key, "End");
        assert_eq!(settings.heroes.meepo.farm_assist.pulse_interval_ms, 700);
        assert_eq!(settings.heroes.meepo.farm_assist.minimum_mana_percent, 35);
        assert_eq!(settings.heroes.meepo.farm_assist.minimum_health_percent, 45);
        assert!(settings.heroes.meepo.farm_assist.right_click_after_poof);
        assert!(settings.heroes.meepo.farm_assist.suspend_on_danger);
        assert_eq!(
            settings.heroes.meepo.farm_assist.suspend_after_manual_combo_ms,
            2500
        );
        assert_eq!(settings.heroes.meepo.farm_assist.poof_press_count, 1);
        assert_eq!(settings.heroes.meepo.farm_assist.poof_press_interval_ms, 35);

        // Verify get_standalone_key returns the correct value for meepo
        assert_eq!(settings.get_standalone_key("meepo"), "Home");
    }

    #[test]
    fn rune_alert_defaults_are_exposed_through_settings() {
        let settings = Settings::default();

        assert!(settings.rune_alerts.enabled);
        assert_eq!(settings.rune_alerts.alert_lead_seconds, 10);
        assert_eq!(settings.rune_alerts.interval_seconds, 120);
        assert!(settings.rune_alerts.audio_enabled);
    }

    #[test]
    fn lane_phase_healing_defaults_are_exposed_through_settings() {
        let settings = Settings::default();

        assert_eq!(settings.common.lane_phase_duration_seconds, 480);
        assert_eq!(settings.common.lane_phase_healing_threshold, 12);
    }
}
