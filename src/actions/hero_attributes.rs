//! Primary attribute for every Dota hero.
//!
//! GENERATED FILE - DO NOT EDIT BY HAND.
//! Regenerate with `pwsh scripts/generate-hero-attributes.ps1` after a patch that adds
//! a hero or changes one's attribute.
//!
//! Source: https://github.com/odota/dotaconstants (generated from Valve's game files).
//!
//! GSI reports `hero.name` but not the hero's attribute. Power Treads bought on a hero
//! start on its primary attribute, so this is what seeds the treads state.
//!
//! A missing key means `None` - a hero this table has never heard of.

use std::collections::HashMap;
use std::sync::LazyLock;

/// A hero's primary attribute. `Universal` heroes draw damage from all three and have
/// no single attribute for Power Treads to start on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimaryAttribute {
    Strength,
    Agility,
    Intelligence,
    Universal,
}

/// Primary attribute keyed by GSI `hero.name`.
#[rustfmt::skip]
pub static HERO_PRIMARY_ATTRIBUTE_TABLE: &[(&str, PrimaryAttribute)] = &[
    ("npc_dota_hero_abaddon", PrimaryAttribute::Universal),  // Abaddon
    ("npc_dota_hero_abyssal_underlord", PrimaryAttribute::Strength),  // Underlord
    ("npc_dota_hero_alchemist", PrimaryAttribute::Strength),  // Alchemist
    ("npc_dota_hero_ancient_apparition", PrimaryAttribute::Intelligence),  // Ancient Apparition
    ("npc_dota_hero_antimage", PrimaryAttribute::Agility),  // Anti-Mage
    ("npc_dota_hero_arc_warden", PrimaryAttribute::Universal),  // Arc Warden
    ("npc_dota_hero_axe", PrimaryAttribute::Strength),  // Axe
    ("npc_dota_hero_bane", PrimaryAttribute::Universal),  // Bane
    ("npc_dota_hero_batrider", PrimaryAttribute::Universal),  // Batrider
    ("npc_dota_hero_beastmaster", PrimaryAttribute::Universal),  // Beastmaster
    ("npc_dota_hero_bloodseeker", PrimaryAttribute::Agility),  // Bloodseeker
    ("npc_dota_hero_bounty_hunter", PrimaryAttribute::Agility),  // Bounty Hunter
    ("npc_dota_hero_brewmaster", PrimaryAttribute::Universal),  // Brewmaster
    ("npc_dota_hero_bristleback", PrimaryAttribute::Strength),  // Bristleback
    ("npc_dota_hero_broodmother", PrimaryAttribute::Agility),  // Broodmother
    ("npc_dota_hero_centaur", PrimaryAttribute::Strength),  // Centaur Warrunner
    ("npc_dota_hero_chaos_knight", PrimaryAttribute::Strength),  // Chaos Knight
    ("npc_dota_hero_chen", PrimaryAttribute::Intelligence),  // Chen
    ("npc_dota_hero_clinkz", PrimaryAttribute::Agility),  // Clinkz
    ("npc_dota_hero_crystal_maiden", PrimaryAttribute::Intelligence),  // Crystal Maiden
    ("npc_dota_hero_dark_seer", PrimaryAttribute::Intelligence),  // Dark Seer
    ("npc_dota_hero_dark_willow", PrimaryAttribute::Intelligence),  // Dark Willow
    ("npc_dota_hero_dawnbreaker", PrimaryAttribute::Strength),  // Dawnbreaker
    ("npc_dota_hero_dazzle", PrimaryAttribute::Universal),  // Dazzle
    ("npc_dota_hero_death_prophet", PrimaryAttribute::Universal),  // Death Prophet
    ("npc_dota_hero_disruptor", PrimaryAttribute::Intelligence),  // Disruptor
    ("npc_dota_hero_doom_bringer", PrimaryAttribute::Strength),  // Doom
    ("npc_dota_hero_dragon_knight", PrimaryAttribute::Strength),  // Dragon Knight
    ("npc_dota_hero_drow_ranger", PrimaryAttribute::Agility),  // Drow Ranger
    ("npc_dota_hero_earth_spirit", PrimaryAttribute::Strength),  // Earth Spirit
    ("npc_dota_hero_earthshaker", PrimaryAttribute::Strength),  // Earthshaker
    ("npc_dota_hero_elder_titan", PrimaryAttribute::Strength),  // Elder Titan
    ("npc_dota_hero_ember_spirit", PrimaryAttribute::Agility),  // Ember Spirit
    ("npc_dota_hero_enchantress", PrimaryAttribute::Intelligence),  // Enchantress
    ("npc_dota_hero_enigma", PrimaryAttribute::Universal),  // Enigma
    ("npc_dota_hero_faceless_void", PrimaryAttribute::Agility),  // Faceless Void
    ("npc_dota_hero_furion", PrimaryAttribute::Universal),  // Nature's Prophet
    ("npc_dota_hero_grimstroke", PrimaryAttribute::Intelligence),  // Grimstroke
    ("npc_dota_hero_gyrocopter", PrimaryAttribute::Agility),  // Gyrocopter
    ("npc_dota_hero_hoodwink", PrimaryAttribute::Agility),  // Hoodwink
    ("npc_dota_hero_huskar", PrimaryAttribute::Strength),  // Huskar
    ("npc_dota_hero_invoker", PrimaryAttribute::Intelligence),  // Invoker
    ("npc_dota_hero_jakiro", PrimaryAttribute::Intelligence),  // Jakiro
    ("npc_dota_hero_juggernaut", PrimaryAttribute::Agility),  // Juggernaut
    ("npc_dota_hero_keeper_of_the_light", PrimaryAttribute::Intelligence),  // Keeper of the Light
    ("npc_dota_hero_kez", PrimaryAttribute::Agility),  // Kez
    ("npc_dota_hero_kunkka", PrimaryAttribute::Strength),  // Kunkka
    ("npc_dota_hero_largo", PrimaryAttribute::Strength),  // Largo
    ("npc_dota_hero_legion_commander", PrimaryAttribute::Strength),  // Legion Commander
    ("npc_dota_hero_leshrac", PrimaryAttribute::Intelligence),  // Leshrac
    ("npc_dota_hero_lich", PrimaryAttribute::Intelligence),  // Lich
    ("npc_dota_hero_life_stealer", PrimaryAttribute::Strength),  // Lifestealer
    ("npc_dota_hero_lina", PrimaryAttribute::Intelligence),  // Lina
    ("npc_dota_hero_lion", PrimaryAttribute::Intelligence),  // Lion
    ("npc_dota_hero_lone_druid", PrimaryAttribute::Agility),  // Lone Druid
    ("npc_dota_hero_luna", PrimaryAttribute::Agility),  // Luna
    ("npc_dota_hero_lycan", PrimaryAttribute::Strength),  // Lycan
    ("npc_dota_hero_magnataur", PrimaryAttribute::Universal),  // Magnus
    ("npc_dota_hero_marci", PrimaryAttribute::Universal),  // Marci
    ("npc_dota_hero_mars", PrimaryAttribute::Strength),  // Mars
    ("npc_dota_hero_medusa", PrimaryAttribute::Agility),  // Medusa
    ("npc_dota_hero_meepo", PrimaryAttribute::Agility),  // Meepo
    ("npc_dota_hero_mirana", PrimaryAttribute::Agility),  // Mirana
    ("npc_dota_hero_monkey_king", PrimaryAttribute::Agility),  // Monkey King
    ("npc_dota_hero_morphling", PrimaryAttribute::Agility),  // Morphling
    ("npc_dota_hero_muerta", PrimaryAttribute::Intelligence),  // Muerta
    ("npc_dota_hero_naga_siren", PrimaryAttribute::Agility),  // Naga Siren
    ("npc_dota_hero_necrolyte", PrimaryAttribute::Intelligence),  // Necrophos
    ("npc_dota_hero_nevermore", PrimaryAttribute::Agility),  // Shadow Fiend
    ("npc_dota_hero_night_stalker", PrimaryAttribute::Strength),  // Night Stalker
    ("npc_dota_hero_nyx_assassin", PrimaryAttribute::Universal),  // Nyx Assassin
    ("npc_dota_hero_obsidian_destroyer", PrimaryAttribute::Intelligence),  // Outworld Devourer
    ("npc_dota_hero_ogre_magi", PrimaryAttribute::Strength),  // Ogre Magi
    ("npc_dota_hero_omniknight", PrimaryAttribute::Strength),  // Omniknight
    ("npc_dota_hero_oracle", PrimaryAttribute::Intelligence),  // Oracle
    ("npc_dota_hero_pangolier", PrimaryAttribute::Universal),  // Pangolier
    ("npc_dota_hero_phantom_assassin", PrimaryAttribute::Agility),  // Phantom Assassin
    ("npc_dota_hero_phantom_lancer", PrimaryAttribute::Agility),  // Phantom Lancer
    ("npc_dota_hero_phoenix", PrimaryAttribute::Strength),  // Phoenix
    ("npc_dota_hero_primal_beast", PrimaryAttribute::Strength),  // Primal Beast
    ("npc_dota_hero_puck", PrimaryAttribute::Intelligence),  // Puck
    ("npc_dota_hero_pudge", PrimaryAttribute::Strength),  // Pudge
    ("npc_dota_hero_pugna", PrimaryAttribute::Intelligence),  // Pugna
    ("npc_dota_hero_queenofpain", PrimaryAttribute::Intelligence),  // Queen of Pain
    ("npc_dota_hero_rattletrap", PrimaryAttribute::Strength),  // Clockwerk
    ("npc_dota_hero_razor", PrimaryAttribute::Agility),  // Razor
    ("npc_dota_hero_riki", PrimaryAttribute::Agility),  // Riki
    ("npc_dota_hero_ringmaster", PrimaryAttribute::Intelligence),  // Ring Master
    ("npc_dota_hero_rubick", PrimaryAttribute::Intelligence),  // Rubick
    ("npc_dota_hero_sand_king", PrimaryAttribute::Universal),  // Sand King
    ("npc_dota_hero_shadow_demon", PrimaryAttribute::Intelligence),  // Shadow Demon
    ("npc_dota_hero_shadow_shaman", PrimaryAttribute::Intelligence),  // Shadow Shaman
    ("npc_dota_hero_shredder", PrimaryAttribute::Strength),  // Timbersaw
    ("npc_dota_hero_silencer", PrimaryAttribute::Intelligence),  // Silencer
    ("npc_dota_hero_skeleton_king", PrimaryAttribute::Strength),  // Wraith King
    ("npc_dota_hero_skywrath_mage", PrimaryAttribute::Intelligence),  // Skywrath Mage
    ("npc_dota_hero_slardar", PrimaryAttribute::Strength),  // Slardar
    ("npc_dota_hero_slark", PrimaryAttribute::Agility),  // Slark
    ("npc_dota_hero_snapfire", PrimaryAttribute::Universal),  // Snapfire
    ("npc_dota_hero_sniper", PrimaryAttribute::Agility),  // Sniper
    ("npc_dota_hero_spectre", PrimaryAttribute::Agility),  // Spectre
    ("npc_dota_hero_spirit_breaker", PrimaryAttribute::Strength),  // Spirit Breaker
    ("npc_dota_hero_storm_spirit", PrimaryAttribute::Intelligence),  // Storm Spirit
    ("npc_dota_hero_sven", PrimaryAttribute::Strength),  // Sven
    ("npc_dota_hero_techies", PrimaryAttribute::Universal),  // Techies
    ("npc_dota_hero_templar_assassin", PrimaryAttribute::Agility),  // Templar Assassin
    ("npc_dota_hero_terrorblade", PrimaryAttribute::Agility),  // Terrorblade
    ("npc_dota_hero_tidehunter", PrimaryAttribute::Strength),  // Tidehunter
    ("npc_dota_hero_tinker", PrimaryAttribute::Intelligence),  // Tinker
    ("npc_dota_hero_tiny", PrimaryAttribute::Strength),  // Tiny
    ("npc_dota_hero_treant", PrimaryAttribute::Strength),  // Treant Protector
    ("npc_dota_hero_troll_warlord", PrimaryAttribute::Agility),  // Troll Warlord
    ("npc_dota_hero_tusk", PrimaryAttribute::Strength),  // Tusk
    ("npc_dota_hero_undying", PrimaryAttribute::Strength),  // Undying
    ("npc_dota_hero_ursa", PrimaryAttribute::Agility),  // Ursa
    ("npc_dota_hero_vengefulspirit", PrimaryAttribute::Agility),  // Vengeful Spirit
    ("npc_dota_hero_venomancer", PrimaryAttribute::Universal),  // Venomancer
    ("npc_dota_hero_viper", PrimaryAttribute::Agility),  // Viper
    ("npc_dota_hero_visage", PrimaryAttribute::Universal),  // Visage
    ("npc_dota_hero_void_spirit", PrimaryAttribute::Universal),  // Void Spirit
    ("npc_dota_hero_warlock", PrimaryAttribute::Intelligence),  // Warlock
    ("npc_dota_hero_weaver", PrimaryAttribute::Agility),  // Weaver
    ("npc_dota_hero_windrunner", PrimaryAttribute::Universal),  // Windranger
    ("npc_dota_hero_winter_wyvern", PrimaryAttribute::Intelligence),  // Winter Wyvern
    ("npc_dota_hero_wisp", PrimaryAttribute::Universal),  // Io
    ("npc_dota_hero_witch_doctor", PrimaryAttribute::Intelligence),  // Witch Doctor
    ("npc_dota_hero_zuus", PrimaryAttribute::Intelligence),  // Zeus
];

static HERO_PRIMARY_ATTRIBUTE: LazyLock<HashMap<&'static str, PrimaryAttribute>> =
    LazyLock::new(|| HERO_PRIMARY_ATTRIBUTE_TABLE.iter().copied().collect());

/// The hero's primary attribute, or `None` when the hero is not in the table.
pub fn primary_attribute(hero_name: &str) -> Option<PrimaryAttribute> {
    HERO_PRIMARY_ATTRIBUTE.get(hero_name).copied()
}
