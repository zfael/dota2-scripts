use serde::{Deserialize, Serialize};

// Every block below is `#[serde(default)]` on purpose. Dota does not send a
// fixed payload: the ability list is as long as the hero's ability panel, and
// Valve adds and removes hero fields between patches. Without container-level
// defaults, one hero whose payload is a field short makes axum reject *every*
// event with 422 before the handler runs, which reads in the UI as "GSI
// Disconnected" with nothing in the logs. Missing data should degrade a single
// field, never the whole pipeline.

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Item {
    pub name: String,
    pub can_cast: Option<bool>,
    pub cooldown: Option<u32>,
    pub item_level: Option<u32>,
    pub passive: Option<bool>,
    pub purchaser: Option<u32>,
    pub charges: Option<u32>,
    pub item_charges: Option<u32>,
}

impl Default for Item {
    fn default() -> Self {
        Self {
            name: String::from("empty"),
            can_cast: None,
            cooldown: None,
            item_level: None,
            passive: None,
            purchaser: None,
            charges: None,
            item_charges: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Items {
    pub neutral0: Item,
    pub slot0: Item,
    pub slot1: Item,
    pub slot2: Item,
    pub slot3: Item,
    pub slot4: Item,
    pub slot5: Item,
    pub slot6: Item,
    pub slot7: Item,
    pub slot8: Item,
    pub stash0: Item,
    pub stash1: Item,
    pub stash2: Item,
    pub stash3: Item,
    pub stash4: Item,
    pub stash5: Item,
    pub teleport0: Item,
}

impl Items {
    /// Get all item slots as a vector of (slot_name, item) tuples
    pub fn all_slots(&self) -> Vec<(&str, &Item)> {
        vec![
            ("slot0", &self.slot0),
            ("slot1", &self.slot1),
            ("slot2", &self.slot2),
            ("slot3", &self.slot3),
            ("slot4", &self.slot4),
            ("slot5", &self.slot5),
            ("neutral0", &self.neutral0),
        ]
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Ability {
    pub ability_active: bool,
    pub can_cast: bool,
    pub cooldown: u32,
    pub level: u32,
    pub name: String,
    pub passive: bool,
    pub ultimate: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Abilities {
    pub ability0: Ability,
    pub ability1: Ability,
    pub ability2: Ability,
    pub ability3: Ability,
    pub ability4: Ability,
    pub ability5: Ability,
}

impl Abilities {
    /// Get ability by index (0-5)
    pub fn get_by_index(&self, index: u8) -> Option<&Ability> {
        match index {
            0 => Some(&self.ability0),
            1 => Some(&self.ability1),
            2 => Some(&self.ability2),
            3 => Some(&self.ability3),
            4 => Some(&self.ability4),
            5 => Some(&self.ability5),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Hero {
    pub aghanims_scepter: bool,
    pub aghanims_shard: bool,
    pub alive: bool,
    pub attributes_level: u32,
    #[serde(rename = "break")]
    pub is_break: bool,
    pub buyback_cooldown: u32,
    pub buyback_cost: u32,
    pub disarmed: bool,
    pub facet: u32,
    pub has_debuff: bool,
    pub health: u32,
    pub health_percent: u32,
    pub hexed: bool,
    /// Signed: Dota reports `-1` while no hero is picked yet.
    pub id: i32,
    pub level: u32,
    pub magicimmune: bool,
    pub mana: u32,
    pub mana_percent: u32,
    pub max_health: u32,
    pub max_mana: u32,
    pub muted: bool,
    pub name: String,
    pub respawn_seconds: u32,
    pub silenced: bool,
    pub smoked: bool,
    pub stunned: bool,
    pub talent_1: bool,
    pub talent_2: bool,
    pub talent_3: bool,
    pub talent_4: bool,
    pub talent_5: bool,
    pub talent_6: bool,
    pub talent_7: bool,
    pub talent_8: bool,
    pub xp: u32,
    pub xpos: i32,
    pub ypos: i32,
}

impl Hero {
    pub fn is_alive(&self) -> bool {
        self.alive
    }

    pub fn is_stunned(&self) -> bool {
        self.stunned
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Map {
    pub clock_time: i32,
    /// `DOTA_GAMERULES_STATE_*`. Confirmed on the wire to reach a *player*, not
    /// just a spectator — this is what gates the draft reader, because the
    /// vision side cannot tell a draft screen from a menu (it once confidently
    /// read three heroes off the main menu). At the main menu Dota sends no
    /// `map` block at all, so `#[serde(default)]` leaves this empty there.
    pub game_state: String,
    /// Scopes a draft session: votes must never carry across games. Dota has
    /// sent this as both a string and a bare number across builds, hence the
    /// custom deserializer.
    #[serde(deserialize_with = "string_or_number", default)]
    pub matchid: String,
}

/// Accepts `"123"`, `123`, or absence, normalising all three to a `String`.
fn string_or_number<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(match value {
        serde_json::Value::String(s) => s,
        serde_json::Value::Number(n) => n.to_string(),
        _ => String::new(),
    })
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Player {
    pub team_name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct GsiWebhookEvent {
    pub hero: Hero,
    pub abilities: Abilities,
    pub items: Items,
    pub map: Map,
    pub player: Option<Player>,
}

impl GsiWebhookEvent {
    /// True when the payload carries no hero at all — the menu, the draft, or a
    /// spectator feed. Those events keep the connection alive but have nothing
    /// for automation to act on.
    pub fn has_hero(&self) -> bool {
        !self.hero.name.is_empty() && self.hero.name != "empty"
    }
}

#[cfg(test)]
mod map_tests {
    use super::*;

    #[test]
    fn matchid_accepts_string_and_number_and_absence() {
        let m: Map = serde_json::from_str(r#"{"matchid":"812345"}"#).unwrap();
        assert_eq!(m.matchid, "812345");

        let m: Map = serde_json::from_str(r#"{"matchid":812345}"#).unwrap();
        assert_eq!(m.matchid, "812345");

        let m: Map = serde_json::from_str("{}").unwrap();
        assert_eq!(m.matchid, "");
    }

    #[test]
    fn game_state_defaults_empty_when_map_block_absent() {
        // The main menu sends no `map` block at all; the event must still parse
        // and the gate field must read as "not in any game state".
        let e: GsiWebhookEvent = serde_json::from_str(r#"{"provider":{}}"#).unwrap();
        assert_eq!(e.map.game_state, "");
    }

    #[test]
    fn game_state_carries_hero_selection() {
        let e: GsiWebhookEvent = serde_json::from_str(
            r#"{"map":{"game_state":"DOTA_GAMERULES_STATE_HERO_SELECTION","clock_time":0}}"#,
        )
        .unwrap();
        assert_eq!(e.map.game_state, "DOTA_GAMERULES_STATE_HERO_SELECTION");
    }
}
