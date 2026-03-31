use dota2_scripts::observability::minimap_zones::{classify_zone, MapZone};
use dota2_scripts::observability::minimap_analysis::{
    build_color_masks, detect_heroes, find_clusters, is_green_pixel, is_red_pixel, rgb_to_hsv, 
    ColorThresholds, TeamColor,
};
use dota2_scripts::observability::minimap_baseline::BaselineMask;
use dota2_scripts::config::MinimapAnalysisConfig;
use dota2_scripts::models::gsi_event::Player;

#[test]
fn classify_zone_top_lane() {
    // nx=10/240=0.042, ny=10/245=0.041 → TopLane [0.00,0.25]×[0.00,0.55]
    assert_eq!(classify_zone(10, 10, 240, 245), MapZone::TopLane);
}

#[test]
fn classify_zone_bot_lane() {
    // nx=220/240=0.917, ny=230/245=0.939 → BotLane [0.75,1.00]×[0.45,1.00]
    assert_eq!(classify_zone(220, 230, 240, 245), MapZone::BotLane);
}

#[test]
fn classify_zone_mid_lane() {
    // nx=120/240=0.500, ny=122/245=0.498 → MidLane [0.25,0.75]×[0.25,0.75]
    assert_eq!(classify_zone(120, 122, 240, 245), MapZone::MidLane);
}

#[test]
fn classify_zone_roshan() {
    // nx=90/240=0.375, ny=90/245=0.367 → Roshan [0.30,0.45]×[0.32,0.45]
    assert_eq!(classify_zone(90, 90, 240, 245), MapZone::Roshan);
}

#[test]
fn classify_zone_dire_jungle() {
    // nx=200/240=0.833, ny=50/245=0.204 → DireJungle [0.45,1.00]×[0.00,0.45]
    assert_eq!(classify_zone(200, 50, 240, 245), MapZone::DireJungle);
}

#[test]
fn classify_zone_radiant_jungle() {
    // nx=50/240=0.208, ny=180/245=0.735 → RadiantJungle [0.00,0.55]×[0.55,1.00]
    assert_eq!(classify_zone(50, 180, 240, 245), MapZone::RadiantJungle);
}

#[test]
fn classify_zone_zero_dimensions_returns_other() {
    assert_eq!(classify_zone(10, 10, 0, 0), MapZone::Other);
}

#[test]
fn hsv_pure_red() {
    let hsv = rgb_to_hsv(255, 0, 0);
    assert!((hsv.h - 0.0).abs() < 1.0);
    assert!((hsv.s - 100.0).abs() < 1.0);
    assert!((hsv.v - 100.0).abs() < 1.0);
}

#[test]
fn hsv_pure_green() {
    let hsv = rgb_to_hsv(0, 255, 0);
    assert!((hsv.h - 120.0).abs() < 1.0);
    assert!((hsv.s - 100.0).abs() < 1.0);
    assert!((hsv.v - 100.0).abs() < 1.0);
}

#[test]
fn hsv_pure_blue() {
    let hsv = rgb_to_hsv(0, 0, 255);
    assert!((hsv.h - 240.0).abs() < 1.0);
    assert!((hsv.s - 100.0).abs() < 1.0);
    assert!((hsv.v - 100.0).abs() < 1.0);
}

#[test]
fn hsv_black_has_zero_value() {
    let hsv = rgb_to_hsv(0, 0, 0);
    assert!(hsv.v.abs() < 0.01);
}

#[test]
fn is_red_detects_dota_hero_red() {
    let t = ColorThresholds::default();
    assert!(is_red_pixel(200, 40, 40, &t));
    assert!(is_red_pixel(255, 0, 0, &t));
}

#[test]
fn is_red_rejects_non_red() {
    let t = ColorThresholds::default();
    assert!(!is_red_pixel(40, 200, 40, &t)); // green
    assert!(!is_red_pixel(30, 10, 10, &t));  // too dark
    assert!(!is_red_pixel(128, 128, 128, &t)); // gray (low saturation)
}

#[test]
fn is_green_detects_dota_hero_green() {
    let t = ColorThresholds::default();
    assert!(is_green_pixel(40, 200, 40, &t));
    assert!(is_green_pixel(0, 255, 0, &t));
}

#[test]
fn is_green_rejects_non_green() {
    let t = ColorThresholds::default();
    assert!(!is_green_pixel(200, 40, 40, &t)); // red
    assert!(!is_green_pixel(10, 30, 10, &t));  // too dark
}

#[test]
fn build_color_masks_separates_red_and_green() {
    let t = ColorThresholds::default();
    // 3x1 image: [red, black, green] in RGBA
    let pixels: Vec<u8> = vec![
        200, 40, 40, 255, // red
        0, 0, 0, 255,     // black
        40, 200, 40, 255, // green
    ];
    let (red_mask, green_mask) = build_color_masks(&pixels, 3, 1, &t);
    assert_eq!(red_mask, vec![true, false, false]);
    assert_eq!(green_mask, vec![false, false, true]);
}

#[test]
fn find_clusters_detects_two_separate_groups() {
    // 6x6 mask with two 2×2 clusters separated by gap
    let width = 6u32;
    let height = 6u32;
    let mut mask = vec![false; 36];
    // Cluster A at (0,0)-(1,1): indices 0,1,6,7
    mask[0] = true;
    mask[1] = true;
    mask[6] = true;
    mask[7] = true;
    // Cluster B at (4,4)-(5,5): indices 28,29,34,35
    mask[28] = true;
    mask[29] = true;
    mask[34] = true;
    mask[35] = true;

    let clusters = find_clusters(&mask, width, height, 3, 100);
    assert_eq!(clusters.len(), 2);
    assert_eq!(clusters[0].size, 4);
    assert_eq!(clusters[1].size, 4);
}

#[test]
fn find_clusters_computes_centroid() {
    // 4x4 mask with L-shaped cluster: (0,0),(1,0),(0,1)
    let width = 4u32;
    let height = 4u32;
    let mut mask = vec![false; 16];
    mask[0] = true; // (0,0)
    mask[1] = true; // (1,0)
    mask[4] = true; // (0,1)

    let clusters = find_clusters(&mask, width, height, 1, 100);
    assert_eq!(clusters.len(), 1);
    // Centroid: x=(0+1+0)/3=0, y=(0+0+1)/3=0 (integer division)
    assert_eq!(clusters[0].center_x, 0);
    assert_eq!(clusters[0].center_y, 0);
    assert_eq!(clusters[0].size, 3);
}

#[test]
fn find_clusters_filters_below_min_size() {
    let width = 4u32;
    let height = 1u32;
    // Two isolated pixels, each cluster size=1
    let mask = vec![true, false, false, true];
    let clusters = find_clusters(&mask, width, height, 3, 100);
    assert_eq!(clusters.len(), 0);
}

#[test]
fn find_clusters_filters_above_max_size() {
    let width = 3u32;
    let height = 3u32;
    let mask = vec![true; 9]; // one 3×3 cluster = 9 pixels
    let clusters = find_clusters(&mask, width, height, 1, 5);
    assert_eq!(clusters.len(), 0); // 9 > max_size 5
}

#[test]
fn find_clusters_empty_mask_returns_empty() {
    let mask = vec![false; 25];
    let clusters = find_clusters(&mask, 5, 5, 1, 100);
    assert!(clusters.is_empty());
}

#[test]
fn baseline_marks_consistent_red_as_static() {
    let mut bl = BaselineMask::new(3, 3, 0.8);
    // 10 frames: pixel 0 is always red
    for _ in 0..10 {
        let red = vec![true, false, false, false, false, false, false, false, false];
        let green = vec![false; 9];
        bl.accumulate_frame(&red, &green);
    }
    bl.build();
    assert!(bl.is_built());
    assert!(bl.is_static_red(0));  // 10/10 = 100% > 80%
    assert!(!bl.is_static_red(1)); // 0/10 = 0%
}

#[test]
fn baseline_marks_consistent_green_as_static() {
    let mut bl = BaselineMask::new(2, 2, 0.8);
    for _ in 0..10 {
        let red = vec![false; 4];
        let green = vec![false, false, false, true]; // pixel 3 always green
        bl.accumulate_frame(&red, &green);
    }
    bl.build();
    assert!(bl.is_static_green(3));
    assert!(!bl.is_static_green(0));
}

#[test]
fn baseline_excludes_infrequent_pixels() {
    let mut bl = BaselineMask::new(3, 3, 0.8);
    // Only 3/10 frames have pixel 4 (center) as red → 30% < 80%
    for i in 0..10 {
        let mut red = vec![false; 9];
        if i < 3 {
            red[4] = true;
        }
        let green = vec![false; 9];
        bl.accumulate_frame(&red, &green);
    }
    bl.build();
    assert!(!bl.is_static_red(4));
}

#[test]
fn baseline_not_built_returns_false() {
    let bl = BaselineMask::new(2, 2, 0.8);
    assert!(!bl.is_built());
    assert!(!bl.is_static_red(0));
    assert!(!bl.is_static_green(0));
}

#[test]
fn baseline_frame_count_tracks_accumulation() {
    let mut bl = BaselineMask::new(2, 2, 0.8);
    assert_eq!(bl.frame_count(), 0);
    bl.accumulate_frame(&[false; 4], &[false; 4]);
    assert_eq!(bl.frame_count(), 1);
    bl.accumulate_frame(&[false; 4], &[false; 4]);
    assert_eq!(bl.frame_count(), 2);
}

#[test]
fn baseline_out_of_bounds_index_returns_false() {
    let mut bl = BaselineMask::new(2, 2, 0.8);
    for _ in 0..5 {
        bl.accumulate_frame(&[true; 4], &[false; 4]);
    }
    bl.build();
    assert!(!bl.is_static_red(99)); // out of bounds
}

#[test]
fn detect_heroes_finds_red_and_green_clusters() {
    let width = 20u32;
    let height = 20u32;
    let mut pixels = vec![0u8; (width * height * 4) as usize];

    // Red 2×3 block at (2,2)-(3,4) → 6 pixels
    for y in 2..=4 {
        for x in 2..=3 {
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx] = 200;
            pixels[idx + 1] = 40;
            pixels[idx + 2] = 40;
            pixels[idx + 3] = 255;
        }
    }

    // Green 2×3 block at (17,17)-(18,19) → 6 pixels
    for y in 17..=19 {
        for x in 17..=18 {
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx] = 40;
            pixels[idx + 1] = 200;
            pixels[idx + 2] = 40;
            pixels[idx + 3] = 255;
        }
    }

    let thresholds = ColorThresholds {
        min_cluster_size: 3,
        max_cluster_size: 50,
        ..ColorThresholds::default()
    };

    let heroes = detect_heroes(&pixels, width, height, None, &thresholds);
    assert_eq!(heroes.len(), 2);

    let reds: Vec<_> = heroes.iter().filter(|h| h.team_color == TeamColor::Red).collect();
    let greens: Vec<_> = heroes.iter().filter(|h| h.team_color == TeamColor::Green).collect();
    assert_eq!(reds.len(), 1);
    assert_eq!(greens.len(), 1);
    assert_eq!(reds[0].cluster_size, 6);
    assert_eq!(greens[0].cluster_size, 6);
}

#[test]
fn detect_heroes_subtracts_baseline_static_elements() {
    let width = 10u32;
    let height = 10u32;
    let mut pixels = vec![0u8; (width * height * 4) as usize];

    // Red 2×2 block at (0,0)-(1,1)
    for y in 0..=1 {
        for x in 0..=1 {
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx] = 200;
            pixels[idx + 1] = 40;
            pixels[idx + 2] = 40;
            pixels[idx + 3] = 255;
        }
    }

    let thresholds = ColorThresholds {
        min_cluster_size: 3,
        max_cluster_size: 50,
        ..ColorThresholds::default()
    };

    // Without baseline: cluster is detected
    let heroes_no_bl = detect_heroes(&pixels, width, height, None, &thresholds);
    assert_eq!(heroes_no_bl.len(), 1);

    // Build baseline that marks (0,0)-(1,1) as static red
    let mut bl = BaselineMask::new(width, height, 0.8);
    for _ in 0..10 {
        let (red, green) = build_color_masks(&pixels, width, height, &thresholds);
        bl.accumulate_frame(&red, &green);
    }
    bl.build();

    // With baseline: static cluster is subtracted
    let heroes_with_bl = detect_heroes(&pixels, width, height, Some(&bl), &thresholds);
    assert_eq!(heroes_with_bl.len(), 0);
}

#[test]
fn detect_heroes_maps_to_zones() {
    let width = 240u32;
    let height = 245u32;
    let mut pixels = vec![0u8; (width * height * 4) as usize];

    // 5×5 green block centered at (90,90): Roshan area
    for y in 88..=92 {
        for x in 88..=92 {
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx] = 40;
            pixels[idx + 1] = 200;
            pixels[idx + 2] = 40;
            pixels[idx + 3] = 255;
        }
    }

    let thresholds = ColorThresholds {
        min_cluster_size: 5,
        max_cluster_size: 200,
        ..ColorThresholds::default()
    };

    let heroes = detect_heroes(&pixels, width, height, None, &thresholds);
    assert_eq!(heroes.len(), 1);
    assert_eq!(heroes[0].zone, MapZone::Roshan);
    assert_eq!(heroes[0].team_color, TeamColor::Green);
}

#[test]
fn detect_heroes_empty_image_returns_empty() {
    let pixels = vec![0u8; 100 * 100 * 4];
    let heroes = detect_heroes(&pixels, 100, 100, None, &ColorThresholds::default());
    assert!(heroes.is_empty());
}

#[test]
fn minimap_analysis_config_defaults() {
    let config = MinimapAnalysisConfig::default();
    assert!(!config.enabled);
    assert_eq!(config.baseline_frames, 10);
    assert!((config.baseline_threshold - 0.8).abs() < 0.01);
    assert_eq!(config.min_cluster_size, 20);
    assert_eq!(config.max_cluster_size, 200);
    assert!((config.red_hue_max - 15.0).abs() < 0.01);
}

#[test]
fn minimap_analysis_config_to_color_thresholds() {
    let config = MinimapAnalysisConfig::default();
    let t = config.to_color_thresholds();
    assert!((t.red_hue_max - 15.0).abs() < 0.01);
    assert!((t.red_hue_min_wrap - 340.0).abs() < 0.01);
    assert!((t.green_hue_min - 80.0).abs() < 0.01);
    assert!((t.green_hue_max - 160.0).abs() < 0.01);
    assert_eq!(t.min_cluster_size, 20);
    assert_eq!(t.max_cluster_size, 200);
}

#[test]
fn gsi_event_deserializes_without_player() {
    let json = std::fs::read_to_string("tests/fixtures/huskar_event.json").unwrap();
    let event: dota2_scripts::models::gsi_event::GsiWebhookEvent =
        serde_json::from_str(&json).unwrap();
    assert!(event.player.is_none());
}

#[test]
fn gsi_event_deserializes_with_player_team() {
    let json = r#"{
        "hero": {
            "aghanims_scepter": false, "aghanims_shard": false, "alive": true,
            "attributes_level": 0, "break": false, "buyback_cooldown": 0,
            "buyback_cost": 0, "disarmed": false, "facet": 0, "has_debuff": false,
            "health": 1000, "health_percent": 100, "hexed": false, "id": 1,
            "level": 1, "magicimmune": false, "mana": 500, "mana_percent": 100,
            "max_health": 1000, "max_mana": 500, "muted": false,
            "name": "npc_dota_hero_huskar", "respawn_seconds": 0, "silenced": false,
            "smoked": false, "stunned": false,
            "talent_1": false, "talent_2": false, "talent_3": false, "talent_4": false,
            "talent_5": false, "talent_6": false, "talent_7": false, "talent_8": false,
            "xp": 0, "xpos": 0, "ypos": 0
        },
        "abilities": {
            "ability0": {"ability_active":true,"can_cast":true,"cooldown":0,"level":1,"name":"huskar_inner_fire","passive":false,"ultimate":false},
            "ability1": {"ability_active":true,"can_cast":true,"cooldown":0,"level":0,"name":"huskar_burning_spear","passive":false,"ultimate":false},
            "ability2": {"ability_active":true,"can_cast":true,"cooldown":0,"level":0,"name":"huskar_berserkers_blood","passive":true,"ultimate":false},
            "ability3": {"ability_active":true,"can_cast":true,"cooldown":0,"level":0,"name":"huskar_inner_vitality","passive":false,"ultimate":false},
            "ability4": {"ability_active":true,"can_cast":true,"cooldown":0,"level":0,"name":"huskar_life_break","passive":false,"ultimate":true},
            "ability5": {"ability_active":true,"can_cast":true,"cooldown":0,"level":0,"name":"empty","passive":false,"ultimate":false}
        },
        "items": {
            "neutral0":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "slot0":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "slot1":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "slot2":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "slot3":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "slot4":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "slot5":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "slot6":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "slot7":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "slot8":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "stash0":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "stash1":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "stash2":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "stash3":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "stash4":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "stash5":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null},
            "teleport0":{"name":"empty","can_cast":null,"cooldown":null,"item_level":null,"passive":null,"purchaser":null,"charges":null,"item_charges":null}
        },
        "map": {"clock_time": 0},
        "player": {"team_name": "dire"}
    }"#;

    let event: dota2_scripts::models::gsi_event::GsiWebhookEvent =
        serde_json::from_str(json).unwrap();
    let player = event.player.unwrap();
    assert_eq!(player.team_name.as_deref(), Some("dire"));
}

// Layer 1: Zone Activity Classifier Tests
use dota2_scripts::observability::lane_heat::{
    classify_zone_activity, ActivityLevel, TeamSide, LaneEvent, LaneHeatTracker, ZoneSnapshot,
};

#[test]
fn team_side_from_dire() {
    let side = TeamSide::from_team_name("dire");
    assert_eq!(side.ally_color, TeamColor::Red);
    assert_eq!(side.enemy_color, TeamColor::Green);
}

#[test]
fn team_side_from_radiant() {
    let side = TeamSide::from_team_name("radiant");
    assert_eq!(side.ally_color, TeamColor::Green);
    assert_eq!(side.enemy_color, TeamColor::Red);
}

#[test]
fn classify_empty_heroes_returns_empty() {
    let side = TeamSide::from_team_name("dire");
    let result = classify_zone_activity(&[], &side);
    assert!(result.is_empty());
}

#[test]
fn classify_single_ally_in_zone() {
    use dota2_scripts::observability::minimap_analysis::DetectedHero;
    let side = TeamSide::from_team_name("dire"); // ally=Red
    let heroes = vec![DetectedHero {
        x: 10,
        y: 10,
        zone: MapZone::TopLane,
        team_color: TeamColor::Red,
        cluster_size: 30,
    }];
    let result = classify_zone_activity(&heroes, &side);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].zone, MapZone::TopLane);
    assert_eq!(result[0].ally_count, 1);
    assert_eq!(result[0].enemy_count, 0);
    assert_eq!(result[0].activity, ActivityLevel::Active);
}

#[test]
fn classify_fight_both_teams() {
    use dota2_scripts::observability::minimap_analysis::DetectedHero;
    let side = TeamSide::from_team_name("dire"); // ally=Red, enemy=Green
    let heroes = vec![
        DetectedHero {
            x: 10, y: 10, zone: MapZone::TopLane,
            team_color: TeamColor::Red, cluster_size: 30,
        },
        DetectedHero {
            x: 15, y: 15, zone: MapZone::TopLane,
            team_color: TeamColor::Green, cluster_size: 25,
        },
    ];
    let result = classify_zone_activity(&heroes, &side);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].zone, MapZone::TopLane);
    assert_eq!(result[0].ally_count, 1);
    assert_eq!(result[0].enemy_count, 1);
    assert_eq!(result[0].activity, ActivityLevel::Fight);
}

#[test]
fn classify_multiple_zones() {
    use dota2_scripts::observability::minimap_analysis::DetectedHero;
    let side = TeamSide::from_team_name("radiant"); // ally=Green, enemy=Red
    let heroes = vec![
        DetectedHero {
            x: 10, y: 10, zone: MapZone::TopLane,
            team_color: TeamColor::Green, cluster_size: 30,
        },
        DetectedHero {
            x: 12, y: 12, zone: MapZone::TopLane,
            team_color: TeamColor::Green, cluster_size: 28,
        },
        DetectedHero {
            x: 200, y: 200, zone: MapZone::BotLane,
            team_color: TeamColor::Red, cluster_size: 35,
        },
    ];
    let result = classify_zone_activity(&heroes, &side);
    assert_eq!(result.len(), 2);

    let top = result.iter().find(|s| s.zone == MapZone::TopLane).unwrap();
    assert_eq!(top.ally_count, 2);
    assert_eq!(top.enemy_count, 0);
    assert_eq!(top.activity, ActivityLevel::Active);

    let bot = result.iter().find(|s| s.zone == MapZone::BotLane).unwrap();
    assert_eq!(bot.ally_count, 0);
    assert_eq!(bot.enemy_count, 1);
    assert_eq!(bot.activity, ActivityLevel::Active);
}

// Layer 2 tests: LaneHeatTracker

#[test]
fn tracker_empty_summary() {
    let tracker = LaneHeatTracker::new(5);
    assert!(tracker.summary().is_empty());
    assert!(tracker.events().is_empty());
}

#[test]
fn tracker_single_frame_summary() {
    let mut tracker = LaneHeatTracker::new(5);
    let snapshots = vec![ZoneSnapshot {
        zone: MapZone::TopLane,
        ally_count: 2,
        enemy_count: 0,
        activity: ActivityLevel::Active,
    }];
    tracker.push_frame(snapshots);
    let summary = tracker.summary();
    assert_eq!(summary.len(), 1);
    assert_eq!(summary[0].zone, MapZone::TopLane);
    assert!((summary[0].avg_ally_count - 2.0).abs() < 0.01);
    assert!((summary[0].avg_enemy_count - 0.0).abs() < 0.01);
    assert_eq!(summary[0].peak_activity, ActivityLevel::Active);
    assert_eq!(summary[0].current_activity, ActivityLevel::Active);
    assert_eq!(summary[0].frames_with_fight, 0);
}

#[test]
fn tracker_fight_detected_event() {
    let mut tracker = LaneHeatTracker::new(5);
    tracker.push_frame(vec![ZoneSnapshot {
        zone: MapZone::TopLane,
        ally_count: 1,
        enemy_count: 0,
        activity: ActivityLevel::Active,
    }]);
    tracker.push_frame(vec![ZoneSnapshot {
        zone: MapZone::TopLane,
        ally_count: 1,
        enemy_count: 1,
        activity: ActivityLevel::Fight,
    }]);
    let events = tracker.events();
    assert!(events.iter().any(|e| matches!(e, LaneEvent::FightDetected { zone } if *zone == MapZone::TopLane)));
}

#[test]
fn tracker_fight_ongoing_event() {
    let mut tracker = LaneHeatTracker::new(5);
    tracker.push_frame(vec![ZoneSnapshot {
        zone: MapZone::MidLane,
        ally_count: 2,
        enemy_count: 2,
        activity: ActivityLevel::Fight,
    }]);
    tracker.push_frame(vec![ZoneSnapshot {
        zone: MapZone::MidLane,
        ally_count: 2,
        enemy_count: 2,
        activity: ActivityLevel::Fight,
    }]);
    let events = tracker.events();
    assert!(events.iter().any(|e| matches!(e, LaneEvent::FightOngoing { zone } if *zone == MapZone::MidLane)));
}

#[test]
fn tracker_enemy_rotation_event() {
    let mut tracker = LaneHeatTracker::new(5);
    for _ in 0..3 {
        tracker.push_frame(vec![ZoneSnapshot {
            zone: MapZone::BotLane,
            ally_count: 1,
            enemy_count: 0,
            activity: ActivityLevel::Active,
        }]);
    }
    tracker.push_frame(vec![ZoneSnapshot {
        zone: MapZone::BotLane,
        ally_count: 1,
        enemy_count: 3,
        activity: ActivityLevel::Fight,
    }]);
    let events = tracker.events();
    assert!(events.iter().any(|e| matches!(e, LaneEvent::EnemyRotation { zone } if *zone == MapZone::BotLane)));
}

#[test]
fn tracker_enemy_grouping_event() {
    let mut tracker = LaneHeatTracker::new(5);
    tracker.push_frame(vec![ZoneSnapshot {
        zone: MapZone::MidLane,
        ally_count: 0,
        enemy_count: 3,
        activity: ActivityLevel::Active,
    }]);
    let events = tracker.events();
    assert!(events.iter().any(|e| matches!(e, LaneEvent::EnemyGrouping { zone, count } if *zone == MapZone::MidLane && *count == 3)));
}

#[test]
fn tracker_window_evicts_old_frames() {
    let mut tracker = LaneHeatTracker::new(3);
    for i in 0..5 {
        tracker.push_frame(vec![ZoneSnapshot {
            zone: MapZone::TopLane,
            ally_count: i + 1,
            enemy_count: 0,
            activity: ActivityLevel::Active,
        }]);
    }
    let summary = tracker.summary();
    let top = summary.iter().find(|s| s.zone == MapZone::TopLane).unwrap();
    // Frames 3,4,5 (ally counts 3,4,5) → avg = 4.0
    assert!((top.avg_ally_count - 4.0).abs() < 0.01);
}

