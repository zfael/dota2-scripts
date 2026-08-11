/// Enum representing all Dota 2 heroes
/// Source: https://developer.valvesoftware.com/wiki/Dota_2_Workshop_Tools/Scripting/Heroes_internal_names
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Hero {
    Abaddon,
    AbyssalUnderlord,
    Alchemist,
    AncientApparition,
    AntiMage,
    ArcWarden,
    Axe,
    Bane,
    Batrider,
    Beastmaster,
    Bloodseeker,
    BountyHunter,
    Brewmaster,
    Bristleback,
    Broodmother,
    Centaur,
    ChaosKnight,
    Chen,
    Clinkz,
    CrystalMaiden,
    DarkSeer,
    DarkWillow,
    Dawnbreaker,
    Dazzle,
    DeathProphet,
    Disruptor,
    DoomBringer,
    DragonKnight,
    DrowRanger,
    EarthSpirit,
    Earthshaker,
    ElderTitan,
    EmberSpirit,
    Enchantress,
    Enigma,
    FacelessVoid,
    Furion,
    Grimstroke,
    Gyrocopter,
    Hoodwink,
    Huskar,
    Invoker,
    Jakiro,
    Juggernaut,
    KeeperOfTheLight,
    Kez,
    Kunkka,
    Largo,
    LegionCommander,
    Leshrac,
    Lich,
    LifeStealer,
    Lina,
    Lion,
    LoneDruid,
    Luna,
    Lycan,
    Magnataur,
    Marci,
    Mars,
    Medusa,
    Meepo,
    Mirana,
    MonkeyKing,
    Morphling,
    Muerta,
    NagaSiren,
    Necrolyte,
    Nevermore,
    NightStalker,
    NyxAssassin,
    ObsidianDestroyer,
    OgreMagi,
    Omniknight,
    Oracle,
    Pangolier,
    PhantomAssassin,
    PhantomLancer,
    Phoenix,
    PrimalBeast,
    Puck,
    Pudge,
    Pugna,
    QueenOfPain,
    Rattletrap,
    Razor,
    Riki,
    Ringmaster,
    Rubick,
    SandKing,
    ShadowDemon,
    ShadowShaman,
    Shredder,
    Silencer,
    SkeletonKing,
    SkywrathMage,
    Slardar,
    Slark,
    Snapfire,
    Sniper,
    Spectre,
    SpiritBreaker,
    StormSpirit,
    Sven,
    Techies,
    TemplarAssassin,
    Terrorblade,
    Tidehunter,
    Tinker,
    Tiny,
    Treant,
    TrollWarlord,
    Tusk,
    Undying,
    Ursa,
    VengefulSpirit,
    Venomancer,
    Viper,
    Visage,
    VoidSpirit,
    Warlock,
    Weaver,
    Windrunner,
    WinterWyvern,
    Wisp,
    WitchDoctor,
    Zuus,
}

/// Every variant of `Hero`. Kept here so the two name tables below can be
/// checked against each other in tests — `to_game_name` is exhaustive by the
/// compiler, but `from_game_name` is not, and a missing arm there is silent.
#[allow(dead_code)]
pub const ALL_HEROES: &[Hero] = &[
    Hero::Abaddon, Hero::AbyssalUnderlord, Hero::Alchemist, Hero::AncientApparition,
    Hero::AntiMage, Hero::ArcWarden, Hero::Axe, Hero::Bane, Hero::Batrider, Hero::Beastmaster,
    Hero::Bloodseeker, Hero::BountyHunter, Hero::Brewmaster, Hero::Bristleback,
    Hero::Broodmother, Hero::Centaur, Hero::ChaosKnight, Hero::Chen, Hero::Clinkz,
    Hero::CrystalMaiden, Hero::DarkSeer, Hero::DarkWillow, Hero::Dawnbreaker, Hero::Dazzle,
    Hero::DeathProphet, Hero::Disruptor, Hero::DoomBringer, Hero::DragonKnight,
    Hero::DrowRanger, Hero::EarthSpirit, Hero::Earthshaker, Hero::ElderTitan, Hero::EmberSpirit,
    Hero::Enchantress, Hero::Enigma, Hero::FacelessVoid, Hero::Furion, Hero::Grimstroke,
    Hero::Gyrocopter, Hero::Hoodwink, Hero::Huskar, Hero::Invoker, Hero::Jakiro,
    Hero::Juggernaut, Hero::KeeperOfTheLight, Hero::Kez, Hero::Kunkka, Hero::Largo,
    Hero::LegionCommander, Hero::Leshrac, Hero::Lich, Hero::LifeStealer, Hero::Lina, Hero::Lion,
    Hero::LoneDruid, Hero::Luna, Hero::Lycan, Hero::Magnataur, Hero::Marci, Hero::Mars,
    Hero::Medusa, Hero::Meepo, Hero::Mirana, Hero::MonkeyKing, Hero::Morphling, Hero::Muerta,
    Hero::NagaSiren, Hero::Necrolyte, Hero::Nevermore, Hero::NightStalker, Hero::NyxAssassin,
    Hero::ObsidianDestroyer, Hero::OgreMagi, Hero::Omniknight, Hero::Oracle, Hero::Pangolier,
    Hero::PhantomAssassin, Hero::PhantomLancer, Hero::Phoenix, Hero::PrimalBeast, Hero::Puck,
    Hero::Pudge, Hero::Pugna, Hero::QueenOfPain, Hero::Rattletrap, Hero::Razor, Hero::Riki,
    Hero::Ringmaster, Hero::Rubick, Hero::SandKing, Hero::ShadowDemon, Hero::ShadowShaman,
    Hero::Shredder, Hero::Silencer, Hero::SkeletonKing, Hero::SkywrathMage, Hero::Slardar,
    Hero::Slark, Hero::Snapfire, Hero::Sniper, Hero::Spectre, Hero::SpiritBreaker,
    Hero::StormSpirit, Hero::Sven, Hero::Techies, Hero::TemplarAssassin, Hero::Terrorblade,
    Hero::Tidehunter, Hero::Tinker, Hero::Tiny, Hero::Treant, Hero::TrollWarlord, Hero::Tusk,
    Hero::Undying, Hero::Ursa, Hero::VengefulSpirit, Hero::Venomancer, Hero::Viper,
    Hero::Visage, Hero::VoidSpirit, Hero::Warlock, Hero::Weaver, Hero::Windrunner,
    Hero::WinterWyvern, Hero::Wisp, Hero::WitchDoctor, Hero::Zuus,
];

impl Hero {
    /// Convert Hero enum to the game's internal hero name string
    pub fn to_game_name(&self) -> &'static str {
        match self {
            Hero::Abaddon => "npc_dota_hero_abaddon",
            Hero::AbyssalUnderlord => "npc_dota_hero_abyssal_underlord",
            Hero::Alchemist => "npc_dota_hero_alchemist",
            Hero::AncientApparition => "npc_dota_hero_ancient_apparition",
            Hero::AntiMage => "npc_dota_hero_antimage",
            Hero::ArcWarden => "npc_dota_hero_arc_warden",
            Hero::Axe => "npc_dota_hero_axe",
            Hero::Bane => "npc_dota_hero_bane",
            Hero::Batrider => "npc_dota_hero_batrider",
            Hero::Beastmaster => "npc_dota_hero_beastmaster",
            Hero::Bloodseeker => "npc_dota_hero_bloodseeker",
            Hero::BountyHunter => "npc_dota_hero_bounty_hunter",
            Hero::Brewmaster => "npc_dota_hero_brewmaster",
            Hero::Bristleback => "npc_dota_hero_bristleback",
            Hero::Broodmother => "npc_dota_hero_broodmother",
            Hero::Centaur => "npc_dota_hero_centaur",
            Hero::ChaosKnight => "npc_dota_hero_chaos_knight",
            Hero::Chen => "npc_dota_hero_chen",
            Hero::Clinkz => "npc_dota_hero_clinkz",
            Hero::CrystalMaiden => "npc_dota_hero_crystal_maiden",
            Hero::DarkSeer => "npc_dota_hero_dark_seer",
            Hero::DarkWillow => "npc_dota_hero_dark_willow",
            Hero::Dawnbreaker => "npc_dota_hero_dawnbreaker",
            Hero::Dazzle => "npc_dota_hero_dazzle",
            Hero::DeathProphet => "npc_dota_hero_death_prophet",
            Hero::Disruptor => "npc_dota_hero_disruptor",
            Hero::DoomBringer => "npc_dota_hero_doom_bringer",
            Hero::DragonKnight => "npc_dota_hero_dragon_knight",
            Hero::DrowRanger => "npc_dota_hero_drow_ranger",
            Hero::EarthSpirit => "npc_dota_hero_earth_spirit",
            Hero::Earthshaker => "npc_dota_hero_earthshaker",
            Hero::ElderTitan => "npc_dota_hero_elder_titan",
            Hero::EmberSpirit => "npc_dota_hero_ember_spirit",
            Hero::Enchantress => "npc_dota_hero_enchantress",
            Hero::Enigma => "npc_dota_hero_enigma",
            Hero::FacelessVoid => "npc_dota_hero_faceless_void",
            Hero::Furion => "npc_dota_hero_furion",
            Hero::Grimstroke => "npc_dota_hero_grimstroke",
            Hero::Gyrocopter => "npc_dota_hero_gyrocopter",
            Hero::Hoodwink => "npc_dota_hero_hoodwink",
            Hero::Huskar => "npc_dota_hero_huskar",
            Hero::Invoker => "npc_dota_hero_invoker",
            Hero::Jakiro => "npc_dota_hero_jakiro",
            Hero::Juggernaut => "npc_dota_hero_juggernaut",
            Hero::KeeperOfTheLight => "npc_dota_hero_keeper_of_the_light",
            Hero::Kez => "npc_dota_hero_kez",
            Hero::Kunkka => "npc_dota_hero_kunkka",
            Hero::Largo => "npc_dota_hero_largo",
            Hero::LegionCommander => "npc_dota_hero_legion_commander",
            Hero::Leshrac => "npc_dota_hero_leshrac",
            Hero::Lich => "npc_dota_hero_lich",
            Hero::LifeStealer => "npc_dota_hero_life_stealer",
            Hero::Lina => "npc_dota_hero_lina",
            Hero::Lion => "npc_dota_hero_lion",
            Hero::LoneDruid => "npc_dota_hero_lone_druid",
            Hero::Luna => "npc_dota_hero_luna",
            Hero::Lycan => "npc_dota_hero_lycan",
            Hero::Magnataur => "npc_dota_hero_magnataur",
            Hero::Marci => "npc_dota_hero_marci",
            Hero::Mars => "npc_dota_hero_mars",
            Hero::Medusa => "npc_dota_hero_medusa",
            Hero::Meepo => "npc_dota_hero_meepo",
            Hero::Mirana => "npc_dota_hero_mirana",
            Hero::MonkeyKing => "npc_dota_hero_monkey_king",
            Hero::Morphling => "npc_dota_hero_morphling",
            Hero::Muerta => "npc_dota_hero_muerta",
            Hero::NagaSiren => "npc_dota_hero_naga_siren",
            Hero::Necrolyte => "npc_dota_hero_necrolyte",
            Hero::Nevermore => "npc_dota_hero_nevermore",
            Hero::NightStalker => "npc_dota_hero_night_stalker",
            Hero::NyxAssassin => "npc_dota_hero_nyx_assassin",
            Hero::ObsidianDestroyer => "npc_dota_hero_obsidian_destroyer",
            Hero::OgreMagi => "npc_dota_hero_ogre_magi",
            Hero::Omniknight => "npc_dota_hero_omniknight",
            Hero::Oracle => "npc_dota_hero_oracle",
            Hero::Pangolier => "npc_dota_hero_pangolier",
            Hero::PhantomAssassin => "npc_dota_hero_phantom_assassin",
            Hero::PhantomLancer => "npc_dota_hero_phantom_lancer",
            Hero::Phoenix => "npc_dota_hero_phoenix",
            Hero::PrimalBeast => "npc_dota_hero_primal_beast",
            Hero::Puck => "npc_dota_hero_puck",
            Hero::Pudge => "npc_dota_hero_pudge",
            Hero::Pugna => "npc_dota_hero_pugna",
            Hero::QueenOfPain => "npc_dota_hero_queenofpain",
            Hero::Rattletrap => "npc_dota_hero_rattletrap",
            Hero::Razor => "npc_dota_hero_razor",
            Hero::Riki => "npc_dota_hero_riki",
            Hero::Ringmaster => "npc_dota_hero_ringmaster",
            Hero::Rubick => "npc_dota_hero_rubick",
            Hero::SandKing => "npc_dota_hero_sand_king",
            Hero::ShadowDemon => "npc_dota_hero_shadow_demon",
            Hero::ShadowShaman => "npc_dota_hero_shadow_shaman",
            Hero::Shredder => "npc_dota_hero_shredder",
            Hero::Silencer => "npc_dota_hero_silencer",
            Hero::SkeletonKing => "npc_dota_hero_skeleton_king",
            Hero::SkywrathMage => "npc_dota_hero_skywrath_mage",
            Hero::Slardar => "npc_dota_hero_slardar",
            Hero::Slark => "npc_dota_hero_slark",
            Hero::Snapfire => "npc_dota_hero_snapfire",
            Hero::Sniper => "npc_dota_hero_sniper",
            Hero::Spectre => "npc_dota_hero_spectre",
            Hero::SpiritBreaker => "npc_dota_hero_spirit_breaker",
            Hero::StormSpirit => "npc_dota_hero_storm_spirit",
            Hero::Sven => "npc_dota_hero_sven",
            Hero::Techies => "npc_dota_hero_techies",
            Hero::TemplarAssassin => "npc_dota_hero_templar_assassin",
            Hero::Terrorblade => "npc_dota_hero_terrorblade",
            Hero::Tidehunter => "npc_dota_hero_tidehunter",
            Hero::Tinker => "npc_dota_hero_tinker",
            Hero::Tiny => "npc_dota_hero_tiny",
            Hero::Treant => "npc_dota_hero_treant",
            Hero::TrollWarlord => "npc_dota_hero_troll_warlord",
            Hero::Tusk => "npc_dota_hero_tusk",
            Hero::Undying => "npc_dota_hero_undying",
            Hero::Ursa => "npc_dota_hero_ursa",
            Hero::VengefulSpirit => "npc_dota_hero_vengefulspirit",
            Hero::Venomancer => "npc_dota_hero_venomancer",
            Hero::Viper => "npc_dota_hero_viper",
            Hero::Visage => "npc_dota_hero_visage",
            Hero::VoidSpirit => "npc_dota_hero_void_spirit",
            Hero::Warlock => "npc_dota_hero_warlock",
            Hero::Weaver => "npc_dota_hero_weaver",
            Hero::Windrunner => "npc_dota_hero_windrunner",
            Hero::WinterWyvern => "npc_dota_hero_winter_wyvern",
            Hero::Wisp => "npc_dota_hero_wisp",
            Hero::WitchDoctor => "npc_dota_hero_witch_doctor",
            Hero::Zuus => "npc_dota_hero_zuus",
        }
    }

    /// Parse a game hero name string into a Hero enum
    #[allow(dead_code)]
    pub fn from_game_name(name: &str) -> Option<Self> {
        match name {
            "npc_dota_hero_abaddon" => Some(Hero::Abaddon),
            "npc_dota_hero_abyssal_underlord" => Some(Hero::AbyssalUnderlord),
            "npc_dota_hero_alchemist" => Some(Hero::Alchemist),
            "npc_dota_hero_ancient_apparition" => Some(Hero::AncientApparition),
            "npc_dota_hero_antimage" => Some(Hero::AntiMage),
            "npc_dota_hero_arc_warden" => Some(Hero::ArcWarden),
            "npc_dota_hero_axe" => Some(Hero::Axe),
            "npc_dota_hero_bane" => Some(Hero::Bane),
            "npc_dota_hero_batrider" => Some(Hero::Batrider),
            "npc_dota_hero_beastmaster" => Some(Hero::Beastmaster),
            "npc_dota_hero_bloodseeker" => Some(Hero::Bloodseeker),
            "npc_dota_hero_bounty_hunter" => Some(Hero::BountyHunter),
            "npc_dota_hero_brewmaster" => Some(Hero::Brewmaster),
            "npc_dota_hero_bristleback" => Some(Hero::Bristleback),
            "npc_dota_hero_broodmother" => Some(Hero::Broodmother),
            "npc_dota_hero_centaur" => Some(Hero::Centaur),
            "npc_dota_hero_chaos_knight" => Some(Hero::ChaosKnight),
            "npc_dota_hero_chen" => Some(Hero::Chen),
            "npc_dota_hero_clinkz" => Some(Hero::Clinkz),
            "npc_dota_hero_crystal_maiden" => Some(Hero::CrystalMaiden),
            "npc_dota_hero_dark_seer" => Some(Hero::DarkSeer),
            "npc_dota_hero_dark_willow" => Some(Hero::DarkWillow),
            "npc_dota_hero_dawnbreaker" => Some(Hero::Dawnbreaker),
            "npc_dota_hero_dazzle" => Some(Hero::Dazzle),
            "npc_dota_hero_death_prophet" => Some(Hero::DeathProphet),
            "npc_dota_hero_disruptor" => Some(Hero::Disruptor),
            "npc_dota_hero_doom_bringer" => Some(Hero::DoomBringer),
            "npc_dota_hero_dragon_knight" => Some(Hero::DragonKnight),
            "npc_dota_hero_drow_ranger" => Some(Hero::DrowRanger),
            "npc_dota_hero_earth_spirit" => Some(Hero::EarthSpirit),
            "npc_dota_hero_earthshaker" => Some(Hero::Earthshaker),
            "npc_dota_hero_elder_titan" => Some(Hero::ElderTitan),
            "npc_dota_hero_ember_spirit" => Some(Hero::EmberSpirit),
            "npc_dota_hero_enchantress" => Some(Hero::Enchantress),
            "npc_dota_hero_enigma" => Some(Hero::Enigma),
            "npc_dota_hero_faceless_void" => Some(Hero::FacelessVoid),
            "npc_dota_hero_furion" => Some(Hero::Furion),
            "npc_dota_hero_grimstroke" => Some(Hero::Grimstroke),
            "npc_dota_hero_gyrocopter" => Some(Hero::Gyrocopter),
            "npc_dota_hero_hoodwink" => Some(Hero::Hoodwink),
            "npc_dota_hero_huskar" => Some(Hero::Huskar),
            "npc_dota_hero_invoker" => Some(Hero::Invoker),
            "npc_dota_hero_jakiro" => Some(Hero::Jakiro),
            "npc_dota_hero_juggernaut" => Some(Hero::Juggernaut),
            "npc_dota_hero_keeper_of_the_light" => Some(Hero::KeeperOfTheLight),
            "npc_dota_hero_kez" => Some(Hero::Kez),
            "npc_dota_hero_kunkka" => Some(Hero::Kunkka),
            "npc_dota_hero_largo" => Some(Hero::Largo),
            "npc_dota_hero_legion_commander" => Some(Hero::LegionCommander),
            "npc_dota_hero_leshrac" => Some(Hero::Leshrac),
            "npc_dota_hero_lich" => Some(Hero::Lich),
            "npc_dota_hero_life_stealer" => Some(Hero::LifeStealer),
            "npc_dota_hero_lina" => Some(Hero::Lina),
            "npc_dota_hero_lion" => Some(Hero::Lion),
            "npc_dota_hero_lone_druid" => Some(Hero::LoneDruid),
            "npc_dota_hero_luna" => Some(Hero::Luna),
            "npc_dota_hero_lycan" => Some(Hero::Lycan),
            "npc_dota_hero_magnataur" => Some(Hero::Magnataur),
            "npc_dota_hero_marci" => Some(Hero::Marci),
            "npc_dota_hero_mars" => Some(Hero::Mars),
            "npc_dota_hero_medusa" => Some(Hero::Medusa),
            "npc_dota_hero_meepo" => Some(Hero::Meepo),
            "npc_dota_hero_mirana" => Some(Hero::Mirana),
            "npc_dota_hero_monkey_king" => Some(Hero::MonkeyKing),
            "npc_dota_hero_morphling" => Some(Hero::Morphling),
            "npc_dota_hero_muerta" => Some(Hero::Muerta),
            "npc_dota_hero_naga_siren" => Some(Hero::NagaSiren),
            "npc_dota_hero_necrolyte" => Some(Hero::Necrolyte),
            "npc_dota_hero_nevermore" => Some(Hero::Nevermore),
            "npc_dota_hero_night_stalker" => Some(Hero::NightStalker),
            "npc_dota_hero_nyx_assassin" => Some(Hero::NyxAssassin),
            "npc_dota_hero_obsidian_destroyer" => Some(Hero::ObsidianDestroyer),
            "npc_dota_hero_ogre_magi" => Some(Hero::OgreMagi),
            "npc_dota_hero_omniknight" => Some(Hero::Omniknight),
            "npc_dota_hero_oracle" => Some(Hero::Oracle),
            "npc_dota_hero_pangolier" => Some(Hero::Pangolier),
            "npc_dota_hero_phantom_assassin" => Some(Hero::PhantomAssassin),
            "npc_dota_hero_phantom_lancer" => Some(Hero::PhantomLancer),
            "npc_dota_hero_phoenix" => Some(Hero::Phoenix),
            "npc_dota_hero_primal_beast" => Some(Hero::PrimalBeast),
            "npc_dota_hero_puck" => Some(Hero::Puck),
            "npc_dota_hero_pudge" => Some(Hero::Pudge),
            "npc_dota_hero_pugna" => Some(Hero::Pugna),
            "npc_dota_hero_queenofpain" => Some(Hero::QueenOfPain),
            "npc_dota_hero_rattletrap" => Some(Hero::Rattletrap),
            "npc_dota_hero_razor" => Some(Hero::Razor),
            "npc_dota_hero_riki" => Some(Hero::Riki),
            "npc_dota_hero_ringmaster" => Some(Hero::Ringmaster),
            "npc_dota_hero_rubick" => Some(Hero::Rubick),
            "npc_dota_hero_sand_king" => Some(Hero::SandKing),
            "npc_dota_hero_shadow_demon" => Some(Hero::ShadowDemon),
            "npc_dota_hero_shadow_shaman" => Some(Hero::ShadowShaman),
            "npc_dota_hero_shredder" => Some(Hero::Shredder),
            "npc_dota_hero_silencer" => Some(Hero::Silencer),
            "npc_dota_hero_skeleton_king" => Some(Hero::SkeletonKing),
            "npc_dota_hero_skywrath_mage" => Some(Hero::SkywrathMage),
            "npc_dota_hero_slardar" => Some(Hero::Slardar),
            "npc_dota_hero_slark" => Some(Hero::Slark),
            "npc_dota_hero_snapfire" => Some(Hero::Snapfire),
            "npc_dota_hero_sniper" => Some(Hero::Sniper),
            "npc_dota_hero_spectre" => Some(Hero::Spectre),
            "npc_dota_hero_spirit_breaker" => Some(Hero::SpiritBreaker),
            "npc_dota_hero_storm_spirit" => Some(Hero::StormSpirit),
            "npc_dota_hero_sven" => Some(Hero::Sven),
            "npc_dota_hero_techies" => Some(Hero::Techies),
            "npc_dota_hero_templar_assassin" => Some(Hero::TemplarAssassin),
            "npc_dota_hero_terrorblade" => Some(Hero::Terrorblade),
            "npc_dota_hero_tidehunter" => Some(Hero::Tidehunter),
            "npc_dota_hero_tinker" => Some(Hero::Tinker),
            "npc_dota_hero_tiny" => Some(Hero::Tiny),
            "npc_dota_hero_treant" => Some(Hero::Treant),
            "npc_dota_hero_troll_warlord" => Some(Hero::TrollWarlord),
            "npc_dota_hero_tusk" => Some(Hero::Tusk),
            "npc_dota_hero_undying" => Some(Hero::Undying),
            "npc_dota_hero_ursa" => Some(Hero::Ursa),
            "npc_dota_hero_vengefulspirit" => Some(Hero::VengefulSpirit),
            "npc_dota_hero_venomancer" => Some(Hero::Venomancer),
            "npc_dota_hero_viper" => Some(Hero::Viper),
            "npc_dota_hero_visage" => Some(Hero::Visage),
            "npc_dota_hero_void_spirit" => Some(Hero::VoidSpirit),
            "npc_dota_hero_warlock" => Some(Hero::Warlock),
            "npc_dota_hero_weaver" => Some(Hero::Weaver),
            "npc_dota_hero_windrunner" => Some(Hero::Windrunner),
            "npc_dota_hero_winter_wyvern" => Some(Hero::WinterWyvern),
            "npc_dota_hero_wisp" => Some(Hero::Wisp),
            "npc_dota_hero_witch_doctor" => Some(Hero::WitchDoctor),
            "npc_dota_hero_zuus" => Some(Hero::Zuus),
            _ => None,
        }
    }
}

/// Internal names whose display name is not simply the name title-cased.
/// Everything absent from this table humanises correctly, including heroes
/// released after this list was written.
#[allow(dead_code)]
const IRREGULAR_DISPLAY_NAMES: &[(&str, &str)] = &[
    ("npc_dota_hero_abyssal_underlord", "Underlord"),
    ("npc_dota_hero_antimage", "Anti-Mage"),
    ("npc_dota_hero_centaur", "Centaur Warrunner"),
    ("npc_dota_hero_doom_bringer", "Doom"),
    ("npc_dota_hero_furion", "Nature's Prophet"),
    ("npc_dota_hero_keeper_of_the_light", "Keeper of the Light"),
    ("npc_dota_hero_life_stealer", "Lifestealer"),
    ("npc_dota_hero_magnataur", "Magnus"),
    ("npc_dota_hero_necrolyte", "Necrophos"),
    ("npc_dota_hero_nevermore", "Shadow Fiend"),
    ("npc_dota_hero_obsidian_destroyer", "Outworld Destroyer"),
    ("npc_dota_hero_queenofpain", "Queen of Pain"),
    ("npc_dota_hero_rattletrap", "Clockwerk"),
    ("npc_dota_hero_shredder", "Timbersaw"),
    ("npc_dota_hero_skeleton_king", "Wraith King"),
    ("npc_dota_hero_treant", "Treant Protector"),
    ("npc_dota_hero_vengefulspirit", "Vengeful Spirit"),
    ("npc_dota_hero_windrunner", "Windranger"),
    ("npc_dota_hero_wisp", "Io"),
    ("npc_dota_hero_zuus", "Zeus"),
];

/// Human-readable name for any `npc_dota_hero_*` string, whether or not the
/// hero has automation in this app. Returns `None` for the empty/placeholder
/// hero Dota sends before a pick.
#[allow(dead_code)]
pub fn display_name_for_game_name(name: &str) -> Option<String> {
    if name.is_empty() || name == "empty" {
        return None;
    }

    if let Some((_, display)) = IRREGULAR_DISPLAY_NAMES
        .iter()
        .find(|(internal, _)| *internal == name)
    {
        return Some((*display).to_string());
    }

    let stripped = name.strip_prefix("npc_dota_hero_").unwrap_or(name);
    if stripped.is_empty() {
        return None;
    }

    let humanised = stripped
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    Some(humanised)
}

#[cfg(test)]
mod tests {
    use super::{display_name_for_game_name, Hero, ALL_HEROES};

    /// Regression: `Hero::Largo` had a `to_game_name` arm but no
    /// `from_game_name` arm, so the lookup silently returned `None` for it.
    #[test]
    fn every_hero_round_trips_through_both_name_tables() {
        for hero in ALL_HEROES {
            let game_name = hero.to_game_name();
            assert_eq!(
                Hero::from_game_name(game_name),
                Some(*hero),
                "{} is missing a from_game_name arm",
                game_name
            );
        }
    }

    #[test]
    fn internal_names_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for hero in ALL_HEROES {
            assert!(
                seen.insert(hero.to_game_name()),
                "duplicate internal name: {}",
                hero.to_game_name()
            );
        }
    }

    #[test]
    fn humanises_a_regular_internal_name() {
        assert_eq!(
            display_name_for_game_name("npc_dota_hero_spirit_breaker").as_deref(),
            Some("Spirit Breaker")
        );
    }

    #[test]
    fn uses_the_override_for_irregular_internal_names() {
        assert_eq!(
            display_name_for_game_name("npc_dota_hero_nevermore").as_deref(),
            Some("Shadow Fiend")
        );
        assert_eq!(
            display_name_for_game_name("npc_dota_hero_keeper_of_the_light").as_deref(),
            Some("Keeper of the Light")
        );
    }

    #[test]
    fn humanises_heroes_that_postdate_the_enum() {
        assert_eq!(
            display_name_for_game_name("npc_dota_hero_brand_new_hero").as_deref(),
            Some("Brand New Hero")
        );
    }

    #[test]
    fn has_no_display_name_before_a_hero_is_picked() {
        assert_eq!(display_name_for_game_name(""), None);
        assert_eq!(display_name_for_game_name("empty"), None);
    }
}
