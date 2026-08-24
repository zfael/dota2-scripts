use crate::actions::common::SurvivabilityActions;
use crate::actions::executor::ActionExecutor;
use crate::actions::heroes::HeroScript;
use crate::config::settings::EarthSpiritConfig;
use crate::config::Settings;
use crate::models::{GsiWebhookEvent, Hero};
use lazy_static::lazy_static;
use std::sync::{mpsc, Arc, LazyLock, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{info, warn};

const GEOMAGNETIC_GRIP_ABILITY_NAME: &str = "earth_spirit_geomagnetic_grip";
const ROLLING_BOULDER_ABILITY_NAME: &str = "earth_spirit_rolling_boulder";
/// Enchant Remnant, the ability an Aghanim's Scepter adds. Absent from the
/// payload entirely without one, which is why the escape needs no separate
/// scepter check: the readiness gate cannot find what is not there.
const ENCHANT_REMNANT_ABILITY_NAME: &str = "earth_spirit_petrify";

lazy_static! {
    static ref EARTH_SPIRIT_LAST_EVENT: Arc<Mutex<Option<GsiWebhookEvent>>> =
        Arc::new(Mutex::new(None));
    static ref LAST_PETRIFY_TRIGGER: Mutex<Option<Instant>> = Mutex::new(None);
}

/// Work item for the dedicated Earth Spirit worker thread.
#[derive(Debug, PartialEq, Eq)]
enum EarthSpiritRequest {
    SilenceCombo {
        remnant_key: char,
        grip_key: char,
        remnant_delay_ms: u64,
    },
    /// Fields are listed in press order: the roll goes first, the remnant
    /// lands during its windup.
    RollCombo {
        roll_key: char,
        double_tap: bool,
        double_tap_delay_ms: u64,
        remnant_key: char,
        roll_to_remnant_delay_ms: u64,
        remnant_alt: bool,
        remnant_double_tap: bool,
        remnant_double_tap_delay_ms: u64,
    },
    /// Scepter panic button: self-cast Enchant Remnant, then kick the remnant
    /// — which is now Earth Spirit himself — away.
    ScepterEscape {
        petrify_key: char,
        petrify_alt: bool,
        petrify_double_tap: bool,
        petrify_double_tap_delay_ms: u64,
        /// `None` when the smash follow-up is off, so a disabled kick cannot be
        /// confused with a key that happens to be unset.
        smash_key: Option<char>,
        petrify_to_smash_delay_ms: u64,
    },
}

fn build_silence_combo_request(
    remnant_key: char,
    grip_key: char,
    remnant_delay_ms: u64,
) -> EarthSpiritRequest {
    EarthSpiritRequest::SilenceCombo {
        remnant_key,
        grip_key,
        remnant_delay_ms,
    }
}

#[allow(clippy::too_many_arguments)]
fn build_roll_combo_request(
    roll_key: char,
    double_tap: bool,
    double_tap_delay_ms: u64,
    remnant_key: char,
    roll_to_remnant_delay_ms: u64,
    remnant_alt: bool,
    remnant_double_tap: bool,
    remnant_double_tap_delay_ms: u64,
) -> EarthSpiritRequest {
    EarthSpiritRequest::RollCombo {
        roll_key,
        double_tap,
        double_tap_delay_ms,
        remnant_key,
        roll_to_remnant_delay_ms,
        remnant_alt,
        remnant_double_tap,
        remnant_double_tap_delay_ms,
    }
}

/// The escape takes every field from config, so it is built from the config
/// rather than from eleven positional arguments.
fn build_scepter_escape_request(config: &EarthSpiritConfig) -> EarthSpiritRequest {
    EarthSpiritRequest::ScepterEscape {
        petrify_key: config.petrify_key,
        petrify_alt: config.petrify_alt,
        petrify_double_tap: config.petrify_double_tap,
        petrify_double_tap_delay_ms: config.petrify_double_tap_delay_ms,
        smash_key: config.petrify_smash_enabled.then_some(config.smash_key),
        petrify_to_smash_delay_ms: config.petrify_to_smash_delay_ms,
    }
}

static EARTH_SPIRIT_REQUEST_QUEUE: LazyLock<mpsc::Sender<EarthSpiritRequest>> =
    LazyLock::new(|| {
        let (tx, rx) = mpsc::channel::<EarthSpiritRequest>();

        thread::spawn(move || {
            info!("🗿 Earth Spirit request worker started");

            while let Ok(request) = rx.recv() {
                run_earth_spirit_request(request);
            }

            info!("🗿 Earth Spirit request worker exited");
        });

        tx
    });

fn run_earth_spirit_request(request: EarthSpiritRequest) {
    match request {
        EarthSpiritRequest::SilenceCombo {
            remnant_key,
            grip_key,
            remnant_delay_ms,
        } => run_silence_combo_request(remnant_key, grip_key, remnant_delay_ms),
        EarthSpiritRequest::RollCombo {
            roll_key,
            double_tap,
            double_tap_delay_ms,
            remnant_key,
            roll_to_remnant_delay_ms,
            remnant_alt,
            remnant_double_tap,
            remnant_double_tap_delay_ms,
        } => run_roll_combo_request(
            roll_key,
            double_tap,
            double_tap_delay_ms,
            remnant_key,
            roll_to_remnant_delay_ms,
            remnant_alt,
            remnant_double_tap,
            remnant_double_tap_delay_ms,
        ),
        EarthSpiritRequest::ScepterEscape {
            petrify_key,
            petrify_alt,
            petrify_double_tap,
            petrify_double_tap_delay_ms,
            smash_key,
            petrify_to_smash_delay_ms,
        } => run_scepter_escape_request(
            petrify_key,
            petrify_alt,
            petrify_double_tap,
            petrify_double_tap_delay_ms,
            smash_key,
            petrify_to_smash_delay_ms,
        ),
    }
}

/// Silence combo: drop a Stone Remnant at the cursor, then grip it back.
///
/// Earth Spirit has no silence button. Geomagnetic Grip pulls a remnant
/// *toward* him and silences everything the remnant passes through, so the
/// silence is really two keys that both target the cursor. The wait between
/// them exists because the remnant has to exist server-side before the grip
/// resolves — press the grip too early and it reaches for whatever remnant was
/// already on the map, or nothing at all.
///
/// **Assumes quickcast on both keys.** Both are point-target: without quickcast
/// the first press only arms the cursor and the second cancels the targeting
/// instead of resolving it.
///
/// Aiming stays manual. The remnant travels back toward Earth Spirit, so the
/// cursor wants to be *past* the target, not on it.
fn run_silence_combo_request(remnant_key: char, grip_key: char, remnant_delay_ms: u64) {
    crate::input::simulation::press_key(remnant_key);

    if remnant_delay_ms > 0 {
        thread::sleep(Duration::from_millis(remnant_delay_ms));
    }
    crate::input::simulation::press_key(grip_key);
}

/// Roll combo: cast Rolling Boulder, then drop the remnant into its path.
///
/// A roll that passes through a remnant travels 1600 units instead of 800 and
/// speeds up, so the good roll is always the two-key one.
///
/// **The roll goes first.** Rolling Boulder has a ~0.6s windup before Earth
/// Spirit actually starts moving, and a remnant placed into the path during
/// that window still counts. Casting the roll first is the documented
/// technique: the roll direction is locked in before the remnant is placed.
///
/// **The remnant is self-cast**, which is what removes aiming from this combo
/// entirely. Self-cast drops the stone on Earth Spirit himself, and the roll
/// starts from Earth Spirit — so the boulder passes through it every time, no
/// matter where the cursor is. Two independent routes to self-cast, because
/// which one works depends on the operator's Dota settings:
///
/// - `remnant_alt` holds ALT across the press (Dota's self-cast modifier, the
///   route that still works when the remnant key is on quickcast).
/// - `remnant_double_tap` presses the key twice (Dota's default self-cast
///   binding).
///
/// Both default on. Turning both off puts the remnant back at the cursor, which
/// then needs a real aiming window — raise `roll_to_remnant_delay_ms` if so.
///
/// Every press has to land inside the windup, so all three delays here are one
/// shared budget against `ROLLING_BOULDER_WINDUP_MS`.
///
/// Rolling Boulder is the one key the operator does **not** run on quickcast,
/// so the first press only arms the cursor and a second press is what fires it.
/// `double_tap` is the switch for that: off, this sends a single press, which
/// is what quickcast on the roll key would want. The wait to the remnant is
/// measured from the press that actually fires the roll, so it starts after the
/// second tap.
#[allow(clippy::too_many_arguments)]
fn run_roll_combo_request(
    roll_key: char,
    double_tap: bool,
    double_tap_delay_ms: u64,
    remnant_key: char,
    roll_to_remnant_delay_ms: u64,
    remnant_alt: bool,
    remnant_double_tap: bool,
    remnant_double_tap_delay_ms: u64,
) {
    crate::input::simulation::press_key(roll_key);

    if double_tap {
        if double_tap_delay_ms > 0 {
            thread::sleep(Duration::from_millis(double_tap_delay_ms));
        }
        crate::input::simulation::press_key(roll_key);
    }

    if roll_to_remnant_delay_ms > 0 {
        thread::sleep(Duration::from_millis(roll_to_remnant_delay_ms));
    }

    // ALT is held across *both* taps rather than pulsed per press: Dota reads
    // the modifier at the moment the cast resolves, and releasing between the
    // taps would leave the second one unmodified.
    if remnant_alt {
        crate::input::simulation::alt_down();
    }

    crate::input::simulation::press_key(remnant_key);

    if remnant_double_tap {
        if remnant_double_tap_delay_ms > 0 {
            thread::sleep(Duration::from_millis(remnant_double_tap_delay_ms));
        }
        crate::input::simulation::press_key(remnant_key);
    }

    if remnant_alt {
        crate::input::simulation::alt_up();
    }
}

/// Scepter escape: self-cast Enchant Remnant, then kick the result away.
///
/// Enchant Remnant is the ability an Aghanim's Scepter adds, and self-casting it
/// turns Earth Spirit into a Stone Remnant — untargetable for the duration,
/// which is the save. The kick is what turns that from a pause into an escape:
/// a remnant is a legal Boulder Smash target, and the remnant standing there is
/// him, so the smash launches him out of whatever he was standing in.
///
/// **The petrify is self-cast**, by the same two independent routes the roll
/// combo uses for its remnant — `petrify_alt` holds ALT across the press,
/// `petrify_double_tap` presses twice. Which one works depends on the
/// operator's Dota settings, so both ship on and either can be dropped. With
/// both off the petrify lands on whatever the cursor is over, which is a
/// different spell entirely: it petrifies *that* hero, not Earth Spirit.
///
/// **The smash is a single plain press.** It is not self-cast and not aimed:
/// Boulder Smash kicks the remnant it finds, and after the petrify the nearest
/// remnant is Earth Spirit himself.
///
/// `petrify_to_smash_delay_ms` is longer than the combo delays for a different
/// reason than they are: it waits on a *cast resolving* rather than on a key
/// press clearing. There is nothing to kick until the petrify has actually
/// turned him to stone.
fn run_scepter_escape_request(
    petrify_key: char,
    petrify_alt: bool,
    petrify_double_tap: bool,
    petrify_double_tap_delay_ms: u64,
    smash_key: Option<char>,
    petrify_to_smash_delay_ms: u64,
) {
    // ALT is held across *both* taps rather than pulsed per press, for the same
    // reason as the roll combo's remnant: Dota reads the modifier when the cast
    // resolves, so releasing between taps would leave the second one unmodified.
    if petrify_alt {
        crate::input::simulation::alt_down();
    }

    crate::input::simulation::press_key(petrify_key);

    if petrify_double_tap {
        if petrify_double_tap_delay_ms > 0 {
            thread::sleep(Duration::from_millis(petrify_double_tap_delay_ms));
        }
        crate::input::simulation::press_key(petrify_key);
    }

    if petrify_alt {
        crate::input::simulation::alt_up();
    }

    let Some(smash_key) = smash_key else {
        return;
    };

    if petrify_to_smash_delay_ms > 0 {
        thread::sleep(Duration::from_millis(petrify_to_smash_delay_ms));
    }
    crate::input::simulation::press_key(smash_key);
}

/// Whether this payload should fire the scepter escape.
///
/// No scepter check of its own: without an Aghanim's Scepter, Enchant Remnant is
/// not in the payload at all, so the readiness gate already answers the
/// question.
fn should_trigger_petrify(
    event: &GsiWebhookEvent,
    config: &EarthSpiritConfig,
    in_danger: bool,
    now: Instant,
    last_trigger: Option<Instant>,
) -> bool {
    if !config.auto_petrify_on_danger || !in_danger {
        return false;
    }

    if !event.hero.alive || event.hero.stunned || event.hero.silenced || event.hero.hexed {
        return false;
    }

    if event.hero.health_percent > config.petrify_hp_threshold_percent {
        return false;
    }

    if !ability_is_ready(event, ENCHANT_REMNANT_ABILITY_NAME) {
        return false;
    }

    if let Some(last_trigger) = last_trigger {
        if now.duration_since(last_trigger)
            < Duration::from_millis(config.petrify_trigger_cooldown_ms)
        {
            return false;
        }
    }

    true
}

fn spawn_earth_spirit_fallback(request: EarthSpiritRequest) {
    thread::spawn(move || {
        run_earth_spirit_request(request);
    });
}

fn enqueue_earth_spirit_request(request: EarthSpiritRequest) {
    if let Err(err) = EARTH_SPIRIT_REQUEST_QUEUE.send(request) {
        warn!("🗿 Earth Spirit request queue unavailable; using fallback thread");
        spawn_earth_spirit_fallback(err.0);
    }
}

/// Whether a named ability is levelled and castable right now.
///
/// Scans every slot rather than indexing one, because **GSI slot order is
/// ability order, not key order**: Earth Spirit carries Stone Remnant as its
/// own entry ahead of the ultimate, and a scepter inserts Enchant Remnant on
/// top of that, so the slot a key implies is not the slot the ability lives in.
/// Slark's shard fallback shipped broken for exactly this reason — see
/// `docs/heroes/slark.md`.
fn ability_is_ready(event: &GsiWebhookEvent, ability_name: &str) -> bool {
    (0..=5).any(|index| {
        event.abilities.get_by_index(index).is_some_and(|ability| {
            ability.name == ability_name && ability.level > 0 && ability.can_cast
        })
    })
}

/// Whether the last GSI payload says `ability_name` is castable.
///
/// Returns `false` before the first payload of the game arrives, which leaves
/// the key unblocked rather than firing a combo blind.
fn last_event_has_ready_ability(ability_name: &str, label: &str) -> bool {
    let event = EARTH_SPIRIT_LAST_EVENT.lock().unwrap().clone();

    let Some(event) = event else {
        info!("🗿 Earth Spirit {label} intercept skipped: no GSI event available");
        return false;
    };

    if !ability_is_ready(&event, ability_name) {
        info!("🗿 Earth Spirit {label} intercept skipped: {label} not ready");
        return false;
    }

    true
}

pub struct EarthSpiritState;

impl EarthSpiritState {
    /// Whether the keyboard hook should swallow the Geomagnetic Grip key.
    ///
    /// Gates on Grip, never on Stone Remnant. Stone Remnant is charge-based and
    /// GSI's `can_cast` is unreliable for charge abilities, so gating on it
    /// would leave the intercept dead while remnants are visibly banked — the
    /// same trap `config/config.toml` already warns about for Mirana's Leap.
    pub fn can_intercept_grip() -> bool {
        last_event_has_ready_ability(GEOMAGNETIC_GRIP_ABILITY_NAME, "grip")
    }

    /// Whether the keyboard hook should swallow the Rolling Boulder key.
    ///
    /// Gates on Rolling Boulder only, for the same reason as the grip.
    pub fn can_intercept_roll() -> bool {
        last_event_has_ready_ability(ROLLING_BOULDER_ABILITY_NAME, "roll")
    }

    /// Run the silence combo: press the remnant key, wait `remnant_delay_ms`,
    /// then press the grip key.
    pub fn execute_silence_combo(remnant_key: char, grip_key: char, remnant_delay_ms: u64) {
        enqueue_earth_spirit_request(build_silence_combo_request(
            remnant_key,
            grip_key,
            remnant_delay_ms,
        ));
    }

    /// Run the roll combo: press the roll key — twice when `double_tap` is on —
    /// then wait `roll_to_remnant_delay_ms` and self-cast the remnant into the
    /// roll's path during the windup.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_roll_combo(
        roll_key: char,
        double_tap: bool,
        double_tap_delay_ms: u64,
        remnant_key: char,
        roll_to_remnant_delay_ms: u64,
        remnant_alt: bool,
        remnant_double_tap: bool,
        remnant_double_tap_delay_ms: u64,
    ) {
        enqueue_earth_spirit_request(build_roll_combo_request(
            roll_key,
            double_tap,
            double_tap_delay_ms,
            remnant_key,
            roll_to_remnant_delay_ms,
            remnant_alt,
            remnant_double_tap,
            remnant_double_tap_delay_ms,
        ));
    }

    /// Run the scepter escape: self-cast Enchant Remnant, then — unless the
    /// follow-up is off — wait `petrify_to_smash_delay_ms` and kick the
    /// resulting remnant, which is Earth Spirit himself, with Boulder Smash.
    pub fn execute_scepter_escape(config: &EarthSpiritConfig) {
        enqueue_earth_spirit_request(build_scepter_escape_request(config));
    }
}

/// Earth Spirit script.
///
/// Both combos are keyboard-driven, and both pair a Stone Remnant with one more
/// ability — but they order that pair differently, because the two abilities
/// give the operator different aiming windows:
///
/// 1. keyboard.rs intercepts the grip key (default E) or the roll key (default
///    W) when Earth Spirit is the active hero and that ability is castable.
/// 2. Calls `EarthSpiritState::execute_silence_combo()` /
///    `execute_roll_combo()`.
/// 3. The dedicated worker runs the sequence:
///    - **silence**: remnant first, then grip. Grip resolves on the press, so
///      the remnant has to already be there. Both land on one cursor position.
///      See `run_silence_combo_request`.
///    - **roll**: roll first, then a self-cast remnant. Rolling Boulder has a
///      ~0.6s windup and a remnant dropped into the path during it still
///      counts, so the direction is locked in first; self-casting the remnant
///      puts it on Earth Spirit, where the roll starts, so the boulder passes
///      through it without any aiming. See `run_roll_combo_request`.
///
/// Off GSI this hero stashes the payload for the readiness gates and runs one
/// auto-cast of its own — the scepter escape, which self-casts Enchant Remnant
/// and kicks the resulting remnant (Earth Spirit himself) clear with Boulder
/// Smash once the danger detector fires below the panic HP line. See
/// `run_scepter_escape_request`. The shared survivability pipeline handles the
/// rest.
pub struct EarthSpiritScript {
    settings: Arc<Mutex<Settings>>,
    executor: Arc<ActionExecutor>,
}

impl EarthSpiritScript {
    pub fn new(settings: Arc<Mutex<Settings>>, executor: Arc<ActionExecutor>) -> Self {
        Self { settings, executor }
    }

    /// Fire the scepter escape when the danger detector says Earth Spirit is
    /// under fire and HP has dropped past the panic line.
    ///
    /// The sequence runs on the hero's own worker rather than the shared
    /// executor, because it sleeps between presses — same as both combos.
    fn maybe_trigger_petrify(
        &self,
        event: &GsiWebhookEvent,
        config: &EarthSpiritConfig,
        in_danger: bool,
    ) {
        let now = Instant::now();
        let mut last_trigger = LAST_PETRIFY_TRIGGER.lock().unwrap();

        if !should_trigger_petrify(event, config, in_danger, now, *last_trigger) {
            return;
        }

        *last_trigger = Some(now);
        info!(
            "🗿 Earth Spirit self-casting Enchant Remnant on danger at {}% HP ({}{})",
            event.hero.health_percent,
            config.petrify_key,
            if config.petrify_smash_enabled {
                format!(" → smash {}", config.smash_key)
            } else {
                String::new()
            }
        );
        EarthSpiritState::execute_scepter_escape(config);
    }
}

impl HeroScript for EarthSpiritScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        {
            let mut last_event = EARTH_SPIRIT_LAST_EVENT.lock().unwrap();
            *last_event = Some(event.clone());
        }

        let settings = self.settings.lock().unwrap();

        // Both combos are keyboard-driven; the scepter escape is the only thing
        // this hero does off GSI, on top of shared survivability.
        let survivability = SurvivabilityActions::new(self.settings.clone(), self.executor.clone());
        let in_danger = crate::actions::danger_detector::update(event, &settings.danger_detection);
        self.maybe_trigger_petrify(event, &settings.heroes.earth_spirit, in_danger);
        drop(settings);
        survivability.check_and_use_healing_items_with_danger(event, in_danger);
        survivability.use_defensive_items_if_danger_with_snapshot(event, in_danger);
        survivability.use_neutral_item_if_danger_with_snapshot(event, in_danger);
    }

    /// The global combo trigger runs the silence combo — the play worth having
    /// on a second key when the grip key itself is awkward to reach.
    fn handle_standalone_trigger(&self) {
        let settings = self.settings.lock().unwrap();
        let earth_spirit = &settings.heroes.earth_spirit;
        let enabled = earth_spirit.enabled && earth_spirit.silence_combo_enabled;
        let remnant_key = earth_spirit.remnant_key;
        let grip_key = earth_spirit.grip_key;
        let remnant_delay_ms = earth_spirit.silence_remnant_delay_ms;
        drop(settings);

        if !enabled {
            info!("🗿 Earth Spirit silence combo skipped: disabled in config");
            return;
        }

        info!("🗿 Earth Spirit standalone silence combo triggered");
        EarthSpiritState::execute_silence_combo(remnant_key, grip_key, remnant_delay_ms);
    }

    fn hero_name(&self) -> &'static str {
        Hero::EarthSpirit.to_game_name()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Handcrafted, not captured from a live game, so the slot ordering is a
    /// plausible guess rather than ground truth. Safe for what it backs: the
    /// readiness check matches by name, which is deliberately independent of
    /// slot order.
    fn earth_spirit_fixture() -> GsiWebhookEvent {
        serde_json::from_str(include_str!(
            "../../../tests/fixtures/earth_spirit_event.json"
        ))
        .expect("Earth Spirit fixture should deserialize")
    }

    #[test]
    fn build_silence_combo_request_preserves_keys_and_delay() {
        let request = build_silence_combo_request('d', 'e', 120);
        assert_eq!(
            request,
            EarthSpiritRequest::SilenceCombo {
                remnant_key: 'd',
                grip_key: 'e',
                remnant_delay_ms: 120,
            }
        );
    }

    #[test]
    fn build_roll_combo_request_preserves_keys_delays_and_double_tap() {
        let request = build_roll_combo_request('w', true, 60, 'd', 120, true, true, 60);
        assert_eq!(
            request,
            EarthSpiritRequest::RollCombo {
                roll_key: 'w',
                double_tap: true,
                double_tap_delay_ms: 60,
                remnant_key: 'd',
                roll_to_remnant_delay_ms: 120,
                remnant_alt: true,
                remnant_double_tap: true,
                remnant_double_tap_delay_ms: 60,
            }
        );
    }

    /// Both toggles are there to be A/B'd in a live game, so they have to
    /// survive into the request rather than being resolved away at build time.
    #[test]
    fn the_double_tap_toggles_reach_the_request() {
        let request = build_roll_combo_request('w', false, 60, 'd', 120, false, false, 60);
        assert_eq!(
            request,
            EarthSpiritRequest::RollCombo {
                roll_key: 'w',
                double_tap: false,
                double_tap_delay_ms: 60,
                remnant_key: 'd',
                roll_to_remnant_delay_ms: 120,
                remnant_alt: false,
                remnant_double_tap: false,
                remnant_double_tap_delay_ms: 60,
            }
        );
    }

    /// The roll's remnant is self-cast and the silence's is not, and they are
    /// both the same key — so nothing but these flags distinguishes "on Earth
    /// Spirit" from "at the cursor". Losing them is a silent gameplay
    /// regression: the roll would still fire and just stop extending.
    #[test]
    fn only_the_roll_combo_carries_the_self_cast_flags() {
        let roll = build_roll_combo_request('w', true, 60, 'd', 120, true, true, 60);

        match roll {
            EarthSpiritRequest::RollCombo {
                remnant_alt,
                remnant_double_tap,
                ..
            } => {
                assert!(remnant_alt);
                assert!(remnant_double_tap);
            }
            other => panic!("expected a roll combo, got {other:?}"),
        }

        // The silence deliberately has no self-cast route: gripping a remnant
        // standing on top of Earth Spirit would silence nobody.
        let silence = build_silence_combo_request('d', 'e', 120);
        assert!(matches!(silence, EarthSpiritRequest::SilenceCombo { .. }));
    }

    /// The two combos deliberately order the same pair of abilities the other
    /// way round. Silence needs the remnant already standing when the grip
    /// resolves; the roll wants its direction locked in first so the windup can
    /// be spent aiming the remnant. Getting these backwards is a silent
    /// gameplay regression, not a compile error — both variants hold the same
    /// two keys.
    #[test]
    fn the_two_combos_order_the_remnant_on_opposite_sides() {
        let silence = build_silence_combo_request('d', 'e', 120);
        let roll = build_roll_combo_request('w', true, 60, 'd', 120, true, true, 60);

        // Silence: remnant is named first, and its delay precedes the grip.
        match silence {
            EarthSpiritRequest::SilenceCombo {
                remnant_key,
                grip_key,
                ..
            } => {
                assert_eq!(remnant_key, 'd');
                assert_eq!(grip_key, 'e');
            }
            other => panic!("expected a silence combo, got {other:?}"),
        }

        // Roll: the remnant delay is measured *from* the roll, not toward it.
        match roll {
            EarthSpiritRequest::RollCombo {
                roll_key,
                remnant_key,
                roll_to_remnant_delay_ms,
                ..
            } => {
                assert_eq!(roll_key, 'w');
                assert_eq!(remnant_key, 'd');
                assert!(roll_to_remnant_delay_ms > 0);
            }
            other => panic!("expected a roll combo, got {other:?}"),
        }
    }

    /// The shipped fixture has no scepter, so Enchant Remnant is absent from it
    /// exactly as it is absent in a real payload before the item is bought.
    /// This is the scepter'd version, hurt enough to be worth saving.
    fn scepter_fixture() -> GsiWebhookEvent {
        let mut event = earth_spirit_fixture();
        event.hero.aghanims_scepter = true;
        event.hero.health_percent = EarthSpiritConfig::default().petrify_hp_threshold_percent;
        // Slot 5 is where the scepter ability lands here; the readiness gate
        // matches by name, so the slot itself carries no meaning.
        event.abilities.ability5.name = ENCHANT_REMNANT_ABILITY_NAME.to_string();
        event.abilities.ability5.level = 1;
        event.abilities.ability5.can_cast = true;
        event.abilities.ability5.ability_active = true;
        event
    }

    #[test]
    fn build_scepter_escape_request_carries_the_self_cast_and_the_kick() {
        let request = build_scepter_escape_request(&EarthSpiritConfig::default());
        assert_eq!(
            request,
            EarthSpiritRequest::ScepterEscape {
                petrify_key: 'f',
                petrify_alt: true,
                petrify_double_tap: true,
                petrify_double_tap_delay_ms: 60,
                smash_key: Some('q'),
                petrify_to_smash_delay_ms: 250,
            }
        );
    }

    /// The kick is a toggle, and off it has to leave *no* smash press behind —
    /// a stray Q while petrified would come out the moment Earth Spirit is
    /// solid again.
    #[test]
    fn the_smash_toggle_off_leaves_no_key_to_press() {
        let mut config = EarthSpiritConfig::default();
        config.petrify_smash_enabled = false;

        match build_scepter_escape_request(&config) {
            EarthSpiritRequest::ScepterEscape { smash_key, .. } => assert_eq!(smash_key, None),
            other => panic!("expected a scepter escape, got {other:?}"),
        }
    }

    /// Both self-cast routes are there to be A/B'd in a live game, so they have
    /// to survive into the request rather than being resolved away at build
    /// time. Losing both is the silent regression: the petrify still fires and
    /// lands on whoever the cursor was over.
    #[test]
    fn the_petrify_self_cast_toggles_reach_the_request() {
        let mut config = EarthSpiritConfig::default();
        config.petrify_alt = false;
        config.petrify_double_tap = false;

        match build_scepter_escape_request(&config) {
            EarthSpiritRequest::ScepterEscape {
                petrify_alt,
                petrify_double_tap,
                ..
            } => {
                assert!(!petrify_alt);
                assert!(!petrify_double_tap);
            }
            other => panic!("expected a scepter escape, got {other:?}"),
        }
    }

    #[test]
    fn petrify_fires_in_danger_below_the_threshold() {
        let event = scepter_fixture();
        assert!(should_trigger_petrify(
            &event,
            &EarthSpiritConfig::default(),
            true,
            Instant::now(),
            None
        ));
    }

    #[test]
    fn petrify_holds_when_not_in_danger() {
        let event = scepter_fixture();
        assert!(!should_trigger_petrify(
            &event,
            &EarthSpiritConfig::default(),
            false,
            Instant::now(),
            None
        ));
    }

    #[test]
    fn petrify_holds_above_the_hp_threshold() {
        let config = EarthSpiritConfig::default();
        let mut event = scepter_fixture();
        event.hero.health_percent = config.petrify_hp_threshold_percent + 1;

        assert!(!should_trigger_petrify(
            &event,
            &config,
            true,
            Instant::now(),
            None
        ));
    }

    /// Without a scepter Enchant Remnant is simply not in the payload, so the
    /// escape must stay silent rather than pressing an empty ability slot.
    #[test]
    fn petrify_holds_without_the_scepter_ability() {
        let event = {
            let mut event = earth_spirit_fixture();
            event.hero.health_percent = 5;
            event
        };

        assert!(!should_trigger_petrify(
            &event,
            &EarthSpiritConfig::default(),
            true,
            Instant::now(),
            None
        ));
    }

    #[test]
    fn petrify_holds_while_earth_spirit_cannot_act() {
        let config = EarthSpiritConfig::default();

        for break_state in [
            |event: &mut GsiWebhookEvent| event.hero.alive = false,
            |event: &mut GsiWebhookEvent| event.hero.stunned = true,
            |event: &mut GsiWebhookEvent| event.hero.silenced = true,
            |event: &mut GsiWebhookEvent| event.hero.hexed = true,
        ] {
            let mut event = scepter_fixture();
            break_state(&mut event);

            assert!(!should_trigger_petrify(
                &event,
                &config,
                true,
                Instant::now(),
                None
            ));
        }
    }

    /// A successful petrify keeps Earth Spirit at the HP that triggered it for
    /// several seconds, so without this the detector would re-fire the whole
    /// escape on every payload of the petrify's own duration.
    #[test]
    fn petrify_respects_its_own_retry_cooldown() {
        let event = scepter_fixture();
        let config = EarthSpiritConfig::default();
        let now = Instant::now();

        assert!(!should_trigger_petrify(
            &event,
            &config,
            true,
            now,
            Some(now - Duration::from_millis(config.petrify_trigger_cooldown_ms - 1))
        ));
        assert!(should_trigger_petrify(
            &event,
            &config,
            true,
            now,
            Some(now - Duration::from_millis(config.petrify_trigger_cooldown_ms))
        ));
    }

    #[test]
    fn petrify_holds_when_the_auto_cast_is_disabled() {
        let event = scepter_fixture();
        let mut config = EarthSpiritConfig::default();
        config.auto_petrify_on_danger = false;

        assert!(!should_trigger_petrify(
            &event,
            &config,
            true,
            Instant::now(),
            None
        ));
    }

    /// The escape is a survivability feature, not a combo one: turning the
    /// keyboard remaps off must not disarm the panic button.
    #[test]
    fn petrify_is_not_gated_by_the_combo_toggles() {
        let event = scepter_fixture();
        let mut config = EarthSpiritConfig::default();
        config.enabled = false;
        config.silence_combo_enabled = false;
        config.roll_combo_enabled = false;

        assert!(should_trigger_petrify(
            &event,
            &config,
            true,
            Instant::now(),
            None
        ));
    }

    #[test]
    fn hero_name_is_the_gsi_internal_name() {
        let script = EarthSpiritScript::new(
            Arc::new(Mutex::new(Settings::default())),
            ActionExecutor::new(),
        );
        assert_eq!(script.hero_name(), "npc_dota_hero_earth_spirit");
    }

    #[test]
    fn finds_grip_and_roll_when_levelled_and_castable() {
        let event = earth_spirit_fixture();
        assert!(ability_is_ready(&event, GEOMAGNETIC_GRIP_ABILITY_NAME));
        assert!(ability_is_ready(&event, ROLLING_BOULDER_ABILITY_NAME));
    }

    #[test]
    fn ability_on_cooldown_is_not_ready() {
        let mut event = earth_spirit_fixture();
        assert_eq!(event.abilities.ability2.name, GEOMAGNETIC_GRIP_ABILITY_NAME);
        event.abilities.ability2.can_cast = false;

        assert!(!ability_is_ready(&event, GEOMAGNETIC_GRIP_ABILITY_NAME));
    }

    #[test]
    fn unlevelled_ability_is_not_ready() {
        let mut event = earth_spirit_fixture();
        event.abilities.ability1.level = 0;

        assert!(!ability_is_ready(&event, ROLLING_BOULDER_ABILITY_NAME));
    }

    #[test]
    fn unknown_ability_name_is_not_ready() {
        let event = earth_spirit_fixture();
        assert!(!ability_is_ready(&event, "earth_spirit_not_a_real_ability"));
    }

    /// Grip is found wherever it sits, not at the index its key suggests.
    ///
    /// `grip_key` defaults to `e`, the third ability slot — but Stone Remnant is
    /// its own GSI entry and a scepter adds Enchant Remnant on top, so the slot
    /// a key implies is already off in a real payload. This is the regression
    /// that cost Slark's shard fallback its entire feature.
    #[test]
    fn grip_is_found_by_name_not_by_the_slot_its_key_suggests() {
        let mut event = earth_spirit_fixture();

        // Swap Grip into a slot no key would ever imply.
        let grip = event.abilities.ability2.clone();
        event.abilities.ability2 = event.abilities.ability5.clone();
        event.abilities.ability5 = grip;

        assert!(ability_is_ready(&event, GEOMAGNETIC_GRIP_ABILITY_NAME));
    }
}
