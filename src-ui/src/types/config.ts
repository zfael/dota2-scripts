export interface ServerConfig {
  port: number;
}

export interface UpdateConfig {
  check_on_startup: boolean;
  include_prereleases: boolean;
}

export interface KeybindingsConfig {
  slot0: string;
  slot1: string;
  slot2: string;
  slot3: string;
  slot4: string;
  slot5: string;
  neutral0: string;
  combo_trigger: string;
}

export interface LoggingConfig {
  level: "debug" | "info" | "warn" | "error";
}

export interface CommonConfig {
  survivability_hp_threshold: number;
}

export interface ArmletConfig {
  enabled: boolean;
  cast_modifier: string;
  toggle_threshold: number;
  predictive_offset: number;
  toggle_cooldown_ms: number;
  roshan: ArmletRoshanConfig;
}

export interface ArmletRoshanConfig {
  enabled: boolean;
  toggle_key: string;
  emergency_margin_hp: number;
  learning_window_ms: number;
  min_confidence_hits: number;
  min_sample_damage: number;
  stale_reset_ms: number;
}

export interface HeroArmletOverride {
  enabled?: boolean;
  toggle_threshold?: number;
  predictive_offset?: number;
  toggle_cooldown_ms?: number;
}

export interface HuskarRoshanSpearsConfig {
  enabled: boolean;
  burning_spear_key: string;
  disable_buffer_hp: number;
  reenable_buffer_hp: number;
}

export interface HuskarConfig {
  armlet_toggle_threshold: number;
  armlet_predictive_offset: number;
  armlet_toggle_cooldown_ms: number;
  berserker_blood_key: string;
  berserker_blood_delay_ms: number;
  standalone_key: string;
  armlet: HeroArmletOverride;
  roshan_spears: HuskarRoshanSpearsConfig;
}

export interface LegionCommanderConfig {
  standalone_key: string;
  armlet: HeroArmletOverride;
}

export interface ShadowFiendConfig {
  raze_intercept_enabled: boolean;
  raze_delay_ms: number;
  auto_bkb_on_ultimate: boolean;
  auto_d_on_ultimate: boolean;
  standalone_key: string;
  armlet: HeroArmletOverride;
}

export interface TinyConfig {
  standalone_key: string;
  armlet: HeroArmletOverride;
}

export interface AutoAbilityConfig {
  index: number;
  key: string;
  hp_threshold?: number;
}

export interface BroodmotherConfig {
  spider_micro_enabled: boolean;
  spider_control_group_key: string;
  reselect_hero_key: string;
  attack_key: string;
  standalone_key: string;
  auto_items_enabled: boolean;
  auto_items_modifier: string;
  auto_items: string[];
  auto_abilities: AutoAbilityConfig[];
  auto_abilities_first: boolean;
  armlet: HeroArmletOverride;
}

export interface LargoConfig {
  amphibian_rhapsody_enabled: boolean;
  auto_toggle_on_danger: boolean;
  mana_threshold_percent: number;
  heal_hp_threshold: number;
  beat_interval_ms: number;
  beat_correction_ms: number;
  beat_correction_every_n_beats: number;
  q_ability_key: string;
  w_ability_key: string;
  e_ability_key: string;
  r_ability_key: string;
  standalone_key: string;
  armlet: HeroArmletOverride;
}

export interface MeepoFarmAssistConfig {
  enabled: boolean;
  toggle_key: string;
  pulse_interval_ms: number;
  minimum_mana_percent: number;
  minimum_health_percent: number;
  right_click_after_poof: boolean;
  suspend_on_danger: boolean;
  suspend_after_manual_combo_ms: number;
  poof_press_count: number;
  poof_press_interval_ms: number;
}

export interface MeepoConfig {
  standalone_key: string;
  earthbind_key: string;
  poof_key: string;
  dig_key: string;
  megameepo_key: string;
  post_blink_delay_ms: number;
  combo_items: string[];
  combo_item_spam_count: number;
  combo_item_delay_ms: number;
  earthbind_press_count: number;
  earthbind_press_interval_ms: number;
  poof_press_count: number;
  poof_press_interval_ms: number;
  auto_dig_on_danger: boolean;
  dig_hp_threshold_percent: number;
  auto_megameepo_on_danger: boolean;
  megameepo_hp_threshold_percent: number;
  defensive_trigger_cooldown_ms: number;
  farm_assist: MeepoFarmAssistConfig;
  armlet: HeroArmletOverride;
}

export interface OutworldDestroyerConfig {
  standalone_key: string;
  objurgation_key: string;
  arcane_orb_key: string;
  astral_imprisonment_key: string;
  auto_objurgation_on_danger: boolean;
  objurgation_hp_threshold_percent: number;
  objurgation_min_mana_percent: number;
  objurgation_trigger_cooldown_ms: number;
  ultimate_intercept_enabled: boolean;
  auto_bkb_on_ultimate: boolean;
  auto_objurgation_on_ultimate: boolean;
  post_bkb_delay_ms: number;
  post_blink_delay_ms: number;
  astral_self_cast_enabled: boolean;
  astral_self_cast_key: string;
  combo_items: string[];
  combo_item_spam_count: number;
  combo_item_delay_ms: number;
  post_ultimate_arcane_orb_presses: number;
  arcane_orb_press_interval_ms: number;
  armlet: HeroArmletOverride;
}

export interface HeroesConfig {
  huskar: HuskarConfig;
  legion_commander: LegionCommanderConfig;
  shadow_fiend: ShadowFiendConfig;
  tiny: TinyConfig;
  outworld_destroyer: OutworldDestroyerConfig;
  largo: LargoConfig;
  broodmother: BroodmotherConfig;
  meepo: MeepoConfig;
}

export interface DangerDetectionConfig {
  enabled: boolean;
  hp_threshold_percent: number;
  rapid_loss_hp: number;
  time_window_ms: number;
  clear_delay_seconds: number;
  healing_threshold_in_danger: number;
  max_healing_items_per_danger: number;
  auto_bkb: boolean;
  auto_satanic: boolean;
  satanic_hp_threshold: number;
  auto_blade_mail: boolean;
  auto_glimmer_cape: boolean;
  auto_ghost_scepter: boolean;
  auto_shivas_guard: boolean;
  auto_manta_on_silence: boolean;
  auto_lotus_on_silence: boolean;
}

export interface NeutralItemConfig {
  enabled: boolean;
  self_cast_key: string;
  log_discoveries: boolean;
  use_in_danger: boolean;
  hp_threshold: number;
  allowed_items: string[];
}

export interface SoulRingConfig {
  enabled: boolean;
  min_mana_percent: number;
  min_health_percent: number;
  delay_before_ability_ms: number;
  trigger_cooldown_ms: number;
  ability_keys: string[];
  intercept_item_keys: boolean;
}

export interface RuneAlertConfig {
  enabled: boolean;
  alert_lead_seconds: number;
  interval_seconds: number;
  audio_enabled: boolean;
}

export interface MinimapCaptureConfig {
  enabled: boolean;
  minimap_x: number;
  minimap_y: number;
  minimap_width: number;
  minimap_height: number;
  capture_interval_ms: number;
  sample_every_n: number;
  artifact_output_dir: string;
}

export interface MinimapAnalysisConfig {
  enabled: boolean;
  baseline_frames: number;
  baseline_threshold: number;
  min_cluster_size: number;
  max_cluster_size: number;
  red_hue_max: number;
  red_hue_min_wrap: number;
  red_min_saturation: number;
  red_min_value: number;
  green_hue_min: number;
  green_hue_max: number;
  green_min_saturation: number;
  green_min_value: number;
}

export interface Settings {
  server: ServerConfig;
  keybindings: KeybindingsConfig;
  logging: LoggingConfig;
  common: CommonConfig;
  armlet: ArmletConfig;
  heroes: HeroesConfig;
  danger_detection: DangerDetectionConfig;
  neutral_items: NeutralItemConfig;
  soul_ring: SoulRingConfig;
  updates: UpdateConfig;
  rune_alerts: RuneAlertConfig;
  minimap_capture: MinimapCaptureConfig;
  minimap_analysis: MinimapAnalysisConfig;
}
