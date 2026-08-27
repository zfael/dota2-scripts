//! GSI game-state probe.
//!
//! Answers one question the draft matcher cannot answer for itself: **does Dota
//! tell a player, not just a spectator, when hero selection is happening?**
//!
//! Why this matters. The draft matcher reads ten fixed screen regions and
//! returns the nearest of 127 hero portraits for each. It is good at that — but
//! it has no idea what screen it is looking at. Pointed at the main menu it
//! confidently reported pangolier, wisp and dazzle for an item icon, a menu
//! caption and a level badge. Contrast, absolute score and margin all fail to
//! separate that case from a real draft (a correct Pugna scored 0.585 while
//! Dazzle-on-menu-art scored 0.573), so no threshold on the vision side fixes
//! it. The matcher needs an external gate telling it when to look.
//!
//! `map.game_state` is the natural gate. The `draft` block is spectator-only —
//! already confirmed — but `game_state` lives in the `map` block, which the
//! shipped .cfg has requested all along. The app throws it away today only
//! because `models::gsi_event::Map` models nothing but `clock_time`.
//!
//! Defaults to port 3000, which the shipped `gamestate_integration_dotaevents.cfg`
//! already points at — so there is no new config to write and no Dota restart.
//! **Stop the main app first**: only one process can hold the port.
//!
//! ```text
//! cargo run --example gsi_state_probe
//! ```
//!
//! To watch a draft without stopping the app, run on a spare port and add a
//! second Dota config beside the existing one (Dota POSTs to every configured
//! endpoint — that is how dotaplus and overwolf coexist here):
//!
//! ```text
//! cargo run --example gsi_state_probe -- --port 3009
//! ```
//!
//! ```text
//! "gsi state probe"
//! {
//!    "uri"       "http://localhost:3009/"
//!    "timeout"   "5.0"
//!    "buffer"    "0.1"
//!    "throttle"  "0.1"
//!    "heartbeat" "30.0"
//!    "data" { "provider" "1" "map" "1" "player" "1" "hero" "1" "draft" "1" }
//! }
//! ```
//!
//! Save that as `gamestate_integration_stateprobe.cfg` in Dota's
//! `game/dota/cfg/gamestate_integration/` and restart Dota.
//!
//! Either way: sit through a draft. Every distinct `game_state` is printed once,
//! as it first appears, and raw payloads are appended to
//! `logs/gsi_state_probe.jsonl` so a state we did not anticipate can still be
//! recovered afterwards.

use axum::{extract::State, routing::post, Json, Router};
use serde_json::Value;
use std::collections::BTreeSet;
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Matches the shipped .cfg, so the common case needs no Dota-side change.
const DEFAULT_PORT: u16 = 3000;
const LOG_PATH: &str = "logs/gsi_state_probe.jsonl";

#[derive(Clone)]
struct Probe {
    /// States already reported, so a 30s heartbeat does not flood the console.
    seen: Arc<Mutex<BTreeSet<String>>>,
    log: Arc<Mutex<PathBuf>>,
}

fn parse_port() -> u16 {
    let raw: Vec<String> = env::args().collect();
    match raw.iter().position(|a| a == "--port") {
        Some(i) => raw.get(i + 1).and_then(|v| v.parse().ok()).unwrap_or_else(|| {
            eprintln!("Error: --port requires a port number");
            std::process::exit(1);
        }),
        None => DEFAULT_PORT,
    }
}

#[tokio::main]
async fn main() {
    let port = parse_port();

    println!("GSI Game-State Probe");
    println!("  listening : http://127.0.0.1:{port}/");
    println!("  raw log   : {LOG_PATH}");
    println!();
    println!("Waiting for Dota. Sit through a draft, then Ctrl+C.");
    println!("Each new game_state prints once, on first sight.");
    println!();

    if let Some(parent) = PathBuf::from(LOG_PATH).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let probe = Probe {
        seen: Arc::new(Mutex::new(BTreeSet::new())),
        log: Arc::new(Mutex::new(PathBuf::from(LOG_PATH))),
    };

    let app = Router::new().route("/", post(handle)).with_state(probe);

    let listener = match tokio::net::TcpListener::bind(("127.0.0.1", port)).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Cannot bind port {port}: {e}");
            if port == DEFAULT_PORT {
                eprintln!("The main app is almost certainly holding it. Stop it and");
                eprintln!("retry, or run on a spare port -- see the header for the");
                eprintln!("second .cfg that needs: cargo run --example gsi_state_probe -- --port 3009");
            } else {
                eprintln!("Another probe already running, or the port is taken.");
            }
            std::process::exit(1);
        }
    };

    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("Server error: {e}");
        std::process::exit(1);
    }
}

/// Describes a block precisely enough to answer "is it usable?".
///
/// Dota sends `"draft":{}` at the main menu. Reporting that as merely "present"
/// would suggest a player gets draft data when the block is in fact empty, which
/// is the exact question this probe exists to settle.
fn describe(body: &Value, key: &str) -> String {
    match body.get(key) {
        None | Some(Value::Null) => "absent".to_string(),
        Some(Value::Object(o)) if o.is_empty() => "empty {}".to_string(),
        Some(Value::Object(o)) => {
            let mut keys: Vec<&str> = o.keys().map(String::as_str).collect();
            keys.sort_unstable();
            format!("{} keys: {}", o.len(), keys.join(", "))
        }
        Some(other) => other.to_string(),
    }
}

async fn handle(State(probe): State<Probe>, mut body: Json<Value>) -> &'static str {
    // The payload echoes the shared token from the .cfg. It is local-only, but
    // there is no reason for it to sit in a log file we may paste around.
    if let Some(obj) = body.as_object_mut() {
        obj.remove("auth");
    }
    let body = body.0;

    // Append first: a payload that panics the reporting below is still evidence.
    if let Ok(path) = probe.log.lock() {
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&*path) {
            let _ = writeln!(f, "{body}");
        }
    }

    let map = body.get("map");
    let state = map
        .and_then(|m| m.get("game_state"))
        .and_then(Value::as_str)
        .unwrap_or("<absent>");

    let mut seen = match probe.seen.lock() {
        Ok(s) => s,
        Err(_) => return "ok",
    };
    if !seen.insert(state.to_string()) {
        return "ok";
    }

    // The three facts that decide whether this can gate the matcher: the state
    // string itself, whether a hero is assigned yet, and whether the
    // spectator-only draft block happens to be present for a player.
    let hero = body
        .get("hero")
        .and_then(|h| h.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("<none>");

    println!("NEW game_state: {state}");
    println!("    hero  : {hero}");
    println!("    draft : {}", describe(&body, "draft"));
    println!("    map   : {}", describe(&body, "map"));
    println!();

    "ok"
}
