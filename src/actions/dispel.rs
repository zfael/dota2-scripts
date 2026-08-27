//! Auto-dispel module
//!
//! Automatically uses dispel items (Manta Style, Lotus Orb) when silenced.
//! Triggers with random jitter for human-like reaction, and keeps watching the
//! silence until a press actually lands rather than firing once and giving up.

use crate::actions::executor::ActionExecutor;
use crate::config::Settings;
use crate::models::gsi_event::Item;
use crate::models::GsiWebhookEvent;
use lazy_static::lazy_static;
use rand::Rng;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tracing::{info, warn};

/// How long a press is given to show up in GSI — as the pressed item going on
/// cooldown — before it is treated as lost and pressed again.
const PRESS_SETTLE_MS: u64 = 600;

/// Presses allowed per silence. Every press only happens on a tick where the
/// item is castable and no cast lock is up, so four losses in a row means the
/// key is not reaching the game and retrying harder would just spam it.
const MAX_PRESSES_PER_SILENCE: u8 = 4;

/// Gap between the two taps that make Lotus Orb self-cast.
const SELF_CAST_DELAY_MS: u64 = 30;

lazy_static! {
    /// Bookkeeping for the silence currently being dispelled.
    static ref DISPEL_STATE: Mutex<DispelState> = Mutex::new(DispelState::default());
}

/// The two items this module knows how to dispel with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DispelItem {
    Manta,
    Lotus,
}

impl DispelItem {
    fn item_name(self) -> &'static str {
        match self {
            DispelItem::Manta => "item_manta",
            DispelItem::Lotus => "item_lotus_orb",
        }
    }

    fn job_label(self) -> &'static str {
        match self {
            DispelItem::Manta => "manta-dispel",
            DispelItem::Lotus => "lotus-dispel",
        }
    }

    fn log_name(self) -> &'static str {
        match self {
            DispelItem::Manta => "🌀 Manta Style",
            DispelItem::Lotus => "🪷 Lotus Orb",
        }
    }

    /// Lotus is a targeted item, so the self-cast is a double tap. Manta is not.
    fn needs_self_cast_tap(self) -> bool {
        matches!(self, DispelItem::Lotus)
    }
}

/// What the dispel watcher should do with the current payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DispelDecision<'a> {
    /// Nothing to dispel — forget the current silence.
    Idle,
    /// Silenced, but no press is due this tick. The silence stays tracked so the
    /// dispel can still fire later in the same silence.
    Hold,
    /// Silenced, castable, and nothing blocking the order — press this slot.
    Cast { slot: &'a str, item: DispelItem },
}

/// What has been tried against the silence currently in progress.
#[derive(Debug, Default, Clone, Copy)]
struct DispelState {
    /// The item last pressed for this silence, and when the press was queued.
    last_press: Option<(DispelItem, u64)>,
    presses: u8,
    /// One "slot has no key bound" warning per silence instead of one per tick.
    unbound_warned: bool,
}

/// Check and use dispel items (Manta/Lotus) if silenced (called every GSI event)
pub fn check_and_dispel_silence(
    event: &GsiWebhookEvent,
    settings: &Settings,
    executor: &Arc<ActionExecutor>,
) {
    // Silence does not break invisibility, but Manta and Lotus both would. A
    // silenced hero can still walk away invisible, so the dispel is worth less
    // than the escape it would cost.
    let invisible = crate::actions::invisibility::suppresses_automation(settings);
    let now_ms = current_time_millis();

    let mut state = DISPEL_STATE.lock().unwrap();

    let decision = plan_dispel(
        event,
        settings.danger_detection.auto_manta_on_silence,
        settings.danger_detection.auto_lotus_on_silence,
        invisible,
        &state,
        now_ms,
    );

    match decision {
        DispelDecision::Idle => *state = DispelState::default(),
        DispelDecision::Hold => {}
        DispelDecision::Cast { slot, item } => {
            let Some(key) = settings.get_key_for_slot(slot) else {
                if !state.unbound_warned {
                    state.unbound_warned = true;
                    warn!(
                        "{} sits in {} but that slot has no key bound — cannot dispel the silence",
                        item.log_name(),
                        slot
                    );
                }
                return;
            };

            state.last_press = Some((item, now_ms));
            state.presses += 1;

            let attempt = state.presses;
            let jitter = rand::rng().random_range(30..100);
            executor.enqueue_after(item.job_label(), Duration::from_millis(jitter), move || {
                info!(
                    "{} (silenced, attempt {}, jitter {}ms)",
                    item.log_name(),
                    attempt,
                    jitter
                );
                press_dispel_item(key, item);
            });
        }
    }
}

/// Decide what to do about the silence from one payload.
///
/// Split out from the state bookkeeping so the gating is testable without
/// touching global state or the clock.
fn plan_dispel<'a>(
    event: &'a GsiWebhookEvent,
    manta_enabled: bool,
    lotus_enabled: bool,
    invisible: bool,
    state: &DispelState,
    now_ms: u64,
) -> DispelDecision<'a> {
    if !manta_enabled && !lotus_enabled {
        return DispelDecision::Idle;
    }

    if !event.hero.is_alive() || !event.hero.silenced {
        return DispelDecision::Idle;
    }

    if invisible {
        return DispelDecision::Hold;
    }

    // Dota drops item orders issued through a stun, a hex, or a mute, so a press
    // now is a press thrown away. `Hold` rather than `Idle`: the silence is still
    // there, and the dispel should fire on the first tick the lock lifts. This is
    // the case that used to latch the module off for the rest of the silence —
    // getting stunned while silenced burned the one allowed trigger on an order
    // the game never accepted.
    if event.hero.stunned || event.hero.hexed || event.hero.muted {
        return DispelDecision::Hold;
    }

    if let Some((pressed, pressed_at_ms)) = state.last_press {
        // The pressed item going on cooldown is the only confirmation GSI offers
        // that the cast actually happened. If it did and the hero is *still*
        // silenced, the silence is not dispellable (Doom and other strong
        // debuffs), so stop rather than burn the second item on it too.
        if ready_slot(event, pressed).is_none() {
            return DispelDecision::Hold;
        }

        // Still ready, but the press may simply not have reached GSI yet.
        if now_ms.saturating_sub(pressed_at_ms) < PRESS_SETTLE_MS {
            return DispelDecision::Hold;
        }
    }

    if state.presses >= MAX_PRESSES_PER_SILENCE {
        return DispelDecision::Hold;
    }

    // Manta is instant where Lotus has a cast point, so it wins across the whole
    // inventory rather than only within a slot.
    for (enabled, item) in [
        (manta_enabled, DispelItem::Manta),
        (lotus_enabled, DispelItem::Lotus),
    ] {
        if !enabled {
            continue;
        }

        if let Some(slot) = ready_slot(event, item) {
            return DispelDecision::Cast { slot, item };
        }
    }

    DispelDecision::Hold
}

/// The slot holding this item, if it is there and castable right now.
fn ready_slot(event: &GsiWebhookEvent, item: DispelItem) -> Option<&str> {
    event
        .items
        .all_slots()
        .into_iter()
        .find(|(_, candidate)| candidate.name == item.item_name() && is_castable(candidate))
        .map(|(slot, _)| slot)
}

fn is_castable(item: &Item) -> bool {
    item.can_cast.unwrap_or(false) && item.cooldown.unwrap_or(0) == 0
}

fn press_dispel_item(key: char, item: DispelItem) {
    crate::input::simulation::press_key(key);

    if item.needs_self_cast_tap() {
        thread::sleep(Duration::from_millis(SELF_CAST_DELAY_MS));
        crate::input::simulation::press_key(key);
    }
}

fn current_time_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::gsi_event::{Abilities, Hero, Items, Map};

    fn ready_item(name: &str) -> Item {
        Item {
            name: name.to_string(),
            can_cast: Some(true),
            cooldown: Some(0),
            ..Item::default()
        }
    }

    fn item_on_cooldown(name: &str) -> Item {
        Item {
            name: name.to_string(),
            can_cast: Some(false),
            cooldown: Some(25),
            ..Item::default()
        }
    }

    fn event(hero: Hero, items: Items) -> GsiWebhookEvent {
        GsiWebhookEvent {
            hero,
            abilities: Abilities::default(),
            items,
            map: Map { clock_time: 600, ..Default::default() },
            player: None,
        }
    }

    fn silenced_hero() -> Hero {
        Hero {
            alive: true,
            silenced: true,
            ..Hero::default()
        }
    }

    /// Manta in slot0, Lotus in slot1, both ready.
    fn both_items_ready() -> Items {
        Items {
            slot0: ready_item("item_manta"),
            slot1: ready_item("item_lotus_orb"),
            ..Items::default()
        }
    }

    fn plan<'a>(
        event: &'a GsiWebhookEvent,
        state: &DispelState,
        now_ms: u64,
    ) -> DispelDecision<'a> {
        plan_dispel(event, true, true, false, state, now_ms)
    }

    #[test]
    fn silence_with_a_ready_manta_casts_it() {
        let payload = event(silenced_hero(), both_items_ready());

        assert_eq!(
            plan(&payload, &DispelState::default(), 1_000),
            DispelDecision::Cast {
                slot: "slot0",
                item: DispelItem::Manta,
            }
        );
    }

    #[test]
    fn manta_wins_over_lotus_sitting_in_an_earlier_slot() {
        let items = Items {
            slot0: ready_item("item_lotus_orb"),
            slot4: ready_item("item_manta"),
            ..Items::default()
        };
        let payload = event(silenced_hero(), items);

        assert_eq!(
            plan(&payload, &DispelState::default(), 1_000),
            DispelDecision::Cast {
                slot: "slot4",
                item: DispelItem::Manta,
            }
        );
    }

    #[test]
    fn lotus_is_used_when_manta_is_on_cooldown() {
        let items = Items {
            slot0: item_on_cooldown("item_manta"),
            slot1: ready_item("item_lotus_orb"),
            ..Items::default()
        };
        let payload = event(silenced_hero(), items);

        assert_eq!(
            plan(&payload, &DispelState::default(), 1_000),
            DispelDecision::Cast {
                slot: "slot1",
                item: DispelItem::Lotus,
            }
        );
    }

    #[test]
    fn no_silence_clears_the_episode() {
        let payload = event(
            Hero {
                alive: true,
                ..Hero::default()
            },
            both_items_ready(),
        );

        assert_eq!(
            plan(&payload, &DispelState::default(), 1_000),
            DispelDecision::Idle
        );
    }

    #[test]
    fn both_toggles_off_stays_idle() {
        let payload = event(silenced_hero(), both_items_ready());

        assert_eq!(
            plan_dispel(
                &payload,
                false,
                false,
                false,
                &DispelState::default(),
                1_000
            ),
            DispelDecision::Idle
        );
    }

    #[test]
    fn invisibility_holds_the_silence_instead_of_dropping_it() {
        let payload = event(silenced_hero(), both_items_ready());

        assert_eq!(
            plan_dispel(&payload, true, true, true, &DispelState::default(), 1_000),
            DispelDecision::Hold
        );
    }

    #[test]
    fn a_cast_lock_holds_rather_than_spending_the_press() {
        for lock in ["stunned", "hexed", "muted"] {
            let mut hero = silenced_hero();
            match lock {
                "stunned" => hero.stunned = true,
                "hexed" => hero.hexed = true,
                _ => hero.muted = true,
            }

            let payload = event(hero, both_items_ready());

            assert_eq!(
                plan(&payload, &DispelState::default(), 1_000),
                DispelDecision::Hold,
                "{lock} should hold"
            );
        }
    }

    #[test]
    fn stun_during_a_silence_still_dispels_once_it_lifts() {
        let stunned = event(
            Hero {
                stunned: true,
                ..silenced_hero()
            },
            both_items_ready(),
        );
        let state = DispelState::default();
        assert_eq!(plan(&stunned, &state, 1_000), DispelDecision::Hold);

        // Nothing was spent while stunned, so the first free tick casts — even
        // seconds into the silence.
        let free = event(silenced_hero(), both_items_ready());
        assert_eq!(
            plan(&free, &state, 4_000),
            DispelDecision::Cast {
                slot: "slot0",
                item: DispelItem::Manta,
            }
        );
    }

    #[test]
    fn a_queued_press_is_not_repeated_inside_the_settle_window() {
        let payload = event(silenced_hero(), both_items_ready());
        let state = DispelState {
            last_press: Some((DispelItem::Manta, 1_000)),
            presses: 1,
            unbound_warned: false,
        };

        assert_eq!(
            plan(&payload, &state, 1_000 + PRESS_SETTLE_MS - 1),
            DispelDecision::Hold
        );
    }

    #[test]
    fn a_press_that_never_landed_is_retried_after_the_settle_window() {
        let payload = event(silenced_hero(), both_items_ready());
        let state = DispelState {
            last_press: Some((DispelItem::Manta, 1_000)),
            presses: 1,
            unbound_warned: false,
        };

        assert_eq!(
            plan(&payload, &state, 1_000 + PRESS_SETTLE_MS),
            DispelDecision::Cast {
                slot: "slot0",
                item: DispelItem::Manta,
            }
        );
    }

    #[test]
    fn a_landed_press_stops_the_episode_instead_of_burning_the_second_item() {
        // Manta went on cooldown, so the press landed. The hero is still
        // silenced, which means this silence cannot be dispelled — Lotus would
        // be thrown away for nothing.
        let items = Items {
            slot0: item_on_cooldown("item_manta"),
            slot1: ready_item("item_lotus_orb"),
            ..Items::default()
        };
        let payload = event(silenced_hero(), items);
        let state = DispelState {
            last_press: Some((DispelItem::Manta, 1_000)),
            presses: 1,
            unbound_warned: false,
        };

        assert_eq!(plan(&payload, &state, 10_000), DispelDecision::Hold);
    }

    #[test]
    fn retries_stop_once_the_press_budget_is_spent() {
        let payload = event(silenced_hero(), both_items_ready());
        let state = DispelState {
            last_press: Some((DispelItem::Manta, 1_000)),
            presses: MAX_PRESSES_PER_SILENCE,
            unbound_warned: false,
        };

        assert_eq!(
            plan(&payload, &state, 1_000 + PRESS_SETTLE_MS),
            DispelDecision::Hold
        );
    }

    #[test]
    fn dead_hero_clears_the_episode() {
        let payload = event(
            Hero {
                alive: false,
                silenced: true,
                ..Hero::default()
            },
            both_items_ready(),
        );

        assert_eq!(
            plan(&payload, &DispelState::default(), 1_000),
            DispelDecision::Idle
        );
    }
}
