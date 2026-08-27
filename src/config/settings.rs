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
    /// Write a log file to `%LOCALAPPDATA%\dota2-scripts\logs\` as well as the
    /// console.
    ///
    /// Read once at startup — tracing subscribers cannot be swapped out on a
    /// live process — so changing this needs a restart.
    #[serde(default = "default_log_to_file")]
    pub file_enabled: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapfireConfig {
    /// Master toggle for the directional cookie intercept.
    #[serde(default = "default_snapfire_enabled")]
    pub enabled: bool,
    /// Key intercepted to start the combo (default Space).
    #[serde(default = "default_snapfire_trigger_key")]
    pub trigger_key: String,
    /// Firesnap Cookie ability key, self-cast via ALT.
    #[serde(default = "default_snapfire_cookie_key")]
    pub cookie_key: char,
    /// Delay between the facing right-click and the self-cast press (ms).
    #[serde(default = "default_snapfire_turn_delay_ms")]
    pub turn_delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagnusConfig {
    /// Master toggle for the directional Reverse Polarity intercept.
    #[serde(default = "default_magnus_enabled")]
    pub enabled: bool,
    /// Reverse Polarity ability key. This is also the key the hook intercepts.
    #[serde(default = "default_magnus_ultimate_key")]
    pub ultimate_key: char,
    /// Delay between the facing right-click and the ultimate cast (ms).
    #[serde(default = "default_magnus_turn_delay_ms")]
    pub turn_delay_ms: u64,
    /// Pass the key through untouched when Reverse Polarity is not castable,
    /// so a cooldown press never issues the facing right-click.
    #[serde(default = "default_magnus_require_ability_ready")]
    pub require_ability_ready: bool,
    /// Double-tap the hero-select key after the cast to recentre the camera on
    /// Magnus for the Skewer follow-up.
    #[serde(default = "default_magnus_center_camera_on_ultimate")]
    pub center_camera_on_ultimate: bool,
    /// Hero-select key to double-tap. Accepts a character (`"1"`) or a named
    /// key (`"F1"`).
    #[serde(default = "default_magnus_camera_center_key")]
    pub camera_center_key: String,
    /// Delay between the ultimate cast and the first camera tap (ms).
    #[serde(default = "default_magnus_camera_center_delay_ms")]
    pub camera_center_delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmberSpiritConfig {
    /// Master toggle for the remnant chase combo.
    #[serde(default = "default_ember_spirit_enabled")]
    pub enabled: bool,
    /// Fire Remnant ability key, pressed first to drop the remnant.
    #[serde(default = "default_ember_spirit_remnant_key")]
    pub remnant_key: char,
    /// Activate Fire Remnant key, pressed second to dash to the remnant.
    #[serde(default = "default_ember_spirit_activate_key")]
    pub activate_key: char,
    /// Delay between the two presses (ms).
    #[serde(default = "default_ember_spirit_activate_delay_ms")]
    pub activate_delay_ms: u64,
    /// Use `scepter_activate_delay_ms` instead of `activate_delay_ms` while
    /// Ember holds an Aghanim's Scepter. Turn off to keep the configured delay
    /// regardless of scepter.
    #[serde(default = "default_ember_spirit_use_scepter_activate_delay")]
    pub use_scepter_activate_delay: bool,
    /// Delay between the two presses (ms) while Ember holds an Aghanim's
    /// Scepter. Shorter than `activate_delay_ms` because the scepter places the
    /// remnant instantly, but not zero: the activate still has to land after
    /// the remnant exists server-side.
    #[serde(default = "default_ember_spirit_scepter_activate_delay_ms")]
    pub scepter_activate_delay_ms: u64,
    /// Cast Flame Guard automatically while the danger detector is tripped and
    /// HP is at or below the threshold below. Independent of `enabled`, which
    /// only gates the remnant chase.
    #[serde(default = "default_ember_spirit_auto_flame_guard_on_danger")]
    pub auto_flame_guard_on_danger: bool,
    /// Flame Guard ability key pressed by the auto-cast.
    #[serde(default = "default_ember_spirit_flame_guard_key")]
    pub flame_guard_key: char,
    /// Only auto-cast Flame Guard at or below this health percentage.
    #[serde(default = "default_ember_spirit_flame_guard_hp_threshold_percent")]
    pub flame_guard_hp_threshold_percent: u32,
    /// Minimum gap between two auto-casts (ms), so one burst of GSI payloads
    /// cannot spam the key.
    #[serde(default = "default_ember_spirit_flame_guard_trigger_cooldown_ms")]
    pub flame_guard_trigger_cooldown_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiranaConfig {
    /// Master toggle for the directional Leap intercept.
    #[serde(default = "default_mirana_enabled")]
    pub enabled: bool,
    /// Leap ability key. This is also the key the hook intercepts.
    #[serde(default = "default_mirana_leap_key")]
    pub leap_key: char,
    /// Delay between the facing right-click and the Leap cast (ms).
    #[serde(default = "default_mirana_turn_delay_ms")]
    pub turn_delay_ms: u64,
    /// Pass the key through untouched when Leap is not castable, so a cooldown
    /// press never issues the facing right-click.
    #[serde(default = "default_mirana_require_ability_ready")]
    pub require_ability_ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EarthSpiritConfig {
    /// Master toggle for both Earth Spirit combos.
    #[serde(default = "default_earth_spirit_enabled")]
    pub enabled: bool,
    /// Stone Remnant key, pressed first by both combos. Assumes quickcast: a
    /// point-target press without it only arms the cursor, and the second key
    /// then cancels the targeting instead of resolving it.
    #[serde(default = "default_earth_spirit_remnant_key")]
    pub remnant_key: char,
    /// Remap the grip key into remnant-then-grip.
    #[serde(default = "default_earth_spirit_silence_combo_enabled")]
    pub silence_combo_enabled: bool,
    /// Geomagnetic Grip key. This is also the key the hook intercepts.
    #[serde(default = "default_earth_spirit_grip_key")]
    pub grip_key: char,
    /// Delay between the remnant press and the grip press (ms).
    #[serde(default = "default_earth_spirit_silence_remnant_delay_ms")]
    pub silence_remnant_delay_ms: u64,
    /// Pass the grip key through untouched when Geomagnetic Grip is not
    /// castable, so a cooldown press never spends a remnant charge for nothing.
    #[serde(default = "default_earth_spirit_require_grip_ready")]
    pub require_grip_ready: bool,
    /// Remap the roll key into remnant-then-roll.
    #[serde(default = "default_earth_spirit_roll_combo_enabled")]
    pub roll_combo_enabled: bool,
    /// Rolling Boulder key. This is also the key the hook intercepts.
    #[serde(default = "default_earth_spirit_roll_key")]
    pub roll_key: char,
    /// Press the roll key twice. Rolling Boulder is the one key commonly left
    /// off quickcast, where the first press only arms the cursor. Turn this off
    /// if quickcast is on for it, or if the second tap cancels the targeting
    /// instead of firing.
    #[serde(default = "default_earth_spirit_roll_double_tap")]
    pub roll_double_tap: bool,
    /// Gap between the two roll presses (ms). Ignored when `roll_double_tap` is
    /// off.
    #[serde(default = "default_earth_spirit_roll_double_tap_delay_ms")]
    pub roll_double_tap_delay_ms: u64,
    /// Gap between the roll press and the remnant press (ms). Measured from the
    /// press that actually fires the roll, so from the second tap when
    /// `roll_double_tap` is on. The whole remnant sequence must finish inside
    /// `ROLLING_BOULDER_WINDUP_MS`.
    #[serde(default = "default_earth_spirit_roll_to_remnant_delay_ms")]
    pub roll_to_remnant_delay_ms: u64,
    /// Hold ALT across the remnant press — Dota's self-cast modifier, needed to
    /// self-cast a point-target ability that is on quickcast.
    #[serde(default = "default_earth_spirit_roll_remnant_alt")]
    pub roll_remnant_alt: bool,
    /// Double-tap the remnant key, Dota's other route to self-cast. Self-cast
    /// drops the stone on Earth Spirit, and the roll starts from Earth Spirit,
    /// so the boulder passes through it without any aiming. Turn this off to go
    /// back to placing the remnant at the cursor.
    #[serde(default = "default_earth_spirit_roll_remnant_double_tap")]
    pub roll_remnant_double_tap: bool,
    /// Gap between the two remnant presses (ms). Ignored when
    /// `roll_remnant_double_tap` is off.
    #[serde(default = "default_earth_spirit_roll_remnant_double_tap_delay_ms")]
    pub roll_remnant_double_tap_delay_ms: u64,
    /// Pass the roll key through untouched when Rolling Boulder is not
    /// castable.
    #[serde(default = "default_earth_spirit_require_roll_ready")]
    pub require_roll_ready: bool,
    /// Self-cast Enchant Remnant (scepter) when the danger detector fires and
    /// HP is at or below the threshold below. Independent of `enabled`, which
    /// only gates the two keyboard combos.
    #[serde(default = "default_earth_spirit_auto_petrify_on_danger")]
    pub auto_petrify_on_danger: bool,
    /// Enchant Remnant key — the scepter ability slot, not one of the four
    /// base abilities.
    #[serde(default = "default_earth_spirit_petrify_key")]
    pub petrify_key: char,
    /// Only self-petrify at or below this health percentage.
    #[serde(default = "default_earth_spirit_petrify_hp_threshold_percent")]
    pub petrify_hp_threshold_percent: u32,
    /// Minimum gap between two auto-casts (ms), so one burst of GSI payloads
    /// cannot spam the key.
    #[serde(default = "default_earth_spirit_petrify_trigger_cooldown_ms")]
    pub petrify_trigger_cooldown_ms: u64,
    /// Hold ALT across the Enchant Remnant press — Dota's self-cast modifier.
    #[serde(default = "default_earth_spirit_petrify_alt")]
    pub petrify_alt: bool,
    /// Double-tap the Enchant Remnant key, Dota's other route to self-cast.
    /// Independent of the ALT route so either can be dropped.
    #[serde(default = "default_earth_spirit_petrify_double_tap")]
    pub petrify_double_tap: bool,
    /// Gap between the two Enchant Remnant presses (ms). Ignored when
    /// `petrify_double_tap` is off.
    #[serde(default = "default_earth_spirit_petrify_double_tap_delay_ms")]
    pub petrify_double_tap_delay_ms: u64,
    /// Follow the self-petrify with a Boulder Smash, kicking the remnant —
    /// which is Earth Spirit himself — away from whatever is killing him.
    #[serde(default = "default_earth_spirit_petrify_smash_enabled")]
    pub petrify_smash_enabled: bool,
    /// Boulder Smash key, pressed once by the escape. Never intercepted.
    #[serde(default = "default_earth_spirit_smash_key")]
    pub smash_key: char,
    /// Gap between the Enchant Remnant press and the Boulder Smash press (ms).
    /// The petrify has to resolve server-side first: there is no remnant to
    /// kick until it does.
    #[serde(default = "default_earth_spirit_petrify_to_smash_delay_ms")]
    pub petrify_to_smash_delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlarkConfig {
    /// Master toggle for the directional Pounce intercept.
    #[serde(default = "default_slark_enabled")]
    pub enabled: bool,
    /// Pounce ability key. This is also the key the hook intercepts.
    #[serde(default = "default_slark_pounce_key")]
    pub pounce_key: char,
    /// Delay between the facing right-click and the Pounce cast (ms).
    #[serde(default = "default_slark_turn_delay_ms")]
    pub turn_delay_ms: u64,
    /// Pass the key through untouched when Pounce is not castable, so a
    /// cooldown press never issues the facing right-click.
    #[serde(default = "default_slark_require_ability_ready")]
    pub require_ability_ready: bool,
    /// Cast Dark Pact automatically when GSI reports a debuff on Slark.
    #[serde(default = "default_slark_auto_dark_pact_on_debuff")]
    pub auto_dark_pact_on_debuff: bool,
    /// Dark Pact ability key, pressed by the auto-cleanse.
    #[serde(default = "default_slark_dark_pact_key")]
    pub dark_pact_key: char,
    /// Settle window after the first debuff before casting, so a burst of
    /// debuffs is cleansed by a single Dark Pact.
    #[serde(default = "default_slark_dark_pact_delay_ms")]
    pub dark_pact_delay_ms: u64,
    /// Also require the danger detector, not just `has_debuff`.
    ///
    /// GSI reports one `has_debuff` bool for every modifier in the game, so a
    /// creep's slow reads exactly like a Doom. On — the default — the cleanse
    /// also needs to be actually under fire, which is what stops Dark Pact
    /// being spent on chip damage while farming at full HP.
    #[serde(default = "default_slark_dark_pact_require_danger")]
    pub dark_pact_require_danger: bool,
    /// Cast Shadow Dance when the danger detector fires below the HP line.
    #[serde(default = "default_slark_auto_shadow_dance_on_low_hp")]
    pub auto_shadow_dance_on_low_hp: bool,
    /// Shadow Dance ability key.
    #[serde(default = "default_slark_shadow_dance_key")]
    pub shadow_dance_key: char,
    /// HP percentage at or below which the escape fires.
    #[serde(default = "default_slark_shadow_dance_hp_threshold_percent")]
    pub shadow_dance_hp_threshold_percent: u32,
    /// Also require the danger detector, not just the HP line.
    ///
    /// On, this reads as "low *and* actually under fire", so limping home at
    /// 30% never spends the ultimate. Off, the HP line alone is enough.
    #[serde(default = "default_slark_shadow_dance_require_danger")]
    pub shadow_dance_require_danger: bool,
    /// Minimum gap between two escape attempts (ms).
    #[serde(default = "default_slark_shadow_dance_trigger_cooldown_ms")]
    pub shadow_dance_trigger_cooldown_ms: u64,
    /// How long Shadow Dance stays active after a cast (ms).
    ///
    /// The shard fallback is held for this long after the ultimate goes off, so
    /// the two do not pop together — while Shadow Dance is running it is
    /// already doing the job. Defaults to its level-1 duration; raise it to
    /// 5000 / 6000 as the ultimate levels up.
    #[serde(default = "default_slark_shadow_dance_active_ms")]
    pub shadow_dance_active_ms: u64,
    /// Fall back to the shard ability when Shadow Dance is on cooldown.
    #[serde(default = "default_slark_shard_fallback_enabled")]
    pub shard_fallback_enabled: bool,
    /// Key the shard ability sits on.
    ///
    /// This is the whole identity of the ability as far as automation is
    /// concerned: it is the key we press, and its slot in Dota's fixed
    /// `Q/W/E/R/D/F` order is what the readiness check reads — the same
    /// index/key pairing [`AutoAbilityConfig`] documents.
    ///
    /// Dota will not self-cast this ability, so it is aimed by clicking the HUD
    /// hero portrait — see `[hud]`. That means the portrait anchor must be
    /// calibrated before the fallback can fire at all.
    #[serde(default = "default_slark_shard_key")]
    pub shard_key: char,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InvokerProfileMode {
    Combo,
    Prep,
}

impl InvokerProfileMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Combo => "combo",
            Self::Prep => "prep",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InvokerProfileExecutionStyle {
    Automatic,
    SemiAuto,
}

fn default_invoker_profile_execution_style() -> InvokerProfileExecutionStyle {
    InvokerProfileExecutionStyle::Automatic
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InvokerProfileStepKind {
    Spell,
    Item,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InvokerProfileStepCompletionMode {
    FixedDelay,
    WaitForCooldown,
}

fn default_invoker_profile_step_completion_mode() -> InvokerProfileStepCompletionMode {
    InvokerProfileStepCompletionMode::FixedDelay
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InvokerProfileStepCastBehavior {
    Normal,
    ManualWaitCooldown,
    AltCast,
    DoubleTap,
    AltDoubleTap,
}

fn default_invoker_profile_step_cast_behavior() -> InvokerProfileStepCastBehavior {
    InvokerProfileStepCastBehavior::Normal
}

fn default_invoker_profile_step_completion_timeout_ms() -> u64 {
    3000
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InvokerProfileStep {
    pub kind: InvokerProfileStepKind,
    pub target: String,
    #[serde(default)]
    pub delay_after_ms: u64,
    #[serde(default = "default_invoker_profile_step_cast_behavior")]
    pub cast_behavior: InvokerProfileStepCastBehavior,
    #[serde(default = "default_invoker_profile_step_completion_mode")]
    pub completion_mode: InvokerProfileStepCompletionMode,
    #[serde(default = "default_invoker_profile_step_completion_timeout_ms")]
    pub completion_timeout_ms: u64,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InvokerProfile {
    pub id: String,
    pub name: String,
    #[serde(default = "default_invoker_profile_enabled")]
    pub enabled: bool,
    pub hotkey: String,
    pub mode: InvokerProfileMode,
    #[serde(default = "default_invoker_profile_execution_style")]
    pub execution_style: InvokerProfileExecutionStyle,
    #[serde(default)]
    pub build_tag: String,
    #[serde(default)]
    pub steps: Vec<InvokerProfileStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvokerConfig {
    #[serde(default = "default_invoker_quas_key")]
    pub quas_key: char,
    #[serde(default = "default_invoker_wex_key")]
    pub wex_key: char,
    #[serde(default = "default_invoker_exort_key")]
    pub exort_key: char,
    #[serde(default = "default_invoker_invoke_key")]
    pub invoke_key: char,
    #[serde(default = "default_invoker_spell_slot_primary_key")]
    pub spell_slot_primary_key: char,
    #[serde(default = "default_invoker_spell_slot_secondary_key")]
    pub spell_slot_secondary_key: char,
    #[serde(default = "default_invoker_cycle_combo_profiles_hotkey")]
    pub cycle_combo_profiles_hotkey: String,
    #[serde(default = "default_invoker_profiles")]
    pub profiles: Vec<InvokerProfile>,
    #[serde(default)]
    pub armlet: HeroArmletOverrideConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroesConfig {
    #[serde(default)]
    pub huskar: HuskarConfig,
    #[serde(default)]
    pub invoker: InvokerConfig,
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
    #[serde(default)]
    pub snapfire: SnapfireConfig,
    #[serde(default)]
    pub magnus: MagnusConfig,
    #[serde(default)]
    pub mirana: MiranaConfig,
    #[serde(default)]
    pub ember_spirit: EmberSpiritConfig,
    #[serde(default)]
    pub earth_spirit: EarthSpiritConfig,
    #[serde(default)]
    pub slark: SlarkConfig,
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
    #[serde(default = "default_auto_mjollnir")]
    pub auto_mjollnir: bool,
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
pub struct PhaseBootsAutomationConfig {
    #[serde(default = "default_phase_boots_automation_enabled")]
    pub enabled: bool,
    #[serde(default = "default_phase_boots_minimum_distance_units")]
    pub minimum_distance_units: u32,
    #[serde(default = "default_phase_boots_excluded_heroes")]
    pub excluded_heroes: Vec<String>,
}

/// What automation may do while the hero is invisible from an item.
///
/// Shadow Blade and Silver Edge invisibility drops the moment the hero casts an
/// ability or activates an item — which is exactly what every automation here
/// does. The invisibility was almost always bought as an escape, so spending it
/// on a Phase Boots activation or a Dark Pact cleanse is a bad trade the player
/// never asked for.
///
/// This was originally `[phase_boots_automation] suppress_while_invisible`, back
/// when Phase Boots was the only automation that could break invisibility.
/// `Settings::load` still honours that key when this section is absent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvisibilityConfig {
    /// Hold every invisibility-breaking automation while the window is running.
    ///
    /// Automation that *grants* invisibility is deliberately not covered — Slark's
    /// Shadow Dance and Depth Shroud replace the window with a better one rather
    /// than ending it. Neither is Soul Ring or Armlet: both are driven by the
    /// player's own keypress or by imminent death, where the invisibility is
    /// already forfeit or worth less than surviving.
    #[serde(default = "default_suppress_automation_while_invisible")]
    pub suppress_automation: bool,
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

/// Draft-phase hero identification (the draft helper's data source).
///
/// Off by default: it spawns a capture thread that reads the screen once a
/// second during hero selection, and nothing else in the app depends on it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftConfig {
    #[serde(default = "default_draft_enabled")]
    pub enabled: bool,
    /// Capture cadence while the draft gate is open.
    #[serde(default = "default_draft_poll_ms")]
    pub poll_ms: u64,
    /// Save strip captures + per-frame read logs for offline evaluation.
    #[serde(default = "default_draft_telemetry_enabled")]
    pub telemetry_enabled: bool,
    /// Session logs land here, one subdirectory per draft. Relative values
    /// resolve against the app data dir, like `[gsi_logging].output_dir`.
    #[serde(default = "default_draft_telemetry_dir")]
    pub telemetry_dir: String,
    /// Every Nth frame's PNG is kept (the JSONL logs every frame regardless).
    /// A ranked draft at 1Hz is several hundred frames; at ~144KB a strip,
    /// keeping them all would be tens of MB per game.
    #[serde(default = "default_draft_telemetry_save_every_n")]
    pub telemetry_save_every_n: u32,
    /// Save portraits the matcher cannot identify (arcanas, personas) as new
    /// exemplars, labelled by GSI. They join the reference pool on the next
    /// draft and can be baked into the shipped pack later.
    #[serde(default = "default_draft_harvest_enabled")]
    pub harvest_enabled: bool,
    /// Harvested exemplars land here. Relative resolves against app data dir.
    #[serde(default = "default_draft_exemplar_dir")]
    pub exemplar_dir: String,
}

/// STRATZ-backed draft advice: counters, synergy and position fit.
///
/// Separate from `[draft]` because identification and advice fail
/// independently — the reader is useful with no API token, and the advice
/// would still be wanted if the reader were replaced.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StratzConfig {
    #[serde(default = "default_stratz_enabled")]
    pub enabled: bool,
    /// STRATZ API token. Leave empty and set `STRATZ_TOKEN` in the
    /// environment instead to keep the secret off disk entirely.
    #[serde(default)]
    pub api_token: String,
    /// Rank bracket the statistics describe: `DIVINE_IMMORTAL`,
    /// `LEGEND_ANCIENT`, `CRUSADER_ARCHON`, `HERALD_GUARDIAN`.
    #[serde(default = "default_stratz_bracket")]
    pub bracket: String,
    /// Cached dataset path. Relative resolves against the app data dir.
    #[serde(default = "default_stratz_cache_path")]
    pub cache_path: String,
    /// How long a cached dataset is reused before a background refresh.
    /// Matchup data is a per-patch aggregate, so a day is generous.
    #[serde(default = "default_stratz_cache_ttl_hours")]
    pub cache_ttl_hours: u64,
    /// Position 1-5 currently being queued for; advice is filtered to it.
    /// 0 means "no position filter".
    #[serde(default)]
    pub position: u8,
    /// Matchup sample size trusted at half weight. Lower trusts small
    /// samples more; see `advisor::DEFAULT_SHRINK_K`.
    #[serde(default = "default_stratz_shrink_k")]
    pub shrink_k: f32,
    /// Weight of a hero's own strength relative to matchup effects.
    #[serde(default = "default_stratz_base_weight")]
    pub base_weight: f32,
    #[serde(default = "default_stratz_synergy_weight")]
    pub synergy_weight: f32,
    /// How many suggestions to surface.
    #[serde(default = "default_stratz_suggestion_count")]
    pub suggestion_count: usize,
}

fn default_stratz_enabled() -> bool {
    false
}
fn default_stratz_bracket() -> String {
    "DIVINE_IMMORTAL".to_string()
}
fn default_stratz_cache_path() -> String {
    "stratz/dataset.bin".to_string()
}
fn default_stratz_cache_ttl_hours() -> u64 {
    24
}
fn default_stratz_shrink_k() -> f32 {
    50.0
}
fn default_stratz_base_weight() -> f32 {
    0.4
}
fn default_stratz_synergy_weight() -> f32 {
    1.0
}
fn default_stratz_suggestion_count() -> usize {
    12
}

impl Default for StratzConfig {
    fn default() -> Self {
        Self {
            enabled: default_stratz_enabled(),
            api_token: String::new(),
            bracket: default_stratz_bracket(),
            cache_path: default_stratz_cache_path(),
            cache_ttl_hours: default_stratz_cache_ttl_hours(),
            position: 0,
            shrink_k: default_stratz_shrink_k(),
            base_weight: default_stratz_base_weight(),
            synergy_weight: default_stratz_synergy_weight(),
            suggestion_count: default_stratz_suggestion_count(),
        }
    }
}

fn default_draft_enabled() -> bool {
    false
}
fn default_draft_poll_ms() -> u64 {
    1000
}
fn default_draft_telemetry_enabled() -> bool {
    true
}
fn default_draft_telemetry_dir() -> String {
    "logs/draft_telemetry".to_string()
}
fn default_draft_telemetry_save_every_n() -> u32 {
    5
}
fn default_draft_harvest_enabled() -> bool {
    true
}
fn default_draft_exemplar_dir() -> String {
    "draft_exemplars".to_string()
}

impl Default for DraftConfig {
    fn default() -> Self {
        Self {
            enabled: default_draft_enabled(),
            poll_ms: default_draft_poll_ms(),
            telemetry_enabled: default_draft_telemetry_enabled(),
            telemetry_dir: default_draft_telemetry_dir(),
            telemetry_save_every_n: default_draft_telemetry_save_every_n(),
            harvest_enabled: default_draft_harvest_enabled(),
            exemplar_dir: default_draft_exemplar_dir(),
        }
    }
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

/// Calibration for clock-driven creep wave prediction.
///
/// The meet values are empirical approximations, not derived constants — they are
/// exposed so they can be retuned against observed play without a rebuild. See
/// `src/observability/wave_tracker.rs` for the model they feed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveTrackerConfig {
    #[serde(default = "default_wave_tracker_enabled")]
    pub enabled: bool,
    /// Seconds after spawn at which the mid waves meet.
    #[serde(default = "default_mid_meet_seconds")]
    pub mid_meet_seconds: f32,
    /// Normalised lane position of the mid clash. 0.5 is the exact midpoint.
    #[serde(default = "default_mid_meet_progress")]
    pub mid_meet_progress: f32,
    /// Seconds after spawn at which the side-lane waves meet.
    #[serde(default = "default_side_meet_seconds")]
    pub side_meet_seconds: f32,
    /// Normalised position of the top-lane clash; below 0.5, biased toward the
    /// Radiant offlane tower.
    #[serde(default = "default_top_meet_progress")]
    pub top_meet_progress: f32,
    /// Normalised position of the bottom-lane clash; above 0.5, mirroring top.
    #[serde(default = "default_bottom_meet_progress")]
    pub bottom_meet_progress: f32,
    /// Game time below which predictions are reported as high confidence.
    #[serde(default = "default_confidence_high_seconds")]
    pub confidence_high_seconds: i32,
    /// Game time below which predictions are reported as degrading, and at or
    /// above which they are reported as low confidence.
    #[serde(default = "default_confidence_degrading_seconds")]
    pub confidence_degrading_seconds: i32,
}

impl Default for WaveTrackerConfig {
    fn default() -> Self {
        Self {
            enabled: default_wave_tracker_enabled(),
            mid_meet_seconds: default_mid_meet_seconds(),
            mid_meet_progress: default_mid_meet_progress(),
            side_meet_seconds: default_side_meet_seconds(),
            top_meet_progress: default_top_meet_progress(),
            bottom_meet_progress: default_bottom_meet_progress(),
            confidence_high_seconds: default_confidence_high_seconds(),
            confidence_degrading_seconds: default_confidence_degrading_seconds(),
        }
    }
}

fn default_wave_tracker_enabled() -> bool {
    true
}
fn default_mid_meet_seconds() -> f32 {
    17.0
}
fn default_mid_meet_progress() -> f32 {
    0.5
}
fn default_side_meet_seconds() -> f32 {
    28.0
}
fn default_top_meet_progress() -> f32 {
    0.42
}
fn default_bottom_meet_progress() -> f32 {
    0.58
}
fn default_confidence_high_seconds() -> i32 {
    600
}
fn default_confidence_degrading_seconds() -> i32 {
    900
}

/// Per-event alert settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AlertEventConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// How many seconds before the event the cue plays.
    #[serde(default = "default_alert_lead_seconds")]
    pub lead_seconds: i32,
    /// Per-event volume, `0.0`-`1.0`, scaled by `[alerts] master_volume`.
    #[serde(default = "default_alert_volume")]
    pub volume: f32,
    /// Path to a custom `.wav` / `.mp3`. Empty means use the built-in cue.
    #[serde(default)]
    pub sound_file: String,
}

impl AlertEventConfig {
    fn new(enabled: bool, lead_seconds: i32) -> Self {
        Self {
            enabled,
            lead_seconds,
            volume: default_alert_volume(),
            sound_file: String::new(),
        }
    }
}

impl Default for AlertEventConfig {
    fn default() -> Self {
        Self::new(true, default_alert_lead_seconds())
    }
}

/// Scheduled map-objective audio alerts.
///
/// Each event is configured independently so lead times can match how much
/// warning that objective actually needs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AlertsConfig {
    /// Master switch. When false nothing sounds, whatever the per-event values.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Master volume, `0.0`-`1.0`, multiplied into every per-event volume.
    #[serde(default = "default_alert_master_volume")]
    pub master_volume: f32,
    /// Voice pack directory name under `assets/voice/`. Empty uses the
    /// built-in synthesised cues.
    #[serde(default)]
    pub voice_pack: String,
    #[serde(default = "default_power_rune_alert")]
    pub power_rune: AlertEventConfig,
    #[serde(default = "default_wisdom_rune_alert")]
    pub wisdom_rune: AlertEventConfig,
    #[serde(default = "default_water_rune_alert")]
    pub water_rune: AlertEventConfig,
    #[serde(default = "default_bounty_rune_alert")]
    pub bounty_rune: AlertEventConfig,
    #[serde(default = "default_tormentor_alert")]
    pub tormentor: AlertEventConfig,
    #[serde(default = "default_neutral_item_alert")]
    pub neutral_item: AlertEventConfig,
    #[serde(default = "default_stack_alert")]
    pub stack: AlertEventConfig,
}

impl AlertsConfig {
    /// Settings for one event.
    pub fn for_event(
        &self,
        event: crate::observability::alerts::AlertEvent,
    ) -> &AlertEventConfig {
        use crate::observability::alerts::AlertEvent;
        match event {
            AlertEvent::PowerRune => &self.power_rune,
            AlertEvent::WisdomRune => &self.wisdom_rune,
            AlertEvent::WaterRune => &self.water_rune,
            AlertEvent::BountyRune => &self.bounty_rune,
            AlertEvent::Tormentor => &self.tormentor,
            AlertEvent::NeutralItem => &self.neutral_item,
            AlertEvent::Stack => &self.stack,
        }
    }
}

impl Default for AlertsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            master_volume: default_alert_master_volume(),
            voice_pack: String::new(),
            power_rune: default_power_rune_alert(),
            wisdom_rune: default_wisdom_rune_alert(),
            water_rune: default_water_rune_alert(),
            bounty_rune: default_bounty_rune_alert(),
            tormentor: default_tormentor_alert(),
            neutral_item: default_neutral_item_alert(),
            stack: default_stack_alert(),
        }
    }
}

fn default_true() -> bool {
    true
}
fn default_alert_lead_seconds() -> i32 {
    15
}
fn default_alert_volume() -> f32 {
    1.0
}
fn default_alert_master_volume() -> f32 {
    0.8
}
fn default_power_rune_alert() -> AlertEventConfig {
    AlertEventConfig::new(true, 15)
}
fn default_wisdom_rune_alert() -> AlertEventConfig {
    AlertEventConfig::new(true, 20)
}
fn default_water_rune_alert() -> AlertEventConfig {
    AlertEventConfig::new(true, 15)
}
fn default_bounty_rune_alert() -> AlertEventConfig {
    AlertEventConfig::new(true, 15)
}
/// Off by default: Tormentor matters only to some roles, and a 20-minute-in
/// alert is noise for everyone else.
fn default_tormentor_alert() -> AlertEventConfig {
    AlertEventConfig::new(false, 30)
}
fn default_neutral_item_alert() -> AlertEventConfig {
    AlertEventConfig::new(true, 10)
}
/// Off by default: this one fires every single minute.
fn default_stack_alert() -> AlertEventConfig {
    AlertEventConfig::new(false, 5)
}

/// Click-through overlay drawn on top of Dota 2's in-game minimap.
///
/// The overlay is positioned from the Dota client rect plus the `[minimap_capture]`
/// region, so those offsets are the single source of truth for where the minimap is;
/// `offset_x` / `offset_y` here are only a nudge for fine alignment.
///
/// # Window space vs map space
///
/// The overlay window covers Dota's whole minimap *panel*, but the playable map
/// texture is inset inside that panel's bezel and corner buttons. Painting
/// normalised map space straight onto the window therefore lands everything a few
/// percent off — the error that `map_offset_*` / `map_scale_*` correct. They
/// describe where map space sits inside the window, so they are a property of
/// Dota's UI layout rather than of the map, and need re-checking after a UI patch
/// or a resolution change. See `WaveMap` on the UI side.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveOverlayConfig {
    #[serde(default = "default_wave_overlay_enabled")]
    pub enabled: bool,
    /// Global hotkey that shows/hides the overlay. Blocked from reaching Dota.
    #[serde(default = "default_wave_overlay_toggle_key")]
    pub toggle_key: String,
    /// Horizontal nudge in physical pixels, applied on top of the minimap region.
    #[serde(default)]
    pub offset_x: i32,
    /// Vertical nudge in physical pixels, applied on top of the minimap region.
    #[serde(default)]
    pub offset_y: i32,
    /// Overall overlay opacity, 0.0-1.0.
    #[serde(default = "default_wave_overlay_opacity")]
    pub opacity: f32,
    /// Draw the lane polylines and river behind the wave dots.
    #[serde(default = "default_wave_overlay_show_lane_lines")]
    pub show_lane_lines: bool,
    /// Horizontal shift of map space within the window, as a fraction of window
    /// width. Positive moves the drawing right.
    #[serde(default = "default_wave_overlay_map_offset_x")]
    pub map_offset_x: f32,
    /// Vertical shift of map space, as a fraction of window height. Positive
    /// moves the drawing *down*, matching screen coordinates.
    #[serde(default = "default_wave_overlay_map_offset_y")]
    pub map_offset_y: f32,
    /// Width of map space as a fraction of the window, scaled about its centre.
    #[serde(default = "default_wave_overlay_map_scale_x")]
    pub map_scale_x: f32,
    /// Height of map space as a fraction of the window, scaled about its centre.
    #[serde(default = "default_wave_overlay_map_scale_y")]
    pub map_scale_y: f32,
    /// Draw the map-space bounding box and centre crosshair, and force the lane
    /// lines on, so the alignment controls have something to align *to*.
    #[serde(default)]
    pub calibrating: bool,
}

impl Default for WaveOverlayConfig {
    fn default() -> Self {
        Self {
            enabled: default_wave_overlay_enabled(),
            toggle_key: default_wave_overlay_toggle_key(),
            offset_x: 0,
            offset_y: 0,
            opacity: default_wave_overlay_opacity(),
            show_lane_lines: default_wave_overlay_show_lane_lines(),
            map_offset_x: default_wave_overlay_map_offset_x(),
            map_offset_y: default_wave_overlay_map_offset_y(),
            map_scale_x: default_wave_overlay_map_scale_x(),
            map_scale_y: default_wave_overlay_map_scale_y(),
            calibrating: false,
        }
    }
}

fn default_wave_overlay_enabled() -> bool {
    false
}
fn default_wave_overlay_toggle_key() -> String {
    "F8".to_string()
}
fn default_wave_overlay_opacity() -> f32 {
    0.85
}
/// Off by default: the overlay sits on top of Dota's own minimap, which already
/// draws the lanes and river. Repeating them adds clutter over the thing they
/// duplicate. The in-app panel has no map underneath it, so it keeps its lines.
fn default_wave_overlay_show_lane_lines() -> bool {
    false
}

// Alignment defaults, fitted to tower positions measured off a 2560x1440
// borderless screenshot at the stock `[minimap_capture]` region. The panel is
// taller than the map texture it frames, which is why the vertical scale is
// further from 1.0 than the horizontal one. These are a starting point, not a
// constant of nature: re-run the in-app calibration if the overlay looks off.
fn default_wave_overlay_map_offset_x() -> f32 {
    -0.020
}
fn default_wave_overlay_map_offset_y() -> f32 {
    0.015
}
fn default_wave_overlay_map_scale_x() -> f32 {
    0.993
}
fn default_wave_overlay_map_scale_y() -> f32 {
    0.929
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
    pub phase_boots_automation: PhaseBootsAutomationConfig,
    #[serde(default)]
    pub invisibility: InvisibilityConfig,
    #[serde(default)]
    pub soul_ring: SoulRingConfig,
    #[serde(default)]
    pub gsi_logging: GsiLoggingConfig,
    #[serde(default)]
    pub updates: UpdateConfig,
    #[serde(default)]
    pub rune_alerts: RuneAlertConfig,
    #[serde(default)]
    pub draft: DraftConfig,
    #[serde(default)]
    pub stratz: StratzConfig,
    #[serde(default)]
    pub minimap_capture: MinimapCaptureConfig,
    #[serde(default)]
    pub minimap_analysis: MinimapAnalysisConfig,
    #[serde(default)]
    pub wave_tracker: WaveTrackerConfig,
    #[serde(default)]
    pub wave_overlay: WaveOverlayConfig,
    #[serde(default)]
    pub alerts: AlertsConfig,
    #[serde(default)]
    pub hud: HudConfig,
}

/// Points on Dota's own HUD that automation needs to click.
///
/// Some abilities cannot be self-cast — Dota resolves them wherever the mouse
/// happens to be. Clicking the hero portrait is the only way to land those on
/// your own hero, which means the app needs to know where the portrait is.
///
/// Positions are fractions of Dota's **client rect**, not of the display, so a
/// calibration survives moving the window, changing resolution, and a second
/// monitor. Where the HUD sits inside that rect still depends on Dota's UI
/// scale, so it has to be calibrated per setup — same situation as
/// `[wave_overlay]`'s `map_offset_*`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HudConfig {
    /// Hero portrait X, as a fraction of Dota's client width.
    #[serde(default = "default_hud_portrait_x_fraction")]
    pub portrait_x_fraction: f32,
    /// Hero portrait Y, as a fraction of Dota's client height.
    #[serde(default = "default_hud_portrait_y_fraction")]
    pub portrait_y_fraction: f32,
    /// Whether the portrait above was actually measured.
    ///
    /// Defaults to `false`, and nothing may click the portrait until it is
    /// true. The shipped fractions are a starting point for the Test button
    /// only — a guessed coordinate must never be able to send a stray click
    /// into the game world.
    #[serde(default)]
    pub portrait_calibrated: bool,
    /// Hotkey that records the cursor's position as the portrait anchor.
    #[serde(default = "default_hud_capture_portrait_key")]
    pub capture_portrait_key: String,
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
fn default_log_to_file() -> bool {
    true
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
fn default_snapfire_enabled() -> bool {
    true
}
fn default_snapfire_trigger_key() -> String {
    "Space".to_string()
}
fn default_snapfire_cookie_key() -> char {
    'w'
}
fn default_snapfire_turn_delay_ms() -> u64 {
    60
}
fn default_magnus_enabled() -> bool {
    true
}
fn default_magnus_ultimate_key() -> char {
    'r'
}
fn default_magnus_turn_delay_ms() -> u64 {
    60
}
fn default_magnus_require_ability_ready() -> bool {
    true
}
fn default_magnus_center_camera_on_ultimate() -> bool {
    true
}
fn default_magnus_camera_center_key() -> String {
    "1".to_string()
}
fn default_magnus_camera_center_delay_ms() -> u64 {
    60
}
fn default_ember_spirit_enabled() -> bool {
    true
}
fn default_ember_spirit_remnant_key() -> char {
    'r'
}
fn default_ember_spirit_activate_key() -> char {
    'd'
}
/// Long enough for the remnant to exist server-side before the activate goes
/// out. Too short and Ember dashes to the remnants that were already on the map
/// while the new one is still being placed; too long and the chase is sluggish.
fn default_ember_spirit_activate_delay_ms() -> u64 {
    150
}
fn default_ember_spirit_use_scepter_activate_delay() -> bool {
    true
}
/// A fraction of the un-upgraded delay, not zero. The scepter takes the
/// remnant's travel time out of the wait, but the activate still has to land
/// after the remnant registers server-side — press both in the same tick and
/// the dash misses the remnant that was just placed.
fn default_ember_spirit_scepter_activate_delay_ms() -> u64 {
    40
}
fn default_ember_spirit_auto_flame_guard_on_danger() -> bool {
    true
}
fn default_ember_spirit_flame_guard_key() -> char {
    'e'
}
/// Higher than OD's 55 because Flame Guard is a damage *shield*, not a nuke:
/// its value comes from being up before the burst lands, so waiting until Ember
/// is nearly dead wastes most of the absorb.
fn default_ember_spirit_flame_guard_hp_threshold_percent() -> u32 {
    65
}
fn default_ember_spirit_flame_guard_trigger_cooldown_ms() -> u64 {
    2000
}
fn default_mirana_enabled() -> bool {
    true
}
fn default_mirana_leap_key() -> char {
    'e'
}
/// Same 200ms Slark's Pounce settled on, and for the same reason: the 60ms the
/// Magnus and Snapfire combos use is enough for a facing that only has to be
/// approximately right, but a leap that fires mid-turn lands somewhere else.
fn default_mirana_turn_delay_ms() -> u64 {
    200
}
fn default_mirana_require_ability_ready() -> bool {
    true
}
fn default_earth_spirit_enabled() -> bool {
    true
}
fn default_earth_spirit_remnant_key() -> char {
    'd'
}
fn default_earth_spirit_silence_combo_enabled() -> bool {
    true
}
fn default_earth_spirit_grip_key() -> char {
    'e'
}
/// The grip resolves the moment it is pressed, so this is only the window the
/// remnant needs to register server-side. Ember's remnant chase settled on
/// 150ms for the same problem with a travelling remnant; Stone Remnant is
/// placed instantly, so it wants less.
fn default_earth_spirit_silence_remnant_delay_ms() -> u64 {
    120
}
fn default_earth_spirit_require_grip_ready() -> bool {
    true
}
fn default_earth_spirit_roll_combo_enabled() -> bool {
    true
}
fn default_earth_spirit_roll_key() -> char {
    'w'
}
/// Gap between the roll firing and the remnant press.
///
/// With the remnant self-cast (the default), this is no longer an aiming
/// window — the remnant lands on Earth Spirit regardless of the cursor, so
/// there is nothing for a human to do here and the delay only has to clear the
/// roll press and land inside the ~600ms windup. Sooner is better: the remnant
/// wants to exist before the boulder leaves its own starting position.
///
/// Turn `roll_remnant_double_tap` off and the remnant goes back to the cursor,
/// which *does* need aiming time — raise this to ~300ms if you do that.
fn default_earth_spirit_roll_to_remnant_delay_ms() -> u64 {
    120
}
/// Hold ALT across the remnant press.
///
/// Dota's self-cast modifier, which is what makes a quickcast point-target
/// ability land on the hero instead of the cursor. Paired with the double-tap
/// below so either route to self-cast works.
fn default_earth_spirit_roll_remnant_alt() -> bool {
    true
}
/// Double-tap the remnant key, which self-casts it in Dota's default binding.
///
/// This is the whole point of the roll combo's remnant: self-cast puts the
/// stone on Earth Spirit himself, and the roll starts from Earth Spirit — so
/// the boulder passes through it every time and the 1600-unit roll needs no
/// aiming at all.
fn default_earth_spirit_roll_remnant_double_tap() -> bool {
    true
}
fn default_earth_spirit_roll_remnant_double_tap_delay_ms() -> u64 {
    60
}
/// Rolling Boulder's windup, and therefore the hard ceiling on the aiming
/// window above. Not configurable: it is a property of the ability, not a
/// preference.
pub const ROLLING_BOULDER_WINDUP_MS: u64 = 600;
fn default_earth_spirit_roll_double_tap() -> bool {
    true
}
fn default_earth_spirit_roll_double_tap_delay_ms() -> u64 {
    60
}
fn default_earth_spirit_require_roll_ready() -> bool {
    true
}
fn default_earth_spirit_auto_petrify_on_danger() -> bool {
    true
}
/// Enchant Remnant sits in the scepter slot, which is `F` on the default
/// bindings — the slot after Stone Remnant's `D`.
fn default_earth_spirit_petrify_key() -> char {
    'f'
}
/// Unlike Ember's Flame Guard, which wants to be up *before* the burst, this is
/// a panic button: it takes Earth Spirit out of the fight entirely and spends
/// the scepter ability doing it. Fire it late, when the alternative is dying.
fn default_earth_spirit_petrify_hp_threshold_percent() -> u32 {
    22
}
/// Longer than Ember's 2s Flame Guard retry: Earth Spirit is petrified for
/// several seconds after a successful cast, and GSI keeps reporting the low HP
/// that triggered it the whole time.
fn default_earth_spirit_petrify_trigger_cooldown_ms() -> u64 {
    5000
}
/// Hold ALT across the Enchant Remnant press.
///
/// Same pair of self-cast routes as the roll combo's remnant, and for the same
/// reason: which one works depends on the operator's Dota settings. ALT is the
/// route that survives quickcast.
fn default_earth_spirit_petrify_alt() -> bool {
    true
}
/// Double-tap the Enchant Remnant key — Dota's default self-cast binding, for a
/// scepter key left on normal cast.
fn default_earth_spirit_petrify_double_tap() -> bool {
    true
}
fn default_earth_spirit_petrify_double_tap_delay_ms() -> u64 {
    60
}
fn default_earth_spirit_petrify_smash_enabled() -> bool {
    true
}
fn default_earth_spirit_smash_key() -> char {
    'q'
}
/// Gap between the petrify and the kick.
///
/// Longer than the combo delays above because this one waits on a *cast*
/// resolving, not just on a key press clearing: Boulder Smash has nothing to
/// kick until the petrify has actually turned Earth Spirit into a remnant.
fn default_earth_spirit_petrify_to_smash_delay_ms() -> u64 {
    250
}
fn default_slark_enabled() -> bool {
    true
}
fn default_slark_pounce_key() -> char {
    'w'
}
fn default_slark_turn_delay_ms() -> u64 {
    200
}
fn default_slark_require_ability_ready() -> bool {
    true
}
fn default_slark_auto_dark_pact_on_debuff() -> bool {
    true
}
fn default_slark_dark_pact_key() -> char {
    'q'
}
fn default_slark_dark_pact_delay_ms() -> u64 {
    300
}
fn default_slark_dark_pact_require_danger() -> bool {
    true
}
fn default_hud_portrait_x_fraction() -> f32 {
    0.44
}
fn default_hud_portrait_y_fraction() -> f32 {
    0.90
}
fn default_hud_capture_portrait_key() -> String {
    "F9".to_string()
}
fn default_slark_auto_shadow_dance_on_low_hp() -> bool {
    true
}
fn default_slark_shadow_dance_key() -> char {
    'r'
}
fn default_slark_shadow_dance_hp_threshold_percent() -> u32 {
    35
}
fn default_slark_shadow_dance_trigger_cooldown_ms() -> u64 {
    3_000
}
fn default_slark_shadow_dance_active_ms() -> u64 {
    4_000
}
fn default_slark_shard_fallback_enabled() -> bool {
    true
}
fn default_slark_shard_key() -> char {
    'd'
}
fn default_slark_shadow_dance_require_danger() -> bool {
    true
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

fn default_invoker_profile_enabled() -> bool {
    true
}
fn default_invoker_quas_key() -> char {
    'q'
}
fn default_invoker_wex_key() -> char {
    'w'
}
fn default_invoker_exort_key() -> char {
    'e'
}
fn default_invoker_invoke_key() -> char {
    'r'
}
fn default_invoker_spell_slot_primary_key() -> char {
    'd'
}
fn default_invoker_spell_slot_secondary_key() -> char {
    'f'
}
fn default_invoker_cycle_combo_profiles_hotkey() -> String {
    "Delete".to_string()
}
fn default_invoker_profiles() -> Vec<InvokerProfile> {
    vec![
        InvokerProfile {
            id: "qw-pickoff".to_string(),
            name: "QW Pickoff".to_string(),
            enabled: true,
            hotkey: "Home".to_string(),
            mode: InvokerProfileMode::Combo,
            execution_style: InvokerProfileExecutionStyle::Automatic,
            build_tag: "qw".to_string(),
            steps: vec![
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Item,
                    target: "item_spirit_vessel".to_string(),
                    delay_after_ms: 50,
                    cast_behavior: InvokerProfileStepCastBehavior::Normal,
                    completion_mode: InvokerProfileStepCompletionMode::FixedDelay,
                    completion_timeout_ms: 3000,
                    notes: String::new(),
                },
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Item,
                    target: "item_rod_of_atos".to_string(),
                    delay_after_ms: 50,
                    cast_behavior: InvokerProfileStepCastBehavior::Normal,
                    completion_mode: InvokerProfileStepCompletionMode::FixedDelay,
                    completion_timeout_ms: 3000,
                    notes: String::new(),
                },
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Spell,
                    target: "invoker_tornado".to_string(),
                    delay_after_ms: 700,
                    cast_behavior: InvokerProfileStepCastBehavior::Normal,
                    completion_mode: InvokerProfileStepCompletionMode::FixedDelay,
                    completion_timeout_ms: 3000,
                    notes: String::new(),
                },
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Spell,
                    target: "invoker_emp".to_string(),
                    delay_after_ms: 100,
                    cast_behavior: InvokerProfileStepCastBehavior::Normal,
                    completion_mode: InvokerProfileStepCompletionMode::FixedDelay,
                    completion_timeout_ms: 3000,
                    notes: String::new(),
                },
            ],
        },
        InvokerProfile {
            id: "qe-burst".to_string(),
            name: "QE Burst".to_string(),
            enabled: false,
            hotkey: "PageDown".to_string(),
            mode: InvokerProfileMode::Combo,
            execution_style: InvokerProfileExecutionStyle::Automatic,
            build_tag: "qe".to_string(),
            steps: vec![
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Spell,
                    target: "invoker_sun_strike".to_string(),
                    delay_after_ms: 150,
                    cast_behavior: InvokerProfileStepCastBehavior::ManualWaitCooldown,
                    completion_mode: InvokerProfileStepCompletionMode::WaitForCooldown,
                    completion_timeout_ms: 3000,
                    notes: String::new(),
                },
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Spell,
                    target: "invoker_chaos_meteor".to_string(),
                    delay_after_ms: 450,
                    cast_behavior: InvokerProfileStepCastBehavior::Normal,
                    completion_mode: InvokerProfileStepCompletionMode::FixedDelay,
                    completion_timeout_ms: 3000,
                    notes: String::new(),
                },
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Spell,
                    target: "invoker_deafening_blast".to_string(),
                    delay_after_ms: 100,
                    cast_behavior: InvokerProfileStepCastBehavior::Normal,
                    completion_mode: InvokerProfileStepCompletionMode::FixedDelay,
                    completion_timeout_ms: 3000,
                    notes: String::new(),
                },
            ],
        },
        InvokerProfile {
            id: "ghost-walk-panic".to_string(),
            name: "Ghost Walk Panic".to_string(),
            enabled: true,
            hotkey: "End".to_string(),
            mode: InvokerProfileMode::Combo,
            execution_style: InvokerProfileExecutionStyle::Automatic,
            build_tag: "general".to_string(),
            steps: vec![InvokerProfileStep {
                kind: InvokerProfileStepKind::Spell,
                target: "invoker_ghost_walk".to_string(),
                delay_after_ms: 100,
                cast_behavior: InvokerProfileStepCastBehavior::Normal,
                completion_mode: InvokerProfileStepCompletionMode::FixedDelay,
                completion_timeout_ms: 3000,
                notes: String::new(),
            }],
        },
        InvokerProfile {
            id: "meteor-blast-prep".to_string(),
            name: "Meteor + Blast Prep".to_string(),
            enabled: true,
            hotkey: "PageUp".to_string(),
            mode: InvokerProfileMode::Prep,
            execution_style: InvokerProfileExecutionStyle::Automatic,
            build_tag: "qe".to_string(),
            steps: vec![
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Spell,
                    target: "invoker_chaos_meteor".to_string(),
                    delay_after_ms: 0,
                    cast_behavior: InvokerProfileStepCastBehavior::Normal,
                    completion_mode: InvokerProfileStepCompletionMode::FixedDelay,
                    completion_timeout_ms: 3000,
                    notes: String::new(),
                },
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Spell,
                    target: "invoker_deafening_blast".to_string(),
                    delay_after_ms: 0,
                    cast_behavior: InvokerProfileStepCastBehavior::Normal,
                    completion_mode: InvokerProfileStepCompletionMode::FixedDelay,
                    completion_timeout_ms: 3000,
                    notes: String::new(),
                },
            ],
        },
        InvokerProfile {
            id: "lane-pressure".to_string(),
            name: "Lane Pressure".to_string(),
            enabled: false,
            hotkey: "F5".to_string(),
            mode: InvokerProfileMode::Combo,
            execution_style: InvokerProfileExecutionStyle::Automatic,
            build_tag: "qe".to_string(),
            steps: vec![InvokerProfileStep {
                kind: InvokerProfileStepKind::Spell,
                target: "invoker_forge_spirit".to_string(),
                delay_after_ms: 150,
                cast_behavior: InvokerProfileStepCastBehavior::Normal,
                completion_mode: InvokerProfileStepCompletionMode::FixedDelay,
                completion_timeout_ms: 3000,
                notes: String::new(),
            }],
        },
        InvokerProfile {
            id: "meta-catch".to_string(),
            name: "Meta Catch".to_string(),
            enabled: false,
            hotkey: "F6".to_string(),
            mode: InvokerProfileMode::Combo,
            execution_style: InvokerProfileExecutionStyle::Automatic,
            build_tag: "qw".to_string(),
            steps: vec![
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Spell,
                    target: "invoker_tornado".to_string(),
                    delay_after_ms: 700,
                    cast_behavior: InvokerProfileStepCastBehavior::Normal,
                    completion_mode: InvokerProfileStepCompletionMode::FixedDelay,
                    completion_timeout_ms: 3000,
                    notes: String::new(),
                },
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Spell,
                    target: "invoker_emp".to_string(),
                    delay_after_ms: 100,
                    cast_behavior: InvokerProfileStepCastBehavior::Normal,
                    completion_mode: InvokerProfileStepCompletionMode::FixedDelay,
                    completion_timeout_ms: 3000,
                    notes: String::new(),
                },
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Spell,
                    target: "invoker_cold_snap".to_string(),
                    delay_after_ms: 100,
                    cast_behavior: InvokerProfileStepCastBehavior::Normal,
                    completion_mode: InvokerProfileStepCompletionMode::FixedDelay,
                    completion_timeout_ms: 3000,
                    notes: String::new(),
                },
            ],
        },
        InvokerProfile {
            id: "shotgun-burst".to_string(),
            name: "Shotgun Burst".to_string(),
            enabled: false,
            hotkey: "F7".to_string(),
            mode: InvokerProfileMode::Combo,
            execution_style: InvokerProfileExecutionStyle::Automatic,
            build_tag: "qe".to_string(),
            steps: vec![
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Item,
                    target: "item_rod_of_atos".to_string(),
                    delay_after_ms: 50,
                    cast_behavior: InvokerProfileStepCastBehavior::Normal,
                    completion_mode: InvokerProfileStepCompletionMode::FixedDelay,
                    completion_timeout_ms: 3000,
                    notes: String::new(),
                },
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Spell,
                    target: "invoker_sun_strike".to_string(),
                    delay_after_ms: 150,
                    cast_behavior: InvokerProfileStepCastBehavior::ManualWaitCooldown,
                    completion_mode: InvokerProfileStepCompletionMode::WaitForCooldown,
                    completion_timeout_ms: 3000,
                    notes: String::new(),
                },
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Spell,
                    target: "invoker_chaos_meteor".to_string(),
                    delay_after_ms: 450,
                    cast_behavior: InvokerProfileStepCastBehavior::Normal,
                    completion_mode: InvokerProfileStepCompletionMode::FixedDelay,
                    completion_timeout_ms: 3000,
                    notes: String::new(),
                },
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Spell,
                    target: "invoker_deafening_blast".to_string(),
                    delay_after_ms: 100,
                    cast_behavior: InvokerProfileStepCastBehavior::Normal,
                    completion_mode: InvokerProfileStepCompletionMode::FixedDelay,
                    completion_timeout_ms: 3000,
                    notes: String::new(),
                },
            ],
        },
        InvokerProfile {
            id: "ice-floe-lockdown".to_string(),
            name: "Ice Floe Lockdown".to_string(),
            enabled: false,
            hotkey: "F8".to_string(),
            mode: InvokerProfileMode::Combo,
            execution_style: InvokerProfileExecutionStyle::Automatic,
            build_tag: "qe".to_string(),
            steps: vec![
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Spell,
                    target: "invoker_ice_wall".to_string(),
                    delay_after_ms: 2500,
                    cast_behavior: InvokerProfileStepCastBehavior::Normal,
                    completion_mode: InvokerProfileStepCompletionMode::FixedDelay,
                    completion_timeout_ms: 3000,
                    notes: String::new(),
                },
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Spell,
                    target: "invoker_chaos_meteor".to_string(),
                    delay_after_ms: 450,
                    cast_behavior: InvokerProfileStepCastBehavior::Normal,
                    completion_mode: InvokerProfileStepCompletionMode::FixedDelay,
                    completion_timeout_ms: 3000,
                    notes: String::new(),
                },
            ],
        },
        InvokerProfile {
            id: "refresher-sequence".to_string(),
            name: "Refresher Sequence".to_string(),
            enabled: false,
            hotkey: "F9".to_string(),
            mode: InvokerProfileMode::Combo,
            execution_style: InvokerProfileExecutionStyle::Automatic,
            build_tag: "general".to_string(),
            steps: vec![
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Spell,
                    target: "invoker_tornado".to_string(),
                    delay_after_ms: 700,
                    cast_behavior: InvokerProfileStepCastBehavior::Normal,
                    completion_mode: InvokerProfileStepCompletionMode::FixedDelay,
                    completion_timeout_ms: 3000,
                    notes: String::new(),
                },
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Spell,
                    target: "invoker_emp".to_string(),
                    delay_after_ms: 100,
                    cast_behavior: InvokerProfileStepCastBehavior::Normal,
                    completion_mode: InvokerProfileStepCompletionMode::FixedDelay,
                    completion_timeout_ms: 3000,
                    notes: String::new(),
                },
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Spell,
                    target: "invoker_chaos_meteor".to_string(),
                    delay_after_ms: 350,
                    cast_behavior: InvokerProfileStepCastBehavior::Normal,
                    completion_mode: InvokerProfileStepCompletionMode::FixedDelay,
                    completion_timeout_ms: 3000,
                    notes: String::new(),
                },
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Spell,
                    target: "invoker_deafening_blast".to_string(),
                    delay_after_ms: 100,
                    cast_behavior: InvokerProfileStepCastBehavior::Normal,
                    completion_mode: InvokerProfileStepCompletionMode::FixedDelay,
                    completion_timeout_ms: 3000,
                    notes: String::new(),
                },
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Item,
                    target: "item_refresher".to_string(),
                    delay_after_ms: 100,
                    cast_behavior: InvokerProfileStepCastBehavior::Normal,
                    completion_mode: InvokerProfileStepCompletionMode::FixedDelay,
                    completion_timeout_ms: 3000,
                    notes: String::new(),
                },
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Spell,
                    target: "invoker_sun_strike".to_string(),
                    delay_after_ms: 100,
                    cast_behavior: InvokerProfileStepCastBehavior::AltDoubleTap,
                    completion_mode: InvokerProfileStepCompletionMode::FixedDelay,
                    completion_timeout_ms: 3000,
                    notes: String::new(),
                },
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Spell,
                    target: "invoker_chaos_meteor".to_string(),
                    delay_after_ms: 350,
                    cast_behavior: InvokerProfileStepCastBehavior::Normal,
                    completion_mode: InvokerProfileStepCompletionMode::FixedDelay,
                    completion_timeout_ms: 3000,
                    notes: String::new(),
                },
                InvokerProfileStep {
                    kind: InvokerProfileStepKind::Spell,
                    target: "invoker_deafening_blast".to_string(),
                    delay_after_ms: 100,
                    cast_behavior: InvokerProfileStepCastBehavior::Normal,
                    completion_mode: InvokerProfileStepCompletionMode::FixedDelay,
                    completion_timeout_ms: 3000,
                    notes: String::new(),
                },
            ],
        },
    ]
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
fn default_auto_mjollnir() -> bool {
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
fn default_phase_boots_automation_enabled() -> bool {
    true
}
fn default_phase_boots_minimum_distance_units() -> u32 {
    100
}
fn default_phase_boots_excluded_heroes() -> Vec<String> {
    Vec::new()
}
fn default_suppress_automation_while_invisible() -> bool {
    true
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
            file_enabled: default_log_to_file(),
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

impl Default for SnapfireConfig {
    fn default() -> Self {
        Self {
            enabled: default_snapfire_enabled(),
            trigger_key: default_snapfire_trigger_key(),
            cookie_key: default_snapfire_cookie_key(),
            turn_delay_ms: default_snapfire_turn_delay_ms(),
        }
    }
}

impl Default for MagnusConfig {
    fn default() -> Self {
        Self {
            enabled: default_magnus_enabled(),
            ultimate_key: default_magnus_ultimate_key(),
            turn_delay_ms: default_magnus_turn_delay_ms(),
            require_ability_ready: default_magnus_require_ability_ready(),
            center_camera_on_ultimate: default_magnus_center_camera_on_ultimate(),
            camera_center_key: default_magnus_camera_center_key(),
            camera_center_delay_ms: default_magnus_camera_center_delay_ms(),
        }
    }
}

impl Default for EmberSpiritConfig {
    fn default() -> Self {
        Self {
            enabled: default_ember_spirit_enabled(),
            remnant_key: default_ember_spirit_remnant_key(),
            activate_key: default_ember_spirit_activate_key(),
            activate_delay_ms: default_ember_spirit_activate_delay_ms(),
            use_scepter_activate_delay: default_ember_spirit_use_scepter_activate_delay(),
            scepter_activate_delay_ms: default_ember_spirit_scepter_activate_delay_ms(),
            auto_flame_guard_on_danger: default_ember_spirit_auto_flame_guard_on_danger(),
            flame_guard_key: default_ember_spirit_flame_guard_key(),
            flame_guard_hp_threshold_percent:
                default_ember_spirit_flame_guard_hp_threshold_percent(),
            flame_guard_trigger_cooldown_ms:
                default_ember_spirit_flame_guard_trigger_cooldown_ms(),
        }
    }
}

impl Default for MiranaConfig {
    fn default() -> Self {
        Self {
            enabled: default_mirana_enabled(),
            leap_key: default_mirana_leap_key(),
            turn_delay_ms: default_mirana_turn_delay_ms(),
            require_ability_ready: default_mirana_require_ability_ready(),
        }
    }
}

impl Default for EarthSpiritConfig {
    fn default() -> Self {
        Self {
            enabled: default_earth_spirit_enabled(),
            remnant_key: default_earth_spirit_remnant_key(),
            silence_combo_enabled: default_earth_spirit_silence_combo_enabled(),
            grip_key: default_earth_spirit_grip_key(),
            silence_remnant_delay_ms: default_earth_spirit_silence_remnant_delay_ms(),
            require_grip_ready: default_earth_spirit_require_grip_ready(),
            roll_combo_enabled: default_earth_spirit_roll_combo_enabled(),
            roll_key: default_earth_spirit_roll_key(),
            roll_double_tap: default_earth_spirit_roll_double_tap(),
            roll_double_tap_delay_ms: default_earth_spirit_roll_double_tap_delay_ms(),
            roll_to_remnant_delay_ms: default_earth_spirit_roll_to_remnant_delay_ms(),
            roll_remnant_alt: default_earth_spirit_roll_remnant_alt(),
            roll_remnant_double_tap: default_earth_spirit_roll_remnant_double_tap(),
            roll_remnant_double_tap_delay_ms:
                default_earth_spirit_roll_remnant_double_tap_delay_ms(),
            require_roll_ready: default_earth_spirit_require_roll_ready(),
            auto_petrify_on_danger: default_earth_spirit_auto_petrify_on_danger(),
            petrify_key: default_earth_spirit_petrify_key(),
            petrify_hp_threshold_percent: default_earth_spirit_petrify_hp_threshold_percent(),
            petrify_trigger_cooldown_ms: default_earth_spirit_petrify_trigger_cooldown_ms(),
            petrify_alt: default_earth_spirit_petrify_alt(),
            petrify_double_tap: default_earth_spirit_petrify_double_tap(),
            petrify_double_tap_delay_ms: default_earth_spirit_petrify_double_tap_delay_ms(),
            petrify_smash_enabled: default_earth_spirit_petrify_smash_enabled(),
            smash_key: default_earth_spirit_smash_key(),
            petrify_to_smash_delay_ms: default_earth_spirit_petrify_to_smash_delay_ms(),
        }
    }
}

impl Default for SlarkConfig {
    fn default() -> Self {
        Self {
            enabled: default_slark_enabled(),
            pounce_key: default_slark_pounce_key(),
            turn_delay_ms: default_slark_turn_delay_ms(),
            require_ability_ready: default_slark_require_ability_ready(),
            auto_dark_pact_on_debuff: default_slark_auto_dark_pact_on_debuff(),
            dark_pact_key: default_slark_dark_pact_key(),
            dark_pact_delay_ms: default_slark_dark_pact_delay_ms(),
            dark_pact_require_danger: default_slark_dark_pact_require_danger(),
            auto_shadow_dance_on_low_hp: default_slark_auto_shadow_dance_on_low_hp(),
            shadow_dance_key: default_slark_shadow_dance_key(),
            shadow_dance_hp_threshold_percent: default_slark_shadow_dance_hp_threshold_percent(),
            shadow_dance_require_danger: default_slark_shadow_dance_require_danger(),
            shadow_dance_trigger_cooldown_ms: default_slark_shadow_dance_trigger_cooldown_ms(),
            shadow_dance_active_ms: default_slark_shadow_dance_active_ms(),
            shard_fallback_enabled: default_slark_shard_fallback_enabled(),
            shard_key: default_slark_shard_key(),
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
            suspend_after_manual_combo_ms: default_meepo_farm_assist_suspend_after_manual_combo_ms(
            ),
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

impl Default for InvokerConfig {
    fn default() -> Self {
        Self {
            quas_key: default_invoker_quas_key(),
            wex_key: default_invoker_wex_key(),
            exort_key: default_invoker_exort_key(),
            invoke_key: default_invoker_invoke_key(),
            spell_slot_primary_key: default_invoker_spell_slot_primary_key(),
            spell_slot_secondary_key: default_invoker_spell_slot_secondary_key(),
            cycle_combo_profiles_hotkey: default_invoker_cycle_combo_profiles_hotkey(),
            profiles: default_invoker_profiles(),
            armlet: HeroArmletOverrideConfig::default(),
        }
    }
}

impl Default for HeroesConfig {
    fn default() -> Self {
        Self {
            huskar: HuskarConfig::default(),
            invoker: InvokerConfig::default(),
            legion_commander: LegionCommanderConfig::default(),
            shadow_fiend: ShadowFiendConfig::default(),
            tiny: TinyConfig::default(),
            outworld_destroyer: OutworldDestroyerConfig::default(),
            largo: LargoConfig::default(),
            broodmother: BroodmotherConfig::default(),
            meepo: MeepoConfig::default(),
            snapfire: SnapfireConfig::default(),
            magnus: MagnusConfig::default(),
            mirana: MiranaConfig::default(),
            ember_spirit: EmberSpiritConfig::default(),
            earth_spirit: EarthSpiritConfig::default(),
            slark: SlarkConfig::default(),
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
            auto_mjollnir: default_auto_mjollnir(),
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

impl Default for PhaseBootsAutomationConfig {
    fn default() -> Self {
        Self {
            enabled: default_phase_boots_automation_enabled(),
            minimum_distance_units: default_phase_boots_minimum_distance_units(),
            excluded_heroes: default_phase_boots_excluded_heroes(),
        }
    }
}

impl Default for InvisibilityConfig {
    fn default() -> Self {
        Self {
            suppress_automation: default_suppress_automation_while_invisible(),
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
            phase_boots_automation: PhaseBootsAutomationConfig::default(),
            invisibility: InvisibilityConfig::default(),
            soul_ring: SoulRingConfig::default(),
            gsi_logging: GsiLoggingConfig::default(),
            updates: UpdateConfig::default(),
            rune_alerts: RuneAlertConfig::default(),
            draft: DraftConfig::default(),
            stratz: StratzConfig::default(),
            minimap_capture: MinimapCaptureConfig::default(),
            minimap_analysis: MinimapAnalysisConfig::default(),
            wave_tracker: WaveTrackerConfig::default(),
            wave_overlay: WaveOverlayConfig::default(),
            alerts: AlertsConfig::default(),
            hud: HudConfig::default(),
        }
    }
}

impl Default for HudConfig {
    fn default() -> Self {
        Self {
            portrait_x_fraction: default_hud_portrait_x_fraction(),
            portrait_y_fraction: default_hud_portrait_y_fraction(),
            portrait_calibrated: false,
            capture_portrait_key: default_hud_capture_portrait_key(),
        }
    }
}

/// Read the pre-`[invisibility]` spelling of the suppression flag.
///
/// The gate used to live at `[phase_boots_automation] suppress_while_invisible`,
/// which is still what an existing config on disk says. Returns `Some` only when
/// the old key is present *and* the new section is not, so a config that has been
/// updated is never second-guessed by a stale key someone left behind.
fn legacy_suppress_while_invisible(contents: &str) -> Option<bool> {
    let document: toml::Value = toml::from_str(contents).ok()?;

    if document.get("invisibility").is_some() {
        return None;
    }

    document
        .get("phase_boots_automation")?
        .get("suppress_while_invisible")?
        .as_bool()
}

impl Settings {
    pub fn load() -> Self {
        let paths = match ConfigPaths::detect() {
            Ok(paths) => paths,
            Err(e) => {
                warn!(
                    "Failed to resolve config paths: {}. Using default settings.",
                    e
                );
                return Settings::default();
            }
        };

        let config_path = match bootstrap_live_config(&paths, EMBEDDED_CONFIG_TEMPLATE) {
            Ok(path) => path,
            Err(e) => {
                warn!(
                    "Failed to bootstrap live config: {}. Using default settings.",
                    e
                );
                return Settings::default();
            }
        };

        match fs::read_to_string(&config_path) {
            Ok(contents) => match toml::from_str(&contents) {
                Ok(settings) => {
                    info!("Loaded configuration from {}", config_path.display());
                    let mut settings: Settings = settings;
                    if let Some(legacy) = legacy_suppress_while_invisible(&contents) {
                        info!(
                            "Carrying [phase_boots_automation] suppress_while_invisible = {} over \
                             to [invisibility] suppress_automation",
                            legacy
                        );
                        settings.invisibility.suppress_automation = legacy;
                    }
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
            "npc_dota_hero_invoker" => Some(self.heroes.invoker.armlet.clone()),
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
            "invoker" => self
                .heroes
                .invoker
                .profiles
                .iter()
                .find(|profile| profile.enabled && profile.mode == InvokerProfileMode::Combo)
                .map(|profile| profile.hotkey.clone())
                .unwrap_or_else(default_standalone_key),
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
        let config_path = persist_live_config(&paths, &desired_contents, EMBEDDED_CONFIG_TEMPLATE)
            .map_err(std::io::Error::other)?;
        info!("Settings saved to {}", config_path.display());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapfire_config_defaults_are_directional_cookie() {
        let cfg = SnapfireConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.trigger_key, "Space");
        assert_eq!(cfg.cookie_key, 'w');
        assert_eq!(cfg.turn_delay_ms, 60);
    }

    #[test]
    fn magnus_config_defaults_gate_the_ultimate_on_readiness() {
        let cfg = MagnusConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.ultimate_key, 'r');
        assert_eq!(cfg.turn_delay_ms, 60);
        assert!(cfg.require_ability_ready);
        assert!(cfg.center_camera_on_ultimate);
        assert_eq!(cfg.camera_center_key, "1");
        assert_eq!(cfg.camera_center_delay_ms, 60);
    }

    #[test]
    fn ember_spirit_config_defaults_chase_with_r_then_d() {
        let cfg = EmberSpiritConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.remnant_key, 'r');
        assert_eq!(cfg.activate_key, 'd');
        assert_eq!(cfg.activate_delay_ms, 150);
        assert!(cfg.use_scepter_activate_delay);
        assert_eq!(cfg.scepter_activate_delay_ms, 40);
    }

    #[test]
    fn ember_spirit_config_defaults_auto_cast_flame_guard_on_danger() {
        let cfg = EmberSpiritConfig::default();
        assert!(cfg.auto_flame_guard_on_danger);
        assert_eq!(cfg.flame_guard_key, 'e');
        assert_eq!(cfg.flame_guard_hp_threshold_percent, 65);
        assert_eq!(cfg.flame_guard_trigger_cooldown_ms, 2000);
    }

    /// Every shipped value here equals the Rust default, so
    /// `the_embedded_config_template_deserializes_into_settings` would still
    /// pass with the section deleted. Check for the section itself instead.
    #[test]
    fn the_embedded_config_template_ships_the_ember_spirit_section() {
        assert!(EMBEDDED_CONFIG_TEMPLATE.contains("[heroes.ember_spirit]"));

        let settings: Settings = toml::from_str(EMBEDDED_CONFIG_TEMPLATE)
            .expect("config/config.toml should parse into Settings");
        assert_eq!(settings.heroes.ember_spirit.remnant_key, 'r');
        assert_eq!(settings.heroes.ember_spirit.activate_key, 'd');
        assert_eq!(settings.heroes.ember_spirit.activate_delay_ms, 150);
        assert_eq!(
            settings.heroes.ember_spirit.flame_guard_hp_threshold_percent,
            65
        );
    }

    #[test]
    fn mirana_config_defaults_gate_leap_on_readiness() {
        let cfg = MiranaConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.leap_key, 'e');
        assert_eq!(cfg.turn_delay_ms, 200);
        assert!(cfg.require_ability_ready);
    }

    #[test]
    fn earth_spirit_config_defaults_arm_both_remnant_combos() {
        let cfg = EarthSpiritConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.remnant_key, 'd');

        assert!(cfg.silence_combo_enabled);
        assert_eq!(cfg.grip_key, 'e');
        assert_eq!(cfg.silence_remnant_delay_ms, 120);
        assert!(cfg.require_grip_ready);

        assert!(cfg.roll_combo_enabled);
        assert_eq!(cfg.roll_key, 'w');
        assert!(cfg.roll_double_tap);
        assert_eq!(cfg.roll_double_tap_delay_ms, 60);
        assert_eq!(cfg.roll_to_remnant_delay_ms, 120);
        assert!(cfg.roll_remnant_alt);
        assert!(cfg.roll_remnant_double_tap);
        assert_eq!(cfg.roll_remnant_double_tap_delay_ms, 60);
        assert!(cfg.require_roll_ready);
    }

    #[test]
    fn earth_spirit_config_defaults_arm_the_scepter_escape() {
        let cfg = EarthSpiritConfig::default();
        assert!(cfg.auto_petrify_on_danger);
        assert_eq!(cfg.petrify_key, 'f');
        assert_eq!(cfg.petrify_hp_threshold_percent, 22);
        assert_eq!(cfg.petrify_trigger_cooldown_ms, 5000);
        assert!(cfg.petrify_smash_enabled);
        assert_eq!(cfg.smash_key, 'q');
        assert_eq!(cfg.petrify_to_smash_delay_ms, 250);
    }

    /// The shipped values all equal the Rust defaults, so a dropped — or
    /// misspelled — field in the template would deserialize silently and every
    /// other assertion would still pass. Check the field names are there.
    #[test]
    fn the_embedded_config_template_ships_the_scepter_escape_fields() {
        for field in [
            "auto_petrify_on_danger",
            "petrify_key",
            "petrify_hp_threshold_percent",
            "petrify_trigger_cooldown_ms",
            "petrify_alt",
            "petrify_double_tap",
            "petrify_double_tap_delay_ms",
            "petrify_smash_enabled",
            "smash_key",
            "petrify_to_smash_delay_ms",
        ] {
            assert!(
                EMBEDDED_CONFIG_TEMPLATE.contains(field),
                "config/config.toml is missing {field}"
            );
        }
    }

    /// The escape only works because the remnant it kicks is Earth Spirit
    /// himself, and only a self-cast puts him there. Losing both routes is a
    /// silent regression: the petrify would still fire, at whatever the cursor
    /// happened to be over.
    #[test]
    fn the_scepter_escape_keeps_a_self_cast_route() {
        let cfg = EarthSpiritConfig::default();
        assert!(
            cfg.petrify_alt || cfg.petrify_double_tap,
            "at least one self-cast route must ship on, or Enchant Remnant \
             lands on the cursor instead of on Earth Spirit"
        );
    }

    /// The petrify fires below the HP threshold the danger detector itself
    /// uses, not above it: it is a last resort that removes Earth Spirit from
    /// the fight, so it must not pre-empt the cheaper defensive items.
    #[test]
    fn the_petrify_threshold_sits_under_the_danger_threshold() {
        let cfg = EarthSpiritConfig::default();
        let danger = DangerDetectionConfig::default();
        assert!(
            cfg.petrify_hp_threshold_percent < danger.hp_threshold_percent,
            "petrify at {}% must stay below the {}% danger threshold",
            cfg.petrify_hp_threshold_percent,
            danger.hp_threshold_percent
        );
    }

    /// Every press of the roll combo has to land inside Rolling Boulder's
    /// windup. Past it the boulder is already moving and a remnant dropped
    /// behind it does nothing — the combo would look like it fired and
    /// silently have no effect.
    ///
    /// This budgets the *whole* sequence, not one delay, because the combo now
    /// has four gaps that stack: the roll double-tap, the wait to the remnant,
    /// and the remnant's own double-tap.
    #[test]
    fn the_whole_roll_combo_fits_inside_the_windup() {
        let cfg = EarthSpiritConfig::default();
        let total = cfg.roll_double_tap_delay_ms
            + cfg.roll_to_remnant_delay_ms
            + cfg.roll_remnant_double_tap_delay_ms;
        assert!(
            total < ROLLING_BOULDER_WINDUP_MS,
            "roll combo spans {total}ms, which must stay inside the \
             {ROLLING_BOULDER_WINDUP_MS}ms windup"
        );
    }

    /// Self-cast is what removes the aiming step, so both routes to it ship on
    /// by default. If these ever default off, the remnant silently goes back to
    /// the cursor and the roll stops extending unless the operator aims.
    #[test]
    fn the_roll_remnant_self_casts_by_default() {
        let cfg = EarthSpiritConfig::default();
        assert!(cfg.roll_remnant_alt || cfg.roll_remnant_double_tap);
    }

    #[test]
    fn slark_config_defaults_gate_pounce_on_readiness() {
        let cfg = SlarkConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.pounce_key, 'w');
        assert_eq!(cfg.turn_delay_ms, 200);
        assert!(cfg.require_ability_ready);
        assert!(cfg.auto_dark_pact_on_debuff);
        assert_eq!(cfg.dark_pact_key, 'q');
        assert_eq!(cfg.dark_pact_delay_ms, 300);
    }

    #[test]
    fn slark_config_defaults_arm_the_low_hp_escape() {
        let cfg = SlarkConfig::default();
        assert!(cfg.auto_shadow_dance_on_low_hp);
        assert_eq!(cfg.shadow_dance_key, 'r');
        assert_eq!(cfg.shadow_dance_hp_threshold_percent, 35);
        assert!(cfg.shadow_dance_require_danger);
        assert_eq!(cfg.shadow_dance_trigger_cooldown_ms, 3_000);
        assert_eq!(cfg.shadow_dance_active_ms, 4_000);
        assert!(cfg.shard_fallback_enabled);
        assert_eq!(cfg.shard_key, 'd');
    }

    /// The shipped template is what every new install starts from, so a typo or
    /// a key that drifted from the structs would ship broken defaults.
    #[test]
    fn the_embedded_config_template_deserializes_into_settings() {
        let settings: Settings = toml::from_str(EMBEDDED_CONFIG_TEMPLATE)
            .expect("config/config.toml should parse into Settings");

        // Spot-check a value from the template rather than a Rust default, so
        // this fails if the section is missing entirely rather than silently
        // falling back to `#[serde(default)]`.
        assert_eq!(settings.hud.capture_portrait_key, "F9");
        assert!(!settings.hud.portrait_calibrated);
        assert_eq!(settings.heroes.slark.shard_key, 'd');
        assert_eq!(settings.heroes.slark.shadow_dance_hp_threshold_percent, 35);

        // `suppress_automation` defaults to the same `true` the template sets, so
        // asserting the value would pass even with the section missing. The
        // legacy reader answers `None` only when `[invisibility]` is really there.
        assert!(settings.invisibility.suppress_automation);
        assert_eq!(
            legacy_suppress_while_invisible(EMBEDDED_CONFIG_TEMPLATE),
            None
        );
    }

    #[test]
    fn hud_portrait_anchor_starts_uncalibrated() {
        let cfg = HudConfig::default();
        // The shipped fractions are only a starting point for the Test button —
        // nothing may click the portrait until it has actually been measured.
        assert!(!cfg.portrait_calibrated);
        assert_eq!(cfg.capture_portrait_key, "F9");
    }

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
            settings
                .heroes
                .meepo
                .farm_assist
                .suspend_after_manual_combo_ms,
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

    #[test]
    fn phase_boots_automation_defaults_are_exposed_through_settings() {
        let settings = Settings::default();

        assert!(settings.phase_boots_automation.enabled);
        assert_eq!(settings.phase_boots_automation.minimum_distance_units, 100);
        assert!(settings.phase_boots_automation.excluded_heroes.is_empty());
    }

    #[test]
    fn invisibility_suppression_defaults_to_on() {
        assert!(Settings::default().invisibility.suppress_automation);
    }

    #[test]
    fn a_config_without_the_invisibility_section_keeps_the_old_phase_boots_flag() {
        let legacy = r#"
[phase_boots_automation]
enabled = true
suppress_while_invisible = false
"#;

        assert_eq!(legacy_suppress_while_invisible(legacy), Some(false));
    }

    #[test]
    fn the_new_section_wins_over_a_stale_phase_boots_key() {
        let both = r#"
[phase_boots_automation]
suppress_while_invisible = false

[invisibility]
suppress_automation = true
"#;

        assert_eq!(legacy_suppress_while_invisible(both), None);
    }

    #[test]
    fn a_config_with_neither_key_reports_nothing_to_carry_over() {
        assert_eq!(
            legacy_suppress_while_invisible("[phase_boots_automation]\nenabled = true\n"),
            None
        );
    }

    #[test]
    fn invoker_defaults_expose_expected_hotkeys() {
        let settings = Settings::default();
        assert_eq!(settings.get_standalone_key("invoker"), "Home");
        assert_eq!(settings.heroes.invoker.quas_key, 'q');
        assert_eq!(settings.heroes.invoker.invoke_key, 'r');
        assert_eq!(
            settings.heroes.invoker.cycle_combo_profiles_hotkey,
            "Delete"
        );
    }

    #[test]
    fn invoker_defaults_seed_expected_profiles() {
        let settings = Settings::default();
        let invoker = settings.heroes.invoker;

        let seeded_profiles: Vec<_> = invoker
            .profiles
            .iter()
            .map(|profile| {
                (
                    profile.id.as_str(),
                    profile.hotkey.as_str(),
                    profile.enabled,
                )
            })
            .collect();

        assert_eq!(
            seeded_profiles,
            vec![
                ("qw-pickoff", "Home", true),
                ("qe-burst", "PageDown", false),
                ("ghost-walk-panic", "End", true),
                ("meteor-blast-prep", "PageUp", true),
                ("lane-pressure", "F5", false),
                ("meta-catch", "F6", false),
                ("shotgun-burst", "F7", false),
                ("ice-floe-lockdown", "F8", false),
                ("refresher-sequence", "F9", false),
            ]
        );
    }

    #[test]
    fn invoker_profiles_default_to_automatic_execution_style() {
        let settings = Settings::default();

        let qw = settings
            .heroes
            .invoker
            .profiles
            .iter()
            .find(|profile| profile.id == "qw-pickoff")
            .expect("QW Pickoff should exist");
        let prep = settings
            .heroes
            .invoker
            .profiles
            .iter()
            .find(|profile| profile.id == "meteor-blast-prep")
            .expect("Meteor + Blast Prep should exist");

        assert_eq!(qw.execution_style, InvokerProfileExecutionStyle::Automatic);
        assert_eq!(prep.execution_style, InvokerProfileExecutionStyle::Automatic);
    }

    #[test]
    fn invoker_profile_execution_style_defaults_when_field_is_missing() {
        let profile: InvokerProfile = toml::from_str(
            r#"
id = "semi-auto-check"
name = "Semi Auto Check"
enabled = true
hotkey = "F10"
mode = "combo"
build_tag = "qw"
"#,
        )
        .expect("profile should deserialize");

        assert_eq!(
            profile.execution_style,
            InvokerProfileExecutionStyle::Automatic
        );
    }

    #[test]
    fn invoker_qe_burst_defaults_to_manual_sun_strike_wait() {
        let settings = Settings::default();
        let qe = settings
            .heroes
            .invoker
            .profiles
            .iter()
            .find(|profile| profile.id == "qe-burst")
            .expect("QE Burst profile should exist");

        let sun_strike = qe
            .steps
            .first()
            .expect("QE Burst should seed Sun Strike as the first step");

        assert_eq!(
            sun_strike.completion_mode,
            InvokerProfileStepCompletionMode::WaitForCooldown
        );
        assert_eq!(sun_strike.completion_timeout_ms, 3000);
        assert_eq!(
            sun_strike.cast_behavior,
            InvokerProfileStepCastBehavior::ManualWaitCooldown
        );
    }

    #[test]
    fn hero_armlet_override_returns_config_for_all_supported_heroes() {
        let settings = Settings::default();

        // Test that all heroes with armlet configs are registered
        assert!(settings
            .hero_armlet_override("npc_dota_hero_huskar")
            .is_some());
        assert!(settings
            .hero_armlet_override("npc_dota_hero_invoker")
            .is_some());
        assert!(settings
            .hero_armlet_override("npc_dota_hero_legion_commander")
            .is_some());
        assert!(settings
            .hero_armlet_override("npc_dota_hero_nevermore")
            .is_some());
        assert!(settings
            .hero_armlet_override("npc_dota_hero_tiny")
            .is_some());
        assert!(settings
            .hero_armlet_override("npc_dota_hero_obsidian_destroyer")
            .is_some());
        assert!(settings
            .hero_armlet_override("npc_dota_hero_largo")
            .is_some());
        assert!(settings
            .hero_armlet_override("npc_dota_hero_broodmother")
            .is_some());
        assert!(settings
            .hero_armlet_override("npc_dota_hero_meepo")
            .is_some());

        // Test that unknown heroes return None
        assert!(settings
            .hero_armlet_override("npc_dota_hero_unknown")
            .is_none());
    }
}
