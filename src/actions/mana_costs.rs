//! Mana costs for every Dota item and hero ability.
//!
//! GENERATED FILE - DO NOT EDIT BY HAND.
//! Regenerate with `pwsh scripts/generate-mana-costs.ps1` after a gameplay patch.
//!
//! Source: Valve's datafeed (https://www.dota2.com/datafeed), the data behind dota2.com.
//!
//! Soul Ring trades 170 HP for mana, so it must only fire ahead of something that
//! actually spends mana. GSI reports readiness (`can_cast`, `cooldown`) but never a
//! cost, so the cost has to come from here.
//!
//! A missing key means `None` - unknown to this table, not free. Callers treat
//! unknown as "do not trigger" so a post-patch item fails safe.

use std::collections::HashMap;
use std::sync::LazyLock;

/// Flat item mana costs, keyed by GSI `item.name`. `0` means the item is known to
/// cost no mana (passive, toggle, or a free active such as Quelling Blade's chop).
#[rustfmt::skip]
pub static ITEM_MANA_COST_TABLE: &[(&str, u32)] = &[
    ("item_abyssal_blade", 75),  // Abyssal Blade
    ("item_aegis", 0),  // Aegis of the Immortal
    ("item_aeon_disk", 0),  // Aeon Disk
    ("item_aether_lens", 0),  // Aether Lens
    ("item_aghanims_shard", 0),  // Aghanim's Shard
    ("item_aghanims_shard_roshan", 0),  // Aghanim's Shard - Consumable
    ("item_ancient_guardian", 0),  // Ancient Guardian
    ("item_ancient_janggo", 0),  // Drum of Endurance
    ("item_angels_demise", 0),  // Khanda
    ("item_apex", 0),  // Apex
    ("item_arcane_blink", 0),  // Arcane Blink
    ("item_arcane_boots", 0),  // Arcane Boots
    ("item_arcane_ring", 0),  // Arcane Ring
    ("item_armlet", 0),  // Armlet of Mordiggian
    ("item_ascetic_cap", 0),  // Ascetic's Cap
    ("item_ash_legion_shield", 0),  // Ash Legion Shield
    ("item_assault", 0),  // Assault Cuirass
    ("item_avianas_feather", 0),  // Aviana's Feather
    ("item_ballista", 0),  // Ballista
    ("item_balmhook", 0),  // Balmhook
    ("item_basher", 0),  // Skull Basher
    ("item_belt_of_strength", 0),  // Belt of Strength
    ("item_berserkers_hood", 0),  // Berserker's Hood
    ("item_berserkers_sabatons", 0),  // Sabatons of Mordiggian
    ("item_bfury", 0),  // Battle Fury
    ("item_black_grimoire", 0),  // Black Grimoire (Warlock)
    ("item_black_king_bar", 50),  // Black King Bar
    ("item_black_powder_bag", 0),  // Blast Rig
    ("item_blade_mail", 25),  // Blade Mail
    ("item_blade_of_alacrity", 0),  // Blade of Alacrity
    ("item_blades_of_attack", 0),  // Blades of Attack
    ("item_blight_stone", 0),  // Orb of Blight
    ("item_blink", 0),  // Blink Dagger
    ("item_blitz_knuckles", 0),  // Blitz Knuckles
    ("item_blood_grenade", 0),  // Blood Grenade
    ("item_bloodslinger", 0),  // Bloodslinger
    ("item_bloodstone", 0),  // Bloodstone
    ("item_bloodthorn", 150),  // Bloodthorn
    ("item_book_of_shadows", 0),  // Book of Shadows
    ("item_boots", 0),  // Boots of Speed
    ("item_boots_of_bearing", 0),  // Boots of Bearing
    ("item_boots_of_elves", 0),  // Band of Elvenskin
    ("item_bottle", 0),  // Bottle
    ("item_bottomless_chalice", 0),
    ("item_bracer", 0),  // Bracer
    ("item_branches", 0),  // Iron Branch
    ("item_brick", 0),  // Brick
    ("item_broadsword", 0),  // Broadsword
    ("item_broom_handle", 0),  // Broom Handle
    ("item_buckler", 0),  // Buckler
    ("item_bullwhip", 0),  // Bullwhip
    ("item_butterfly", 0),  // Butterfly
    ("item_caduceus", 150),  // Caduceus
    ("item_caster_rapier", 0),
    ("item_ceremonial_robe", 0),  // Ceremonial Robe
    ("item_chainmail", 0),  // Chainmail
    ("item_chasm_stone", 0),  // Chasm Stone
    ("item_cheese", 0),  // Cheese
    ("item_chipped_vest", 0),  // Chipped Vest
    ("item_circlet", 0),  // Circlet
    ("item_clarity", 0),  // Clarity
    ("item_claszian_timepiece", 0),  // Claszian Timepiece
    ("item_claymore", 0),  // Claymore
    ("item_cloak", 0),  // Cloak
    ("item_cloak_of_flames", 0),  // Cloak of Flames
    ("item_clumsy_net", 0),  // Clumsy Net
    ("item_conjurers_catalyst", 0),  // Conjurer's Catalyst
    ("item_consecrated_wraps", 0),  // Consecrated Wraps
    ("item_cornucopia", 0),  // Cornucopia
    ("item_courier", 0),  // Animal Courier
    ("item_craggy_coat", 0),  // Craggy Coat
    ("item_crellas_crozier", 0),  // Crella's Crozier
    ("item_crimson_guard", 75),  // Crimson Guard
    ("item_crippling_crossbow", 50),  // Crippling Crossbow
    ("item_crown", 0),  // Crown
    ("item_cyclone", 175),  // Eul's Scepter of Divinity
    ("item_dagger_of_ristul", 0),  // Dagger of Ristul
    ("item_dagon", 200),  // Dagon
    ("item_dagon_2", 200),  // Dagon
    ("item_dagon_3", 200),  // Dagon
    ("item_dagon_4", 200),  // Dagon
    ("item_dagon_5", 200),  // Dagon
    ("item_dandelion_amulet", 0),  // Dandelion Amulet
    ("item_defiant_shell", 0),  // Defiant Shell
    ("item_demon_edge", 0),  // Demon Edge
    ("item_demonicon", 0),  // Book of the Dead
    ("item_desolator", 0),  // Desolator
    ("item_devastator", 0),  // Parasma
    ("item_dezun_bloodrite", 0),  // Dezun Bloodrite
    ("item_diadem", 0),  // Diadem
    ("item_diffusal_blade", 25),  // Diffusal Blade
    ("item_disperser", 75),  // Disperser
    ("item_divine_regalia", 0),  // Divine Regalia
    ("item_divine_regalia_broken", 0),  // Disgraced Regalia
    ("item_dormant_curio", 0),  // Dormant Curio
    ("item_doubloon", 0),  // Doubloon
    ("item_dragon_lance", 0),  // Dragon Lance
    ("item_dragon_scale", 0),  // Dragon Scale
    ("item_duelist_gloves", 0),  // Duelist Gloves
    ("item_dust", 0),  // Dust of Appearance
    ("item_eagle", 0),  // Eaglesong
    ("item_echo_sabre", 0),  // Echo Sabre
    ("item_eldwurms_edda", 0),  // Eldwurm's Edda
    ("item_elixer", 0),  // Elixir
    ("item_elven_tunic", 0),  // Elven Tunic
    ("item_enchanted_mango", 0),  // Enchanted Mango
    ("item_enchanted_quiver", 0),  // Enchanted Quiver
    ("item_enchanters_bauble", 0),  // Enchanter's Bauble
    ("item_energy_booster", 0),  // Energy Booster
    ("item_enhancement_alert", 0),  // Alert
    ("item_enhancement_audacious", 0),  // Audacious
    ("item_enhancement_boundless", 0),  // Boundless
    ("item_enhancement_brawny", 0),  // Brawny
    ("item_enhancement_crude", 0),  // Crude
    ("item_enhancement_curious", 0),  // Unleashed
    ("item_enhancement_dominant", 0),  // Dominant
    ("item_enhancement_evolved", 0),  // Evolved
    ("item_enhancement_feverish", 0),  // Feverish
    ("item_enhancement_fierce", 0),  // Fierce
    ("item_enhancement_fleetfooted", 0),  // Fleetfooted
    ("item_enhancement_greedy", 0),  // Greedy
    ("item_enhancement_hulking", 0),  // Hulking
    ("item_enhancement_keen_eyed", 0),  // Keen-eyed
    ("item_enhancement_manic", 0),  // Manic
    ("item_enhancement_mystical", 0),  // Mystical
    ("item_enhancement_nimble", 0),  // Nimble
    ("item_enhancement_quickened", 0),  // Quickened
    ("item_enhancement_restorative", 0),  // Restorative
    ("item_enhancement_thick", 0),  // Thick
    ("item_enhancement_timeless", 0),  // Timeless
    ("item_enhancement_titanic", 0),  // Titanic
    ("item_enhancement_tough", 0),  // Tough
    ("item_enhancement_vampiric", 0),  // Vampiric
    ("item_enhancement_vast", 0),  // Vast
    ("item_enhancement_vital", 0),  // Vital
    ("item_enhancement_wise", 0),  // Wise
    ("item_essence_distiller", 0),  // Essence Distiller
    ("item_essence_ring", 160),  // Essence Ring
    ("item_eternal_shroud", 0),  // Eternal Shroud
    ("item_ethereal_blade", 100),  // Ethereal Blade
    ("item_ex_machina", 350),  // Ex Machina
    ("item_eye_of_the_vizier", 0),  // Eye of the Vizier
    ("item_faded_broach", 0),  // Faded Broach
    ("item_faerie_fire", 0),  // Faerie Fire
    ("item_falcon_blade", 0),  // Falcon Blade
    ("item_fallen_sky", 0),  // Fallen Sky
    ("item_famango", 0),  // Healing Lotus
    ("item_flask", 0),  // Healing Salve
    ("item_flayers_bota", 0),  // Flayer's Bota
    ("item_flicker", 0),  // Flicker
    ("item_fluffy_hat", 0),  // Fluffy Hat
    ("item_flying_courier", 0),  // Flying Courier
    ("item_foragers_health", 0),  // Vital Toadstool
    ("item_foragers_kit", 0),  // Forager's Kit
    ("item_foragers_mana", 0),  // Tomo'kan Ringcap
    ("item_foragers_stats", 0),  // Ironwood Nut
    ("item_force_boots", 75),  // Force Boots
    ("item_force_field", 0),  // Arcanist's Armor
    ("item_force_staff", 150),  // Force Staff
    ("item_furion_gold_bag", 0),  // Bag of Gold
    ("item_fusion_rune", 0),  // Fusion Rune
    ("item_gale_guard", 0),  // Gale Guard
    ("item_gauntlets", 0),  // Gauntlets of Strength
    ("item_gem", 0),  // Gem of True Sight
    ("item_ghost", 0),  // Ghost Scepter
    ("item_giant_maul", 0),  // Giant's Maul
    ("item_giants_ring", 0),  // Giant's Ring
    ("item_glimmer_cape", 125),  // Glimmer Cape
    ("item_gloves", 0),  // Gloves of Haste
    ("item_gossamer_cape", 0),  // Gossamer Cape
    ("item_great_famango", 0),  // Great Healing Lotus
    ("item_greater_crit", 0),  // Daedalus
    ("item_greater_faerie_fire", 0),  // Greater Faerie Fire
    ("item_greater_famango", 0),  // Greater Healing Lotus
    ("item_greater_mango", 0),
    ("item_grisgris", 0),  // Gris-Gris
    ("item_grove_bow", 0),  // Grove Bow
    ("item_guardian_greaves", 0),  // Guardian Greaves
    ("item_gungir", 150),  // Gleipnir
    ("item_gunpowder_gauntlets", 0),  // Gunpowder Gauntlet
    ("item_hand_of_midas", 0),  // Hand of Midas
    ("item_harmonizer", 0),  // Harmonizer
    ("item_harpoon", 50),  // Harpoon
    ("item_havoc_hammer", 0),  // Havoc Hammer
    ("item_headdress", 0),  // Headdress
    ("item_heart", 0),  // Heart of Tarrasque
    ("item_heavens_halberd", 50),  // Heaven's Halberd
    ("item_heavy_blade", 50),  // Witchbane
    ("item_helm_of_iron_will", 0),  // Helm of Iron Will
    ("item_helm_of_the_dominator", 50),  // Helm of the Dominator
    ("item_helm_of_the_hivemind", 0),  // Helm of the Hivemind
    ("item_helm_of_the_overlord", 50),  // Helm of the Overlord
    ("item_helm_of_the_undying", 0),  // Helm of the Undying
    ("item_holy_locket", 0),  // Holy Locket
    ("item_hood_of_defiance", 50),  // Hood of Defiance
    ("item_horizon", 0),
    ("item_horn_of_battle", 0),  // Horn of Battle
    ("item_horn_of_battle_greater", 0),  // Greater Horn of Battle
    ("item_hurricane_pike", 150),  // Hurricane Pike
    ("item_hydras_breath", 0),  // Hydra's Breath
    ("item_hyperstone", 0),  // Hyperstone
    ("item_idol_of_screeauk", 0),  // Idol of Scree'auk
    ("item_illusionsts_cape", 0),  // Illusionist's Cape
    ("item_imp_claw", 0),  // Imp Claw
    ("item_infused_raindrop", 0),  // Infused Raindrops
    ("item_invis_sword", 75),  // Shadow Blade
    ("item_iron_talon", 0),  // Iron Talon
    ("item_ironwood_tree", 0),  // Ironwood Tree
    ("item_javelin", 0),  // Javelin
    ("item_jidi_pollen_bag", 0),  // Jidi Pollen Bag
    ("item_kaya", 0),  // Kaya
    ("item_kaya_and_sange", 0),  // Kaya and Sange
    ("item_keen_optic", 0),  // Keen Optic
    ("item_kobold_cup", 40),  // Kobold Cup
    ("item_lance_of_pursuit", 0),  // Lance of Pursuit
    ("item_lesser_crit", 0),  // Crystalys
    ("item_lifesteal", 0),  // Morbid Mask
    ("item_light_collector", 0),  // Light Collector
    ("item_lotus_orb", 175),  // Lotus Orb
    ("item_madstone_bundle", 0),  // Madstone Bundle
    ("item_maelstrom", 0),  // Maelstrom
    ("item_mage_slayer", 0),  // Mage Slayer
    ("item_magic_stick", 0),  // Magic Stick
    ("item_magic_wand", 0),  // Magic Wand
    ("item_magnifying_monocle", 0),  // Magnifying Monocle
    ("item_mana_draught", 0),  // Mana Draught
    ("item_mango_tree", 0),  // Mango Tree
    ("item_manta", 125),  // Manta Style
    ("item_mantle", 0),  // Mantle of Intelligence
    ("item_martyrs_plate", 0),  // Martyr's Plate
    ("item_mask_of_madness", 25),  // Mask of Madness
    ("item_mechanical_arm", 0),
    ("item_medallion_of_courage", 30),  // Medallion of Courage
    ("item_mekansm", 100),  // Mekansm
    ("item_metamorphic_mandible", 0),  // Metamorphic Mandible
    ("item_meteor_hammer", 75),  // Meteor Hammer
    ("item_mind_breaker", 0),  // Mind Breaker
    ("item_miniboss_minion_summoner", 0),
    ("item_minotaur_horn", 0),  // Minotaur Horn
    ("item_mirror_shield", 0),  // Mirror Shield
    ("item_misericorde", 0),  // Brigand's Blade
    ("item_mithril_hammer", 0),  // Mithril Hammer
    ("item_mjollnir", 50),  // Mjollnir
    ("item_monkey_king_bar", 0),  // Monkey King Bar
    ("item_moon_shard", 0),  // Moon Shard
    ("item_muertas_gun", 160),  // Mercy & Grace
    ("item_mutation_tombstone", 0),  // Tombstone
    ("item_mysterious_hat", 0),  // Fairy's Trinket
    ("item_mystic_staff", 0),  // Mystic Staff
    ("item_necronomicon", 150),  // Necronomicon
    ("item_necronomicon_2", 150),  // Necronomicon
    ("item_necronomicon_3", 150),  // Necronomicon
    ("item_nemesis_curse", 0),  // Nemesis Curse
    ("item_nether_shawl", 0),  // Nether Shawl
    ("item_ninja_gear", 100),  // Ninja Gear
    ("item_nothl_sigil", 0),  // Nothl Sigil
    ("item_null_talisman", 0),  // Null Talisman
    ("item_nullifier", 0),  // Nullifier
    ("item_oblivion_staff", 0),  // Oblivion Staff
    ("item_occult_bracelet", 0),  // Occult Bracelet
    ("item_ocean_heart", 0),  // Ocean Heart
    ("item_octarine_core", 0),  // Octarine Core
    ("item_ofrenda", 0),  // Beloved Memory
    ("item_ofrenda_pledge", 0),  // Forebearer's Fortune
    ("item_ofrenda_shovel", 0),  // Scrying Shovel
    ("item_ogre_axe", 0),  // Ogre Axe
    ("item_ogre_seal_totem", 25),  // Ogre Seal Totem
    ("item_orb_of_corrosion", 0),  // Orb of Corrosion
    ("item_orb_of_destruction", 0),  // Orb of Destruction
    ("item_orb_of_frost", 0),  // Orb of Frost
    ("item_orb_of_venom", 0),  // Orb of Venom
    ("item_orchid", 125),  // Orchid Malevolence
    ("item_outworld_staff", 65),  // Outworld Staff
    ("item_overwhelming_blink", 0),  // Overwhelming Blink
    ("item_paintball", 25),  // Fae Grenade
    ("item_paladin_sword", 0),  // Paladin Sword
    ("item_panic_button", 0),  // Magic Lamp
    ("item_partisans_brand", 0),  // Partisan's Brand
    ("item_pavise", 60),  // Pavise
    ("item_penta_edged_sword", 0),  // Penta-Edged Sword
    ("item_pers", 0),  // Perseverance
    ("item_phase_boots", 0),  // Phase Boots
    ("item_philosophers_stone", 0),  // Philosopher's Stone
    ("item_phoenix_ash", 0),  // Phoenix Ash
    ("item_phylactery", 0),  // Phylactery
    ("item_pipe", 150),  // Pipe of Insight
    ("item_pirate_hat", 0),  // Pirate Hat
    ("item_platemail", 0),  // Platemail
    ("item_pocket_roshan", 0),  // Pocket Roshan
    ("item_pocket_tower", 0),  // Pocket Tower
    ("item_pogo_stick", 0),  // Tumbler's Toy
    ("item_point_booster", 0),  // Point Booster
    ("item_polliwog_charm", 0),  // Pollywog Charm
    ("item_poor_mans_shield", 0),  // Poor Man's Shield
    ("item_possessed_mask", 0),  // Possessed Mask
    ("item_power_treads", 0),  // Power Treads
    ("item_princes_knife", 0),  // Prince's Knife
    ("item_prophets_pendulum", 0),  // Prophet's Pendulum
    ("item_psychic_headband", 0),  // Psychic Headband
    ("item_pupils_gift", 0),  // Pupil's Gift
    ("item_pyrrhic_cloak", 0),  // Pyrrhic Cloak
    ("item_quarterstaff", 0),  // Quarterstaff
    ("item_quelling_blade", 0),  // Quelling Blade
    ("item_quickening_charm", 0),  // Quickening Charm
    ("item_quicksilver_amulet", 0),  // Quicksilver Amulet
    ("item_radiance", 0),  // Radiance
    ("item_rapier", 0),  // Divine Rapier
    ("item_rattlecage", 0),  // Rattlecage
    ("item_reaver", 0),  // Reaver
    ("item_red_ledger", 0),  // Red Ledger
    ("item_refresher", 325),  // Refresher Orb
    ("item_refresher_shard", 200),  // Refresher Shard
    ("item_relic", 0),  // Sacred Relic
    ("item_repair_kit", 0),  // Repair Kit
    ("item_revenants_brooch", 0),  // Revenant's Brooch
    ("item_riftshadow_prism", 0),  // Riftshadow Prism
    ("item_ring_of_aquila", 0),  // Ring of Aquila
    ("item_ring_of_basilius", 0),  // Ring of Basilius
    ("item_ring_of_health", 0),  // Ring of Health
    ("item_ring_of_protection", 0),  // Ring of Protection
    ("item_ring_of_regen", 0),  // Ring of Regen
    ("item_ring_of_tarrasque", 0),  // Ring of Tarrasque
    ("item_rippers_lash", 0),  // Ripper's Lash
    ("item_ristuls_devotion", 0),  // Ristul's Devotion
    ("item_river_painter", 0),  // River Vial: Chrome
    ("item_river_painter2", 0),  // River Vial: Dry
    ("item_river_painter3", 0),  // River Vial: Slime
    ("item_river_painter4", 0),  // River Vial: Oil
    ("item_river_painter5", 0),  // River Vial: Electrified
    ("item_river_painter6", 0),  // River Vial: Potion
    ("item_river_painter7", 0),  // River Vial: Blood
    ("item_robe", 0),  // Robe of the Magi
    ("item_rod_of_atos", 100),  // Rod of Atos
    ("item_roshans_banner", 0),  // Roshan's Banner
    ("item_royal_jelly", 0),  // Royal Jelly
    ("item_royale_with_cheese", 0),  // Block of Cheese
    ("item_safety_bubble", 0),  // Safety Bubble
    ("item_sange", 0),  // Sange
    ("item_sange_and_yasha", 0),  // Sange and Yasha
    ("item_satanic", 0),  // Satanic
    ("item_searing_signet", 0),  // Searing Signet
    ("item_seeds_of_serenity", 0),  // Seeds of Serenity
    ("item_seer_stone", 0),  // Seer Stone
    ("item_serrated_shiv", 0),  // Serrated Shiv
    ("item_shadow_amulet", 0),  // Shadow Amulet
    ("item_shawl", 0),  // Shawl
    ("item_sheepstick", 250),  // Scythe of Vyse
    ("item_shivas_guard", 75),  // Shiva's Guard
    ("item_silver_edge", 75),  // Silver Edge
    ("item_sisters_shroud", 0),  // Sister's Shroud
    ("item_skadi", 0),  // Eye of Skadi
    ("item_skypiercer", 50),  // Skypiercer
    ("item_slippers", 0),  // Slippers of Agility
    ("item_smoke_of_deceit", 0),  // Smoke of Deceit
    ("item_sobi_mask", 0),  // Sage's Mask
    ("item_solar_crest", 100),  // Solar Crest
    ("item_soul_booster", 0),  // Soul Booster
    ("item_soul_ring", 0),  // Soul Ring
    ("item_spark_of_courage", 0),  // Spark of Courage
    ("item_specialists_array", 0),  // Specialist's Array
    ("item_spell_prism", 0),  // Spell Prism
    ("item_spellslinger", 0),  // Spellslinger
    ("item_sphere", 0),  // Linken's Sphere
    ("item_spider_legs", 0),  // Spider Legs
    ("item_spirit_vessel", 0),  // Spirit Vessel
    ("item_splintmail", 0),  // Splintmail
    ("item_spy_gadget", 0),  // Telescope
    ("item_staff_of_wizardry", 0),  // Staff of Wizardry
    ("item_starforge", 175),  // Starforge
    ("item_stone_of_recall", 0),  // Stone of Recall
    ("item_stonefeather_satchel", 0),  // Stonefeather Satchel
    ("item_stormcrafter", 0),  // Stormcrafter
    ("item_stout_shield", 0),  // Stout Shield
    ("item_stygian_desolator", 0),  // Stygian Desolator
    ("item_super_blink", 0),
    ("item_swift_blink", 0),  // Swift Blink
    ("item_talisman_of_evasion", 0),  // Talisman of Evasion
    ("item_tango", 0),  // Tango
    ("item_tango_single", 0),  // Tango (Shared)
    ("item_the_leveller", 0),  // The Leveller
    ("item_third_eye", 0),  // Third Eye
    ("item_tiara_of_selemene", 0),  // Tiara of Selemene
    ("item_tidehunter_fish", 0),  // Leviathan's Fish
    ("item_tier1_token", 0),  // Tier 1 Token
    ("item_tier2_token", 0),  // Tier 2 Token
    ("item_tier3_token", 0),  // Tier 3 Token
    ("item_tier4_token", 0),  // Tier 4 Token
    ("item_tier5_token", 0),  // Tier 5 Token
    ("item_timeless_relic", 0),  // Timeless Relic
    ("item_titan_sliver", 0),  // Titan Sliver
    ("item_tome_of_aghanim", 0),  // Tome of Aghanim
    ("item_tome_of_knowledge", 0),  // Tome of Knowledge
    ("item_tpscroll", 75),  // Town Portal Scroll
    ("item_tranquil_boots", 0),  // Tranquil Boots
    ("item_travel_boots", 0),  // Boots of Travel
    ("item_travel_boots_2", 0),  // Boots of Travel 2
    ("item_trickster_cloak", 0),  // Trickster Cloak
    ("item_trident", 0),  // Trident
    ("item_trusty_shovel", 0),  // Trusty Shovel
    ("item_ultimate_orb", 0),  // Ultimate Orb
    ("item_ultimate_scepter", 0),  // Aghanim's Scepter
    ("item_ultimate_scepter_2", 0),  // Aghanim's Blessing
    ("item_ultimate_scepter_roshan", 0),  // Aghanim's Blessing - Roshan
    ("item_unrelenting_eye", 0),  // Unrelenting Eye
    ("item_unstable_wand", 0),  // Pig Pole
    ("item_unwavering_condition", 0),  // Unwavering Condition
    ("item_urn_of_shadows", 0),  // Urn of Shadows
    ("item_vambrace", 0),  // Vambrace
    ("item_vampire_fangs", 0),  // Vampire Fangs
    ("item_vanguard", 0),  // Vanguard
    ("item_veil_of_discord", 25),  // Veil of Discord
    ("item_vile_phial", 0),  // Vile Phial
    ("item_vindicators_axe", 0),  // Vindicator's Axe
    ("item_vitality_booster", 0),  // Vitality Booster
    ("item_vladmir", 0),  // Vladmir's Offering
    ("item_void_stone", 0),  // Void Stone
    ("item_voodoo_mask", 0),  // Voodoo Mask
    ("item_ward_dispenser", 0),  // Observer and Sentry Wards
    ("item_ward_observer", 0),  // Observer Ward
    ("item_ward_sentry", 0),  // Sentry Ward
    ("item_weighted_dice", 0),  // Weighted Dice
    ("item_whisper_of_the_dread", 0),  // Whisper of the Dread
    ("item_wind_lace", 0),  // Wind Lace
    ("item_wind_waker", 175),  // Wind Waker
    ("item_witch_blade", 0),  // Witch Blade
    ("item_witless_shako", 0),  // Witless Shako
    ("item_wizard_hat", 0),  // Wizard Hat
    ("item_woodland_striders", 0),  // Woodland Striders
    ("item_wraith_band", 0),  // Wraith Band
    ("item_wraith_pact", 100),  // Wraith Pact
    ("item_yasha", 0),  // Yasha
    ("item_yasha_and_kaya", 0),  // Yasha and Kaya
];

/// Per-level ability mana costs, keyed by GSI `ability.name`. Index with
/// `ability.level - 1`; see [`ability_mana_cost`] which does the clamping.
#[rustfmt::skip]
pub static ABILITY_MANA_COST_TABLE: &[(&str, &[u32])] = &[
    ("abaddon_aphotic_shield", &[110, 120, 130, 140]),  // Aphotic Shield
    ("abaddon_borrowed_time", &[0, 0, 0]),  // Borrowed Time
    ("abaddon_death_coil", &[50, 55, 60, 65]),  // Mist Coil
    ("abaddon_frostmourne", &[0]),  // Curse of Avernus
    ("abaddon_withering_mist", &[0]),  // Withering Mist
    ("abyssal_underlord_abyssal_horde", &[0]),  // Abyssal Horde
    ("abyssal_underlord_atrophy_aura", &[0]),  // Atrophy Aura
    ("abyssal_underlord_cancel_dark_rift", &[0]),
    ("abyssal_underlord_dark_portal", &[175]),  // Fiend's Gate
    ("abyssal_underlord_dark_rift", &[100, 200, 300]),
    ("abyssal_underlord_firestorm", &[110, 125, 140, 155]),  // Firestorm
    ("abyssal_underlord_pit_of_malice", &[110, 120, 130, 140]),  // Pit of Malice
    ("abyssal_underlord_portal_warp", &[0]),
    ("abyssal_underlord_raid_boss", &[0]),  // Invading Force
    ("alchemist_acid_spray", &[120]),  // Acid Spray
    ("alchemist_berserk_potion", &[75]),  // Berserk Potion
    ("alchemist_chemical_rage", &[50, 75, 100]),  // Chemical Rage
    ("alchemist_corrosive_weaponry", &[0]),  // Corrosive Weaponry
    ("alchemist_goblins_greed", &[0]),  // Greevil's Greed
    ("alchemist_unstable_concoction", &[100]),  // Unstable Concoction
    ("alchemist_unstable_concoction_throw", &[0]),  // Unstable Concoction Throw
    ("ancient_apparition_bone_chill", &[0]),  // Bone Chill
    ("ancient_apparition_chilling_touch", &[30]),  // Chilling Touch
    ("ancient_apparition_cold_feet", &[110, 115, 120, 125]),  // Cold Feet
    ("ancient_apparition_death_rime", &[0]),
    ("ancient_apparition_ice_age", &[200]),
    ("ancient_apparition_ice_blast", &[175]),  // Ice Blast
    ("ancient_apparition_ice_blast_release", &[0]),  // Release
    ("ancient_apparition_ice_vortex", &[40, 55, 70, 85]),  // Ice Vortex
    ("antimage_blink", &[60, 55, 50, 45]),  // Blink
    ("antimage_counterspell", &[50]),  // Counterspell
    ("antimage_counterspell_ally", &[50]),  // Counterspell Ally
    ("antimage_mana_break", &[0]),  // Mana Break
    ("antimage_mana_overload", &[50]),
    ("antimage_mana_thirst", &[0]),
    ("antimage_mana_void", &[100, 150, 200]),  // Mana Void
    ("antimage_persectur", &[0]),  // Persecutor
    ("antimage_spell_shield", &[0]),
    ("arc_warden_flux", &[75]),  // Flux
    ("arc_warden_magnetic_field", &[60, 70, 80, 90]),  // Magnetic Field
    ("arc_warden_runic_infusion", &[0]),  // Runic Infusion
    ("arc_warden_scepter", &[100]),
    ("arc_warden_spark_wraith", &[80]),  // Spark Wraith
    ("arc_warden_tempest_double", &[0]),  // Tempest Double
    ("arc_warden_tempest_recall", &[125]),
    ("axe_battle_hunger", &[50, 60, 70, 80]),  // Battle Hunger
    ("axe_berserkers_call", &[90, 100, 110, 120]),  // Berserker's Call
    ("axe_counter_helix", &[0]),  // Counter Helix
    ("axe_culling_blade", &[100, 125, 150]),  // Culling Blade
    ("axe_one_man_army", &[0]),  // One Man Army
    ("bane_brain_sap", &[105, 120, 135, 150]),  // Brain Sap
    ("bane_enfeeble", &[100, 110, 120, 130]),  // Enfeeble
    ("bane_fiends_grip", &[200, 300, 400]),  // Fiend's Grip
    ("bane_ichor_of_nyctasha", &[0]),  // Ichor of Nyctasha
    ("bane_nightmare", &[120, 130, 140, 150]),  // Nightmare
    ("bane_nightmare_end", &[0]),  // Nightmare End
    ("batrider_firefly", &[100]),  // Firefly
    ("batrider_flamebreak", &[110]),  // Flamebreak
    ("batrider_flaming_lasso", &[125, 150, 175]),  // Flaming Lasso
    ("batrider_smoldering_resin", &[0]),  // Smoldering Resin
    ("batrider_sticky_napalm", &[25]),  // Sticky Napalm
    ("batrider_sticky_napalm_application_damage", &[0]),  // APPLICATION DAMAGE:
    ("batrider_stoked", &[0]),
    ("beastmaster_boar_poison", &[0]),  // Poison
    ("beastmaster_call_of_the_wild", &[0]),
    ("beastmaster_drums_of_slom", &[0]),  // Drums of Slom
    ("beastmaster_greater_boar_poison", &[0]),  // Poison
    ("beastmaster_hawk_dive", &[0]),  // Dive Bomb
    ("beastmaster_hawk_invisibility", &[0]),
    ("beastmaster_hawk_perch", &[0]),
    ("beastmaster_inner_beast", &[0]),  // Inner Beast
    ("beastmaster_mark_of_the_beast", &[75]),
    ("beastmaster_primal_roar", &[100, 125, 150]),  // Primal Roar
    ("beastmaster_rugged", &[0]),  // Rugged
    ("beastmaster_summon_raptor", &[50]),  // Summon Raptors
    ("beastmaster_summon_raptor_attack", &[0]),
    ("beastmaster_summon_razorback", &[60]),  // Summon Razorback
    ("beastmaster_summon_thunderhide", &[100]),  // Summon Ancient Thunderhide
    ("beastmaster_thunderhide_drum", &[0]),  // Thunderhide Drum
    ("beastmaster_wild_axes", &[65]),  // Wild Axes
    ("bloodseeker_blood_bath", &[90, 100, 110, 120]),  // Blood Rite
    ("bloodseeker_blood_mist", &[0]),  // Blood Mist
    ("bloodseeker_bloodrage", &[0]),  // Bloodrage
    ("bloodseeker_rupture", &[125, 175, 225]),  // Rupture
    ("bloodseeker_sanguivore", &[0]),  // Sanguivore
    ("bloodseeker_thirst", &[0]),  // Thirst
    ("bounty_hunter_big_game_hunter", &[0]),  // Big Game Hunter
    ("bounty_hunter_cutpurse", &[0]),  // Cutpurse
    ("bounty_hunter_jinada", &[0]),  // Jinada
    ("bounty_hunter_lookout", &[30]),
    ("bounty_hunter_shuriken_toss", &[75, 85, 95, 105]),  // Shuriken Toss
    ("bounty_hunter_track", &[50]),  // Track
    ("bounty_hunter_wind_walk", &[50]),  // Shadow Walk
    ("bounty_hunter_wind_walk_ally", &[50]),  // Friendly Shadow
    ("brewmaster_cinder_brew", &[50, 60, 70, 80]),  // Cinder Brew
    ("brewmaster_drunken_brawler", &[0]),  // Drunken Brawler
    ("brewmaster_drunken_brawler_brew_up", &[0]),  // Drunken Brawler - Brew Up
    ("brewmaster_drunken_haze", &[25]),
    ("brewmaster_earth_hurl_boulder", &[0]),  // Hurl Boulder
    ("brewmaster_earth_pulverize", &[0]),  // Demolish
    ("brewmaster_earth_spell_immunity", &[0]),  // Earth Element
    ("brewmaster_fire_permanent_immolation", &[0]),  // Permanent Immolation
    ("brewmaster_fire_phase", &[0]),  // Permanent Phase
    ("brewmaster_fire_pull", &[50]),  // Cinder Dash
    ("brewmaster_liquid_courage", &[0]),  // Liquid Courage
    ("brewmaster_primal_companion", &[50]),  // Primal Companion
    ("brewmaster_primal_split", &[150, 200, 250, 250]),  // Primal Split
    ("brewmaster_primal_split_cancel", &[0]),  // Primal Split Cancel
    ("brewmaster_primal_split_earth_brewling", &[0]),  // Earth Brewling
    ("brewmaster_primal_split_fire_brewling", &[0]),  // Fire Brewling
    ("brewmaster_primal_split_storm_brewling", &[0]),  // Storm Brewling
    ("brewmaster_storm_cyclone", &[150, 150, 150, 150]),  // Cyclone
    ("brewmaster_storm_dispel_magic", &[75, 75, 75, 75]),  // Dispel Magic
    ("brewmaster_storm_wind_walk", &[75, 75, 75, 75]),  // Wind Walk
    ("brewmaster_thunder_clap", &[100]),  // Thunder Clap
    ("brewmaster_void_astral_pulse", &[0]),
    ("bristleback_brawlers_grit", &[0]),  // Brawler's Grit
    ("bristleback_bristleback", &[0]),  // Bristleback
    ("bristleback_hairball", &[60]),  // Hairball
    ("bristleback_prickly", &[0]),  // Prickly
    ("bristleback_quill_spray", &[35, 35, 35, 35]),  // Quill Spray
    ("bristleback_viscous_nasal_goo", &[10, 15, 20, 25]),  // Viscous Nasal Goo
    ("bristleback_warpath", &[0]),  // Warpath
    ("broodmother_incapacitating_bite", &[0]),  // Incapacitating Bite
    ("broodmother_insatiable_hunger", &[70]),  // Insatiable Hunger
    ("broodmother_poison_sting", &[0]),  // Poison Sting
    ("broodmother_silken_bola", &[70, 75, 80, 85]),
    ("broodmother_spawn_spiderite", &[0]),  // Spawn Spiderling
    ("broodmother_spawn_spiderlings", &[100]),  // Spawn Spiderlings
    ("broodmother_spiders_milk", &[0]),  // Spider's Milk
    ("broodmother_spin_web", &[40]),  // Spin Web
    ("broodmother_spin_web_destroy", &[0]),  // Destroy Spin Web
    ("broodmother_sticky_snare", &[70]),  // Spinner's Snare
    ("centaur_double_edge", &[0, 0, 0, 0]),  // Double Edge
    ("centaur_hoof_stomp", &[100, 110, 120, 130]),  // Hoof Stomp
    ("centaur_horsepower", &[0]),  // Horsepower
    ("centaur_khan_endurance_aura", &[0]),  // Swiftness Aura
    ("centaur_khan_war_stomp", &[50]),  // War Stomp
    ("centaur_mount", &[75]),  // Hitch A Ride
    ("centaur_overrun", &[50]),
    ("centaur_rawhide", &[0]),  // Rawhide
    ("centaur_return", &[0]),  // Retaliate
    ("centaur_stampede", &[150, 200, 250]),  // Stampede
    ("centaur_work_horse", &[75]),  // Work Horse
    ("chaos_knight_armageddon", &[50]),  // Ahead, Armageddon
    ("chaos_knight_armageddons_flank", &[25]),  // Armageddon's Flank
    ("chaos_knight_armageddons_flank_end", &[0]),  // Heel, Armageddon
    ("chaos_knight_chaos_bolt", &[110]),  // Chaos Bolt
    ("chaos_knight_chaos_strike", &[0]),  // Chaos Strike
    ("chaos_knight_fundamental_forging", &[0]),  // Fundamental Forging
    ("chaos_knight_phantasm", &[100, 200, 300]),  // Phantasm
    ("chaos_knight_phantasmagoria", &[0]),  // Phantasmagoria
    ("chaos_knight_reality_rift", &[50]),  // Reality Rift
    ("chaos_knight_reins_of_chaos", &[0]),  // Reins of Chaos
    ("chen_anoint", &[0]),  // Anoint Zealot
    ("chen_divine_favor", &[75]),  // Divine Favor
    ("chen_hand_of_god", &[200, 300, 400]),  // Hand of God
    ("chen_holy_persuasion", &[110, 130, 150, 170]),  // Holy Persuasion
    ("chen_innate_check_for_team_change", &[0]),
    ("chen_martyrdom", &[0]),  // Martyrdom
    ("chen_penitence", &[80, 90, 100, 110]),  // Penitence
    ("chen_test_of_faith", &[60, 80, 100, 120]),
    ("chen_test_of_faith_teleport", &[200, 200, 200, 200]),
    ("chen_zealot", &[0]),  // Zealot
    ("clinkz_bone_and_arrow", &[0]),  // Bone and Arrow
    ("clinkz_burning_army", &[150]),  // Burning Army
    ("clinkz_burning_barrage", &[75]),  // Burning Barrage
    ("clinkz_death_pact", &[50]),  // Death Pact
    ("clinkz_empty1", &[0]),
    ("clinkz_infernal_shred", &[0]),  // Infernal Shred
    ("clinkz_scepter", &[0]),
    ("clinkz_searing_arrows", &[10]),  // Searing Arrows
    ("clinkz_strafe", &[55, 60, 65, 70]),  // Strafe
    ("clinkz_tar_bomb", &[45, 50, 55, 60]),  // Tar Bomb
    ("clinkz_wind_walk", &[80, 105, 130]),  // Skeleton Walk
    ("crystal_maiden_brilliance_aura", &[0]),  // Arcane Aura
    ("crystal_maiden_crystal_clone", &[150]),  // Crystal Clone
    ("crystal_maiden_crystal_nova", &[115, 135, 155, 175]),  // Crystal Nova
    ("crystal_maiden_freezing_field", &[250, 425, 600]),  // Freezing Field
    ("crystal_maiden_freezing_field_stop", &[0]),  // Stop Freezing Field
    ("crystal_maiden_frostbite", &[125, 135, 145, 155]),  // Frostbite
    ("crystal_maiden_glacial_guard", &[0]),  // Glacial Guard
    ("crystal_maiden_let_it_go", &[100]),
    ("dark_seer_aggrandize", &[0]),  // Quick Wit
    ("dark_seer_heart_of_battle", &[0]),  // Heart of Battle
    ("dark_seer_ion_shell", &[100, 110, 120, 130]),  // Ion Shell
    ("dark_seer_mental_fortitude", &[0]),  // Mental Fortitude
    ("dark_seer_normal_punch", &[0]),  // Normal Punch
    ("dark_seer_quick_wit", &[0]),  // Quick Wit
    ("dark_seer_surge", &[50]),  // Surge
    ("dark_seer_vacuum", &[60, 90, 120, 150]),  // Vacuum
    ("dark_seer_wall_of_replica", &[125, 250, 375]),  // Wall of Replica
    ("dark_willow_bedlam", &[100, 150, 200]),  // Bedlam
    ("dark_willow_bramble_maze", &[100, 120, 140, 160]),  // Bramble Maze
    ("dark_willow_cursed_crown", &[80, 90, 100, 110]),  // Cursed Crown
    ("dark_willow_pixie_dust", &[0]),  // Pixie Dust
    ("dark_willow_shadow_realm", &[80, 90, 100, 110]),  // Shadow Realm
    ("dark_willow_terrorize", &[150]),  // Terrorize
    ("dawnbreaker_break_of_dawn", &[0]),  // Break of Dawn
    ("dawnbreaker_celestial_hammer", &[100, 110, 120, 130]),  // Celestial Hammer
    ("dawnbreaker_converge", &[0]),  // Converge
    ("dawnbreaker_fire_wreath", &[110]),  // Starbreaker
    ("dawnbreaker_land", &[0]),
    ("dawnbreaker_luminosity", &[0]),  // Luminosity
    ("dawnbreaker_solar_guardian", &[150, 200, 250]),  // Solar Guardian
    ("dawnbreaker_unbreakable", &[0]),
    ("dazzle_bad_juju", &[0]),
    ("dazzle_good_juju", &[0]),
    ("dazzle_innate_weave", &[0]),  // Weave
    ("dazzle_nothl_boon", &[0]),  // Nothl Boon
    ("dazzle_nothl_projection", &[100, 150, 200]),  // Nothl Projection
    ("dazzle_nothl_projection_end", &[0]),  // End Projection
    ("dazzle_poison_touch", &[125, 130, 135, 140]),  // Poison Touch
    ("dazzle_rain_of_vermin", &[150]),
    ("dazzle_shadow_wave", &[90]),  // Shadow Wave
    ("dazzle_shallow_grave", &[90, 100, 110, 120]),  // Shallow Grave
    ("dazzle_through_the_realm", &[0]),  // Through the Realm
    ("dazzle_weave", &[100, 100, 100]),
    ("death_prophet_carrion_swarm", &[80, 90, 100, 110]),  // Crypt Swarm
    ("death_prophet_exorcism", &[200, 300, 400]),  // Exorcism
    ("death_prophet_innate_healing_mastery", &[0]),
    ("death_prophet_mourning_ritual", &[0]),  // Mourning Ritual
    ("death_prophet_silence", &[80, 90, 100, 110]),  // Silence
    ("death_prophet_spirit_collector", &[0]),  // Spirit Collector
    ("death_prophet_spirit_siphon", &[60]),  // Spirit Siphon
    ("death_prophet_witchcraft", &[0]),  // Witchcraft
    ("disruptor_electromagnetic_repulsion", &[0]),  // Electromagnetic Repulsion
    ("disruptor_glimpse", &[70, 85, 100, 115]),  // Glimpse
    ("disruptor_innate_kinetic_circle", &[0]),
    ("disruptor_kinetic_fence", &[50]),  // Kinetic Fence
    ("disruptor_kinetic_field", &[70]),  // Kinetic Field
    ("disruptor_static_storm", &[125, 175, 225]),  // Static Storm
    ("disruptor_thunder_strike", &[115, 120, 125, 130]),  // Thunder Strike
    ("doom_bringer_devils_bargain", &[0]),  // Devil's Bargain
    ("doom_bringer_devour", &[66]),  // Devour
    ("doom_bringer_doom", &[150, 200, 250]),  // Doom
    ("doom_bringer_empty1", &[0]),  // Devoured Ability
    ("doom_bringer_empty2", &[0]),  // Devoured Ability
    ("doom_bringer_infernal_blade", &[35]),  // Infernal Blade
    ("doom_bringer_lvl_pain", &[0]),  // Lvl ? Pain
    ("doom_bringer_scorched_earth", &[60, 70, 80, 90]),  // Scorched Earth
    ("dragon_knight_breathe_fire", &[90, 95, 100, 105]),  // Breathe Fire
    ("dragon_knight_dragon_blood", &[0]),  // Dragon Blood
    ("dragon_knight_dragon_tail", &[70, 80, 90, 100]),  // Dragon Tail
    ("dragon_knight_elder_dragon_form", &[50]),  // Elder Dragon Form
    ("dragon_knight_fireball", &[80]),  // Fireball
    ("dragon_knight_wyrms_wrath", &[0]),  // Wyrm's Wrath
    ("drow_ranger_creep_rally", &[0]),
    ("drow_ranger_frost_arrows", &[9, 10, 11, 12]),  // Frost Arrows
    ("drow_ranger_glacier", &[50]),  // Glacier
    ("drow_ranger_marksmanship", &[0]),  // Marksmanship
    ("drow_ranger_multishot", &[70, 85, 100, 115]),  // Multishot
    ("drow_ranger_silence", &[90, 90, 90, 90]),
    ("drow_ranger_trueshot", &[0]),  // Precision Aura
    ("drow_ranger_wave_of_silence", &[55]),  // Gust
    ("earth_spirit_boulder_smash", &[100]),  // Boulder Smash
    ("earth_spirit_geomagnetic_grip", &[75]),  // Geomagnetic Grip
    ("earth_spirit_magnetize", &[100]),  // Magnetize
    ("earth_spirit_petrify", &[150]),  // Enchant Remnant
    ("earth_spirit_rolling_boulder", &[50]),  // Rolling Boulder
    ("earth_spirit_stone_caller", &[0]),  // Stone Remnant
    ("earthshaker_aftershock", &[0]),  // Aftershock
    ("earthshaker_echo_slam", &[150, 200, 250]),  // Echo Slam
    ("earthshaker_enchant_totem", &[45, 55, 65, 75]),  // Enchant Totem
    ("earthshaker_fissure", &[115, 120, 125, 130]),  // Fissure
    ("earthshaker_slugger", &[0]),  // Slugger
    ("earthshaker_spirit_cairn", &[0]),  // Spirit Cairn
    ("elder_titan_ancestral_spirit", &[80, 90, 100, 110]),  // Astral Spirit
    ("elder_titan_earth_splitter", &[125, 175, 225]),  // Earth Splitter
    ("elder_titan_echo_stomp", &[100]),  // Echo Stomp
    ("elder_titan_echo_stomp_spirit", &[0, 0, 0, 0]),  // Echo Stomp
    ("elder_titan_fundamental_fury", &[50, 65, 80, 95]),
    ("elder_titan_fundamental_fury_spirit", &[0]),
    ("elder_titan_momentum", &[0]),  // Momentum
    ("elder_titan_move_spirit", &[0]),  // Move Astral Spirit
    ("elder_titan_natural_order", &[0]),  // Natural Order
    ("elder_titan_natural_order_spirit", &[0]),  // Natural Order
    ("elder_titan_return_spirit", &[0]),  // Return Astral Spirit
    ("elder_titan_tip_the_scales", &[0]),  // Tip the Scales
    ("ember_spirit_activate_fire_remnant", &[100, 125, 150]),  // Activate Fire Remnant
    ("ember_spirit_fire_remnant", &[0]),  // Fire Remnant
    ("ember_spirit_flame_guard", &[65, 80, 95, 110]),  // Flame Guard
    ("ember_spirit_immolation", &[0]),  // Immolation
    ("ember_spirit_searing_chains", &[80, 90, 100, 110]),  // Searing Chains
    ("ember_spirit_sleight_of_fist", &[60]),  // Sleight of Fist
    ("enchantress_bunny_hop", &[60]),  // Sproink
    ("enchantress_enchant", &[70]),  // Enchant
    ("enchantress_impetus", &[40, 45, 50, 55]),  // Impetus
    ("enchantress_little_friends", &[75]),  // Little Friends
    ("enchantress_natures_attendants", &[140]),  // Nature's Attendants
    ("enchantress_rabblerouser", &[0]),  // Rabble-Rouser
    ("enchantress_untouchable", &[0]),  // Untouchable
    ("enigma_black_hole", &[300, 400, 500]),  // Black Hole
    ("enigma_demonic_conversion", &[70, 80, 90, 100]),  // Demonic Summoning
    ("enigma_event_horizon", &[0]),  // Event Horizon
    ("enigma_gravity_well", &[0]),  // Gravity Well
    ("enigma_malefice", &[100, 110, 120, 130]),  // Malefice
    ("enigma_midnight_pulse", &[65, 90, 115, 140]),  // Midnight Pulse
    ("enigma_splitting_image", &[0]),  // Splitting Image
    ("faceless_void_backtrack", &[0]),
    ("faceless_void_chronosphere", &[125, 200, 275]),  // Chronosphere
    ("faceless_void_distortion_field", &[0]),  // Distortion Field
    ("faceless_void_time_dilation", &[90]),  // Time Dilation
    ("faceless_void_time_lock", &[0]),  // Time Lock
    ("faceless_void_time_walk", &[40]),  // Time Walk
    ("faceless_void_time_walk_reverse", &[0]),  // Reverse Time Walk
    ("faceless_void_time_zone", &[0]),  // Time Zone
    ("furion_arboreal_might", &[80, 90, 100, 110]),
    ("furion_curse_of_the_forest", &[0]),  // Curse of the Oldgrowth
    ("furion_fecundity", &[0]),
    ("furion_force_of_nature", &[85, 90, 95, 100]),  // Nature's Call
    ("furion_greater_sprout", &[130]),
    ("furion_hedgerow", &[80, 70, 60, 50]),
    ("furion_natures_profit", &[0]),  // Nature's Profit
    ("furion_spirit_of_the_forest", &[0]),  // Spirit of the Forest
    ("furion_sprout", &[70, 80, 90, 100]),  // Sprout
    ("furion_summon_fey", &[75]),
    ("furion_teleportation", &[50, 60, 70, 80]),  // Teleportation
    ("furion_wrath_of_nature", &[130, 160, 190]),  // Wrath of Nature
    ("grimstroke_dark_artistry", &[100, 110, 120, 130]),  // Stroke of Fate
    ("grimstroke_dark_portrait", &[200]),  // Dark Portrait
    ("grimstroke_ink_creature", &[80, 100, 120, 140]),  // Phantom's Embrace
    ("grimstroke_ink_over", &[125]),
    ("grimstroke_ink_trail", &[0]),  // Ink Trail
    ("grimstroke_return", &[0]),  // Ink Explosion
    ("grimstroke_soul_chain", &[150, 200, 250]),  // Soulbind
    ("grimstroke_spirit_walk", &[120, 130, 140, 150]),  // Ink Swell
    ("gyrocopter_afterburner", &[0]),  // Afterburner
    ("gyrocopter_call_down", &[150, 200, 250]),  // Call Down
    ("gyrocopter_chop_shop", &[0]),  // Chop Shop
    ("gyrocopter_flak_cannon", &[50, 60, 70, 80]),  // Flak Cannon
    ("gyrocopter_homing_missile", &[120]),  // Homing Missile
    ("gyrocopter_lock_on", &[0]),
    ("gyrocopter_rocket_barrage", &[75]),  // Rocket Barrage
    ("gyrocopter_side_gunner", &[0]),
    ("gyrocopter_side_gunner_spawn_ability", &[0]),  // Side Gunner
    ("hoodwink_acorn_shot", &[70, 80, 90, 100]),  // Acorn Shot
    ("hoodwink_bushwhack", &[90, 100, 110, 120]),  // Bushwhack
    ("hoodwink_decoy", &[60]),  // Decoy
    ("hoodwink_forager", &[0]),
    ("hoodwink_hunters_boomerang", &[125]),  // Hunter's Boomerang
    ("hoodwink_mistwoods_wayfarer", &[0]),  // Mistwoods Wayfarer
    ("hoodwink_scurry", &[35]),  // Scurry
    ("hoodwink_sharpshooter", &[150, 200, 250]),  // Sharpshooter
    ("hoodwink_sharpshooter_release", &[0]),  // End Sharpshooter
    ("hoodwink_tomokan_tracker", &[0]),  // Tomo'kan Tracker
    ("huskar_berserkers_blood", &[0]),  // Berserker's Blood
    ("huskar_blood_magic", &[0]),  // Blood Magic
    ("huskar_burning_spear", &[0, 0, 0, 0]),  // Burning Spear
    ("huskar_inner_fire", &[0]),  // Inner Fire
    ("huskar_inner_vitality", &[140, 150, 160, 170]),
    ("huskar_life_break", &[0, 0, 0]),  // Life Break
    ("invoker_alacrity", &[75]),  // Alacrity
    ("invoker_alacrity_ad", &[75]),  // Alacrity (AD)
    ("invoker_attribute_bonus", &[0]),  // Attribute Bonus
    ("invoker_chaos_meteor", &[200]),  // Chaos Meteor
    ("invoker_chaos_meteor_ad", &[200]),  // Chaos Meteor (AD)
    ("invoker_cold_snap", &[100]),  // Cold Snap
    ("invoker_cold_snap_ad", &[100]),  // Cold Snap (AD)
    ("invoker_deafening_blast", &[250]),  // Deafening Blast
    ("invoker_deafening_blast_ad", &[250]),  // Deafening Blast
    ("invoker_emp", &[125]),  // E.M.P.
    ("invoker_emp_ad", &[125]),  // E.M.P. (AD)
    ("invoker_empty1", &[0]),  // Invoked Spell
    ("invoker_empty2", &[0]),  // Invoked Spell
    ("invoker_exort", &[0]),  // Exort
    ("invoker_forge_spirit", &[75]),  // Forge Spirit
    ("invoker_forge_spirit_ad", &[75]),  // Forge Spirit (AD)
    ("invoker_ghost_walk", &[175]),  // Ghost Walk
    ("invoker_ghost_walk_ad", &[175]),  // Ghost Walk (AD)
    ("invoker_ice_wall", &[125]),  // Ice Wall
    ("invoker_ice_wall_ad", &[125]),  // Ice Wall (AD)
    ("invoker_invoke", &[0]),  // Invoke
    ("invoker_mastermind", &[0]),  // Mastermind
    ("invoker_quas", &[0]),  // Quas
    ("invoker_sun_strike", &[175]),  // Sun Strike
    ("invoker_sun_strike_ad", &[175]),  // Sun Strike (AD)
    ("invoker_tornado", &[140]),  // Tornado
    ("invoker_tornado_ad", &[140]),  // Tornado (AD)
    ("invoker_wex", &[0]),  // Wex
    ("jakiro_double_trouble", &[0]),  // Double Trouble
    ("jakiro_dual_breath", &[135, 150, 165, 180]),  // Dual Breath
    ("jakiro_ice_path", &[100]),  // Ice Path
    ("jakiro_ice_path_detonate", &[0]),  // Destroy Ice Path
    ("jakiro_liquid_fire", &[20]),  // Liquid Fire
    ("jakiro_liquid_ice", &[20]),  // Liquid Frost
    ("jakiro_macropyre", &[225, 325, 425]),  // Macropyre
    ("juggernaut_blade_dance", &[0]),  // Blade Dance
    ("juggernaut_blade_fury", &[110]),  // Blade Fury
    ("juggernaut_bladeform", &[0]),  // Bladeform
    ("juggernaut_duelist", &[0]),  // Duelist
    ("juggernaut_healing_ward", &[120]),  // Healing Ward
    ("juggernaut_innate_mask_crit_lifesteal", &[0]),
    ("juggernaut_innate_mask_spin_crit", &[0]),
    ("juggernaut_omni_slash", &[200, 275, 350]),  // Omnislash
    ("juggernaut_swift_slash", &[150]),  // Swiftslash
    ("juggernaut_trinity", &[60]),
    ("juggernaut_vaulted_strike", &[60]),
    ("keeper_of_the_light_blinding_light", &[120, 130, 140, 150]),  // Blinding Light
    ("keeper_of_the_light_bright_speed", &[0]),  // Bright Speed
    ("keeper_of_the_light_chakra_magic", &[0]),  // Chakra Magic
    ("keeper_of_the_light_empty1", &[0]),
    ("keeper_of_the_light_empty2", &[0]),
    ("keeper_of_the_light_illuminate", &[100, 125, 150, 175]),  // Illuminate
    ("keeper_of_the_light_illuminate_end", &[0]),  // Release Illuminate
    ("keeper_of_the_light_mana_leak", &[160]),
    ("keeper_of_the_light_mana_magnifier", &[0]),
    ("keeper_of_the_light_radiant_bind", &[120]),  // Solar Bind
    ("keeper_of_the_light_recall", &[150]),  // Recall
    ("keeper_of_the_light_special_reserve", &[0]),  // Special Reserve
    ("keeper_of_the_light_spirit_form", &[75, 125, 175]),  // Spirit Form
    ("keeper_of_the_light_will_o_wisp", &[150]),  // Will-O-Wisp
    ("kez_echo_slash", &[75, 90, 105, 120]),  // Echo Slash
    ("kez_falcon_rush", &[85, 90, 95, 100]),  // Falcon Rush
    ("kez_falcon_rush_ad", &[85, 90, 95, 100]),
    ("kez_grappling_claw", &[40]),  // Grappling Claw
    ("kez_kazurai_katana", &[40]),  // Kazurai Katana
    ("kez_raptor_dance", &[100, 125, 150]),  // Raptor Dance
    ("kez_ravens_veil", &[100, 125, 150]),  // Raven's Veil
    ("kez_ravens_veil_ad", &[100, 125, 150]),
    ("kez_shadowhawk_passive", &[0]),
    ("kez_shodo_sai", &[30, 20, 10, 0]),  // Shodo Sai
    ("kez_shodo_sai_ad", &[30, 20, 10, 0]),
    ("kez_shodo_sai_parry_cancel", &[0]),  // Cancel
    ("kez_switch_weapons", &[0]),  // Switch Discipline
    ("kez_talon_toss", &[60, 65, 70, 75]),  // Talon Toss
    ("kez_talon_toss_ad", &[60, 65, 70, 75]),
    ("kunkka_admirals_rum", &[0]),  // Admiral's Rum
    ("kunkka_ghostship", &[125, 175, 225]),  // Ghostship
    ("kunkka_return", &[0]),  // Return
    ("kunkka_tidal_wave", &[75]),  // Tidal Wave
    ("kunkka_tidebringer", &[0]),  // Tidebringer
    ("kunkka_torrent", &[90]),  // Torrent
    ("kunkka_torrent_storm", &[275]),
    ("kunkka_x_marks_the_spot", &[50]),  // X Marks the Spot
    ("largo_amphibian_rhapsody", &[0]),  // Amphibian Rhapsody
    ("largo_catchy_lick", &[80, 85, 90, 95]),  // Catchy Lick
    ("largo_croak_of_genius", &[40]),  // Croak of Genius
    ("largo_encore", &[0]),  // Encore
    ("largo_frogstomp", &[85, 95, 105, 115]),  // Frogstomp
    ("largo_song_double_time", &[25, 35, 45]),  // Hotfeet Hustle
    ("largo_song_fight_song", &[25, 35, 45]),  // Bullbelly Blitz
    ("largo_song_good_vibrations", &[25, 35, 45]),  // Island Elixir
    ("legion_commander_duel", &[80, 100, 120]),  // Duel
    ("legion_commander_intimidate", &[0]),
    ("legion_commander_moment_of_courage", &[0]),  // Moment of Courage
    ("legion_commander_outfight_them", &[0]),  // Outfight Them!
    ("legion_commander_overwhelming_odds", &[90, 105, 120, 135]),  // Overwhelming Odds
    ("legion_commander_press_the_attack", &[90]),  // Press The Attack
    ("leshrac_chronoptic_nourishment", &[0]),
    ("leshrac_defilement", &[0]),  // Defilement
    ("leshrac_diabolic_edict", &[90, 120, 150, 180]),  // Diabolic Edict
    ("leshrac_greater_lightning_storm", &[125]),  // Nihilism
    ("leshrac_lightning_storm", &[80, 100, 120, 140]),  // Lightning Storm
    ("leshrac_pulse_nova", &[50, 60, 70]),  // Pulse Nova
    ("leshrac_split_earth", &[80, 100, 120, 140]),  // Split Earth
    ("lich_chain_frost", &[180, 300, 420]),  // Chain Frost
    ("lich_dark_sorcery", &[0]),
    ("lich_death_charge", &[0]),  // Sacrifice
    ("lich_frost_armor", &[50, 50, 50, 50]),
    ("lich_frost_aura", &[0]),
    ("lich_frost_nova", &[105, 120, 135, 150]),  // Frost Blast
    ("lich_frost_shield", &[100, 110, 120, 130]),  // Frost Shield
    ("lich_ice_spire", &[150]),  // Ice Spire
    ("lich_sinister_gaze", &[25]),  // Sinister Gaze
    ("life_stealer_assimilate", &[50]),
    ("life_stealer_assimilate_eject", &[0]),
    ("life_stealer_consume", &[0]),  // Consume
    ("life_stealer_control", &[0]),
    ("life_stealer_corpse_eater", &[0]),
    ("life_stealer_empty_1", &[0]),
    ("life_stealer_empty_2", &[0]),
    ("life_stealer_empty_3", &[0]),
    ("life_stealer_empty_4", &[0]),
    ("life_stealer_feast", &[0]),  // Feast
    ("life_stealer_ghoul_frenzy", &[0]),  // Ghoul Frenzy
    ("life_stealer_infest", &[100, 125, 150]),  // Infest
    ("life_stealer_open_wounds", &[90]),  // Open Wounds
    ("life_stealer_rage", &[80, 100, 120, 140]),  // Rage
    ("life_stealer_unfettered", &[80, 100, 120, 140]),  // Unfettered
    ("lina_combustion", &[0]),  // Combustion
    ("lina_dragon_slave", &[90, 100, 110, 120]),  // Dragon Slave
    ("lina_fiery_soul", &[0]),  // Fiery Soul
    ("lina_flame_cloak", &[50]),  // Flame Cloak
    ("lina_laguna_blade", &[150, 300, 450]),  // Laguna Blade
    ("lina_light_strike_array", &[100, 110, 120, 130]),  // Light Strike Array
    ("lina_slow_burn", &[0]),  // Slow Burn
    ("lion_finger_of_death", &[200, 400, 600]),  // Finger of Death
    ("lion_impale", &[90, 110, 130, 150]),  // Earth Spike
    ("lion_mana_drain", &[0]),  // Mana Drain
    ("lion_to_hell_and_back", &[0]),  // To Hell and Back
    ("lion_voodoo", &[110, 140, 170, 200]),  // Hex
    ("lone_druid_bear_necessities", &[0]),  // Bear Necessities
    ("lone_druid_entangle", &[60]),  // Entangle
    ("lone_druid_gift_bearer", &[0]),  // Gift Bearer
    ("lone_druid_rabid", &[50, 50, 50, 50]),
    ("lone_druid_savage_roar", &[50]),  // Savage Roar
    ("lone_druid_savage_roar_bear", &[50]),  // Savage Roar
    ("lone_druid_spirit_bear", &[100]),  // Summon Spirit Bear
    ("lone_druid_spirit_bear_defender", &[0]),
    ("lone_druid_spirit_bear_demolish", &[0]),  // Demolish
    ("lone_druid_spirit_bear_entangle", &[0]),  // Entangling Claws
    ("lone_druid_spirit_bear_fetch", &[75]),  // Fetch
    ("lone_druid_spirit_bear_return", &[0]),  // Return
    ("lone_druid_spirit_bear_spirit_link", &[0]),  // Spirit Link
    ("lone_druid_spirit_bear_summon_druid", &[100]),  // Summon Lone Druid
    ("lone_druid_spirit_link", &[0]),  // Spirit Link
    ("lone_druid_true_form", &[80]),  // True Form
    ("lone_druid_true_form_battle_cry", &[50, 50, 50]),
    ("lone_druid_true_form_druid", &[25, 25, 25]),
    ("luna_eclipse", &[150, 200, 250]),  // Eclipse
    ("luna_lucent_beam", &[90, 100, 110, 120]),  // Lucent Beam
    ("luna_lunar_blessing", &[0]),  // Lunar Blessing
    ("luna_lunar_grace", &[40]),
    ("luna_lunar_orbit", &[65, 70, 75, 80]),  // Lunar Orbit
    ("luna_moon_glaive", &[0]),  // Moon Glaives
    ("lycan_apex_predator", &[0]),  // Apex Predator
    ("lycan_feral_impulse", &[0]),  // Feral Impulse
    ("lycan_howl", &[40]),  // Howl
    ("lycan_pack_hunter", &[0]),  // Pack Hunter
    ("lycan_shapeshift", &[100]),  // Shapeshift
    ("lycan_summon_wolves", &[115, 120, 125, 130]),  // Summon Wolves
    ("lycan_summon_wolves_bash", &[0]),
    ("lycan_summon_wolves_critical_strike", &[0]),  // Cripple
    ("lycan_summon_wolves_hamstring", &[0]),  // Hamstring
    ("lycan_summon_wolves_hightail", &[0]),  // Hightail
    ("lycan_summon_wolves_invisibility", &[0]),  // Invisibility
    ("lycan_wolf_bite", &[150]),  // Wolf Bite
    ("magnataur_empower", &[45, 55, 65, 75]),  // Empower
    ("magnataur_greater_shockwave", &[100]),
    ("magnataur_horn_toss", &[100]),  // Horn Toss
    ("magnataur_reverse_polarity", &[150, 225, 300]),  // Reverse Polarity
    ("magnataur_shockwave", &[85, 90, 95, 100]),  // Shockwave
    ("magnataur_skewer", &[80, 80, 80, 80]),  // Skewer
    ("magnataur_solid_core", &[0]),  // Solid Core
    ("marci_bodyguard", &[60]),  // Bodyguard
    ("marci_companion_run", &[70, 80, 90, 100]),  // Rebound
    ("marci_grapple", &[80]),  // Dispose
    ("marci_guardian", &[45, 40, 35, 30]),
    ("marci_special_delivery", &[0]),  // Special Delivery
    ("marci_unleash", &[100, 125, 150]),  // Unleash
    ("mars_arena_of_blood", &[150, 200, 250]),  // Arena Of Blood
    ("mars_bulwark", &[0]),  // Bulwark
    ("mars_dauntless", &[0]),  // Dauntless
    ("mars_gods_rebuke", &[90]),  // God's Rebuke
    ("mars_immovable", &[0]),  // Immovable
    ("mars_spear", &[90, 100, 110, 120]),  // Spear of Mars
    ("medusa_cold_blooded", &[0]),  // Cold Blooded
    ("medusa_gorgon_grasp", &[65, 85, 105, 125]),  // Gorgon's Grasp
    ("medusa_mana_shield", &[0]),  // Mana Shield
    ("medusa_mystic_snake", &[80, 95, 110, 125]),  // Mystic Snake
    ("medusa_split_shot", &[0]),  // Split Shot
    ("medusa_stone_gaze", &[250]),  // Stone Gaze
    ("medusa_undulation", &[0]),  // Undulation
    ("meepo_divided_we_stand", &[0]),  // Divided We Stand
    ("meepo_earthbind", &[70, 80, 90, 100]),  // Earthbind
    ("meepo_fling_release", &[0]),  // Fling
    ("meepo_geomancy", &[0]),  // Geomancy
    ("meepo_megameepo", &[0]),  // MegaMeepo
    ("meepo_megameepo_fling", &[0]),  // MegaMeepo Fling
    ("meepo_pack_rat", &[0]),  // Pack Rat
    ("meepo_petrify", &[125]),  // Dig
    ("meepo_poof", &[80]),  // Poof
    ("meepo_ransack", &[0]),  // Ransack
    ("meepo_sticky_fingers", &[0]),  // Sticky Fingers
    ("mirana_arrow", &[90]),  // Sacred Arrow
    ("mirana_celestial_quiver", &[0]),  // Celestial Quiver
    ("mirana_invis", &[125]),  // Moonlight Shadow
    ("mirana_leap", &[50]),  // Leap
    ("mirana_nightveil", &[0]),
    ("mirana_selemenes_faithful", &[0]),  // Selemene's Faithful
    ("mirana_solar_flare", &[125]),  // Solar Flare
    ("mirana_starfall", &[80, 90, 100, 110]),  // Starstorm
    ("monkey_king_boundless_strike", &[85, 90, 95, 100]),  // Boundless Strike
    ("monkey_king_jingu_mastery", &[0]),  // Jingu Mastery
    ("monkey_king_mischief", &[0, 0, 0, 0]),  // Mischief
    ("monkey_king_primal_spring", &[100]),  // Primal Spring
    ("monkey_king_primal_spring_early", &[0]),  // Spring Early
    ("monkey_king_transfiguration", &[0]),  // Changing of the Guard
    ("monkey_king_tree_dance", &[0]),  // Tree Dance
    ("monkey_king_untransform", &[0, 0, 0, 0]),  // Revert Form
    ("monkey_king_wukongs_command", &[100]),  // Wukong's Command
    ("morphling_accumulation", &[0]),  // Accumulation
    ("morphling_adaptive_strike_agi", &[40, 50, 60, 70]),  // Adaptive Strike
    ("morphling_ebb", &[0]),  // Ebb
    ("morphling_ebb_and_flow", &[0]),  // Ebb and Flow
    ("morphling_flow", &[0]),  // Flow
    ("morphling_hybrid", &[200]),
    ("morphling_morph_agi", &[0]),  // Attribute Shift (Agility Gain)
    ("morphling_morph_replicate", &[0]),  // Morph Replicate
    ("morphling_morph_str", &[0]),  // Attribute Shift (Strength Gain)
    ("morphling_replicate", &[50]),  // Morph
    ("morphling_syntropy", &[10, 12, 14, 16]),  // Syntropy
    ("morphling_waveform", &[115]),  // Waveform
    ("muerta_dead_shot", &[100, 120, 140, 160]),  // Dead Shot
    ("muerta_grave_tracker", &[0]),
    ("muerta_grave_visitation", &[100]),
    ("muerta_gunslinger", &[0]),  // Gunslinger
    ("muerta_ofrenda", &[50]),  // Ofrenda
    ("muerta_ofrenda_destroy", &[0]),  // Destroy Ofrenda
    ("muerta_parting_shot", &[250]),
    ("muerta_pierce_the_veil", &[150, 250, 350]),  // Pierce the Veil
    ("muerta_spectral_slug", &[75]),  // Spectral Slug
    ("muerta_supernatural", &[0]),  // Supernatural
    ("muerta_the_calling", &[135, 150, 165, 180]),  // The Calling
    ("naga_siren_crit", &[0]),
    ("naga_siren_deluge", &[80]),  // Deluge
    ("naga_siren_eelskin", &[0]),  // Eelskin
    ("naga_siren_ensnare", &[70, 80, 90, 100]),  // Ensnare
    ("naga_siren_mirror_image", &[75, 90, 105, 120]),  // Mirror Image
    ("naga_siren_reel_in", &[0]),  // Reel In
    ("naga_siren_reel_in_ad", &[0]),
    ("naga_siren_rip_tide", &[0]),  // Rip Tide
    ("naga_siren_slithereen_cutlass", &[0]),  // Slithereen Cutlass
    ("naga_siren_song_of_the_siren", &[150, 250, 350]),  // Song of the Siren
    ("naga_siren_song_of_the_siren_cancel", &[0]),  // Song of the Siren End
    ("necrolyte_death_pulse", &[115, 130, 145, 160]),  // Death Pulse
    ("necrolyte_death_seeker", &[160]),  // Death Seeker
    ("necrolyte_ghost_shroud", &[75]),  // Ghost Shroud
    ("necrolyte_heartstopper_aura", &[0]),  // Heartstopper Aura
    ("necrolyte_reapers_scythe", &[250, 375, 500]),  // Reaper's Scythe
    ("necrolyte_sadist", &[0]),  // Sadist
    ("necrolyte_sadist_stop", &[0]),
    ("nevermore_dark_lord", &[0]),  // Presence of the Dark Lord
    ("nevermore_frenzy", &[60, 65, 70, 75]),  // Feast of Souls
    ("nevermore_necromastery", &[0]),  // Necromastery
    ("nevermore_requiem", &[150, 175, 200]),  // Requiem of Souls
    ("nevermore_shadowraze1", &[75]),  // Shadowraze
    ("nevermore_shadowraze2", &[75]),  // Shadowraze
    ("nevermore_shadowraze3", &[75]),  // Shadowraze
    ("night_stalker_crippling_fear", &[50]),  // Crippling Fear
    ("night_stalker_darkness", &[125, 175, 225]),  // Dark Ascension
    ("night_stalker_heart_of_darkness", &[0]),  // Heart of Darkness
    ("night_stalker_hunter_in_the_night", &[0]),  // Hunter in the Night
    ("night_stalker_midnight_feast", &[0]),  // Midnight Feast
    ("night_stalker_night_reign", &[0]),  // Night Reign
    ("night_stalker_void", &[90, 95, 100, 105]),  // Void
    ("nyx_assassin_burrow", &[0]),  // Burrow
    ("nyx_assassin_impale", &[90, 100, 110, 120]),  // Impale
    ("nyx_assassin_innate_mana_burn", &[0]),
    ("nyx_assassin_jolt", &[100, 105, 110, 115]),  // Mind Flare
    ("nyx_assassin_mana_burn", &[70, 80, 90, 100]),  // Mind Flare
    ("nyx_assassin_neuro_sting", &[0]),  // Mana Burn
    ("nyx_assassin_nyxth_sense", &[0]),  // Nyxth Sense
    ("nyx_assassin_scarab_metabolism", &[0]),  // Scarab Metabolism
    ("nyx_assassin_spiked_carapace", &[40, 40, 40, 40]),  // Spiked Carapace
    ("nyx_assassin_unburrow", &[0]),  // Unburrow
    ("nyx_assassin_vendetta", &[180, 240, 300]),  // Vendetta
    ("obsidian_destroyer_arcane_orb", &[0]),  // Arcane Orb
    ("obsidian_destroyer_astral_imprisonment", &[150]),  // Astral Imprisonment
    ("obsidian_destroyer_equilibrium", &[0]),  // Essence Flux
    ("obsidian_destroyer_essence_aura", &[0]),
    ("obsidian_destroyer_objurgation", &[175]),  // Objurgation
    ("obsidian_destroyer_ominous_discernment", &[0]),  // Ominous Discernment
    ("obsidian_destroyer_sanity_eclipse", &[200, 300, 400]),  // Sanity's Eclipse
    ("ogre_magi_bloodlust", &[40, 50, 60, 70]),  // Bloodlust
    ("ogre_magi_dumb_luck", &[0]),  // Dumb Luck
    ("ogre_magi_fireblast", &[85, 95, 105, 115]),  // Fireblast
    ("ogre_magi_frost_armor", &[40]),  // Ice Armor
    ("ogre_magi_ignite", &[80, 90, 100, 110]),  // Ignite
    ("ogre_magi_multicast", &[0]),  // Multicast
    ("ogre_magi_school", &[0]),
    ("ogre_magi_smash", &[50]),  // Fire Shield
    ("ogre_magi_unrefined_fireblast", &[400]),  // Unrefined Fireblast
    ("omniknight_angelic_flight", &[125]),
    ("omniknight_degen_aura", &[0]),  // Degen Aura
    ("omniknight_guardian_angel", &[125, 175, 225]),  // Guardian Angel
    ("omniknight_hammer_of_purity", &[0, 0, 0, 0]),  // Hammer of Purity
    ("omniknight_healing_hammer", &[0]),  // Healing Hammer
    ("omniknight_martyr", &[90, 105, 120, 135]),  // Repel
    ("omniknight_pacify", &[50, 60, 70, 80]),
    ("omniknight_purification", &[90, 105, 120, 135]),  // Purification
    ("omniknight_repel", &[90, 105, 120, 135]),
    ("oracle_clairvoyant_cure", &[0]),  // Clairvoyant Cure
    ("oracle_clairvoyant_curse", &[0]),  // Clairvoyant Curse
    ("oracle_diviners_deck", &[0]),  // Diviner's Deck
    ("oracle_false_promise", &[125, 175, 225]),  // False Promise
    ("oracle_fates_edict", &[70]),  // Fate's Edict
    ("oracle_fortunes_end", &[80]),  // Fortune's End
    ("oracle_prognosticate", &[0]),  // Prognosticate
    ("oracle_purifying_flames", &[60, 65, 70, 75]),  // Purifying Flames
    ("oracle_rain_of_destiny", &[150]),  // Rain of Destiny
    ("pangolier_fortune_favors_the_bold", &[0]),  // Fortune Favors the Bold
    ("pangolier_gyroshell", &[100, 125, 150]),  // Rolling Thunder
    ("pangolier_gyroshell_stop", &[0]),  // Stop Rolling
    ("pangolier_heartpiercer", &[0]),
    ("pangolier_lucky_shot", &[0]),  // Lucky Shot
    ("pangolier_rollup", &[75]),  // Roll Up
    ("pangolier_rollup_stop", &[0]),  // End Roll Up
    ("pangolier_shield_crash", &[75]),  // Shield Crash
    ("pangolier_swashbuckle", &[85, 90, 95, 100]),  // Swashbuckle
    ("phantom_assassin_blur", &[40]),  // Blur
    ("phantom_assassin_coup_de_grace", &[0]),  // Coup de Grace
    ("phantom_assassin_fan_of_knives", &[80]),  // Fan of Knives
    ("phantom_assassin_immaterial", &[0]),  // Immaterial
    ("phantom_assassin_phantom_strike", &[35, 40, 45, 50]),  // Phantom Strike
    ("phantom_assassin_stifling_dagger", &[30]),  // Stifling Dagger
    ("phantom_lancer_doppelwalk", &[70]),  // Doppelganger
    ("phantom_lancer_illusory_armaments", &[0]),  // Illusory Armaments
    ("phantom_lancer_juxtapose", &[0]),  // Juxtapose
    ("phantom_lancer_phantom_edge", &[0]),  // Phantom Rush
    ("phantom_lancer_spirit_lance", &[120]),  // Spirit Lance
    ("phoenix_blinding_sun", &[0]),  // Blinding Sun
    ("phoenix_dying_light", &[0]),  // Dying Light
    ("phoenix_fire_spirits", &[100]),  // Fire Spirits
    ("phoenix_icarus_dive", &[0]),  // Icarus Dive
    ("phoenix_icarus_dive_stop", &[0]),  // Stop Icarus Dive
    ("phoenix_launch_fire_spirit", &[0, 0, 0, 0]),  // Launch Fire Spirit
    ("phoenix_sun_ray", &[90, 105, 120, 135]),  // Sun Ray
    ("phoenix_sun_ray_stop", &[0]),  // Stop Sun Ray
    ("phoenix_sun_ray_toggle_move", &[0]),  // Toggle Movement
    ("phoenix_supernova", &[150, 200, 250]),  // Supernova
    ("primal_beast_colossal", &[0]),  // Colossal
    ("primal_beast_innate_slow_resistance_per_time", &[0]),
    ("primal_beast_innate_status_resistance_per_time", &[0]),
    ("primal_beast_onslaught", &[120]),  // Onslaught
    ("primal_beast_onslaught_release", &[0]),  // Begin Onslaught
    ("primal_beast_pulverize", &[100]),  // Pulverize
    ("primal_beast_rock_throw", &[85]),  // Rock Throw
    ("primal_beast_trample", &[100]),  // Trample
    ("primal_beast_uproar", &[0]),  // Uproar
    ("puck_dream_coil", &[125, 175, 225]),  // Dream Coil
    ("puck_ethereal_jaunt", &[0]),  // Ethereal Jaunt
    ("puck_illusory_orb", &[90, 100, 110, 120]),  // Illusory Orb
    ("puck_phase_shift", &[0, 0, 0, 0]),  // Phase Shift
    ("puck_puckish", &[0]),  // Puckish
    ("puck_waning_rift", &[100, 110, 120, 130]),  // Waning Rift
    ("pudge_dismember", &[100, 130, 170]),  // Dismember
    ("pudge_eject", &[0]),  // Eject
    ("pudge_flesh_heap", &[65, 70, 75, 80]),  // Meat Shield
    ("pudge_innate_graft_flesh", &[0]),  // Flesh Heap
    ("pudge_meat_hook", &[120]),  // Meat Hook
    ("pudge_rot", &[0]),  // Rot
    ("pugna_decrepify", &[50, 60, 70, 80]),  // Decrepify
    ("pugna_life_drain", &[100, 150, 200]),  // Life Drain
    ("pugna_nether_blast", &[100, 115, 130, 145]),  // Nether Blast
    ("pugna_nether_ward", &[75, 100, 125, 150]),  // Nether Ward
    ("pugna_oblivion_savant", &[0]),  // Oblivion Savant
    ("queenofpain_blink", &[65]),  // Blink
    ("queenofpain_bondage", &[0]),  // Bondage
    ("queenofpain_masochist", &[0]),  // Masochist
    ("queenofpain_scream_of_pain", &[120]),  // Scream Of Pain
    ("queenofpain_shadow_strike", &[100, 105, 110, 115]),  // Shadow Strike
    ("queenofpain_sonic_wave", &[250, 400, 550]),  // Sonic Wave
    ("queenofpain_succubus", &[0]),  // Succubus
    ("rattletrap_armor_power", &[0]),  // Armor Power
    ("rattletrap_battery_assault", &[70, 75, 80, 85]),  // Battery Assault
    ("rattletrap_hookshot", &[100, 125, 150]),  // Hookshot
    ("rattletrap_jetpack", &[75]),  // Jetpack
    ("rattletrap_jetpack_toggle", &[0]),  // Jetpack Toggle
    ("rattletrap_overclocking", &[0]),  // Overclocking
    ("rattletrap_power_cogs", &[90]),  // Power Cogs
    ("rattletrap_rocket_flare", &[35, 40, 45, 50]),  // Rocket Flare
    ("razor_dynamo", &[0]),  // Dynamo
    ("razor_eye_of_the_storm", &[100, 150, 200]),  // Eye of the Storm
    ("razor_plasma_field", &[125]),  // Plasma Field
    ("razor_static_link", &[45, 50, 55, 60]),  // Static Link
    ("razor_storm_surge", &[0]),  // Storm Surge
    ("razor_unstable_current", &[0]),  // Unstable Current
    ("riki_backstab", &[0]),  // Cloak and Dagger
    ("riki_blink_strike", &[50, 55, 60, 65]),  // Blink Strike
    ("riki_innate_backstab", &[0]),  // Backstab
    ("riki_permanent_invisibility", &[0]),
    ("riki_poison_dart", &[75]),
    ("riki_smoke_screen", &[75]),  // Smoke Screen
    ("riki_tricks_of_the_trade", &[65]),  // Tricks of the Trade
    ("ringmaster_crystal_ball", &[0]),  // Crystal Ball
    ("ringmaster_dark_carnival_souvenirs", &[0]),  // Dark Carnival Barker
    ("ringmaster_empty_souvenir", &[0]),  // Souvenir Slot
    ("ringmaster_funhouse_mirror", &[0]),  // Funhouse Mirror
    ("ringmaster_impalement", &[50]),  // Impalement Arts
    ("ringmaster_spotlight", &[50]),  // Spotlight
    ("ringmaster_strongman_tonic", &[0]),  // Strongman Tonic
    ("ringmaster_summon_unicycle", &[0]),  // Unicycle
    ("ringmaster_tame_the_beasts", &[90, 105, 120, 135]),  // Tame the Beasts
    ("ringmaster_tame_the_beasts_crack", &[0]),  // Crack
    ("ringmaster_the_box", &[120]),  // Escape Act
    ("ringmaster_weighted_pie", &[0]),  // Weighted Pie
    ("ringmaster_wheel", &[150, 225, 300]),  // Wheel of Wonder
    ("ringmaster_whoopee_cushion", &[0]),  // Whoopee Cushion
    ("rubick_arcane_supremacy", &[0]),  // Arcane Supremacy
    ("rubick_curiosity", &[0]),  // Curiosity
    ("rubick_empty1", &[0]),  // Stolen Spell
    ("rubick_empty2", &[0]),  // Stolen Spell
    ("rubick_fade_bolt", &[110, 125, 140, 155]),  // Fade Bolt
    ("rubick_hidden1", &[0]),
    ("rubick_hidden2", &[0]),
    ("rubick_hidden3", &[0]),
    ("rubick_hidden4", &[0]),
    ("rubick_hidden5", &[0]),
    ("rubick_might_and_magus", &[0]),  // Might and Magus
    ("rubick_null_field", &[0]),
    ("rubick_spell_steal", &[25, 25, 25]),  // Spell Steal
    ("rubick_telekinesis", &[110]),  // Telekinesis
    ("rubick_telekinesis_land", &[0]),  // Telekinesis Land
    ("rubick_telekinesis_land_self", &[0]),  // Telekinesis Land
    ("sandking_burrowstrike", &[100, 110, 120, 130]),  // Burrowstrike
    ("sandking_caustic_finale", &[0]),  // Caustic Finale
    ("sandking_epicenter", &[150, 225, 300]),  // Epicenter
    ("sandking_sand_storm", &[85]),  // Sand Storm
    ("sandking_scorpion_strike", &[35, 40, 45, 50]),  // Stinger
    ("shadow_demon_demonic_cleanse", &[150]),  // Demonic Cleanse
    ("shadow_demon_demonic_purge", &[150, 175, 200]),  // Demonic Purge
    ("shadow_demon_disruption", &[120]),  // Disruption
    ("shadow_demon_disseminate", &[100]),  // Disseminate
    ("shadow_demon_menace", &[0]),  // Menace
    ("shadow_demon_shadow_poison", &[40]),  // Shadow Poison
    ("shadow_demon_shadow_poison_release", &[0]),  // Shadow Poison Release
    ("shadow_demon_shadow_servant", &[0]),  // Soul Catcher
    ("shadow_demon_soul_catcher", &[80, 100, 120, 140]),
    ("shadow_shaman_ether_shock", &[90, 105, 120, 135]),  // Ether Shock
    ("shadow_shaman_fowl_play", &[0]),  // Fowl Play
    ("shadow_shaman_innate_piglet_pole", &[0]),
    ("shadow_shaman_mass_serpent_ward", &[200, 350, 550]),  // Mass Serpent Ward
    ("shadow_shaman_serpentine", &[100]),
    ("shadow_shaman_shackles", &[125, 140, 155, 170]),  // Shackles
    ("shadow_shaman_urnaconda", &[140]),  // Urnaconda
    ("shadow_shaman_voodoo", &[120, 140, 160, 180]),  // Hex
    ("shadow_shaman_voodoo_hands", &[0]),  // Chicken Fingers
    ("shredder_chakram", &[90, 120, 150]),  // Chakram
    ("shredder_chakram_2", &[100, 140, 180]),
    ("shredder_exposure_therapy", &[0]),  // Exposure Therapy
    ("shredder_flamethrower", &[100]),  // Flamethrower
    ("shredder_reactive_armor", &[0]),  // Reactive Armor
    ("shredder_return_chakram", &[0]),  // Return Chakram
    ("shredder_return_chakram_2", &[0]),
    ("shredder_timber_chain", &[60, 70, 80, 90]),  // Timber Chain
    ("shredder_twisted_chakram", &[100, 140, 180]),  // Twisted Chakram
    ("shredder_whirling_death", &[100]),  // Whirling Death
    ("silencer_brain_drain", &[0]),  // Suffer In Silence
    ("silencer_curse_of_the_silent", &[120, 130, 140, 150]),  // Arcane Curse
    ("silencer_glaives_of_wisdom", &[12, 14, 16, 18]),  // Glaives of Wisdom
    ("silencer_global_silence", &[300, 450, 600]),  // Global Silence
    ("silencer_irrepressible", &[0]),
    ("silencer_last_word", &[100, 105, 110, 115]),  // Last Word
    ("silencer_oppressive_silence", &[0]),  // Suffer In Silence
    ("skeleton_king_bone_guard", &[70, 80, 90, 100]),  // Bone Guard
    ("skeleton_king_bone_guard_damage_tracker", &[0]),
    ("skeleton_king_hellfire_blast", &[95, 110, 125, 140]),  // Wraithfire Blast
    ("skeleton_king_mortal_strike", &[0]),  // Mortal Strike
    ("skeleton_king_reincarnation", &[220, 110, 0]),  // Reincarnation
    ("skeleton_king_spectral_blade", &[0]),  // Spectral Blade
    ("skeleton_king_vampiric_spirit", &[0]),  // Vampiric Spirit
    ("skywrath_mage_ancient_seal", &[80, 90, 100, 110]),  // Ancient Seal
    ("skywrath_mage_arcane_bolt", &[70]),  // Arcane Bolt
    ("skywrath_mage_concussive_shot", &[80, 85, 90, 95]),  // Concussive Shot
    ("skywrath_mage_mystic_flare", &[300, 550, 800]),  // Mystic Flare
    ("skywrath_mage_ruin_and_restoration", &[0]),  // Ruin and Restoration
    ("skywrath_mage_shield_of_the_scion", &[0]),  // Shield of the Scion
    ("skywrath_mage_staff_of_the_scion", &[0]),  // Staff of the Scion
    ("slardar_amplify_damage", &[25]),  // Corrosive Haze
    ("slardar_bash", &[0]),  // Bash of the Deep
    ("slardar_scepter", &[20]),
    ("slardar_seaborn_sentinel", &[0]),  // Seaborn Sentinel
    ("slardar_slithereen_crush", &[100]),  // Slithereen Crush
    ("slardar_sprint", &[25]),  // Guardian Sprint
    ("slark_dark_pact", &[65]),  // Dark Pact
    ("slark_depth_shroud", &[75]),  // Depth Shroud
    ("slark_essence_shift", &[0]),  // Essence Shift
    ("slark_fish_bait", &[50]),
    ("slark_pounce", &[75]),  // Pounce
    ("slark_saltwater_shiv", &[25, 30, 35, 40]),  // Saltwater Shiv
    ("slark_shadow_dance", &[100]),  // Shadow Dance
    ("snapfire_boomstick", &[0]),  // Boomstick
    ("snapfire_buckshot", &[0]),  // Buckshot
    ("snapfire_firesnap_cookie", &[105]),  // Firesnap Cookie
    ("snapfire_gobble_up", &[120]),  // Gobble Up
    ("snapfire_lil_shredder", &[70, 80, 90, 100]),  // Lil' Shredder
    ("snapfire_mortimer_kisses", &[125, 150, 175]),  // Mortimer Kisses
    ("snapfire_scatterblast", &[85, 90, 95, 100]),  // Scatterblast
    ("snapfire_spit_creep", &[0]),  // Spit Out
    ("sniper_assassinate", &[175]),  // Assassinate
    ("sniper_concussive_grenade", &[50]),  // Concussive Grenade
    ("sniper_headshot", &[0]),  // Headshot
    ("sniper_keen_scope", &[0]),  // Keen Scope
    ("sniper_shrapnel", &[75]),  // Shrapnel
    ("sniper_take_aim", &[50]),  // Take Aim
    ("sniper_take_aim_stop", &[0]),  // Stop Take Aim
    ("spectre_desolate", &[0]),  // Desolate
    ("spectre_dispersion", &[0]),  // Dispersion
    ("spectre_haunt", &[125, 175, 225]),  // Haunt
    ("spectre_reality", &[25]),  // Reality
    ("spectre_shadow_step", &[60, 65, 70, 75]),  // Shadow Step
    ("spectre_spectral", &[0]),  // Spectral
    ("spectre_spectral_dagger", &[100, 110, 120, 130]),  // Spectral Dagger
    ("spirit_breaker_bull_rush", &[0]),  // Empowering Haste
    ("spirit_breaker_bulldoze", &[30, 40, 50, 60]),  // Bulldoze
    ("spirit_breaker_charge_of_darkness", &[80, 90, 100, 110]),  // Charge of Darkness
    ("spirit_breaker_empowering_haste", &[0]),
    ("spirit_breaker_greater_bash", &[0]),  // Greater Bash
    ("spirit_breaker_herd_mentality", &[0]),  // Herd Mentality
    ("spirit_breaker_knockback_amp", &[0]),
    ("spirit_breaker_nether_strike", &[125, 150, 175]),  // Nether Strike
    ("spirit_breaker_planar_pocket", &[100]),  // Planar Pocket
    ("storm_spirit_ball_lightning", &[30]),  // Ball Lightning
    ("storm_spirit_electric_rave", &[100]),  // Electric Rave
    ("storm_spirit_electric_vortex", &[60, 70, 80, 90]),  // Electric Vortex
    ("storm_spirit_galvanized", &[0]),  // Galvanized
    ("storm_spirit_overload", &[0]),  // Overload
    ("storm_spirit_static_remnant", &[70, 80, 90, 100]),  // Static Remnant
    ("sven_gods_strength", &[100, 125, 150]),  // God's Strength
    ("sven_great_cleave", &[0]),  // Great Cleave
    ("sven_storm_bolt", &[110]),  // Storm Hammer
    ("sven_vanquisher", &[0]),  // Vanquisher
    ("sven_warcry", &[45]),  // Warcry
    ("sven_wrath_of_god", &[0]),  // Wrath of God
    ("techies_focused_detonate", &[0]),  // Detonate M.A.D.
    ("techies_land_mines", &[125, 175, 225]),  // Proximity Mines
    ("techies_minefield_sign", &[0]),  // Minefield Sign
    ("techies_mutually_assured_destruction", &[0]),  // M.A.D.
    ("techies_reactive_tazer", &[60]),  // Reactive Tazer
    ("techies_reactive_tazer_stop", &[0]),  // Detonate Tazer
    ("techies_remote_mines", &[120, 180, 240]),  // Remote Mines
    ("techies_remote_mines_self_detonate", &[0]),  // PINPOINT DETONATE
    ("techies_snare_trap", &[140]),  // Snare Trap
    ("techies_spoons_stash", &[0]),  // Spoon's Stash
    ("techies_squees_scope", &[0]),  // Squee's Scope
    ("techies_stasis_trap", &[80, 110, 140, 160]),  // Stasis Trap
    ("techies_sticky_bomb", &[100, 115, 130, 145]),  // Sticky Bomb
    ("techies_suicide", &[100, 125, 150, 175]),  // Blast Off!
    ("templar_assassin_hidden_gates", &[25]),  // Hidden Gates
    ("templar_assassin_inner_peace", &[0]),  // Inner Peace
    ("templar_assassin_meditation_end", &[0]),  // End Meditation
    ("templar_assassin_meld", &[35, 40, 45, 50]),  // Meld
    ("templar_assassin_psi_blades", &[0]),  // Psi Blades
    ("templar_assassin_psionic_trap", &[15, 15, 15]),  // Psionic Trap
    ("templar_assassin_refraction", &[95]),  // Refraction
    ("templar_assassin_self_trap", &[0]),  // Trap
    ("templar_assassin_third_eye", &[0]),  // Third Eye
    ("templar_assassin_trap", &[0]),  // Trap
    ("templar_assassin_trap_teleport", &[50]),  // Psionic Projection
    ("terrorblade_conjure_image", &[50, 60, 70, 80]),  // Conjure Image
    ("terrorblade_dark_unity", &[0]),  // Dark Unity
    ("terrorblade_demon_zeal", &[0]),  // Demon Zeal
    ("terrorblade_metamorphosis", &[100]),  // Metamorphosis
    ("terrorblade_reflection", &[60, 65, 70, 75]),  // Reflection
    ("terrorblade_sunder", &[100, 75, 50]),  // Sunder
    ("terrorblade_terror_wave", &[75]),  // Terror Wave
    ("tidehunter_anchor_smash", &[40, 45, 50, 55]),  // Anchor Smash
    ("tidehunter_arm_of_the_deep", &[80]),
    ("tidehunter_blubber", &[0]),  // Blubber
    ("tidehunter_dead_in_the_water", &[110]),  // Dead in the Water
    ("tidehunter_gush", &[100]),  // Gush
    ("tidehunter_kraken_shell", &[45]),  // Kraken Shell
    ("tidehunter_leviathans_catch", &[0]),  // Leviathan's Catch
    ("tidehunter_ravage", &[125, 225, 325]),  // Ravage
    ("tinker_defense_matrix", &[120]),  // Defense Matrix
    ("tinker_deploy_turrets", &[100, 120, 140, 160]),  // Deploy Turrets
    ("tinker_eureka", &[0]),  // Eureka!
    ("tinker_heat_seeking_missile", &[105, 115, 125, 135]),
    ("tinker_innate_keen_teleport_gold_on_death", &[0]),
    ("tinker_innate_keen_teleport_xp_on_death", &[0]),
    ("tinker_keen_teleport", &[75]),  // Keen Conveyance
    ("tinker_laser", &[95, 105, 115, 125]),  // Laser
    ("tinker_march_of_the_machines", &[100, 120, 140, 160]),  // March of the Machines
    ("tinker_rearm", &[100, 150, 200]),  // Rearm
    ("tinker_shrink_ray", &[50]),
    ("tinker_warp_grenade", &[80]),  // Warp Flare
    ("tiny_avalanche", &[105, 120, 135, 150]),  // Avalanche
    ("tiny_craggy_exterior", &[0]),  // Craggy Exterior
    ("tiny_grow", &[0]),  // Grow
    ("tiny_insurmountable", &[0]),  // Insurmountable
    ("tiny_rocksteady", &[0]),
    ("tiny_toss", &[125]),  // Toss
    ("tiny_toss_tree", &[0]),  // Tree Throw
    ("tiny_tree_channel", &[150]),  // Tree Volley
    ("tiny_tree_grab", &[40, 35, 30, 25]),  // Tree Grab
    ("treant_eyes_in_the_forest", &[30]),  // Eyes In The Forest
    ("treant_innate_attack_damage", &[0]),
    ("treant_leech_seed", &[20]),  // Leech Seed
    ("treant_lifebomb_explode", &[0]),
    ("treant_living_armor", &[80]),  // Living Armor
    ("treant_natures_grasp", &[90]),  // Nature's Grasp
    ("treant_natures_guise", &[0]),  // Nature's Guise
    ("treant_overgrowth", &[200, 250, 300]),  // Overgrowth
    ("treant_super_bloom", &[150]),  // Super Bloom
    ("troll_warlord_battle_trance", &[150]),  // Battle Trance
    ("troll_warlord_berserkers_rage", &[0]),  // Berserker's Rage
    ("troll_warlord_fervor", &[0]),  // Fervor
    ("troll_warlord_innate_masterful", &[0]),
    ("troll_warlord_rampage", &[75]),
    ("troll_warlord_scepter", &[150]),
    ("troll_warlord_switch_stance", &[0]),  // Battle Stance
    ("troll_warlord_whirling_axes_melee", &[50]),  // Whirling Axes (Melee)
    ("troll_warlord_whirling_axes_ranged", &[50]),  // Whirling Axes (Ranged)
    ("tusk_bitter_chill", &[0]),  // Bitter Chill
    ("tusk_drinking_buddies", &[80]),  // Drinking Buddies
    ("tusk_frozen_sigil", &[70]),
    ("tusk_ice_shards", &[100]),  // Ice Shards
    ("tusk_ice_shards_stop", &[0]),
    ("tusk_launch_snowball", &[0]),  // Launch Snowball
    ("tusk_snowball", &[75, 75, 75, 75]),  // Snowball
    ("tusk_tag_team", &[70]),  // Tag Team
    ("tusk_walrus_kick", &[100]),  // Walrus Kick
    ("tusk_walrus_punch", &[75]),  // Walrus PUNCH!
    ("undying_bite", &[70]),  // Zombie Bite
    ("undying_ceaseless_dirge", &[0]),  // Ceaseless Dirge
    ("undying_decay", &[110]),  // Decay
    ("undying_flesh_golem", &[100, 125, 150]),  // Flesh Golem
    ("undying_soul_rip", &[80, 90, 100, 110]),  // Soul Rip
    ("undying_tombstone", &[125, 150, 175, 200]),  // Tombstone
    ("undying_tombstone_grab", &[0]),
    ("undying_tombstone_unit_grab", &[0]),
    ("undying_tombstone_zombie_aura", &[0]),
    ("undying_tombstone_zombie_deathstrike", &[0]),  // Deathlust
    ("ursa_bear_down", &[0]),  // Bear Down
    ("ursa_earthshock", &[95]),  // Earthshock
    ("ursa_enrage", &[0, 0, 0]),  // Enrage
    ("ursa_fury_swipes", &[0]),  // Fury Swipes
    ("ursa_maul", &[0]),  // Maul
    ("ursa_overpower", &[45, 50, 55, 60]),  // Overpower
    ("vengefulspirit_command_aura", &[0]),  // Vengeance Aura
    ("vengefulspirit_magic_missile", &[90, 95, 100, 105]),  // Magic Missile
    ("vengefulspirit_nether_swap", &[100, 150, 200]),  // Nether Swap
    ("vengefulspirit_retribution", &[0]),  // Retribution
    ("vengefulspirit_soul_strike", &[0]),  // Soul Strike
    ("vengefulspirit_wave_of_terror", &[50]),  // Wave of Terror
    ("venomancer_area_poison", &[150]),
    ("venomancer_latent_poison", &[75]),
    ("venomancer_noxious_plague", &[200, 250, 300]),  // Noxious Plague
    ("venomancer_plague_ward", &[24, 26, 28, 30]),  // Plague Ward
    ("venomancer_poison_nova", &[200, 300, 400]),  // Poison Nova
    ("venomancer_poison_sting", &[0]),  // Poison Sting
    ("venomancer_sepsis", &[0]),  // Septic Shock
    ("venomancer_snakebite", &[70, 80, 90, 100]),  // Snakebite
    ("venomancer_venomous_gale", &[95, 105, 115, 125]),  // Venomous Gale
    ("viper_corrosive_skin", &[0]),  // Corrosive Skin
    ("viper_nethertoxin", &[70]),  // Nethertoxin
    ("viper_nose_dive", &[75]),  // Nosedive
    ("viper_poison_attack", &[20]),  // Poison Attack
    ("viper_predator", &[0]),  // Predator
    ("viper_viper_strike", &[100, 150, 200]),  // Viper Strike
    ("visage_grave_chill", &[75]),  // Grave Chill
    ("visage_gravekeepers_cloak", &[0]),  // Gravekeeper's Cloak
    ("visage_lurker", &[0]),  // Lurker
    ("visage_silent_as_the_grave", &[50]),  // Silent as the Grave
    ("visage_soul_assumption", &[110]),  // Soul Assumption
    ("visage_stone_form_self_cast", &[0]),  // Stone Form
    ("visage_summon_familiars", &[150, 150, 150]),  // Summon Familiars
    ("visage_summon_familiars_recall", &[0]),  // Recall Familiar
    ("visage_summon_familiars_stone_form", &[0]),  // Stone Form
    ("void_spirit_aether_remnant", &[75, 80, 85, 90]),  // Aether Remnant
    ("void_spirit_astral_step", &[90]),  // Astral Step
    ("void_spirit_dissimilate", &[110]),  // Dissimilate
    ("void_spirit_intrinsic_edge", &[0]),  // Intrinsic Edge
    ("void_spirit_resonant_pulse", &[115, 120, 125, 130]),  // Resonant Pulse
    ("void_spirit_symmetry", &[0]),  // Symmetry
    ("warlock_black_grimoire", &[0]),  // Black Grimoire
    ("warlock_eldritch_summoning", &[0]),  // Eldritch Summoning
    ("warlock_fatal_bonds", &[120, 130, 140, 150]),  // Fatal Bonds
    ("warlock_golem_flaming_fists", &[0]),  // Flaming Fists
    ("warlock_golem_permanent_immolation", &[0]),  // Permanent Immolation
    ("warlock_imp_explode", &[0]),  // Eldritch Explosion
    ("warlock_imp_explode_major", &[0]),
    ("warlock_rain_of_chaos", &[200, 400, 600]),  // Chaotic Offering
    ("warlock_shadow_word", &[110, 120, 130, 140]),  // Shadow Word
    ("warlock_upheaval", &[100]),  // Upheaval
    ("weaver_geminate_attack", &[0]),  // Geminate Attack
    ("weaver_rewoven", &[0]),  // Rewoven
    ("weaver_shukuchi", &[65]),  // Shukuchi
    ("weaver_the_swarm", &[110, 105, 100, 95]),  // The Swarm
    ("weaver_threads_of_fate", &[0]),  // Threads of Fate
    ("weaver_time_lapse", &[150, 75, 0]),  // Time Lapse
    ("weaver_time_lapse_ally", &[0]),  // Time Lapse (Ally)
    ("windrunner_easy_breezy", &[0]),  // Easy Breezy
    ("windrunner_focusfire", &[75, 100, 125]),  // Focus Fire
    ("windrunner_focusfire_cancel", &[0]),  // Focus Fire Cancel
    ("windrunner_gale_force", &[125]),  // Gale Force
    ("windrunner_powershot", &[90, 100, 110, 120]),  // Powershot
    ("windrunner_shackleshot", &[70, 80, 90, 100]),  // Shackleshot
    ("windrunner_tailwind", &[0]),  // Tailwind
    ("windrunner_windrun", &[50]),  // Windrun
    ("winter_wyvern_accelerated_learning", &[0]),  // Eldwurm Studies
    ("winter_wyvern_arctic_burn", &[100, 105, 110, 115]),  // Arctic Burn
    ("winter_wyvern_cold_embrace", &[50, 60, 70, 80]),  // Cold Embrace
    ("winter_wyvern_dragon_sight", &[0]),  // Dragon Sight
    ("winter_wyvern_eldwurm_scholar", &[0]),  // Eldwurm Scholar
    ("winter_wyvern_eldwurms_edda", &[0]),  // Eldwurm's Edda
    ("winter_wyvern_essence_of_the_blueheart", &[0]),  // Essence of the Blueheart
    ("winter_wyvern_splinter_blast", &[105, 115, 125, 135]),  // Splinter Blast
    ("winter_wyvern_winters_curse", &[150, 200, 250]),  // Winter's Curse
    ("wisp_equilibrium", &[0]),  // Equilibrium
    ("wisp_essence_conduction", &[0]),  // Wellspring
    ("wisp_overcharge", &[40, 60, 80, 100]),  // Overcharge
    ("wisp_relocate", &[175]),  // Relocate
    ("wisp_spirits", &[90, 100, 110, 120]),  // Spirits
    ("wisp_spirits_in", &[0]),  // Spirits In
    ("wisp_spirits_out", &[0]),  // Spirits Out
    ("wisp_tether", &[40, 40, 40, 40]),  // Tether
    ("wisp_tether_break", &[0]),  // Break Tether
    ("witch_doctor_death_ward", &[200, 250, 300]),  // Death Ward
    ("witch_doctor_gris_gris", &[0]),  // Gris-Gris
    ("witch_doctor_innate_maledict_heal_reduction", &[0]),
    ("witch_doctor_maledict", &[105, 110, 115, 120]),  // Maledict
    ("witch_doctor_paralyzing_cask", &[80, 100, 120, 140]),  // Paralyzing Cask
    ("witch_doctor_voodoo_restoration", &[25]),  // Voodoo Restoration
    ("witch_doctor_voodoo_switcheroo", &[200]),  // Voodoo Switcheroo
    ("zuus_arc_lightning", &[85, 90, 95, 100]),  // Arc Lightning
    ("zuus_cloud", &[275]),  // Nimbus
    ("zuus_heavenly_jump", &[50, 60, 70, 80]),  // Heavenly Jump
    ("zuus_lightning_bolt", &[120, 125, 130, 135]),  // Lightning Bolt
    ("zuus_lightning_hands", &[0]),  // Lightning Hands
    ("zuus_static_field", &[0]),  // Static Field
    ("zuus_thundergods_wrath", &[250, 375, 500]),  // Thundergod's Wrath
];

static ITEM_MANA_COST: LazyLock<HashMap<&'static str, u32>> =
    LazyLock::new(|| ITEM_MANA_COST_TABLE.iter().copied().collect());

static ABILITY_MANA_COST: LazyLock<HashMap<&'static str, &'static [u32]>> =
    LazyLock::new(|| ABILITY_MANA_COST_TABLE.iter().copied().collect());

/// Mana an item spends when activated, or `None` when the item is not in the table.
///
/// `Some(0)` and `None` mean different things: the first is "known to be free", the
/// second is "this build has never heard of it". Both suppress Soul Ring, but only the
/// second is worth logging.
pub fn item_mana_cost(name: &str) -> Option<u32> {
    ITEM_MANA_COST.get(name).copied()
}

/// Mana an ability spends at `level`, or `None` when the ability is not in the table.
///
/// `level` is GSI's 1-based `ability.level`; level `0` is unlearned and yields `None`.
/// Levels past the end of the table clamp to the last entry, which keeps Aghanim's and
/// talent-granted extra levels from falling off.
pub fn ability_mana_cost(name: &str, level: u32) -> Option<u32> {
    if level == 0 {
        return None;
    }
    let costs = ABILITY_MANA_COST.get(name)?;
    let index = ((level - 1) as usize).min(costs.len().saturating_sub(1));
    costs.get(index).copied()
}
