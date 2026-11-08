//! Holds the player's left and right clicks while a self-cast double tap is in
//! flight, and replays them the moment it lands.
//!
//! A self-cast double tap is two presses of the same key: the first puts Dota in
//! targeting mode, the second resolves the cast on our own hero. A physical
//! click between them resolves or cancels that targeting instead — a right-click
//! cancels it and walks, a left-click casts on whatever is under the cursor — and
//! the second tap then arrives with nothing to finish, so it arms targeting again
//! and the item sits waiting for a click. Mid-fight the player right-clicks every
//! 100–150ms, so any gap at all catches one regularly.
//!
//! Only left and right are held: they are the buttons that resolve or cancel
//! targeting. Middle and the side buttons can be item bindings themselves, so a
//! double tap on a mouse-bound slot is never held up by its own presses.
//!
//! Replays are sent through `rdev::simulate` and recognised on the way back
//! through the hook, so they reach Dota instead of being held again. They skip
//! the rest of the hook: a click held during a double tap reaches Dota plain,
//! without the Phantom Lancer Ctrl wrap or the Broodmother Space+click remap.
//! One click, a few tens of milliseconds late, is the cost.

use lazy_static::lazy_static;
use rdev::{Button, EventType};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::{debug, warn};

/// A hold that has run this long is abandoned for new clicks: something stalled
/// the worker, and a mouse that stops working is worse than a missed self-cast.
/// Clicks already held are still replayed when the hold ends.
const MAX_HOLD: Duration = Duration::from_millis(500);

/// A replay that never came back through the hook is forgotten after this, so a
/// lost echo cannot let a later physical click slip past as one of ours.
const ECHO_TIMEOUT: Duration = Duration::from_millis(250);

/// What the hook should do with a mouse event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickVerdict {
    /// Not ours to decide: the rest of the hook handles it.
    Continue,
    /// One of our replays coming back: send it to Dota as is.
    PassReplay,
    /// Held until the double tap lands; the hook returns `None`.
    Hold,
}

#[derive(Debug, Default)]
struct ClickHold {
    holding_since: Option<Instant>,
    held: VecDeque<EventType>,
    expected_echoes: VecDeque<(EventType, Instant)>,
}

impl ClickHold {
    fn begin(&mut self, now: Instant) {
        self.holding_since = Some(now);
    }

    fn on_event(&mut self, event: &EventType, now: Instant) -> ClickVerdict {
        if !is_held_button(event) {
            return ClickVerdict::Continue;
        }

        while self
            .expected_echoes
            .front()
            .is_some_and(|(_, sent_at)| now.saturating_duration_since(*sent_at) > ECHO_TIMEOUT)
        {
            self.expected_echoes.pop_front();
        }
        if self
            .expected_echoes
            .front()
            .is_some_and(|(sent, _)| sent == event)
        {
            self.expected_echoes.pop_front();
            return ClickVerdict::PassReplay;
        }

        match self.holding_since {
            Some(since) if now.saturating_duration_since(since) < MAX_HOLD => {
                self.held.push_back(*event);
                ClickVerdict::Hold
            }
            _ => ClickVerdict::Continue,
        }
    }

    /// The clicks to replay next, recorded as expected echoes. `None` once
    /// nothing is left, which is also what ends the hold — so a click arriving
    /// while a batch is replaying is held and goes out in the next batch, after
    /// the ones before it, instead of overtaking them.
    fn take_for_replay(&mut self, now: Instant) -> Option<Vec<EventType>> {
        if self.held.is_empty() {
            self.holding_since = None;
            return None;
        }

        let batch: Vec<EventType> = self.held.drain(..).collect();
        self.expected_echoes
            .extend(batch.iter().map(|event| (*event, now)));
        Some(batch)
    }
}

fn is_held_button(event: &EventType) -> bool {
    matches!(
        event,
        EventType::ButtonPress(Button::Left | Button::Right)
            | EventType::ButtonRelease(Button::Left | Button::Right)
    )
}

lazy_static! {
    static ref CLICK_HOLD: Mutex<ClickHold> = Mutex::new(ClickHold::default());
}

/// Fast path for the hook: nothing to look at unless a hold or a replay is live.
static ENGAGED: AtomicBool = AtomicBool::new(false);

/// Called from the hook for every event, before anything else decides it.
pub fn intercept(event: &EventType) -> ClickVerdict {
    if !ENGAGED.load(Ordering::SeqCst) || !is_held_button(event) {
        return ClickVerdict::Continue;
    }

    let mut hold = CLICK_HOLD.lock().unwrap();
    let verdict = hold.on_event(event, Instant::now());
    if verdict == ClickVerdict::Hold {
        debug!("🖱️ Holding {:?} until the self-cast lands", event);
    }
    sync_engaged(&hold);
    verdict
}

/// Hold clicks until the returned guard drops. The guard replays them, so an
/// early return or a panic inside the double tap cannot strand a click.
pub fn hold_clicks() -> ClickHoldGuard {
    let mut hold = CLICK_HOLD.lock().unwrap();
    hold.begin(Instant::now());
    sync_engaged(&hold);
    ClickHoldGuard
}

pub struct ClickHoldGuard;

impl Drop for ClickHoldGuard {
    fn drop(&mut self) {
        loop {
            let batch = {
                let mut hold = CLICK_HOLD.lock().unwrap();
                let batch = hold.take_for_replay(Instant::now());
                sync_engaged(&hold);
                batch
            };
            let Some(batch) = batch else {
                return;
            };

            debug!("🖱️ Replaying {} held click event(s)", batch.len());
            for event in batch {
                if let Err(error) = rdev::simulate(&event) {
                    warn!("🖱️ Could not replay held {:?}: {:?}", event, error);
                }
            }
        }
    }
}

fn sync_engaged(hold: &ClickHold) {
    ENGAGED.store(
        hold.holding_since.is_some() || !hold.expected_echoes.is_empty(),
        Ordering::SeqCst,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    const RIGHT_DOWN: EventType = EventType::ButtonPress(Button::Right);
    const RIGHT_UP: EventType = EventType::ButtonRelease(Button::Right);
    const LEFT_DOWN: EventType = EventType::ButtonPress(Button::Left);

    #[test]
    fn nothing_is_held_outside_a_double_tap() {
        let mut hold = ClickHold::default();
        assert_eq!(
            hold.on_event(&RIGHT_DOWN, Instant::now()),
            ClickVerdict::Continue
        );
    }

    #[test]
    fn clicks_during_a_double_tap_are_held_in_order() {
        let mut hold = ClickHold::default();
        let start = Instant::now();
        hold.begin(start);

        assert_eq!(hold.on_event(&RIGHT_DOWN, start), ClickVerdict::Hold);
        assert_eq!(hold.on_event(&RIGHT_UP, start), ClickVerdict::Hold);

        assert_eq!(hold.take_for_replay(start), Some(vec![RIGHT_DOWN, RIGHT_UP]));
    }

    #[test]
    fn middle_and_side_buttons_are_never_held() {
        let mut hold = ClickHold::default();
        let start = Instant::now();
        hold.begin(start);

        for event in [
            EventType::ButtonPress(Button::Middle),
            EventType::ButtonPress(Button::Unknown(1)),
            EventType::KeyPress(rdev::Key::KeyQ),
        ] {
            assert_eq!(hold.on_event(&event, start), ClickVerdict::Continue);
        }
    }

    #[test]
    fn replays_pass_instead_of_being_held_again() {
        let mut hold = ClickHold::default();
        let start = Instant::now();
        hold.begin(start);
        hold.on_event(&RIGHT_DOWN, start);

        let batch = hold.take_for_replay(start).unwrap();
        assert_eq!(batch, vec![RIGHT_DOWN]);

        // Still holding while the batch is in flight; the echo is recognised.
        assert_eq!(hold.on_event(&RIGHT_DOWN, start), ClickVerdict::PassReplay);
    }

    #[test]
    fn a_click_during_replay_waits_for_the_next_batch() {
        let mut hold = ClickHold::default();
        let start = Instant::now();
        hold.begin(start);
        hold.on_event(&RIGHT_DOWN, start);
        hold.take_for_replay(start).unwrap();

        // The physical release arrives before our replayed press came back:
        // it must not overtake it.
        assert_eq!(hold.on_event(&RIGHT_UP, start), ClickVerdict::Hold);
        assert_eq!(hold.on_event(&RIGHT_DOWN, start), ClickVerdict::PassReplay);

        assert_eq!(hold.take_for_replay(start), Some(vec![RIGHT_UP]));
    }

    #[test]
    fn an_empty_replay_ends_the_hold() {
        let mut hold = ClickHold::default();
        let start = Instant::now();
        hold.begin(start);

        assert_eq!(hold.take_for_replay(start), None);
        assert_eq!(hold.on_event(&LEFT_DOWN, start), ClickVerdict::Continue);
    }

    #[test]
    fn a_stalled_hold_stops_eating_new_clicks() {
        let mut hold = ClickHold::default();
        let start = Instant::now();
        hold.begin(start);

        assert_eq!(
            hold.on_event(&LEFT_DOWN, start + MAX_HOLD),
            ClickVerdict::Continue
        );
    }

    #[test]
    fn a_lost_echo_expires_rather_than_swallowing_a_later_click() {
        let mut hold = ClickHold::default();
        let start = Instant::now();
        hold.begin(start);
        hold.on_event(&RIGHT_DOWN, start);
        hold.take_for_replay(start).unwrap();
        hold.take_for_replay(start);

        let later = start + ECHO_TIMEOUT + Duration::from_millis(1);
        assert_eq!(hold.on_event(&RIGHT_DOWN, later), ClickVerdict::Continue);
    }
}
