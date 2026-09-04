//! Tracks the defensive buff windows the danger kit has to reason about.
//!
//! Glimmer Cape and Ghost Scepter are the same panic button pressed twice: both
//! stop physical damage for a few seconds. Firing them together spends two items
//! on one moment and leaves nothing for the next one, so Ghost Scepter waits for
//! Glimmer to run out. Blade Mail waits for both — it returns damage nobody is
//! dealing us while we are hidden or ghosted.
//!
//! GSI never reports modifiers, so these windows are inferred the way
//! [`crate::actions::invisibility`] infers Shadow Blade: a `0 -> N` cooldown edge
//! on the item is the cast, and the item's own cooldown is the clock. Cooldowns
//! tick once per real second and freeze while the game is paused, so a window
//! stays correct across pauses and calibrates itself to whatever cooldown the
//! current patch ships. The wall clock is only a fallback for heavy cooldown
//! reduction, where an item can come back up before its buff expires.
//!
//! Unlike the invisibility tracker, nothing closes a window early. Glimmer's
//! magic resistance lasts the full duration even after an action drops the fade,
//! and holding the later items for that whole duration is the entire point — a
//! healing item pressed in between must not release them.
//!
//! Known blind spot: a Glimmer cast on an ally is indistinguishable from one cast
//! on ourselves, so a manual ally shield holds our own Ghost Scepter for five
//! seconds. Cheap next to the alternative of burning both panic items at once.

use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Mutex;
use tracing::debug;

use crate::models::gsi_event::GsiWebhookEvent;

pub const GLIMMER_ITEM: &str = "item_glimmer_cape";
pub const GHOST_ITEM: &str = "item_ghost";

/// An item whose activation opens a window later items have to wait out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSource {
    pub item_name: &'static str,
    pub duration_seconds: u32,
}

pub const WINDOW_SOURCES: &[WindowSource] = &[
    WindowSource {
        item_name: GLIMMER_ITEM,
        duration_seconds: 5,
    },
    WindowSource {
        item_name: GHOST_ITEM,
        duration_seconds: 4,
    },
];

/// A window we believe is currently running.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveWindow {
    /// The item's cooldown in the payload where the cast was spotted.
    pub cooldown_at_cast: u32,
    pub duration_seconds: u32,
    /// Only consulted once the cooldown has run out early.
    pub started_at_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WindowSnapshot {
    pub hero_name: String,
    /// Previous payload's cooldown per tracked item, absent when it is not in
    /// the inventory.
    pub cooldowns: HashMap<&'static str, u32>,
    pub active: HashMap<&'static str, ActiveWindow>,
}

lazy_static! {
    static ref LAST_WINDOW_SNAPSHOT: Mutex<Option<WindowSnapshot>> = Mutex::new(None);
}

pub fn read_snapshot() -> Option<WindowSnapshot> {
    LAST_WINDOW_SNAPSHOT.lock().unwrap().clone()
}

pub fn clear_snapshot() {
    *LAST_WINDOW_SNAPSHOT.lock().unwrap() = None;
}

/// True while the named item's buff is still running on us.
pub fn is_active(item_name: &str) -> bool {
    LAST_WINDOW_SNAPSHOT
        .lock()
        .unwrap()
        .as_ref()
        .is_some_and(|snapshot| snapshot.active.contains_key(item_name))
}

/// Advance the tracker with one GSI payload. Must run for *every* event, whether
/// or not any automation is enabled, or the cast edge is missed.
pub fn observe_event(event: &GsiWebhookEvent) {
    let previous = read_snapshot();
    let next = advance_window_snapshot(previous.as_ref(), event, current_time_millis());

    for source in WINDOW_SOURCES {
        let was_active = previous
            .as_ref()
            .is_some_and(|snapshot| snapshot.active.contains_key(source.item_name));
        match (was_active, next.active.contains_key(source.item_name)) {
            (false, true) => debug!("✨ {} up, holding the items behind it", source.item_name),
            (true, false) => debug!("✨ {} window ended", source.item_name),
            _ => {}
        }
    }

    *LAST_WINDOW_SNAPSHOT.lock().unwrap() = Some(next);
}

fn advance_window_snapshot(
    previous: Option<&WindowSnapshot>,
    event: &GsiWebhookEvent,
    now_ms: u64,
) -> WindowSnapshot {
    let cooldowns = read_cooldowns(event);

    // A dead hero carries no buff, and a different hero is a different game.
    let Some(previous) =
        previous.filter(|snapshot| event.hero.is_alive() && snapshot.hero_name == event.hero.name)
    else {
        return WindowSnapshot {
            hero_name: event.hero.name.clone(),
            cooldowns,
            active: HashMap::new(),
        };
    };

    let mut active = HashMap::new();

    for source in WINDOW_SOURCES {
        let cooldown = cooldowns.get(source.item_name).copied();
        let cast = previous.cooldowns.get(source.item_name) == Some(&0)
            && cooldown.is_some_and(|cooldown| cooldown > 0);

        if cast {
            active.insert(
                source.item_name,
                ActiveWindow {
                    cooldown_at_cast: cooldown.unwrap_or_default(),
                    duration_seconds: source.duration_seconds,
                    started_at_ms: now_ms,
                },
            );
            continue;
        }

        let carried = previous
            .active
            .get(source.item_name)
            // Sold or swapped out: whatever it was doing is not ours to wait on.
            .filter(|_| cooldown.is_some())
            .filter(|window| elapsed_seconds(window, cooldown, now_ms) < window.duration_seconds);

        if let Some(window) = carried {
            active.insert(source.item_name, window.clone());
        }
    }

    WindowSnapshot {
        hero_name: event.hero.name.clone(),
        cooldowns,
        active,
    }
}

fn elapsed_seconds(window: &ActiveWindow, cooldown: Option<u32>, now_ms: u64) -> u32 {
    match cooldown {
        // The usual path: the cooldown is still ticking, so it *is* the clock.
        Some(cooldown) if cooldown > 0 => window.cooldown_at_cast.saturating_sub(cooldown),
        // Cooldown reduction can retire the item mid-window; fall back to the
        // wall clock for the remainder.
        _ => (now_ms.saturating_sub(window.started_at_ms) / 1_000) as u32,
    }
}

/// Cooldowns for the tracked items that are in the inventory. A payload that
/// omits `cooldown` reads as ready, which is what a missing cooldown means.
fn read_cooldowns(event: &GsiWebhookEvent) -> HashMap<&'static str, u32> {
    let slots = event.items.all_slots();

    WINDOW_SOURCES
        .iter()
        .filter_map(|source| {
            slots
                .iter()
                .find(|(_, item)| item.name == source.item_name)
                .map(|(_, item)| (source.item_name, item.cooldown.unwrap_or(0)))
        })
        .collect()
}

#[cfg(test)]
pub fn replace_snapshot_for_tests(snapshot: Option<WindowSnapshot>) {
    *LAST_WINDOW_SNAPSHOT.lock().unwrap() = snapshot;
}

/// Build a snapshot that reports one item's window as running, for tests that
/// only care about the gate rather than how the window was opened.
#[cfg(test)]
pub fn active_snapshot_for_tests(hero_name: &str, item_name: &str) -> WindowSnapshot {
    let source = WINDOW_SOURCES
        .iter()
        .find(|source| source.item_name == item_name)
        .expect("item_name must be a tracked window source");

    WindowSnapshot {
        hero_name: hero_name.to_string(),
        cooldowns: HashMap::from([(source.item_name, 13)]),
        active: HashMap::from([(
            source.item_name,
            ActiveWindow {
                cooldown_at_cast: 13,
                duration_seconds: source.duration_seconds,
                started_at_ms: 0,
            },
        )]),
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
    use crate::models::gsi_event::{Abilities, Hero, Item, Items, Map};

    fn item(name: &str, cooldown: u32) -> Item {
        Item {
            name: name.to_string(),
            cooldown: Some(cooldown),
            ..Item::default()
        }
    }

    /// Glimmer Cape in slot0, Ghost Scepter in slot1, Magic Wand in slot2.
    fn event_with(
        glimmer_cooldown: u32,
        ghost_cooldown: u32,
        wand_cooldown: u32,
    ) -> GsiWebhookEvent {
        GsiWebhookEvent {
            hero: Hero {
                alive: true,
                name: "npc_dota_hero_lion".to_string(),
                ..Hero::default()
            },
            abilities: Abilities::default(),
            items: Items {
                slot0: item(GLIMMER_ITEM, glimmer_cooldown),
                slot1: item(GHOST_ITEM, ghost_cooldown),
                slot2: item("item_magic_wand", wand_cooldown),
                ..Items::default()
            },
            map: Map { clock_time: 600 },
            player: None,
        }
    }

    fn seed(event: &GsiWebhookEvent) -> WindowSnapshot {
        advance_window_snapshot(None, event, 0)
    }

    fn active_items(snapshot: &WindowSnapshot) -> Vec<&str> {
        let mut names = snapshot.active.keys().copied().collect::<Vec<_>>();
        names.sort_unstable();
        names
    }

    #[test]
    fn a_cooldown_edge_opens_that_item_s_window() {
        let previous = seed(&event_with(0, 0, 0));

        let snapshot = advance_window_snapshot(Some(&previous), &event_with(13, 0, 0), 1_000);

        assert_eq!(active_items(&snapshot), vec![GLIMMER_ITEM]);
        assert_eq!(snapshot.active[GLIMMER_ITEM].cooldown_at_cast, 13);
        assert_eq!(snapshot.active[GLIMMER_ITEM].duration_seconds, 5);
    }

    #[test]
    fn ghost_scepter_opens_its_own_four_second_window() {
        let previous = seed(&event_with(0, 0, 0));

        let snapshot = advance_window_snapshot(Some(&previous), &event_with(0, 20, 0), 1_000);

        assert_eq!(active_items(&snapshot), vec![GHOST_ITEM]);
        assert_eq!(snapshot.active[GHOST_ITEM].duration_seconds, 4);
    }

    #[test]
    fn both_windows_can_run_at_once() {
        let previous = seed(&event_with(0, 0, 0));
        let mut snapshot = advance_window_snapshot(Some(&previous), &event_with(13, 0, 0), 1_000);

        // Glimmer went up first; Ghost follows a couple of seconds later.
        snapshot = advance_window_snapshot(Some(&snapshot), &event_with(11, 20, 0), 3_000);

        assert_eq!(active_items(&snapshot), vec![GHOST_ITEM, GLIMMER_ITEM]);
    }

    #[test]
    fn each_window_closes_on_its_own_duration() {
        let previous = seed(&event_with(0, 0, 0));
        let mut snapshot = advance_window_snapshot(Some(&previous), &event_with(0, 20, 0), 1_000);
        assert_eq!(active_items(&snapshot), vec![GHOST_ITEM]);

        // 3 of Ghost's 4 seconds spent: cooldown 20 -> 17.
        snapshot = advance_window_snapshot(Some(&snapshot), &event_with(0, 17, 0), 4_000);
        assert_eq!(active_items(&snapshot), vec![GHOST_ITEM]);

        // 4 seconds: the window is done.
        snapshot = advance_window_snapshot(Some(&snapshot), &event_with(0, 16, 0), 5_000);
        assert!(active_items(&snapshot).is_empty());
    }

    #[test]
    fn other_items_going_on_cooldown_do_not_close_a_window() {
        let previous = seed(&event_with(0, 0, 0));
        let mut snapshot = advance_window_snapshot(Some(&previous), &event_with(13, 0, 0), 1_000);

        // A healing item fired a tick later; the magic resistance is still ours.
        snapshot = advance_window_snapshot(Some(&snapshot), &event_with(12, 0, 15), 2_000);

        assert_eq!(active_items(&snapshot), vec![GLIMMER_ITEM]);
    }

    #[test]
    fn wall_clock_covers_a_cooldown_that_ends_early() {
        // Pretend cooldown reduction left a 3s cooldown against a 5s duration.
        let previous = seed(&event_with(0, 0, 0));
        let mut snapshot = advance_window_snapshot(Some(&previous), &event_with(3, 0, 0), 1_000);

        // Cooldown is spent but only 4s of the 5s duration have passed.
        snapshot = advance_window_snapshot(Some(&snapshot), &event_with(0, 0, 0), 5_000);
        assert_eq!(active_items(&snapshot), vec![GLIMMER_ITEM]);

        snapshot = advance_window_snapshot(Some(&snapshot), &event_with(0, 0, 0), 6_500);
        assert!(active_items(&snapshot).is_empty());
    }

    #[test]
    fn dropping_the_item_closes_its_window() {
        let previous = seed(&event_with(0, 0, 0));
        let mut snapshot = advance_window_snapshot(Some(&previous), &event_with(13, 0, 0), 1_000);
        assert_eq!(active_items(&snapshot), vec![GLIMMER_ITEM]);

        let mut sold = event_with(12, 0, 0);
        sold.items.slot0 = Item::default();
        snapshot = advance_window_snapshot(Some(&snapshot), &sold, 2_000);

        assert!(active_items(&snapshot).is_empty());
    }

    #[test]
    fn dying_closes_every_window() {
        let previous = seed(&event_with(0, 0, 0));
        let mut snapshot = advance_window_snapshot(Some(&previous), &event_with(13, 20, 0), 1_000);
        assert_eq!(active_items(&snapshot), vec![GHOST_ITEM, GLIMMER_ITEM]);

        let mut dead = event_with(12, 19, 0);
        dead.hero.alive = false;
        snapshot = advance_window_snapshot(Some(&snapshot), &dead, 2_000);

        assert!(active_items(&snapshot).is_empty());
    }

    #[test]
    fn switching_hero_closes_every_window() {
        let previous = seed(&event_with(0, 0, 0));
        let mut snapshot = advance_window_snapshot(Some(&previous), &event_with(13, 0, 0), 1_000);
        assert_eq!(active_items(&snapshot), vec![GLIMMER_ITEM]);

        let mut other_hero = event_with(12, 0, 0);
        other_hero.hero.name = "npc_dota_hero_riki".to_string();
        snapshot = advance_window_snapshot(Some(&snapshot), &other_hero, 2_000);

        assert!(active_items(&snapshot).is_empty());
    }

    #[test]
    fn an_item_already_on_cooldown_at_first_sight_does_not_open_a_window() {
        let snapshot = advance_window_snapshot(None, &event_with(13, 20, 0), 1_000);

        assert!(active_items(&snapshot).is_empty());
    }

    #[test]
    fn an_item_not_in_the_inventory_never_opens_a_window() {
        let mut empty = event_with(0, 0, 0);
        empty.items.slot0 = Item::default();
        empty.items.slot1 = Item::default();

        let previous = seed(&empty);
        let snapshot = advance_window_snapshot(Some(&previous), &empty, 1_000);

        assert!(active_items(&snapshot).is_empty());
    }

    #[test]
    fn every_source_has_a_usable_duration() {
        assert!(WINDOW_SOURCES
            .iter()
            .all(|source| source.duration_seconds > 0));
        assert!(WINDOW_SOURCES
            .iter()
            .any(|source| source.item_name == GLIMMER_ITEM));
        assert!(WINDOW_SOURCES
            .iter()
            .any(|source| source.item_name == GHOST_ITEM));
    }
}
