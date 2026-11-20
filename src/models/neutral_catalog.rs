//! The neutral item pool, by tier.
//!
//! GENERATED FILE - DO NOT EDIT BY HAND.
//! Regenerate with `pwsh scripts/generate-neutral-catalog.ps1` after a gameplay patch.
//!
//! Source: Valve's datafeed (https://www.dota2.com/datafeed), the data behind dota2.com.
//!
//! This only says what is in the pool and how each item is cast. Whether the runtime
//! automates one is `src/actions/item_automation.rs`'s call.

/// How the game casts the item, from its `DOTA_ABILITY_BEHAVIOR` flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeutralTargeting {
    Passive,
    NoTarget,
    UnitTarget,
    PointTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NeutralCatalogEntry {
    /// GSI's `item.name`.
    pub name: &'static str,
    pub display_name: &'static str,
    /// 1-based, as the game shows it.
    pub tier: u8,
    pub targeting: NeutralTargeting,
}

#[rustfmt::skip]
pub static NEUTRAL_ITEM_CATALOG: &[NeutralCatalogEntry] = &[
    NeutralCatalogEntry { name: "item_ash_legion_shield", display_name: "Ash Legion Shield", tier: 1, targeting: NeutralTargeting::NoTarget },
    NeutralCatalogEntry { name: "item_chipped_vest", display_name: "Chipped Vest", tier: 1, targeting: NeutralTargeting::Passive },
    NeutralCatalogEntry { name: "item_dormant_curio", display_name: "Dormant Curio", tier: 1, targeting: NeutralTargeting::Passive },
    NeutralCatalogEntry { name: "item_foragers_kit", display_name: "Forager's Kit", tier: 1, targeting: NeutralTargeting::Passive },
    NeutralCatalogEntry { name: "item_kobold_cup", display_name: "Kobold Cup", tier: 1, targeting: NeutralTargeting::NoTarget },
    NeutralCatalogEntry { name: "item_polliwog_charm", display_name: "Pollywog Charm", tier: 1, targeting: NeutralTargeting::UnitTarget },
    NeutralCatalogEntry { name: "item_possessed_mask", display_name: "Possessed Mask", tier: 1, targeting: NeutralTargeting::Passive },
    NeutralCatalogEntry { name: "item_red_ledger", display_name: "Red Ledger", tier: 1, targeting: NeutralTargeting::NoTarget },
    NeutralCatalogEntry { name: "item_seeds_of_serenity", display_name: "Seeds of Serenity", tier: 1, targeting: NeutralTargeting::PointTarget },
    NeutralCatalogEntry { name: "item_stonefeather_satchel", display_name: "Stonefeather Satchel", tier: 1, targeting: NeutralTargeting::NoTarget },
    NeutralCatalogEntry { name: "item_pogo_stick", display_name: "Tumbler's Toy", tier: 1, targeting: NeutralTargeting::NoTarget },
    NeutralCatalogEntry { name: "item_weighted_dice", display_name: "Weighted Dice", tier: 1, targeting: NeutralTargeting::Passive },
    NeutralCatalogEntry { name: "item_brick", display_name: "Brick", tier: 2, targeting: NeutralTargeting::UnitTarget },
    NeutralCatalogEntry { name: "item_crippling_crossbow", display_name: "Crippling Crossbow", tier: 2, targeting: NeutralTargeting::UnitTarget },
    NeutralCatalogEntry { name: "item_defiant_shell", display_name: "Defiant Shell", tier: 2, targeting: NeutralTargeting::Passive },
    NeutralCatalogEntry { name: "item_gunpowder_gauntlets", display_name: "Gunpowder Gauntlet", tier: 2, targeting: NeutralTargeting::Passive },
    NeutralCatalogEntry { name: "item_imp_claw", display_name: "Imp Claw", tier: 2, targeting: NeutralTargeting::Passive },
    NeutralCatalogEntry { name: "item_jidi_pollen_bag", display_name: "Jidi Pollen Bag", tier: 2, targeting: NeutralTargeting::NoTarget },
    NeutralCatalogEntry { name: "item_mana_draught", display_name: "Mana Draught", tier: 2, targeting: NeutralTargeting::NoTarget },
    NeutralCatalogEntry { name: "item_metamorphic_mandible", display_name: "Metamorphic Mandible", tier: 2, targeting: NeutralTargeting::NoTarget },
    NeutralCatalogEntry { name: "item_partisans_brand", display_name: "Partisan's Brand", tier: 2, targeting: NeutralTargeting::Passive },
    NeutralCatalogEntry { name: "item_psychic_headband", display_name: "Psychic Headband", tier: 2, targeting: NeutralTargeting::UnitTarget },
    NeutralCatalogEntry { name: "item_safety_bubble", display_name: "Safety Bubble", tier: 2, targeting: NeutralTargeting::Passive },
    NeutralCatalogEntry { name: "item_spellslinger", display_name: "Spellslinger", tier: 2, targeting: NeutralTargeting::Passive },
    NeutralCatalogEntry { name: "item_balmhook", display_name: "Balmhook", tier: 3, targeting: NeutralTargeting::Passive },
    NeutralCatalogEntry { name: "item_ceremonial_robe", display_name: "Ceremonial Robe", tier: 3, targeting: NeutralTargeting::Passive },
    NeutralCatalogEntry { name: "item_dandelion_amulet", display_name: "Dandelion Amulet", tier: 3, targeting: NeutralTargeting::Passive },
    NeutralCatalogEntry { name: "item_enchanters_bauble", display_name: "Enchanter's Bauble", tier: 3, targeting: NeutralTargeting::Passive },
    NeutralCatalogEntry { name: "item_harmonizer", display_name: "Harmonizer", tier: 3, targeting: NeutralTargeting::Passive },
    NeutralCatalogEntry { name: "item_idol_of_screeauk", display_name: "Idol of Scree'auk", tier: 3, targeting: NeutralTargeting::NoTarget },
    NeutralCatalogEntry { name: "item_prophets_pendulum", display_name: "Prophet's Pendulum", tier: 3, targeting: NeutralTargeting::Passive },
    NeutralCatalogEntry { name: "item_rattlecage", display_name: "Rattlecage", tier: 3, targeting: NeutralTargeting::Passive },
    NeutralCatalogEntry { name: "item_revenants_brooch", display_name: "Revenant's Brooch", tier: 3, targeting: NeutralTargeting::Passive },
    NeutralCatalogEntry { name: "item_serrated_shiv", display_name: "Serrated Shiv", tier: 3, targeting: NeutralTargeting::Passive },
    NeutralCatalogEntry { name: "item_stormcrafter", display_name: "Stormcrafter", tier: 3, targeting: NeutralTargeting::Passive },
    NeutralCatalogEntry { name: "item_vile_phial", display_name: "Vile Phial", tier: 3, targeting: NeutralTargeting::Passive },
    NeutralCatalogEntry { name: "item_demonicon", display_name: "Book of the Dead", tier: 4, targeting: NeutralTargeting::NoTarget },
    NeutralCatalogEntry { name: "item_conjurers_catalyst", display_name: "Conjurer's Catalyst", tier: 4, targeting: NeutralTargeting::Passive },
    NeutralCatalogEntry { name: "item_dezun_bloodrite", display_name: "Dezun Bloodrite", tier: 4, targeting: NeutralTargeting::Passive },
    NeutralCatalogEntry { name: "item_divine_regalia", display_name: "Divine Regalia", tier: 4, targeting: NeutralTargeting::Passive },
    NeutralCatalogEntry { name: "item_fallen_sky", display_name: "Fallen Sky", tier: 4, targeting: NeutralTargeting::PointTarget },
    NeutralCatalogEntry { name: "item_flicker", display_name: "Flicker", tier: 4, targeting: NeutralTargeting::NoTarget },
    NeutralCatalogEntry { name: "item_giants_ring", display_name: "Giant's Ring", tier: 4, targeting: NeutralTargeting::Passive },
    NeutralCatalogEntry { name: "item_helm_of_the_hivemind", display_name: "Helm of the Hivemind", tier: 4, targeting: NeutralTargeting::Passive },
    NeutralCatalogEntry { name: "item_mind_breaker", display_name: "Mind Breaker", tier: 4, targeting: NeutralTargeting::Passive },
    NeutralCatalogEntry { name: "item_minotaur_horn", display_name: "Minotaur Horn", tier: 4, targeting: NeutralTargeting::NoTarget },
    NeutralCatalogEntry { name: "item_riftshadow_prism", display_name: "Riftshadow Prism", tier: 4, targeting: NeutralTargeting::NoTarget },
    NeutralCatalogEntry { name: "item_heavy_blade", display_name: "Witchbane", tier: 4, targeting: NeutralTargeting::PointTarget },
];
