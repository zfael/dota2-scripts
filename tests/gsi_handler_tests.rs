use dota2_scripts::models::GsiWebhookEvent;
use std::fs;

#[tokio::test]
async fn test_load_huskar_fixture() {
    let json_data = fs::read_to_string("tests/fixtures/huskar_event.json")
        .expect("Failed to read huskar fixture");
    
    let event: GsiWebhookEvent = serde_json::from_str(&json_data)
        .expect("Failed to deserialize huskar event");
    
    assert_eq!(event.hero.name, "npc_dota_hero_huskar");
    assert_eq!(event.hero.health, 280);
    assert_eq!(event.hero.health_percent, 25);
    assert!(event.hero.alive);
}

#[tokio::test]
async fn test_load_tiny_fixture() {
    let json_data = fs::read_to_string("tests/fixtures/tiny_event.json")
        .expect("Failed to read tiny fixture");
    
    let event: GsiWebhookEvent = serde_json::from_str(&json_data)
        .expect("Failed to deserialize tiny event");
    
    assert_eq!(event.hero.name, "npc_dota_hero_tiny");
    assert_eq!(event.hero.level, 15);
    assert!(event.hero.aghanims_scepter);
}

#[tokio::test]
async fn test_huskar_armlet_detection() {
    let json_data = fs::read_to_string("tests/fixtures/huskar_event.json")
        .expect("Failed to read huskar fixture");
    
    let event: GsiWebhookEvent = serde_json::from_str(&json_data)
        .expect("Failed to deserialize huskar event");
    
    // Check that armlet is in slot1
    assert_eq!(event.items.slot1.name, "item_armlet");
    assert_eq!(event.items.slot1.can_cast, Some(true));
}

#[tokio::test]
async fn test_healing_item_detection() {
    let json_data = fs::read_to_string("tests/fixtures/huskar_event.json")
        .expect("Failed to read huskar fixture");
    
    let event: GsiWebhookEvent = serde_json::from_str(&json_data)
        .expect("Failed to deserialize huskar event");
    
    // Check that magic_wand is in slot2
    assert_eq!(event.items.slot2.name, "item_magic_wand");
    assert_eq!(event.items.slot2.can_cast, Some(true));
    assert_eq!(event.items.slot2.charges, Some(15));
}
