//! Probe: the keybinding path, end to end, with no game and no GUI (issue #20).
//!
//! Written first as a diagnosis — every step below reproduced a real failure —
//! and kept as the regression check for the fix. Each step now asserts the
//! *fixed* behaviour and prints what it used to do, so a regression fails loudly
//! rather than quietly returning the app to defaults.
//!
//! | Step | Path exercised | Used to |
//! |---|---|---|
//! | 1 | `toml::from_str::<Settings>` on a hand-edited `config.toml` | reject the whole file over one key |
//! | 2 | the `update_config` command's `serde_json` round-trip | reject any key that was not one character |
//! | 3 | press vs. intercept mappings | let Space save and then half-work |
//! | 4 | `persist_config_section` | overwrite a hand edit anywhere in the file |
//! | 5 | mouse buttons | be unrepresentable at all |
//!
//! Run: `cargo run --example keybind_config_probe`
//! Exits non-zero if any expectation is not met.

use dota2_scripts::config::settings::recover_readable_sections;
use dota2_scripts::config::storage::{merge_saved_settings_with_existing, persist_config_section};
use dota2_scripts::config::Settings;
use dota2_scripts::input::binding::{KeyBinding, NamedKey};

fn main() {
    let mut failures = 0usize;
    let mut total = 0usize;
    let mut check = |label: &str, passed: bool| {
        println!("  [{}] {label}", if passed { "PASS" } else { "FAIL" });
        total += 1;
        if !passed {
            failures += 1;
        }
    };

    println!("== Step 1: hand-edited config.toml =========================\n");
    println!("  Was: one unrepresentable key made Settings::load() discard the");
    println!("       entire file and revert every other setting too.\n");

    // A plausible edit from a player whose item slots are not single letters.
    let hand_edited = r#"
[server]
port = 3100

[keybindings]
slot0 = "Space"
slot1 = "F1"
slot2 = "Mouse4"
slot3 = "Numpad7"
slot4 = "b"
slot5 = "n"
neutral0 = "0"
combo_trigger = "Home"
"#;

    match toml::from_str::<Settings>(hand_edited) {
        Ok(settings) => {
            let k = &settings.keybindings;
            println!(
                "  parsed: slot0={} slot1={} slot2={} slot3={}",
                k.slot0, k.slot1, k.slot2, k.slot3
            );
            check("the whole file parses", true);
            check(
                "Space survives as the named key",
                k.slot0 == KeyBinding::Named(NamedKey::Space),
            );
            check("Mouse4 survives", k.slot2 == KeyBinding::parse("Mouse4"));
            check(
                "an unrelated section is untouched",
                settings.server.port == 3100,
            );
        }
        Err(e) => {
            println!("  parse failed: {e}");
            check("the whole file parses", false);
        }
    }

    println!("\n  A value we still do not understand degrades to that one slot:");
    let with_garbage = hand_edited.replace("\"Space\"", "\"Ctrl+Shift+Q\"");
    match toml::from_str::<Settings>(&with_garbage) {
        Ok(settings) => {
            check("the file still parses", true);
            check(
                "the bad slot is inert but kept verbatim",
                settings.keybindings.slot0.is_unknown()
                    && settings.keybindings.slot0.to_string() == "Ctrl+Shift+Q",
            );
            check(
                "the other slots are unaffected",
                settings.keybindings.slot1 == KeyBinding::Named(NamedKey::F1),
            );
        }
        Err(e) => {
            println!("  parse failed: {e}");
            check("the file still parses", false);
        }
    }

    println!("\n  And a section that is broken beyond a binding drops alone:");
    let broken_section = r#"
[server]
port = "not a number"

[keybindings]
slot0 = "q"
"#;
    let recovered = recover_readable_sections(broken_section);
    check(
        "the readable section is kept",
        recovered.keybindings.slot0 == KeyBinding::Char('q'),
    );
    check(
        "the unreadable section falls back to its default",
        recovered.server.port == Settings::default().server.port,
    );

    println!("\n== Step 2: Settings UI -> update_config ====================\n");
    println!("  Was: anything longer than one character was rejected, and the UI");
    println!("       only console.error()d it — so the key looked set until restart.\n");

    let attempt = |value: serde_json::Value| -> Option<KeyBinding> {
        let mut config_value = serde_json::to_value(Settings::default()).unwrap();
        config_value["keybindings"]["slot0"] = value;
        serde_json::from_value::<Settings>(config_value)
            .ok()
            .map(|s| s.keybindings.slot0)
    };

    for (label, value, expected) in [
        ("a letter \"Z\"", serde_json::json!("Z"), "z"),
        ("the space bar \" \"", serde_json::json!(" "), "Space"),
        ("\"F1\"", serde_json::json!("F1"), "F1"),
        ("\"Mouse4\"", serde_json::json!("Mouse4"), "Mouse4"),
        ("\"Numpad0\"", serde_json::json!("Numpad0"), "Numpad0"),
    ] {
        match attempt(value) {
            Some(binding) => {
                println!("    {label:<22} -> stored as \"{binding}\"");
                check(label, binding.to_string() == expected);
            }
            None => {
                println!("    {label:<22} -> REJECTED");
                check(label, false);
            }
        }
    }

    println!("\n  The space bar is the one the user has to be able to see:");
    check(
        "\" \" displays as \"Space\", not a blank",
        KeyBinding::parse(" ").to_string() == "Space",
    );

    println!("\n== Step 3: press vs. intercept =============================\n");
    println!("  Was: Space saved and could be pressed, but rdev could not see it,");
    println!("       so Soul Ring and armlet chords silently ignored that slot.\n");

    for raw in ["z", "Space", "F1", "Mouse4", "Numpad7", "-"] {
        let binding = KeyBinding::parse(raw);
        let pressable = binding.pressable().is_some();
        let interceptable = binding.is_interceptable();
        println!(
            "    {raw:<8} press={} intercept={}",
            if pressable { "yes" } else { "NO " },
            if interceptable { "yes" } else { "NO " }
        );
        check(
            &format!("{raw} works on both layers"),
            pressable && interceptable,
        );
    }

    println!("\n== Step 4: a UI change next to a hand edit =================\n");
    println!("  Was: the next UI toggle serialised the whole in-memory Settings");
    println!("       over the file, reverting hand edits in every other section.\n");

    // What the app holds at startup, versus what the user typed since.
    let on_disk = "[keybindings]\nslot0 = \"q\"\n\n[server]\nport = 3100\n";
    let one_section = "[keybindings]\nslot0 = \"z\"\n";

    let merged = merge_saved_settings_with_existing(on_disk, one_section).unwrap();
    let merged: toml::Value = toml::from_str(&merged).unwrap();

    check(
        "the edited section is written",
        merged["keybindings"]["slot0"].as_str() == Some("z"),
    );
    check(
        "a hand edit in another section survives",
        merged["server"]["port"].as_integer() == Some(3100),
    );

    // And through the real file-writing path.
    let temp = std::env::temp_dir().join("dota2-scripts-keybind-probe");
    let _ = std::fs::remove_dir_all(&temp);
    let paths = dota2_scripts::config::storage::ConfigPaths::from_parts(
        temp.join("LocalAppData"),
        temp.join("install"),
    );
    std::fs::create_dir_all(paths.live_config_path().parent().unwrap()).unwrap();
    std::fs::write(paths.live_config_path(), on_disk).unwrap();

    let section = toml::Value::try_from(Settings::default())
        .unwrap()
        .get("keybindings")
        .unwrap()
        .clone();
    persist_config_section(&paths, "keybindings", &section, "").unwrap();

    let written: toml::Value =
        toml::from_str(&std::fs::read_to_string(paths.live_config_path()).unwrap()).unwrap();
    check(
        "persist_config_section leaves [server] alone",
        written["server"]["port"].as_integer() == Some(3100),
    );
    let _ = std::fs::remove_dir_all(&temp);

    println!("\n== Step 5: mouse buttons ===================================\n");
    println!("  Was: unrepresentable in a `char`, invisible to a keydown-only");
    println!("       capture, and Mouse4 navigated the app back to Dashboard.\n");

    let mouse4 = KeyBinding::parse("Mouse4");
    check(
        "Mouse4 maps to the Windows X button rdev reports",
        mouse4.rdev_button() == Some(rdev::Button::Unknown(1)),
    );
    check(
        "Mouse4 is matched from a real grab event",
        mouse4.matches_event(&rdev::EventType::ButtonPress(rdev::Button::Unknown(1))),
    );
    check(
        "Mouse5 maps to the other X button",
        KeyBinding::parse("Mouse5").rdev_button() == Some(rdev::Button::Unknown(2)),
    );
    check(
        "left click stays unbindable — Dota needs it",
        KeyBinding::from_event(&rdev::EventType::ButtonPress(rdev::Button::Left)).is_none(),
    );

    println!("\n  Not checkable here (WebView2 behaviour): App.tsx now uses");
    println!("  <MemoryRouter>, so the webview's Back gesture has no in-app history");
    println!("  to pop, and a global handler swallows buttons 3/4. Verify by hand.");

    println!("\n===========================================================");
    if failures == 0 {
        println!("All {total} expectations held.");
        println!("Every step above reproduced a real bug before the fix.");
    } else {
        println!("{failures} expectation(s) FAILED — the keybinding path has regressed.");
        std::process::exit(1);
    }
}
