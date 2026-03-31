use dota2_scripts::observability::minimap_zones::{classify_zone, MapZone};
use dota2_scripts::observability::minimap_analysis::{
    build_color_masks, find_clusters, is_green_pixel, is_red_pixel, rgb_to_hsv, ColorThresholds,
};

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

use dota2_scripts::observability::minimap_baseline::BaselineMask;

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
