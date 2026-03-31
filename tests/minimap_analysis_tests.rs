use dota2_scripts::observability::minimap_zones::{classify_zone, MapZone};

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
