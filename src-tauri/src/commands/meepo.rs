use crate::ipc_types::MeepoStateDto;
use dota2_scripts::actions::heroes::meepo_state;

/// Returns the latest observed Meepo state, or null if not playing Meepo.
#[tauri::command]
pub fn get_meepo_state() -> Option<MeepoStateDto> {
    meepo_state::latest_meepo_observed_state().map(|s| MeepoStateDto {
        health_percent: s.health_percent,
        mana_percent: s.mana_percent,
        in_danger: s.in_danger,
        alive: s.alive,
        stunned: s.stunned,
        silenced: s.silenced,
        poof_ready: s.poof_ready,
        dig_ready: s.dig_ready,
        megameepo_ready: s.megameepo_ready,
        has_shard: s.has_shard,
        has_scepter: s.has_scepter,
        blink_available: s.blink_slot_key.is_some(),
        combo_items: s
            .combo_item_keys
            .into_iter()
            .map(|(name, _)| name)
            .collect(),
    })
}
