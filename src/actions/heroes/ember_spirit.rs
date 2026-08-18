use crate::actions::common::SurvivabilityActions;
use crate::actions::executor::ActionExecutor;
use crate::actions::heroes::HeroScript;
use crate::config::settings::EmberSpiritConfig;
use crate::config::Settings;
use crate::input::simulation::press_key;
use crate::models::{GsiWebhookEvent, Hero};
use lazy_static::lazy_static;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, LazyLock, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{info, warn};

const FLAME_GUARD_ABILITY_NAME: &str = "ember_spirit_flame_guard";
const ULTIMATE_SCEPTER_ITEM_NAME: &str = "item_ultimate_scepter";

lazy_static! {
    /// Last time the auto-cast fired, for its own retry cooldown.
    static ref LAST_FLAME_GUARD_TRIGGER: Mutex<Option<Instant>> = Mutex::new(None);
}

/// Scepter state from the last GSI payload, for the hotkey path to read.
///
/// The remnant chase is hotkey-driven and has no event in hand, so the GSI
/// handler leaves the answer here. Starts `false`, which only means the chase
/// keeps its configured delay until the first payload of the game arrives.
static HAS_SCEPTER: AtomicBool = AtomicBool::new(false);

/// Whether a named ability is levelled and castable right now.
///
/// Scans every slot rather than indexing one, because **GSI slot order is
/// ability order, not key order**: Activate Fire Remnant, shard- and
/// scepter-granted abilities, and innates are all inserted as their own
/// entries, so the slot a key implies is not the slot the ability lives in.
/// Slark's shard fallback shipped broken for exactly this reason — see
/// `docs/heroes/slark.md`.
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

/// Whether Ember has the scepter upgrade right now.
///
/// The inventory scan backs up `hero.aghanims_scepter`, which is the flag Dota
/// sets for the upgrade itself: same belt-and-braces pair Largo uses, for the
/// same reason — a scepter sitting in a slot already grants the upgrade.
fn has_scepter(event: &GsiWebhookEvent) -> bool {
    event.hero.aghanims_scepter
        || event
            .items
            .all_slots()
            .iter()
            .any(|(_, item)| item.name == ULTIMATE_SCEPTER_ITEM_NAME)
}

/// Delay to use between the remnant press and the activate press.
///
/// The scepter takes the remnant's travel time out of the wait, so the chase
/// switches to the shorter `scepter_activate_delay_ms`. It is a shorter wait,
/// not no wait: the activate still has to land after the remnant registers
/// server-side, and pressing both in the same tick loses the new remnant
/// exactly as it does without the scepter. Without a scepter — or with
/// `use_scepter_activate_delay` off — the configured delay stands.
fn resolve_activate_delay_ms(config: &EmberSpiritConfig, has_scepter: bool) -> u64 {
    if config.use_scepter_activate_delay && has_scepter {
        config.scepter_activate_delay_ms
    } else {
        config.activate_delay_ms
    }
}

/// Whether this payload should fire the Flame Guard auto-cast.
///
/// No mana floor: `can_cast` already encodes affordability, and unlike OD's
/// Objurgation there is nothing to reserve mana *for* — Flame Guard is the
/// cheapest thing Ember can do when he is the one being focused.
fn should_trigger_flame_guard(
    event: &GsiWebhookEvent,
    config: &EmberSpiritConfig,
    in_danger: bool,
    now: Instant,
    last_trigger: Option<Instant>,
) -> bool {
    if !config.auto_flame_guard_on_danger || !in_danger {
        return false;
    }

    if !event.hero.alive || event.hero.stunned || event.hero.silenced || event.hero.hexed {
        return false;
    }

    if event.hero.health_percent > config.flame_guard_hp_threshold_percent {
        return false;
    }

    // Flame Guard's cooldown outlasts its own duration, so `can_cast` being
    // true also means the shield is not already up. That is the only signal
    // available: GSI exposes no modifiers.
    if !ability_is_ready(event, FLAME_GUARD_ABILITY_NAME) {
        return false;
    }

    if let Some(last_trigger) = last_trigger {
        if now.duration_since(last_trigger)
            < Duration::from_millis(config.flame_guard_trigger_cooldown_ms)
        {
            return false;
        }
    }

    true
}

/// Work item for the dedicated Ember Spirit worker thread.
#[derive(Debug, PartialEq, Eq)]
enum EmberSpiritRequest {
    RemnantChase {
        remnant_key: char,
        activate_key: char,
        activate_delay_ms: u64,
    },
}

fn build_remnant_chase_request(
    remnant_key: char,
    activate_key: char,
    activate_delay_ms: u64,
) -> EmberSpiritRequest {
    EmberSpiritRequest::RemnantChase {
        remnant_key,
        activate_key,
        activate_delay_ms,
    }
}

static EMBER_SPIRIT_REQUEST_QUEUE: LazyLock<mpsc::Sender<EmberSpiritRequest>> = LazyLock::new(|| {
    let (tx, rx) = mpsc::channel::<EmberSpiritRequest>();

    thread::spawn(move || {
        info!("🔥 Ember Spirit request worker started");

        while let Ok(request) = rx.recv() {
            run_ember_spirit_request(request);
        }

        info!("🔥 Ember Spirit request worker exited");
    });

    tx
});

fn run_ember_spirit_request(request: EmberSpiritRequest) {
    match request {
        EmberSpiritRequest::RemnantChase {
            remnant_key,
            activate_key,
            activate_delay_ms,
        } => run_remnant_chase_request(remnant_key, activate_key, activate_delay_ms),
    }
}

/// Remnant chase: drop a Fire Remnant, then dash to it.
///
/// Fire Remnant (R) places a remnant at the cursor and Activate Fire Remnant
/// (D) dashes to every remnant on the map, so the pair is one chase move that
/// costs two keys. The wait between them exists because the remnant has to
/// exist server-side before the activate can pick it up — press D too early and
/// Ember dashes to the *previous* remnants only.
///
/// **Assumes quickcast on the remnant key.** R is point-target: without
/// quickcast it only arms the cursor, and the no-target D that follows cancels
/// the targeting instead of resolving it.
/// The caller resolves which delay applies (scepter or not); a configured `0`
/// skips the sleep outright rather than sleeping for nothing.
fn run_remnant_chase_request(remnant_key: char, activate_key: char, activate_delay_ms: u64) {
    crate::input::simulation::press_key(remnant_key);

    if activate_delay_ms > 0 {
        thread::sleep(Duration::from_millis(activate_delay_ms));
    }
    crate::input::simulation::press_key(activate_key);
}

fn spawn_ember_spirit_fallback(request: EmberSpiritRequest) {
    thread::spawn(move || {
        run_ember_spirit_request(request);
    });
}

fn enqueue_ember_spirit_request(request: EmberSpiritRequest) {
    if let Err(err) = EMBER_SPIRIT_REQUEST_QUEUE.send(request) {
        warn!("🔥 Ember Spirit request queue unavailable; using fallback thread");
        spawn_ember_spirit_fallback(err.0);
    }
}

pub struct EmberSpiritState;

impl EmberSpiritState {
    /// Run the remnant chase: press the remnant key, wait
    /// `activate_delay_ms`, then press the activate key.
    pub fn execute_remnant_chase(
        remnant_key: char,
        activate_key: char,
        activate_delay_ms: u64,
    ) {
        enqueue_ember_spirit_request(build_remnant_chase_request(
            remnant_key,
            activate_key,
            activate_delay_ms,
        ));
    }
}

/// Ember Spirit script.
///
/// Remnant chase flow:
/// 1. The generic standalone combo hotkey (`AppState.trigger_key`, default
///    `Home`) fires `HotkeyEvent::ComboTrigger` while Ember Spirit is the
///    active hero.
/// 2. The dispatcher routes it to `handle_standalone_trigger()`.
/// 3. The dedicated worker presses the remnant key, waits, then presses the
///    activate key. The wait drops to the shorter `scepter_activate_delay_ms`
///    while Ember holds an Aghanim's Scepter, which places the remnant
///    instantly. `use_scepter_activate_delay` turns that off for anyone who
///    wants the configured delay regardless.
///
/// There is no readiness gate. Unlike the facing combos (Magnus, Mirana,
/// Slark), a wasted press here costs nothing: Dota ignores a Fire Remnant press
/// with no charges banked, and the activate that follows is still worth sending
/// because it dashes to remnants already on the map.
pub struct EmberSpiritScript {
    settings: Arc<Mutex<Settings>>,
    executor: Arc<ActionExecutor>,
}

impl EmberSpiritScript {
    pub fn new(settings: Arc<Mutex<Settings>>, executor: Arc<ActionExecutor>) -> Self {
        Self { settings, executor }
    }

    fn maybe_trigger_flame_guard(
        &self,
        event: &GsiWebhookEvent,
        config: &EmberSpiritConfig,
        in_danger: bool,
    ) {
        let now = Instant::now();
        let mut last_trigger = LAST_FLAME_GUARD_TRIGGER.lock().unwrap();

        if !should_trigger_flame_guard(event, config, in_danger, now, *last_trigger) {
            return;
        }

        *last_trigger = Some(now);
        let key = config.flame_guard_key;
        let health_percent = event.hero.health_percent;
        self.executor.enqueue("ember-flame-guard-danger", move || {
            info!(
                "🔥 Ember Spirit auto-casting Flame Guard on danger at {}% HP ({})",
                health_percent, key
            );
            press_key(key);
        });
    }
}

impl HeroScript for EmberSpiritScript {
    fn handle_gsi_event(&self, event: &GsiWebhookEvent) {
        let settings = self.settings.lock().unwrap();

        // The remnant chase is hotkey-driven; Flame Guard is the only thing
        // this hero does off GSI, on top of shared survivability.
        let survivability = SurvivabilityActions::new(self.settings.clone(), self.executor.clone());
        let in_danger = crate::actions::danger_detector::update(event, &settings.danger_detection);
        // Left here for the hotkey path, which has no event of its own.
        HAS_SCEPTER.store(has_scepter(event), Ordering::Relaxed);
        self.maybe_trigger_flame_guard(event, &settings.heroes.ember_spirit, in_danger);
        drop(settings);
        survivability.check_and_use_healing_items_with_danger(event, in_danger);
        survivability.use_defensive_items_if_danger_with_snapshot(event, in_danger);
        survivability.use_neutral_item_if_danger_with_snapshot(event, in_danger);
    }

    fn handle_standalone_trigger(&self) {
        let settings = self.settings.lock().unwrap();
        let ember = &settings.heroes.ember_spirit;
        let enabled = ember.enabled;
        let remnant_key = ember.remnant_key;
        let activate_key = ember.activate_key;
        let has_scepter = HAS_SCEPTER.load(Ordering::Relaxed);
        let on_the_scepter_delay = has_scepter && ember.use_scepter_activate_delay;
        let activate_delay_ms = resolve_activate_delay_ms(ember, has_scepter);
        drop(settings);

        if !enabled {
            info!("🔥 Ember Spirit remnant chase skipped: disabled in config");
            return;
        }

        if on_the_scepter_delay {
            info!(
                "🔥 Ember Spirit remnant chase triggered ({}ms activate delay, Aghanim's)",
                activate_delay_ms
            );
        } else {
            info!(
                "🔥 Ember Spirit remnant chase triggered ({}ms activate delay)",
                activate_delay_ms
            );
        }
        EmberSpiritState::execute_remnant_chase(remnant_key, activate_key, activate_delay_ms);
    }

    fn hero_name(&self) -> &'static str {
        Hero::EmberSpirit.to_game_name()
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
    fn ember_spirit_fixture() -> GsiWebhookEvent {
        serde_json::from_str(include_str!(
            "../../../tests/fixtures/ember_spirit_event.json"
        ))
        .expect("Ember Spirit fixture should deserialize")
    }

    fn default_config() -> EmberSpiritConfig {
        Settings::default().heroes.ember_spirit
    }

    #[test]
    fn build_remnant_chase_request_preserves_keys_and_delay() {
        let request = build_remnant_chase_request('r', 'd', 150);
        assert_eq!(
            request,
            EmberSpiritRequest::RemnantChase {
                remnant_key: 'r',
                activate_key: 'd',
                activate_delay_ms: 150,
            }
        );
    }

    #[test]
    fn scepter_switches_to_the_shorter_activate_delay() {
        let config = default_config();
        assert_eq!(
            resolve_activate_delay_ms(&config, true),
            config.scepter_activate_delay_ms
        );
    }

    /// Shorter, but still a wait — the activate has to land after the remnant
    /// registers server-side even with the scepter.
    #[test]
    fn the_scepter_delay_is_shorter_than_the_plain_one_but_not_zero() {
        let config = default_config();
        assert!(config.scepter_activate_delay_ms > 0);
        assert!(config.scepter_activate_delay_ms < config.activate_delay_ms);
    }

    #[test]
    fn the_configured_delay_stands_without_a_scepter() {
        let config = default_config();
        assert_eq!(
            resolve_activate_delay_ms(&config, false),
            config.activate_delay_ms
        );
    }

    /// The checkbox is the escape hatch: with it off the scepter changes
    /// nothing about the timing.
    #[test]
    fn the_scepter_delay_can_be_turned_off() {
        let mut config = default_config();
        config.use_scepter_activate_delay = false;

        assert_eq!(
            resolve_activate_delay_ms(&config, true),
            config.activate_delay_ms
        );
    }

    #[test]
    fn scepter_is_detected_from_the_hero_flag() {
        let mut event = ember_spirit_fixture();
        assert!(!has_scepter(&event));

        event.hero.aghanims_scepter = true;
        assert!(has_scepter(&event));
    }

    /// A scepter sitting in a slot already grants the upgrade, so the
    /// inventory scan backs up the hero flag.
    #[test]
    fn scepter_is_detected_from_an_inventory_slot() {
        let mut event = ember_spirit_fixture();
        event.hero.aghanims_scepter = false;
        event.items.slot0.name = ULTIMATE_SCEPTER_ITEM_NAME.to_string();

        assert!(has_scepter(&event));
    }

    #[test]
    fn hero_name_is_the_gsi_internal_name() {
        let script = EmberSpiritScript::new(
            Arc::new(Mutex::new(Settings::default())),
            ActionExecutor::new(),
        );
        assert_eq!(script.hero_name(), "npc_dota_hero_ember_spirit");
    }

    #[test]
    fn finds_flame_guard_when_levelled_and_castable() {
        let event = ember_spirit_fixture();
        assert!(ability_is_ready(&event, FLAME_GUARD_ABILITY_NAME));
    }

    /// Flame Guard is found wherever it sits, not at the index its key
    /// suggests. Ember carries Activate Fire Remnant as its own GSI entry, so
    /// the slot a key implies is already off by one in a real payload.
    #[test]
    fn flame_guard_is_found_by_name_not_by_the_slot_its_key_suggests() {
        let mut event = ember_spirit_fixture();
        let flame_guard = event.abilities.ability2.clone();
        event.abilities.ability2 = event.abilities.ability5.clone();
        event.abilities.ability5 = flame_guard;

        assert!(ability_is_ready(&event, FLAME_GUARD_ABILITY_NAME));
    }

    #[test]
    fn flame_guard_fires_in_danger_below_the_threshold() {
        let event = ember_spirit_fixture();
        assert!(should_trigger_flame_guard(
            &event,
            &default_config(),
            true,
            Instant::now(),
            None
        ));
    }

    #[test]
    fn flame_guard_holds_when_not_in_danger() {
        let event = ember_spirit_fixture();
        assert!(!should_trigger_flame_guard(
            &event,
            &default_config(),
            false,
            Instant::now(),
            None
        ));
    }

    /// Danger alone is not enough — the danger detector trips on a rapid HP
    /// drop, which happens at full health too.
    #[test]
    fn flame_guard_holds_above_the_hp_threshold() {
        let mut event = ember_spirit_fixture();
        event.hero.health_percent = default_config().flame_guard_hp_threshold_percent + 1;

        assert!(!should_trigger_flame_guard(
            &event,
            &default_config(),
            true,
            Instant::now(),
            None
        ));
    }

    #[test]
    fn flame_guard_holds_when_the_ability_is_not_castable() {
        let mut event = ember_spirit_fixture();
        assert_eq!(event.abilities.ability2.name, FLAME_GUARD_ABILITY_NAME);
        event.abilities.ability2.can_cast = false;

        assert!(!should_trigger_flame_guard(
            &event,
            &default_config(),
            true,
            Instant::now(),
            None
        ));
    }

    #[test]
    fn flame_guard_holds_while_ember_cannot_act() {
        let config = default_config();
        let now = Instant::now();

        for break_state in [
            |event: &mut GsiWebhookEvent| event.hero.stunned = true,
            |event: &mut GsiWebhookEvent| event.hero.silenced = true,
            |event: &mut GsiWebhookEvent| event.hero.hexed = true,
            |event: &mut GsiWebhookEvent| event.hero.alive = false,
        ] {
            let mut event = ember_spirit_fixture();
            break_state(&mut event);
            assert!(!should_trigger_flame_guard(&event, &config, true, now, None));
        }
    }

    #[test]
    fn flame_guard_respects_its_own_retry_cooldown() {
        let event = ember_spirit_fixture();
        let config = default_config();
        let now = Instant::now();

        assert!(!should_trigger_flame_guard(
            &event,
            &config,
            true,
            now,
            Some(now - Duration::from_millis(config.flame_guard_trigger_cooldown_ms - 1))
        ));
        assert!(should_trigger_flame_guard(
            &event,
            &config,
            true,
            now,
            Some(now - Duration::from_millis(config.flame_guard_trigger_cooldown_ms))
        ));
    }

    #[test]
    fn flame_guard_holds_when_the_auto_cast_is_disabled() {
        let event = ember_spirit_fixture();
        let mut config = default_config();
        config.auto_flame_guard_on_danger = false;

        assert!(!should_trigger_flame_guard(
            &event,
            &config,
            true,
            Instant::now(),
            None
        ));
    }

    /// The remnant chase toggle and the Flame Guard auto-cast are independent:
    /// turning the chase off must not silently disable the defensive half.
    #[test]
    fn flame_guard_is_not_gated_by_the_remnant_chase_toggle() {
        let event = ember_spirit_fixture();
        let mut config = default_config();
        config.enabled = false;

        assert!(should_trigger_flame_guard(
            &event,
            &config,
            true,
            Instant::now(),
            None
        ));
    }
}
