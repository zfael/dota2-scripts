use crate::actions::common::SurvivabilityActions;
use crate::actions::executor::ActionExecutor;
use crate::actions::heroes::HeroScript;
use crate::config::settings::SlarkConfig;
use crate::config::Settings;
use crate::models::{GsiWebhookEvent, Hero};
use lazy_static::lazy_static;
use std::sync::{mpsc, Arc, LazyLock, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

const POUNCE_ABILITY_NAME: &str = "slark_pounce";
const DARK_PACT_ABILITY_NAME: &str = "slark_dark_pact";
const SHADOW_DANCE_ABILITY_NAME: &str = "slark_shadow_dance";

/// The ability Aghanim's Shard grants.
///
/// A constant rather than config: it is as fixed as the other three ability
/// names above, and matching by name is the only thing that works — see
/// [`ability_is_ready`].
const DEPTH_SHROUD_ABILITY_NAME: &str = "slark_depth_shroud";

/// Settle time before the facing right-click, matching the other facing combos.
const PRE_TURN_SETTLE_MS: u64 = 50;

/// Gap between the facing right-click and releasing ALT.
const ALT_RELEASE_DELAY_MS: u64 = 50;

lazy_static! {
    static ref SLARK_LAST_EVENT: Arc<Mutex<Option<GsiWebhookEvent>>> = Arc::new(Mutex::new(None));
    /// When the current run of debuffs was first seen, for the Dark Pact
    /// settle window. `None` means Slark is clean.
    static ref SLARK_DEBUFF_DETECTED: Mutex<Option<Instant>> = Mutex::new(None);
    /// Last time each escape fired, for their own retry cooldowns.
    static ref SLARK_LAST_ESCAPE: Mutex<EscapeCooldowns> = Mutex::new(EscapeCooldowns::default());
    /// Throttle for the "wanted an escape, got none" explanation.
    static ref SLARK_LAST_ESCAPE_DIAGNOSTIC: Mutex<Option<Instant>> = Mutex::new(None);
}

/// How often the escape diagnostic may repeat while the situation persists.
const ESCAPE_DIAGNOSTIC_INTERVAL_MS: u64 = 3_000;

/// Work item for the dedicated Slark worker thread.
#[derive(Debug, PartialEq, Eq)]
enum SlarkRequest {
    DirectionalPounce { pounce_key: char, turn_delay_ms: u64 },
}

fn build_directional_pounce_request(pounce_key: char, turn_delay_ms: u64) -> SlarkRequest {
    SlarkRequest::DirectionalPounce {
        pounce_key,
        turn_delay_ms,
    }
}

static SLARK_REQUEST_QUEUE: LazyLock<mpsc::Sender<SlarkRequest>> = LazyLock::new(|| {
    let (tx, rx) = mpsc::channel::<SlarkRequest>();

    thread::spawn(move || {
        info!("🐟 Slark request worker started");

        while let Ok(request) = rx.recv() {
            run_slark_request(request);
        }

        info!("🐟 Slark request worker exited");
    });

    tx
});

fn run_slark_request(request: SlarkRequest) {
    match request {
        SlarkRequest::DirectionalPounce {
            pounce_key,
            turn_delay_ms,
        } => run_directional_pounce_request(pounce_key, turn_delay_ms),
    }
}

/// Directional Pounce.
///
/// Pounce leaps along Slark's facing at cast time, so turning toward the cursor
/// first is what decides where the leash lands. Same shape as the Magnus
/// directional ultimate.
///
/// ALT is released before the ability press — Pounce takes no target, and
/// holding ALT over an ability key pings it to allies instead of casting it.
fn run_directional_pounce_request(pounce_key: char, turn_delay_ms: u64) {
    thread::sleep(Duration::from_millis(PRE_TURN_SETTLE_MS));

    crate::input::simulation::alt_down();
    crate::input::simulation::mouse_click();

    thread::sleep(Duration::from_millis(ALT_RELEASE_DELAY_MS));
    crate::input::simulation::alt_up();

    thread::sleep(Duration::from_millis(turn_delay_ms));
    crate::input::simulation::press_key(pounce_key);
}

fn spawn_slark_fallback(request: SlarkRequest) {
    thread::spawn(move || {
        run_slark_request(request);
    });
}

fn enqueue_slark_request(request: SlarkRequest) {
    if let Err(err) = SLARK_REQUEST_QUEUE.send(request) {
        warn!("🐟 Slark request queue unavailable; using fallback thread");
        spawn_slark_fallback(err.0);
    }
}

/// Whether a named ability is levelled and castable right now.
///
/// Scans every slot rather than indexing one, because **GSI slot order is
/// ability order, not key order**. Shard- and scepter-granted abilities are
/// inserted before the ultimate, so on a shard Slark `slark_depth_shroud` sits
/// at index 3 and `slark_shadow_dance` — the R ability — moves to index 4.
/// Deriving a slot from the key it is bound to therefore reads the wrong
/// ability, which is why the shard fallback silently checked the ultimate's
/// cooldown instead of its own.
fn ability_is_ready(event: &GsiWebhookEvent, ability_name: &str) -> bool {
    (0..=5).any(|index| {
        event
            .abilities
            .get_by_index(index)
            .is_some_and(|ability| {
                ability.name == ability_name && ability.level > 0 && ability.can_cast
            })
    })
}

/// What the debuff watcher should do with the current payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DarkPactDecision {
    /// Nothing to cleanse — drop any pending settle window.
    Idle,
    /// Debuffed, but Dark Pact cannot be cast right now. The settle window
    /// keeps running so the cleanse fires the moment it becomes castable.
    Hold,
    /// Debuffed and castable — start or finish the settle window.
    Arm,
}

/// Decide what to do about Slark's debuffs from one payload.
///
/// Split out from the timer bookkeeping so the gating is testable without
/// touching global state or the clock.
fn plan_dark_pact(event: &GsiWebhookEvent, enabled: bool) -> DarkPactDecision {
    if !enabled || !event.hero.is_alive() || !event.hero.has_debuff {
        return DarkPactDecision::Idle;
    }

    // Dark Pact cannot be cast through any of these, but whatever else is on
    // Slark is still worth cleansing the moment the lock lifts — so hold the
    // window rather than dropping it.
    if event.hero.stunned || event.hero.hexed || event.hero.silenced {
        return DarkPactDecision::Hold;
    }

    if !ability_is_ready(event, DARK_PACT_ABILITY_NAME) {
        return DarkPactDecision::Hold;
    }

    DarkPactDecision::Arm
}

/// When each escape last fired.
///
/// Tracked separately because they are separate resources. A single shared
/// timer meant spending Shadow Dance also locked out the shard for the same
/// window — precisely when the fallback is wanted, since the ultimate being
/// down is its entire trigger condition.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct EscapeCooldowns {
    shadow_dance: Option<Instant>,
    shard: Option<Instant>,
}

/// Whether enough time has passed since an escape last fired.
fn off_retry_cooldown(last: Option<Instant>, now: Instant, cooldown_ms: u64) -> bool {
    match last {
        Some(last) => now.duration_since(last) >= Duration::from_millis(cooldown_ms),
        None => true,
    }
}

/// Which escape the low-HP watcher should spend, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LowHpEscape {
    /// Hold everything — not in danger, above the line, or nothing castable.
    None,
    /// Shadow Dance is up. Always preferred: it is the stronger escape.
    ShadowDance,
    /// Shadow Dance is down but the shard ability is up.
    Shard,
}

/// Whether the hero can issue a cast at all this instant.
///
/// Deliberately excludes `has_debuff`: most debuffs do not stop a cast, and
/// being debuffed is usually *why* the escape is wanted.
fn can_cast_now(event: &GsiWebhookEvent) -> bool {
    event.hero.is_alive() && !event.hero.stunned && !event.hero.hexed && !event.hero.silenced
}

/// Pick the low-HP escape for one payload.
///
/// With `shadow_dance_require_danger` on — the default — this needs the danger
/// detector *and* the HP line, matching the Outworld Destroyer barrier.
/// `in_danger` already means "losing HP or lost a burst of it", so the pair
/// reads as "low and actually under fire", and sitting at 30% in the fountain
/// never spends the ultimate. Off, the HP line alone is enough.
fn plan_low_hp_escape(
    event: &GsiWebhookEvent,
    config: &SlarkConfig,
    in_danger: bool,
    now: Instant,
    cooldowns: EscapeCooldowns,
) -> LowHpEscape {
    if !config.auto_shadow_dance_on_low_hp {
        return LowHpEscape::None;
    }

    if config.shadow_dance_require_danger && !in_danger {
        return LowHpEscape::None;
    }

    if !can_cast_now(event) {
        return LowHpEscape::None;
    }

    if event.hero.health_percent > config.shadow_dance_hp_threshold_percent {
        return LowHpEscape::None;
    }

    let retry_ms = config.shadow_dance_trigger_cooldown_ms;

    if ability_is_ready(event, SHADOW_DANCE_ABILITY_NAME)
        && off_retry_cooldown(cooldowns.shadow_dance, now, retry_ms)
    {
        return LowHpEscape::ShadowDance;
    }

    // The shard is the consolation prize, so it is only reached once Shadow
    // Dance has already been ruled out — and it carries its own retry timer, so
    // spending the ultimate does not lock it out for the same window.
    if config.shard_fallback_enabled
        && event.hero.aghanims_shard
        && ability_is_ready(event, DEPTH_SHROUD_ABILITY_NAME)
        && off_retry_cooldown(cooldowns.shard, now, retry_ms)
    {
        return LowHpEscape::Shard;
    }

    LowHpEscape::None
}

/// Explain why a wanted escape did not happen.
///
/// `plan_low_hp_escape` collapses every reason down to one `None`, which makes
/// "the shard never fires" impossible to diagnose from outside — a danger flag
/// that cleared looks exactly like a shard ability sitting in a slot we did not
/// expect. This reports the whole decision, throttled so a sustained low-HP
/// fight cannot flood the log.
///
/// The full ability dump is the point: it is the only way to find out where
/// Dota actually puts a shard-granted ability, and what it reports for `level`
/// and `can_cast` on one.
fn explain_missing_escape(
    event: &GsiWebhookEvent,
    config: &SlarkConfig,
    in_danger: bool,
    cooldowns: EscapeCooldowns,
    now: Instant,
) {
    let Ok(mut last_logged) = SLARK_LAST_ESCAPE_DIAGNOSTIC.try_lock() else {
        return;
    };

    if let Some(logged_at) = *last_logged {
        if now.duration_since(logged_at) < Duration::from_millis(ESCAPE_DIAGNOSTIC_INTERVAL_MS) {
            return;
        }
    }
    *last_logged = Some(now);

    let remaining = |last: Option<Instant>| {
        last.map(|last| {
            Duration::from_millis(config.shadow_dance_trigger_cooldown_ms)
                .saturating_sub(now.duration_since(last))
                .as_millis()
        })
        .unwrap_or(0)
    };

    let abilities: Vec<String> = (0..=5)
        .filter_map(|index| event.abilities.get_by_index(index).map(|a| (index, a)))
        .map(|(index, ability)| {
            format!(
                "{}={} (lvl {}, can_cast {})",
                index, ability.name, ability.level, ability.can_cast
            )
        })
        .collect();

    warn!(
        "🐟 Low HP escape wanted but nothing fired. hp={}% (line {}%), in_danger={} \
         (required {}), stunned={}, hexed={}, silenced={}, \
         shadow_dance_ready={} (retry in {}ms), shard_fallback_enabled={}, \
         aghanims_shard={}, {}_ready={} (retry in {}ms), shard_key='{}'. Abilities: [{}]",
        event.hero.health_percent,
        config.shadow_dance_hp_threshold_percent,
        in_danger,
        config.shadow_dance_require_danger,
        event.hero.stunned,
        event.hero.hexed,
        event.hero.silenced,
        ability_is_ready(event, SHADOW_DANCE_ABILITY_NAME),
        remaining(cooldowns.shadow_dance),
        config.shard_fallback_enabled,
        event.hero.aghanims_shard,
        DEPTH_SHROUD_ABILITY_NAME,
        ability_is_ready(event, DEPTH_SHROUD_ABILITY_NAME),
        remaining(cooldowns.shard),
        config.shard_key,
        abilities.join(", ")
    );
}

pub struct SlarkState;

impl SlarkState {
    /// Whether the keyboard hook should swallow the Pounce key.
    ///
    /// Returns `false` when Pounce is unlevelled, on cooldown, or no GSI event
    /// has arrived yet. The hook then lets the key through untouched, so a
    /// cooldown press never issues the facing right-click and never walks a
    /// squishy carry toward the cursor.
    pub fn can_intercept_pounce() -> bool {
        let event = SLARK_LAST_EVENT.lock().unwrap().clone();

        let Some(event) = event else {
            info!("🐟 Slark pounce intercept skipped: no GSI event available");
            return false;
        };

        if !ability_is_ready(&event, POUNCE_ABILITY_NAME) {
            info!("🐟 Slark pounce intercept skipped: Pounce not ready");
            return false;
        }

        true
    }

    /// Run the directional pounce: ALT down → right-click to face cursor →
    /// ALT up → wait `turn_delay_ms` → cast Pounce.
    pub fn execute_directional_pounce(pounce_key: char, turn_delay_ms: u64) {
        enqueue_slark_request(build_directional_pounce_request(pounce_key, turn_delay_ms));
    }
}

/// Slark script.
///
/// Directional Pounce flow:
/// 1. keyboard.rs intercepts the Pounce key (default W) when Slark is the
///    active hero and Pounce is castable.
/// 2. Calls `SlarkState::execute_directional_pounce()`.
/// 3. The dedicated worker holds ALT, right-clicks to face the cursor, releases
///    ALT, waits, then casts Pounce.
pub struct SlarkScript {
    settings: Arc<Mutex<Settings>>,
    executor: Arc<ActionExecutor>,
}

impl SlarkScript {
    pub fn new(settings: Arc<Mutex<Settings>>, executor: Arc<ActionExecutor>) -> Self {
        Self { settings, executor }
    }

    /// Cast Dark Pact to shed debuffs.
    ///
    /// Dark Pact applies a basic dispel to Slark when it pulses, which is the
    /// cheapest cleanse he has. GSI only exposes a single `has_debuff` flag —
    /// there is no way to see *which* modifier landed — so this fires on any
    /// debuff at all. The settle window exists so a burst of debuffs from one
    /// engagement is cleansed by one cast instead of the first one spending it.
    fn dark_pact_cleanse(&self, event: &GsiWebhookEvent) {
        let settings = self.settings.lock().unwrap();
        let slark = &settings.heroes.slark;
        let enabled = slark.auto_dark_pact_on_debuff;
        let key = slark.dark_pact_key;
        let delay_ms = slark.dark_pact_delay_ms;
        drop(settings);

        let decision = plan_dark_pact(event, enabled);

        // try_lock: a contended tick is worth skipping, not blocking the GSI
        // handler for. The next payload is 0.1s away.
        let Ok(mut debuff_since) = SLARK_DEBUFF_DETECTED.try_lock() else {
            return;
        };

        match decision {
            DarkPactDecision::Idle => {
                if debuff_since.is_some() {
                    debug!("🐟 Slark is clean, dropping the Dark Pact settle window");
                    *debuff_since = None;
                }
            }
            DarkPactDecision::Hold => {
                if debuff_since.is_none() {
                    debug!("🐟 Debuff detected while Dark Pact is unavailable, holding");
                    *debuff_since = Some(Instant::now());
                }
            }
            DarkPactDecision::Arm => match *debuff_since {
                Some(first_seen) if first_seen.elapsed() >= Duration::from_millis(delay_ms) => {
                    info!(
                        "🐟 Dark Pact cleansing debuffs ({}ms settle window elapsed)",
                        delay_ms
                    );
                    crate::input::simulation::press_key(key);
                    *debuff_since = None;
                }
                Some(_) => {}
                None => {
                    debug!(
                        "🐟 Debuff detected, starting {}ms Dark Pact settle window",
                        delay_ms
                    );
                    *debuff_since = Some(Instant::now());
                }
            },
        }
    }

    /// Spend Shadow Dance — or the shard ability behind it — to survive.
    ///
    /// Shadow Dance is always preferred; the shard is only reached when the
    /// ultimate is on cooldown, which is the order asked for.
    fn low_hp_escape(&self, event: &GsiWebhookEvent, in_danger: bool) {
        let settings = self.settings.lock().unwrap();
        let config = settings.heroes.slark.clone();
        drop(settings);

        let now = Instant::now();

        // try_lock for the same reason as the Dark Pact window: never block the
        // GSI handler on a contended tick.
        let Ok(mut cooldowns) = SLARK_LAST_ESCAPE.try_lock() else {
            return;
        };

        match plan_low_hp_escape(event, &config, in_danger, now, *cooldowns) {
            LowHpEscape::None => {
                // Only when an escape was actually wanted — below the line and
                // alive with the feature on. Otherwise this fires on every
                // healthy payload, which is most of them.
                if config.auto_shadow_dance_on_low_hp
                    && event.hero.is_alive()
                    && event.hero.health_percent <= config.shadow_dance_hp_threshold_percent
                {
                    explain_missing_escape(event, &config, in_danger, *cooldowns, now);
                }
            }
            LowHpEscape::ShadowDance => {
                info!(
                    "🐟 Shadow Dance escape at {}% HP",
                    event.hero.health_percent
                );
                crate::input::simulation::press_key(config.shadow_dance_key);
                cooldowns.shadow_dance = Some(now);
            }
            LowHpEscape::Shard => {
                // Cast at the cursor rather than on Slark himself. Dota will
                // not self-cast this one, and a synthetic click on the HUD hero
                // portrait — which would target him — does not resolve the
                // cast. Mid-fight the cursor is already pointed somewhere
                // useful, so aiming there is the option that actually works.
                info!(
                    "🐟 Shadow Dance is down — falling back to {} on '{}' at {}% HP, cast at \
                     the cursor",
                    DEPTH_SHROUD_ABILITY_NAME, config.shard_key, event.hero.health_percent
                );
                crate::input::simulation::cast_at_cursor(config.shard_key);
                cooldowns.shard = Some(now);
            }
        }

    }
}

impl HeroScript for SlarkScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        {
            let mut last_event = SLARK_LAST_EVENT.lock().unwrap();
            *last_event = Some(event.clone());
        }

        // Debuff cleansing is the most time-sensitive thing on this path, so it
        // runs before the shared survivability checks.
        self.dark_pact_cleanse(event);

        let settings = self.settings.lock().unwrap();

        // Shared survivability only — the directional pounce is keyboard-driven.
        let survivability = SurvivabilityActions::new(self.settings.clone(), self.executor.clone());
        let in_danger = crate::actions::danger_detector::update(event, &settings.danger_detection);
        drop(settings);

        // Before the item checks: the ultimate is worth more than a salve.
        self.low_hp_escape(event, in_danger);

        survivability.check_and_use_healing_items_with_danger(event, in_danger);
        survivability.use_defensive_items_if_danger_with_snapshot(event, in_danger);
        survivability.use_neutral_item_if_danger_with_snapshot(event, in_danger);
    }

    fn handle_standalone_trigger(&self) {
        let settings = self.settings.lock().unwrap();
        let slark = &settings.heroes.slark;
        let pounce_key = slark.pounce_key;
        let turn_delay_ms = slark.turn_delay_ms;
        drop(settings);
        info!("🐟 Slark standalone directional pounce triggered");
        SlarkState::execute_directional_pounce(pounce_key, turn_delay_ms);
    }

    fn hero_name(&self) -> &'static str {
        Hero::Slark.to_game_name()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slark_fixture() -> GsiWebhookEvent {
        serde_json::from_str(include_str!("../../../tests/fixtures/slark_event.json"))
            .expect("Slark fixture should deserialize")
    }

    #[test]
    fn build_directional_pounce_request_preserves_key_and_delay() {
        let request = build_directional_pounce_request('w', 60);
        assert_eq!(
            request,
            SlarkRequest::DirectionalPounce {
                pounce_key: 'w',
                turn_delay_ms: 60,
            }
        );
    }

    #[test]
    fn finds_pounce_when_levelled_and_castable() {
        let event = slark_fixture();
        assert!(ability_is_ready(&event, POUNCE_ABILITY_NAME));
    }

    #[test]
    fn ability_on_cooldown_is_not_ready() {
        let mut event = slark_fixture();
        event.abilities.ability1.can_cast = false;

        assert!(!ability_is_ready(&event, POUNCE_ABILITY_NAME));
    }

    #[test]
    fn unlevelled_ability_is_not_ready() {
        let mut event = slark_fixture();
        event.abilities.ability1.level = 0;

        assert!(!ability_is_ready(&event, POUNCE_ABILITY_NAME));
    }

    #[test]
    fn unknown_ability_name_is_not_ready() {
        let event = slark_fixture();
        assert!(!ability_is_ready(&event, "slark_not_a_real_ability"));
    }

    /// The fixture is clean by default; debuff cases opt in.
    fn debuffed_fixture() -> GsiWebhookEvent {
        let mut event = slark_fixture();
        event.hero.has_debuff = true;
        event
    }

    #[test]
    fn dark_pact_arms_on_a_debuff_when_castable() {
        assert_eq!(
            plan_dark_pact(&debuffed_fixture(), true),
            DarkPactDecision::Arm
        );
    }

    #[test]
    fn dark_pact_is_idle_without_a_debuff() {
        assert_eq!(
            plan_dark_pact(&slark_fixture(), true),
            DarkPactDecision::Idle
        );
    }

    #[test]
    fn dark_pact_is_idle_when_the_toggle_is_off() {
        assert_eq!(
            plan_dark_pact(&debuffed_fixture(), false),
            DarkPactDecision::Idle
        );
    }

    #[test]
    fn dark_pact_is_idle_while_dead() {
        let mut event = debuffed_fixture();
        event.hero.alive = false;

        assert_eq!(plan_dark_pact(&event, true), DarkPactDecision::Idle);
    }

    #[test]
    fn dark_pact_holds_through_a_cast_lock() {
        for lock in ["stunned", "hexed", "silenced"] {
            let mut event = debuffed_fixture();
            match lock {
                "stunned" => event.hero.stunned = true,
                "hexed" => event.hero.hexed = true,
                _ => event.hero.silenced = true,
            }

            assert_eq!(
                plan_dark_pact(&event, true),
                DarkPactDecision::Hold,
                "{lock} should hold the settle window, not drop it"
            );
        }
    }

    #[test]
    fn dark_pact_holds_while_on_cooldown() {
        let mut event = debuffed_fixture();
        event.abilities.ability0.can_cast = false;

        assert_eq!(plan_dark_pact(&event, true), DarkPactDecision::Hold);
    }

    #[test]
    fn dark_pact_holds_while_unlevelled() {
        let mut event = debuffed_fixture();
        event.abilities.ability0.level = 0;

        assert_eq!(plan_dark_pact(&event, true), DarkPactDecision::Hold);
    }

    /// Low enough to be over the line, with the shard granted so both escape
    /// routes are on the table.
    fn low_hp_fixture() -> GsiWebhookEvent {
        let mut event = slark_fixture();
        event.hero.health_percent = 20;
        event.hero.aghanims_shard = true;
        event
    }

    /// Put Shadow Dance on cooldown.
    ///
    /// It lives at index **4** on a shard Slark, not 3: the shard ability is
    /// inserted ahead of the ultimate. Captured from a real payload — see the
    /// fixture.
    fn with_shadow_dance_down(event: &mut GsiWebhookEvent) {
        assert_eq!(event.abilities.ability4.name, SHADOW_DANCE_ABILITY_NAME);
        event.abilities.ability4.can_cast = false;
    }

    fn now() -> Instant {
        Instant::now()
    }

    #[test]
    fn shadow_dance_fires_when_low_and_in_danger() {
        assert_eq!(
            plan_low_hp_escape(
                &low_hp_fixture(),
                &SlarkConfig::default(),
                true,
                now(),
                EscapeCooldowns::default()
            ),
            LowHpEscape::ShadowDance
        );
    }

    #[test]
    fn nothing_fires_above_the_hp_line() {
        let mut event = low_hp_fixture();
        event.hero.health_percent = 80;

        assert_eq!(
            plan_low_hp_escape(
                &event,
                &SlarkConfig::default(),
                true,
                now(),
                EscapeCooldowns::default()
            ),
            LowHpEscape::None
        );
    }

    #[test]
    fn requiring_danger_holds_the_ultimate_when_merely_low() {
        // Low HP but not under fire — walking home, regenerating in lane.
        assert_eq!(
            plan_low_hp_escape(
                &low_hp_fixture(),
                &SlarkConfig::default(),
                false,
                now(),
                EscapeCooldowns::default()
            ),
            LowHpEscape::None
        );
    }

    #[test]
    fn without_requiring_danger_the_hp_line_alone_is_enough() {
        let config = SlarkConfig {
            shadow_dance_require_danger: false,
            ..SlarkConfig::default()
        };

        assert_eq!(
            plan_low_hp_escape(
                &low_hp_fixture(),
                &config,
                false,
                now(),
                EscapeCooldowns::default()
            ),
            LowHpEscape::ShadowDance
        );
    }

    #[test]
    fn the_shard_is_only_reached_once_shadow_dance_is_down() {
        let config = SlarkConfig::default();
        let mut event = low_hp_fixture();

        // Both up: the ultimate wins.
        assert_eq!(
            plan_low_hp_escape(&event, &config, true, now(), EscapeCooldowns::default()),
            LowHpEscape::ShadowDance
        );

        // Ultimate on cooldown: fall through to the shard.
        with_shadow_dance_down(&mut event);
        assert_eq!(
            plan_low_hp_escape(&event, &config, true, now(), EscapeCooldowns::default()),
            LowHpEscape::Shard
        );
    }

    #[test]
    fn the_shard_needs_the_shard_to_actually_be_granted() {
        let mut event = low_hp_fixture();
        with_shadow_dance_down(&mut event);
        event.hero.aghanims_shard = false;

        assert_eq!(
            plan_low_hp_escape(
                &event,
                &SlarkConfig::default(),
                true,
                now(),
                EscapeCooldowns::default()
            ),
            LowHpEscape::None
        );
    }

    #[test]
    fn the_shard_is_skipped_while_depth_shroud_itself_is_on_cooldown() {
        let mut event = low_hp_fixture();
        with_shadow_dance_down(&mut event);
        assert_eq!(event.abilities.ability3.name, DEPTH_SHROUD_ABILITY_NAME);
        event.abilities.ability3.can_cast = false;

        assert_eq!(
            plan_low_hp_escape(
                &event,
                &SlarkConfig::default(),
                true,
                now(),
                EscapeCooldowns::default()
            ),
            LowHpEscape::None
        );
    }

    /// The regression this whole correction exists for.
    ///
    /// Depth Shroud sits at index 3 and Shadow Dance at index 4 on a shard
    /// Slark. Deriving the slot from the `d` key landed on index 4, so the
    /// fallback read the *ultimate's* cooldown — false exactly when the shard
    /// was wanted — and never fired.
    #[test]
    fn the_shard_is_found_by_name_not_by_the_slot_its_key_suggests() {
        let mut event = low_hp_fixture();
        with_shadow_dance_down(&mut event);

        assert_eq!(event.abilities.ability3.name, DEPTH_SHROUD_ABILITY_NAME);
        assert_eq!(event.abilities.ability4.name, SHADOW_DANCE_ABILITY_NAME);
        assert!(ability_is_ready(&event, DEPTH_SHROUD_ABILITY_NAME));
        assert!(!ability_is_ready(&event, SHADOW_DANCE_ABILITY_NAME));

        assert_eq!(
            plan_low_hp_escape(
                &event,
                &SlarkConfig::default(),
                true,
                now(),
                EscapeCooldowns::default()
            ),
            LowHpEscape::Shard
        );
    }

    /// Spending the ultimate must not lock out the shard, which is a separate
    /// resource — and whose entire trigger is the ultimate being unavailable.
    #[test]
    fn a_recent_shadow_dance_does_not_suppress_the_shard() {
        let config = SlarkConfig::default();
        let now = now();
        let mut event = low_hp_fixture();
        with_shadow_dance_down(&mut event);

        let cooldowns = EscapeCooldowns {
            shadow_dance: Some(now - Duration::from_millis(50)),
            shard: None,
        };

        assert_eq!(
            plan_low_hp_escape(&event, &config, true, now, cooldowns),
            LowHpEscape::Shard
        );
    }

    #[test]
    fn a_recent_escape_debounces_the_same_kind() {
        let config = SlarkConfig::default();
        let now = now();
        // No shard, so there is nothing to escalate to and the debounce on
        // Shadow Dance is what the result actually shows.
        let mut event = low_hp_fixture();
        event.hero.aghanims_shard = false;

        let cooldowns = EscapeCooldowns {
            shadow_dance: Some(
                now - Duration::from_millis(config.shadow_dance_trigger_cooldown_ms / 2),
            ),
            shard: None,
        };

        assert_eq!(
            plan_low_hp_escape(&event, &config, true, now, cooldowns),
            LowHpEscape::None
        );
    }

    /// A debounced ultimate escalates rather than stalling.
    ///
    /// The debounce stops the same key being spammed; it is not a reason to sit
    /// on a second escape while still under fire.
    #[test]
    fn a_debounced_shadow_dance_escalates_to_the_shard() {
        let config = SlarkConfig::default();
        let now = now();
        let cooldowns = EscapeCooldowns {
            shadow_dance: Some(
                now - Duration::from_millis(config.shadow_dance_trigger_cooldown_ms / 2),
            ),
            shard: None,
        };

        assert_eq!(
            plan_low_hp_escape(&low_hp_fixture(), &config, true, now, cooldowns),
            LowHpEscape::Shard
        );
    }

    #[test]
    fn the_escape_reopens_once_the_trigger_cooldown_expires() {
        let config = SlarkConfig::default();
        let now = now();
        let cooldowns = EscapeCooldowns {
            shadow_dance: Some(
                now - Duration::from_millis(config.shadow_dance_trigger_cooldown_ms + 1),
            ),
            shard: None,
        };

        assert_eq!(
            plan_low_hp_escape(&low_hp_fixture(), &config, true, now, cooldowns),
            LowHpEscape::ShadowDance
        );
    }

    #[test]
    fn a_cast_lock_blocks_the_escape_entirely() {
        // Unlike Dark Pact there is no settle window to preserve here, so a
        // stun simply means "not this tick".
        for lock in ["stunned", "hexed", "silenced"] {
            let mut event = low_hp_fixture();
            match lock {
                "stunned" => event.hero.stunned = true,
                "hexed" => event.hero.hexed = true,
                _ => event.hero.silenced = true,
            }

            assert_eq!(
                plan_low_hp_escape(
                    &event,
                    &SlarkConfig::default(),
                    true,
                    now(),
                    EscapeCooldowns::default()
                ),
                LowHpEscape::None,
                "{lock} should block the escape"
            );
        }
    }

    #[test]
    fn the_escape_is_off_when_the_toggle_is_off() {
        let config = SlarkConfig {
            auto_shadow_dance_on_low_hp: false,
            ..SlarkConfig::default()
        };

        assert_eq!(
            plan_low_hp_escape(
                &low_hp_fixture(),
                &config,
                true,
                now(),
                EscapeCooldowns::default()
            ),
            LowHpEscape::None
        );
    }
}
