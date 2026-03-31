import type { Settings } from "../types/config";
import type { ActivityEntry } from "../types/activity";

export const mockConfig: Settings = {
  server: { port: 3000 },
  keybindings: {
    slot0: "z", slot1: "x", slot2: "c", slot3: "v", slot4: "b", slot5: "n",
    neutral0: "0", combo_trigger: "Home",
  },
  logging: { level: "info" },
  common: { survivability_hp_threshold: 30 },
  armlet: {
    enabled: true, cast_modifier: "Alt", toggle_threshold: 320,
    predictive_offset: 30, toggle_cooldown_ms: 250,
  },
  heroes: {
    huskar: {
      armlet_toggle_threshold: 120, armlet_predictive_offset: 150,
      armlet_toggle_cooldown_ms: 300, berserker_blood_key: "e",
      berserker_blood_delay_ms: 300, standalone_key: "Home",
      armlet: {},
    },
    legion_commander: { standalone_key: "Home", armlet: {} },
    shadow_fiend: {
      raze_intercept_enabled: true, raze_delay_ms: 10,
      auto_bkb_on_ultimate: true, auto_d_on_ultimate: true,
      standalone_key: "Home", armlet: {},
    },
    tiny: { standalone_key: "Home", armlet: {} },
    outworld_destroyer: {
      standalone_key: "Home", objurgation_key: "w", arcane_orb_key: "q",
      astral_imprisonment_key: "e", auto_objurgation_on_danger: true,
      objurgation_hp_threshold_percent: 55, objurgation_min_mana_percent: 25,
      objurgation_trigger_cooldown_ms: 1500, ultimate_intercept_enabled: true,
      auto_bkb_on_ultimate: true, auto_objurgation_on_ultimate: true,
      post_bkb_delay_ms: 50, post_blink_delay_ms: 80,
      astral_self_cast_enabled: true, astral_self_cast_key: "F5",
      combo_items: ["sheepstick", "bloodthorn"], combo_item_spam_count: 3,
      combo_item_delay_ms: 30, post_ultimate_arcane_orb_presses: 3,
      arcane_orb_press_interval_ms: 50, armlet: {},
    },
    largo: {
      amphibian_rhapsody_enabled: true, auto_toggle_on_danger: true,
      mana_threshold_percent: 20, heal_hp_threshold: 50,
      beat_interval_ms: 995, beat_correction_ms: 30,
      beat_correction_every_n_beats: 5, q_ability_key: "q",
      w_ability_key: "w", e_ability_key: "e", r_ability_key: "r",
      standalone_key: "Home", armlet: {},
    },
    broodmother: {
      spider_micro_enabled: true, spider_control_group_key: "F3",
      reselect_hero_key: "1", attack_key: "a", standalone_key: "Space",
      auto_items_enabled: true, auto_items_modifier: "Space",
      auto_items: ["orchid", "bloodthorn", "diffusal_blade", "disperser", "nullifier", "abyssal_blade"],
      auto_abilities: [
        { index: 0, key: "q", hp_threshold: 80 },
        { index: 3, key: "r" },
      ],
      auto_abilities_first: false, armlet: {},
    },
    meepo: {
      standalone_key: "Home", earthbind_key: "q", poof_key: "w",
      dig_key: "e", megameepo_key: "r", post_blink_delay_ms: 80,
      combo_items: ["sheepstick", "disperser"], combo_item_spam_count: 3,
      combo_item_delay_ms: 30, earthbind_press_count: 2,
      earthbind_press_interval_ms: 50, poof_press_count: 3,
      poof_press_interval_ms: 50, auto_dig_on_danger: true,
      dig_hp_threshold_percent: 32, auto_megameepo_on_danger: true,
      megameepo_hp_threshold_percent: 45, defensive_trigger_cooldown_ms: 1500,
      farm_assist: {
        enabled: true, toggle_key: "End", pulse_interval_ms: 700,
        minimum_mana_percent: 35, minimum_health_percent: 45,
        right_click_after_poof: true, suspend_on_danger: true,
        suspend_after_manual_combo_ms: 2500, poof_press_count: 3,
        poof_press_interval_ms: 50,
      },
      armlet: {},
    },
  },
  danger_detection: {
    enabled: true, hp_threshold_percent: 70, rapid_loss_hp: 100,
    time_window_ms: 500, clear_delay_seconds: 3,
    healing_threshold_in_danger: 50, max_healing_items_per_danger: 3,
    auto_bkb: true, auto_satanic: true, satanic_hp_threshold: 40,
    auto_blade_mail: true, auto_glimmer_cape: true,
    auto_ghost_scepter: true, auto_shivas_guard: true,
    auto_manta_on_silence: true, auto_lotus_on_silence: true,
  },
  neutral_items: {
    enabled: false, self_cast_key: "0", log_discoveries: false,
    use_in_danger: true, hp_threshold: 50,
    allowed_items: ["essence_ring", "minotaur_horn", "metamorphic_mandible"],
  },
  soul_ring: {
    enabled: true, min_mana_percent: 100, min_health_percent: 20,
    delay_before_ability_ms: 30, trigger_cooldown_ms: 10,
    ability_keys: ["q", "w", "e", "r", "d", "f"],
    intercept_item_keys: true,
  },
  updates: { check_on_startup: true, include_prereleases: false },
  rune_alerts: {
    enabled: true, alert_lead_seconds: 10,
    interval_seconds: 120, audio_enabled: true,
  },
  minimap_capture: {
    enabled: false, minimap_x: 0, minimap_y: 0,
    minimap_width: 256, minimap_height: 256,
    capture_interval_ms: 1000, sample_every_n: 5,
    artifact_output_dir: "artifacts/minimap",
  },
};

export const mockActivityLog: ActivityEntry[] = [
  { id: "1", timestamp: "14:32:01.234", category: "system", message: "GSI server started on port 3000" },
  { id: "2", timestamp: "14:32:05.112", category: "system", message: "Hero detected: Shadow Fiend" },
  { id: "3", timestamp: "14:33:12.456", category: "action", message: "Soul Ring → Raze (Q)" },
  { id: "4", timestamp: "14:33:15.789", category: "danger", message: "⚠ Danger detected — HP 28%" },
  { id: "5", timestamp: "14:33:15.820", category: "action", message: "Auto-BKB activated" },
  { id: "6", timestamp: "14:33:16.100", category: "action", message: "Satanic activated (HP 22%)" },
  { id: "7", timestamp: "14:34:00.000", category: "system", message: "🔮 Rune spawning in 10s" },
  { id: "8", timestamp: "14:35:22.333", category: "action", message: "Armlet toggled ON" },
];
