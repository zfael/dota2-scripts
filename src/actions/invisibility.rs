//! Tracks whether the local hero is invisible from an item we would break.
//!
//! GSI has no invisibility field. Valve's `hero` block carries `silenced`,
//! `stunned`, `hexed`, `disarmed`, `muted`, `break`, `magicimmune`, `has_debuff`
//! and `smoked` — and that is the whole set. Modifiers are never exposed on any
//! block, so `modifier_invisible` is simply unreadable.
//!
//! What GSI does give us is `Item.cooldown`, which ticks down in every payload.
//! A `0 -> N` transition on Shadow Blade or Silver Edge means the item was just
//! activated, which is the cast we care about. At the configured GSI throttle of
//! 0.1s that edge cannot be missed.
//!
//! Elapsed time is measured from the source item's *own cooldown* rather than a
//! wall clock: item cooldowns tick once per real second and freeze while the game
//! is paused, so the window stays correct across pauses for free and calibrates
//! itself to whatever the current patch's cooldown happens to be. The wall clock
//! is only a fallback for heavy cooldown reduction, where the item can come off
//! cooldown while the hero is still invisible.
//!
//! `hero.smoked` is deliberately *not* part of this. Smoke of Deceit is not broken
//! by activating an item — it drops on attacking, casting at an enemy, or walking
//! near an enemy hero or tower. Suppressing on it would cost us nothing but uptime.
//!
//! Known blind spot: a right-click attack breaks invisibility and produces no GSI
//! signal whatsoever, so the window stays open until it times out. That is the
//! right direction to fail — a skipped Phase activation costs a little movespeed
//! and a skipped Dark Pact costs a debuff a moment longer, while a wrongly fired
//! one costs the whole Shadow Blade.

use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Mutex;
use tracing::debug;

use crate::config::Settings;
use crate::models::gsi_event::GsiWebhookEvent;

/// Ability panel width GSI reports, matching `Abilities::get_by_index`.
const ABILITY_SLOTS: usize = 6;

/// An item that grants the local hero invisibility when activated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvisItemSource {
    pub item_name: &'static str,
    pub duration_seconds: u32,
}

pub const INVIS_ITEM_SOURCES: &[InvisItemSource] = &[
    InvisItemSource {
        item_name: "item_invis_sword",
        duration_seconds: 17,
    },
    InvisItemSource {
        item_name: "item_silver_edge",
        duration_seconds: 17,
    },
];

/// An invisibility window we believe is currently running.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveInvis {
    pub source_item: String,
    /// The source item's cooldown in the payload where the cast was spotted.
    pub cooldown_at_cast: u32,
    pub duration_seconds: u32,
    /// Only consulted once the source cooldown has run out early.
    pub started_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvisSnapshot {
    pub hero_name: String,
    /// Previous payload's cooldown per inventory item, keyed by item name.
    pub item_cooldowns: HashMap<String, u32>,
    /// Previous payload's cooldown per ability slot.
    pub ability_cooldowns: [u32; ABILITY_SLOTS],
    pub active: Option<ActiveInvis>,
}

lazy_static! {
    static ref LAST_INVIS_SNAPSHOT: Mutex<Option<InvisSnapshot>> = Mutex::new(None);
}

pub fn read_snapshot() -> Option<InvisSnapshot> {
    LAST_INVIS_SNAPSHOT.lock().unwrap().clone()
}

pub fn clear_snapshot() {
    *LAST_INVIS_SNAPSHOT.lock().unwrap() = None;
}

/// True only for invisibility that activating an item would break. Smoke is not
/// included on purpose — see the module comment.
pub fn is_invisible() -> bool {
    LAST_INVIS_SNAPSHOT
        .lock()
        .unwrap()
        .as_ref()
        .is_some_and(|snapshot| snapshot.active.is_some())
}

/// Whether an automation that would break invisibility must hold this tick.
///
/// The single gate for every automation that casts an ability or activates an
/// item. Callers that *grant* invisibility — Slark's Shadow Dance and Depth
/// Shroud — must not consult it: they replace the window rather than ending it.
pub fn suppresses_automation(settings: &Settings) -> bool {
    settings.invisibility.suppress_automation && is_invisible()
}

/// Advance the tracker with one GSI payload. Must run for *every* event, whether
/// or not any automation is enabled, or the cast edge is missed.
pub fn observe_event(event: &GsiWebhookEvent) {
    let previous = read_snapshot();
    let next = advance_invis_snapshot(previous.as_ref(), event, current_time_millis());

    let was_active = previous
        .as_ref()
        .and_then(|snapshot| snapshot.active.as_ref());
    match (was_active, next.active.as_ref()) {
        (None, Some(active)) => debug!(
            "🌑 Invisible via {}, holding automation",
            active.source_item
        ),
        (Some(active), None) => debug!("🌑 Invisibility from {} ended", active.source_item),
        _ => {}
    }

    *LAST_INVIS_SNAPSHOT.lock().unwrap() = Some(next);
}

fn advance_invis_snapshot(
    previous: Option<&InvisSnapshot>,
    event: &GsiWebhookEvent,
    now_ms: u64,
) -> InvisSnapshot {
    let item_cooldowns = read_item_cooldowns(event);
    let ability_cooldowns = read_ability_cooldowns(event);

    // A dead hero is a visible hero, and a different hero is a different game.
    let Some(previous) =
        previous.filter(|snapshot| event.hero.is_alive() && snapshot.hero_name == event.hero.name)
    else {
        return InvisSnapshot {
            hero_name: event.hero.name.clone(),
            item_cooldowns,
            ability_cooldowns,
            active: None,
        };
    };

    let cast_source = INVIS_ITEM_SOURCES.iter().find(|source| {
        went_on_cooldown(&previous.item_cooldowns, &item_cooldowns, source.item_name)
    });

    let active = if let Some(source) = cast_source {
        Some(ActiveInvis {
            source_item: source.item_name.to_string(),
            cooldown_at_cast: item_cooldowns
                .get(source.item_name)
                .copied()
                .unwrap_or_default(),
            duration_seconds: source.duration_seconds,
            started_at_ms: now_ms,
        })
    } else {
        previous
            .active
            .as_ref()
            .filter(|active| {
                !broke_invisibility(previous, &item_cooldowns, &ability_cooldowns, active)
            })
            .filter(|active| {
                elapsed_seconds(active, &item_cooldowns, now_ms) < active.duration_seconds
            })
            .cloned()
    };

    InvisSnapshot {
        hero_name: event.hero.name.clone(),
        item_cooldowns,
        ability_cooldowns,
        active,
    }
}

/// Anything the hero casts or activates drops invisibility. We can see every
/// ability and item that way; only plain attacks are invisible to us.
fn broke_invisibility(
    previous: &InvisSnapshot,
    item_cooldowns: &HashMap<String, u32>,
    ability_cooldowns: &[u32; ABILITY_SLOTS],
    active: &ActiveInvis,
) -> bool {
    if !item_cooldowns.contains_key(&active.source_item) {
        return true;
    }

    let ability_cast = ability_cooldowns
        .iter()
        .zip(previous.ability_cooldowns.iter())
        .any(|(current, previous)| *previous == 0 && *current > 0);

    let other_item_used = item_cooldowns.keys().any(|item_name| {
        item_name != &active.source_item
            && went_on_cooldown(&previous.item_cooldowns, item_cooldowns, item_name)
    });

    ability_cast || other_item_used
}

fn elapsed_seconds(
    active: &ActiveInvis,
    item_cooldowns: &HashMap<String, u32>,
    now_ms: u64,
) -> u32 {
    match item_cooldowns.get(&active.source_item).copied() {
        // The usual path: the source is still ticking, so it *is* the clock.
        Some(cooldown) if cooldown > 0 => active.cooldown_at_cast.saturating_sub(cooldown),
        // Cooldown reduction can retire the item mid-window; fall back to the wall
        // clock for the remainder.
        _ => (now_ms.saturating_sub(active.started_at_ms) / 1_000) as u32,
    }
}

fn went_on_cooldown(
    previous: &HashMap<String, u32>,
    current: &HashMap<String, u32>,
    item_name: &str,
) -> bool {
    previous.get(item_name) == Some(&0)
        && current.get(item_name).is_some_and(|cooldown| *cooldown > 0)
}

fn read_item_cooldowns(event: &GsiWebhookEvent) -> HashMap<String, u32> {
    event
        .items
        .all_slots()
        .into_iter()
        .filter(|(_, item)| item.name != "empty")
        .filter_map(|(_, item)| item.cooldown.map(|cooldown| (item.name.clone(), cooldown)))
        .collect()
}

fn read_ability_cooldowns(event: &GsiWebhookEvent) -> [u32; ABILITY_SLOTS] {
    let mut cooldowns = [0u32; ABILITY_SLOTS];
    for (index, cooldown) in cooldowns.iter_mut().enumerate() {
        if let Some(ability) = event.abilities.get_by_index(index as u8) {
            *cooldown = ability.cooldown;
        }
    }
    cooldowns
}

#[cfg(test)]
pub fn replace_snapshot_for_tests(snapshot: Option<InvisSnapshot>) {
    *LAST_INVIS_SNAPSHOT.lock().unwrap() = snapshot;
}

/// Serialises every test that swaps the global snapshot.
///
/// The tracker is process-wide and `cargo test` runs the crate's tests on
/// threads, so a test that installs an invisibility window would otherwise be
/// visible to unrelated tests running beside it. One lock for the whole crate,
/// not one per module — two locks would not serialise against each other.
#[cfg(test)]
pub fn snapshot_test_lock() -> &'static Mutex<()> {
    static TEST_LOCK: std::sync::OnceLock<Mutex<()>> = std::sync::OnceLock::new();
    TEST_LOCK.get_or_init(|| Mutex::new(()))
}

/// Build a snapshot that reports the hero as invisible, for tests that only care
/// about the gate rather than how the window was opened.
#[cfg(test)]
pub fn active_snapshot_for_tests(hero_name: &str) -> InvisSnapshot {
    InvisSnapshot {
        hero_name: hero_name.to_string(),
        item_cooldowns: HashMap::new(),
        ability_cooldowns: [0; ABILITY_SLOTS],
        active: Some(ActiveInvis {
            source_item: "item_invis_sword".to_string(),
            cooldown_at_cast: 25,
            duration_seconds: 17,
            started_at_ms: 0,
        }),
    }
}

fn current_time_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::gsi_event::{Abilities, Ability, Hero, Item, Items, Map};

    fn item(name: &str, cooldown: u32) -> Item {
        Item {
            name: name.to_string(),
            cooldown: Some(cooldown),
            ..Item::default()
        }
    }

    /// Shadow Blade in slot0, Phase Boots in slot1, one ability with a cooldown.
    fn event_with(
        shadow_blade_cooldown: u32,
        phase_cooldown: u32,
        ability_cooldown: u32,
    ) -> GsiWebhookEvent {
        GsiWebhookEvent {
            hero: Hero {
                alive: true,
                name: "npc_dota_hero_juggernaut".to_string(),
                ..Hero::default()
            },
            abilities: Abilities {
                ability0: Ability {
                    name: "juggernaut_blade_fury".to_string(),
                    cooldown: ability_cooldown,
                    ..Ability::default()
                },
                ..Abilities::default()
            },
            items: Items {
                slot0: item("item_invis_sword", shadow_blade_cooldown),
                slot1: item("item_phase_boots", phase_cooldown),
                ..Items::default()
            },
            map: Map { clock_time: 600 },
            player: None,
        }
    }

    fn seed(event: &GsiWebhookEvent) -> InvisSnapshot {
        advance_invis_snapshot(None, event, 0)
    }

    #[test]
    fn shadow_blade_cooldown_edge_opens_a_window() {
        let idle = event_with(0, 0, 0);
        let previous = seed(&idle);

        let cast = event_with(25, 0, 0);
        let snapshot = advance_invis_snapshot(Some(&previous), &cast, 1_000);

        let active = snapshot.active.expect("cast should open a window");
        assert_eq!(active.source_item, "item_invis_sword");
        assert_eq!(active.cooldown_at_cast, 25);
        assert_eq!(active.duration_seconds, 17);
    }

    #[test]
    fn window_stays_open_while_the_source_cooldown_ticks_down() {
        let previous = seed(&event_with(0, 0, 0));
        let mut snapshot = advance_invis_snapshot(Some(&previous), &event_with(25, 0, 0), 1_000);

        // 16 seconds of invisibility used: cooldown 25 -> 9.
        snapshot = advance_invis_snapshot(Some(&snapshot), &event_with(9, 0, 0), 17_000);

        assert!(snapshot.active.is_some());
    }

    #[test]
    fn window_closes_once_the_duration_has_elapsed() {
        let previous = seed(&event_with(0, 0, 0));
        let mut snapshot = advance_invis_snapshot(Some(&previous), &event_with(25, 0, 0), 1_000);

        // 17 seconds elapsed: cooldown 25 -> 8.
        snapshot = advance_invis_snapshot(Some(&snapshot), &event_with(8, 0, 0), 18_000);

        assert!(snapshot.active.is_none());
    }

    #[test]
    fn wall_clock_covers_a_source_that_leaves_cooldown_early() {
        // Pretend cooldown reduction left a 12s cooldown against a 17s duration.
        let previous = seed(&event_with(0, 0, 0));
        let mut snapshot = advance_invis_snapshot(Some(&previous), &event_with(12, 0, 0), 1_000);

        // Cooldown is spent but only 13s of the 17s duration have passed.
        snapshot = advance_invis_snapshot(Some(&snapshot), &event_with(0, 0, 0), 14_000);
        assert!(snapshot.active.is_some());

        snapshot = advance_invis_snapshot(Some(&snapshot), &event_with(0, 0, 0), 19_000);
        assert!(snapshot.active.is_none());
    }

    #[test]
    fn casting_an_ability_closes_the_window_early() {
        let previous = seed(&event_with(0, 0, 0));
        let mut snapshot = advance_invis_snapshot(Some(&previous), &event_with(25, 0, 0), 1_000);
        assert!(snapshot.active.is_some());

        snapshot = advance_invis_snapshot(Some(&snapshot), &event_with(24, 0, 12), 2_000);

        assert!(snapshot.active.is_none());
    }

    #[test]
    fn using_another_item_closes_the_window_early() {
        let previous = seed(&event_with(0, 0, 0));
        let mut snapshot = advance_invis_snapshot(Some(&previous), &event_with(25, 0, 0), 1_000);
        assert!(snapshot.active.is_some());

        snapshot = advance_invis_snapshot(Some(&snapshot), &event_with(24, 8, 0), 2_000);

        assert!(snapshot.active.is_none());
    }

    #[test]
    fn dropping_the_source_item_closes_the_window() {
        let previous = seed(&event_with(0, 0, 0));
        let mut snapshot = advance_invis_snapshot(Some(&previous), &event_with(25, 0, 0), 1_000);
        assert!(snapshot.active.is_some());

        let mut sold = event_with(0, 0, 0);
        sold.items.slot0 = Item::default();
        snapshot = advance_invis_snapshot(Some(&snapshot), &sold, 2_000);

        assert!(snapshot.active.is_none());
    }

    #[test]
    fn dying_closes_the_window() {
        let previous = seed(&event_with(0, 0, 0));
        let mut snapshot = advance_invis_snapshot(Some(&previous), &event_with(25, 0, 0), 1_000);
        assert!(snapshot.active.is_some());

        let mut dead = event_with(24, 0, 0);
        dead.hero.alive = false;
        snapshot = advance_invis_snapshot(Some(&snapshot), &dead, 2_000);

        assert!(snapshot.active.is_none());
    }

    #[test]
    fn switching_hero_closes_the_window() {
        let previous = seed(&event_with(0, 0, 0));
        let mut snapshot = advance_invis_snapshot(Some(&previous), &event_with(25, 0, 0), 1_000);
        assert!(snapshot.active.is_some());

        let mut other_hero = event_with(24, 0, 0);
        other_hero.hero.name = "npc_dota_hero_riki".to_string();
        snapshot = advance_invis_snapshot(Some(&snapshot), &other_hero, 2_000);

        assert!(snapshot.active.is_none());
    }

    #[test]
    fn smoke_alone_does_not_count_as_invisible() {
        let mut smoked = event_with(0, 0, 0);
        smoked.hero.smoked = true;

        let previous = seed(&smoked);
        let snapshot = advance_invis_snapshot(Some(&previous), &smoked, 1_000);

        assert!(snapshot.active.is_none());
    }

    #[test]
    fn silver_edge_is_tracked_as_well_as_shadow_blade() {
        let mut idle = event_with(0, 0, 0);
        idle.items.slot0 = item("item_silver_edge", 0);
        let previous = seed(&idle);

        let mut cast = event_with(0, 0, 0);
        cast.items.slot0 = item("item_silver_edge", 20);
        let snapshot = advance_invis_snapshot(Some(&previous), &cast, 1_000);

        assert_eq!(
            snapshot.active.map(|active| active.source_item),
            Some("item_silver_edge".to_string())
        );
    }

    #[test]
    fn an_item_already_on_cooldown_at_first_sight_does_not_open_a_window() {
        let snapshot = advance_invis_snapshot(None, &event_with(25, 0, 0), 1_000);

        assert!(snapshot.active.is_none());
    }

    /// The gate every automation shares. Both halves have to be true, so a
    /// player who turns the suppression off keeps the old behaviour outright.
    #[test]
    fn the_shared_gate_needs_both_the_setting_and_a_live_window() {
        let _guard = snapshot_test_lock().lock().unwrap();
        let mut settings = Settings::default();

        replace_snapshot_for_tests(None);
        assert!(!suppresses_automation(&settings));

        replace_snapshot_for_tests(Some(active_snapshot_for_tests("npc_dota_hero_slark")));
        assert!(suppresses_automation(&settings));

        settings.invisibility.suppress_automation = false;
        assert!(!suppresses_automation(&settings));

        replace_snapshot_for_tests(None);
    }

    #[test]
    fn every_tracked_source_has_a_usable_duration() {
        assert!(INVIS_ITEM_SOURCES
            .iter()
            .all(|source| source.duration_seconds > 0));
        assert!(INVIS_ITEM_SOURCES
            .iter()
            .any(|source| source.item_name == "item_invis_sword"));
        assert!(INVIS_ITEM_SOURCES
            .iter()
            .any(|source| source.item_name == "item_silver_edge"));
    }
}
