//! One bindable input — a character key, a named key, a mouse button.
//!
//! Item slots used to be stored as `char`, which quietly ruled out every bind a
//! Dota player might actually use for an item: Space, F-keys, numpad, and the
//! side mouse buttons. Worse, `char` made a bad value a *parse error*, so one
//! unrepresentable key made `Settings::load()` throw the whole config file away
//! and fall back to defaults (issue #20).
//!
//! Two rules hold this together:
//!
//! - **Deserializing never fails.** Anything unrecognised becomes
//!   [`KeyBinding::Unknown`], which keeps the user's text verbatim so saving the
//!   config back does not destroy what they typed. It is inert at the input
//!   layer and says so in the log.
//! - **The two input layers are asymmetric and both must be checked.** Outbound
//!   presses go through `enigo`; interception (Soul Ring, armlet chords) goes
//!   through `rdev`. A binding is only fully usable when both know it, so
//!   [`KeyBinding::is_interceptable`] is separate from [`KeyBinding::pressable`].
//!
//! `examples/keybind_config_probe.rs` is the regression check.

use std::fmt;

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize, Serializer};

/// A key or button the user has bound to something.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KeyBinding {
    /// A printable character key, always stored lowercased.
    ///
    /// Build one with [`KeyBinding::parse`] or `KeyBinding::from(char)`, never
    /// the variant directly: equality and hashing are plain derives, so an
    /// uppercase char constructed by hand would silently fail to match the same
    /// key arriving from the config or the grab layer.
    Char(char),
    /// A key with a name rather than a character.
    Named(NamedKey),
    /// An extra mouse button. Left and right are deliberately absent: Dota needs
    /// them for move and attack orders, and grabbing one would break the game.
    Mouse(MouseBinding),
    /// Something we could not parse, kept verbatim so a save does not lose it.
    Unknown(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NamedKey {
    Space,
    Tab,
    Enter,
    Escape,
    Backspace,
    Insert,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    Up,
    Down,
    Left,
    Right,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseBinding {
    /// Middle click / mouse wheel button.
    Middle,
    /// The "back" thumb button. `XBUTTON1` on Windows.
    Mouse4,
    /// The "forward" thumb button. `XBUTTON2` on Windows.
    Mouse5,
}

/// A binding reduced to what the press path needs, and nothing more.
///
/// Deliberately `Copy`: it rides inside the synthetic-input command enum, which
/// is copied through a channel. [`KeyBinding::Unknown`] has no representation
/// here, so an unusable binding cannot reach the press path at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressableKey {
    Key(enigo::Key),
    Button(enigo::Button),
}

/// Canonical name, rdev key, enigo key — one row per named key.
///
/// Kept as a table rather than three matches so a new key cannot be added to one
/// layer and forgotten in the other.
const NAMED_KEYS: &[(NamedKey, &str, rdev::Key, enigo::Key)] = &[
    (NamedKey::Space, "Space", rdev::Key::Space, enigo::Key::Space),
    (NamedKey::Tab, "Tab", rdev::Key::Tab, enigo::Key::Tab),
    (NamedKey::Enter, "Enter", rdev::Key::Return, enigo::Key::Return),
    (NamedKey::Escape, "Escape", rdev::Key::Escape, enigo::Key::Escape),
    (
        NamedKey::Backspace,
        "Backspace",
        rdev::Key::Backspace,
        enigo::Key::Backspace,
    ),
    (
        NamedKey::Insert,
        "Insert",
        rdev::Key::Insert,
        enigo::Key::Insert,
    ),
    (
        NamedKey::Delete,
        "Delete",
        rdev::Key::Delete,
        enigo::Key::Delete,
    ),
    (NamedKey::Home, "Home", rdev::Key::Home, enigo::Key::Home),
    (NamedKey::End, "End", rdev::Key::End, enigo::Key::End),
    (
        NamedKey::PageUp,
        "PageUp",
        rdev::Key::PageUp,
        enigo::Key::PageUp,
    ),
    (
        NamedKey::PageDown,
        "PageDown",
        rdev::Key::PageDown,
        enigo::Key::PageDown,
    ),
    (
        NamedKey::Up,
        "Up",
        rdev::Key::UpArrow,
        enigo::Key::UpArrow,
    ),
    (
        NamedKey::Down,
        "Down",
        rdev::Key::DownArrow,
        enigo::Key::DownArrow,
    ),
    (
        NamedKey::Left,
        "Left",
        rdev::Key::LeftArrow,
        enigo::Key::LeftArrow,
    ),
    (
        NamedKey::Right,
        "Right",
        rdev::Key::RightArrow,
        enigo::Key::RightArrow,
    ),
    (NamedKey::F1, "F1", rdev::Key::F1, enigo::Key::F1),
    (NamedKey::F2, "F2", rdev::Key::F2, enigo::Key::F2),
    (NamedKey::F3, "F3", rdev::Key::F3, enigo::Key::F3),
    (NamedKey::F4, "F4", rdev::Key::F4, enigo::Key::F4),
    (NamedKey::F5, "F5", rdev::Key::F5, enigo::Key::F5),
    (NamedKey::F6, "F6", rdev::Key::F6, enigo::Key::F6),
    (NamedKey::F7, "F7", rdev::Key::F7, enigo::Key::F7),
    (NamedKey::F8, "F8", rdev::Key::F8, enigo::Key::F8),
    (NamedKey::F9, "F9", rdev::Key::F9, enigo::Key::F9),
    (NamedKey::F10, "F10", rdev::Key::F10, enigo::Key::F10),
    (NamedKey::F11, "F11", rdev::Key::F11, enigo::Key::F11),
    (NamedKey::F12, "F12", rdev::Key::F12, enigo::Key::F12),
    // Numpad. enigo has no named numpad variants, so these go through
    // `Key::Other` with the Windows virtual-key code (VK_NUMPAD0 = 0x60).
    // This app is Windows-only, so that is a fact rather than an assumption.
    (
        NamedKey::Numpad0,
        "Numpad0",
        rdev::Key::Kp0,
        enigo::Key::Other(0x60),
    ),
    (
        NamedKey::Numpad1,
        "Numpad1",
        rdev::Key::Kp1,
        enigo::Key::Other(0x61),
    ),
    (
        NamedKey::Numpad2,
        "Numpad2",
        rdev::Key::Kp2,
        enigo::Key::Other(0x62),
    ),
    (
        NamedKey::Numpad3,
        "Numpad3",
        rdev::Key::Kp3,
        enigo::Key::Other(0x63),
    ),
    (
        NamedKey::Numpad4,
        "Numpad4",
        rdev::Key::Kp4,
        enigo::Key::Other(0x64),
    ),
    (
        NamedKey::Numpad5,
        "Numpad5",
        rdev::Key::Kp5,
        enigo::Key::Other(0x65),
    ),
    (
        NamedKey::Numpad6,
        "Numpad6",
        rdev::Key::Kp6,
        enigo::Key::Other(0x66),
    ),
    (
        NamedKey::Numpad7,
        "Numpad7",
        rdev::Key::Kp7,
        enigo::Key::Other(0x67),
    ),
    (
        NamedKey::Numpad8,
        "Numpad8",
        rdev::Key::Kp8,
        enigo::Key::Other(0x68),
    ),
    (
        NamedKey::Numpad9,
        "Numpad9",
        rdev::Key::Kp9,
        enigo::Key::Other(0x69),
    ),
];

/// Spellings we accept on the way in but never write back out.
///
/// Left side is already lowercased; right side must match a `NAMED_KEYS` name
/// (also compared lowercased).
const NAME_ALIASES: &[(&str, &str)] = &[
    ("return", "Enter"),
    ("esc", "Escape"),
    ("del", "Delete"),
    ("ins", "Insert"),
    ("pgup", "PageUp"),
    ("pgdn", "PageDown"),
    ("pagedn", "PageDown"),
    ("uparrow", "Up"),
    ("downarrow", "Down"),
    ("leftarrow", "Left"),
    ("rightarrow", "Right"),
    ("spacebar", "Space"),
    ("num0", "Numpad0"),
    ("num1", "Numpad1"),
    ("num2", "Numpad2"),
    ("num3", "Numpad3"),
    ("num4", "Numpad4"),
    ("num5", "Numpad5"),
    ("num6", "Numpad6"),
    ("num7", "Numpad7"),
    ("num8", "Numpad8"),
    ("num9", "Numpad9"),
];

const MOUSE_BUTTONS: &[(MouseBinding, &str, u8, enigo::Button)] = &[
    (MouseBinding::Middle, "Mouse3", 0, enigo::Button::Middle),
    (MouseBinding::Mouse4, "Mouse4", 1, enigo::Button::Back),
    (MouseBinding::Mouse5, "Mouse5", 2, enigo::Button::Forward),
];

const MOUSE_ALIASES: &[(&str, &str)] = &[
    ("middlemouse", "Mouse3"),
    ("mousemiddle", "Mouse3"),
    ("mmb", "Mouse3"),
    ("mousewheel", "Mouse3"),
    ("mouseback", "Mouse4"),
    ("mouse_back", "Mouse4"),
    ("xbutton1", "Mouse4"),
    ("mouseforward", "Mouse5"),
    ("mouse_forward", "Mouse5"),
    ("xbutton2", "Mouse5"),
];

/// Punctuation `rdev` can recognise on a US layout.
///
/// Only used for interception — the press path sends these as Unicode, which
/// works regardless of whether this table has the key.
const PUNCTUATION_KEYS: &[(char, rdev::Key)] = &[
    ('-', rdev::Key::Minus),
    ('=', rdev::Key::Equal),
    ('[', rdev::Key::LeftBracket),
    (']', rdev::Key::RightBracket),
    (';', rdev::Key::SemiColon),
    ('\'', rdev::Key::Quote),
    ('\\', rdev::Key::BackSlash),
    (',', rdev::Key::Comma),
    ('.', rdev::Key::Dot),
    ('/', rdev::Key::Slash),
    ('`', rdev::Key::BackQuote),
];

impl KeyBinding {
    /// Parse a binding from config text or from what the UI captured.
    ///
    /// Never returns an error: anything unrecognised becomes
    /// [`KeyBinding::Unknown`] holding the original text.
    pub fn parse(raw: &str) -> Self {
        // The space bar reaches us from the UI as `e.key === " "`, and from a
        // hand-written config as `" "`. Both must become the named key so the
        // value round-trips as "Space" and shows as "Space" rather than a blank
        // button — and this has to happen *before* trimming, which would erase
        // the character entirely.
        if raw == " " {
            return KeyBinding::Named(NamedKey::Space);
        }

        let trimmed = raw.trim();

        // A single character is the common case and needs no table.
        let mut chars = trimmed.chars();
        if let (Some(ch), None) = (chars.next(), chars.next()) {
            return KeyBinding::Char(ch.to_ascii_lowercase());
        }

        let lowered = trimmed.to_ascii_lowercase();

        let canonical_name = NAME_ALIASES
            .iter()
            .find(|(alias, _)| *alias == lowered)
            .map(|(_, canonical)| canonical.to_ascii_lowercase())
            .unwrap_or_else(|| lowered.clone());

        if let Some((named, _, _, _)) = NAMED_KEYS
            .iter()
            .find(|(_, name, _, _)| name.to_ascii_lowercase() == canonical_name)
        {
            return KeyBinding::Named(*named);
        }

        let canonical_mouse = MOUSE_ALIASES
            .iter()
            .find(|(alias, _)| *alias == lowered)
            .map(|(_, canonical)| canonical.to_ascii_lowercase())
            .unwrap_or(lowered);

        if let Some((button, _, _, _)) = MOUSE_BUTTONS
            .iter()
            .find(|(_, name, _, _)| name.to_ascii_lowercase() == canonical_mouse)
        {
            return KeyBinding::Mouse(*button);
        }

        KeyBinding::Unknown(trimmed.to_string())
    }

    /// True when this binding cannot drive anything — the config had a value we
    /// do not understand.
    pub fn is_unknown(&self) -> bool {
        matches!(self, KeyBinding::Unknown(_))
    }

    /// What to hand the press path, or `None` when the binding is unusable.
    pub fn pressable(&self) -> Option<PressableKey> {
        match self {
            KeyBinding::Char(ch) => Some(PressableKey::Key(enigo::Key::Unicode(*ch))),
            KeyBinding::Named(named) => NAMED_KEYS
                .iter()
                .find(|(candidate, _, _, _)| candidate == named)
                .map(|(_, _, _, key)| PressableKey::Key(*key)),
            KeyBinding::Mouse(button) => MOUSE_BUTTONS
                .iter()
                .find(|(candidate, _, _, _)| candidate == button)
                .map(|(_, _, _, enigo_button)| PressableKey::Button(*enigo_button)),
            KeyBinding::Unknown(_) => None,
        }
    }

    /// The `rdev` key this binding listens for, if it is a key at all.
    pub fn rdev_key(&self) -> Option<rdev::Key> {
        match self {
            KeyBinding::Char(ch) => char_to_rdev_key(*ch),
            KeyBinding::Named(named) => NAMED_KEYS
                .iter()
                .find(|(candidate, _, _, _)| candidate == named)
                .map(|(_, _, key, _)| *key),
            KeyBinding::Mouse(_) | KeyBinding::Unknown(_) => None,
        }
    }

    /// The `rdev` button this binding listens for, if it is a mouse button.
    pub fn rdev_button(&self) -> Option<rdev::Button> {
        match self {
            KeyBinding::Mouse(button) => MOUSE_BUTTONS
                .iter()
                .find(|(candidate, _, _, _)| candidate == button)
                .map(|(_, _, code, _)| match code {
                    0 => rdev::Button::Middle,
                    other => rdev::Button::Unknown(*other),
                }),
            _ => None,
        }
    }

    /// Whether the grab layer can see this binding being pressed.
    ///
    /// Not the same question as [`Self::pressable`]. A binding can be perfectly
    /// pressable and still invisible to interception — that asymmetry is what
    /// made a Space-bound slot half-work before.
    pub fn is_interceptable(&self) -> bool {
        self.rdev_key().is_some() || self.rdev_button().is_some()
    }

    /// True when this binding is what the given grab event represents.
    pub fn matches_event(&self, event: &rdev::EventType) -> bool {
        match event {
            rdev::EventType::KeyPress(key) => self.rdev_key() == Some(*key),
            rdev::EventType::ButtonPress(button) => {
                self.rdev_button().as_ref() == Some(button)
            }
            _ => false,
        }
    }

    /// Build a binding from a grab event, so an observed press can be compared
    /// against a whole set of bindings at once.
    pub fn from_event(event: &rdev::EventType) -> Option<Self> {
        match event {
            rdev::EventType::KeyPress(key) => rdev_key_to_binding(*key),
            rdev::EventType::ButtonPress(button) => MOUSE_BUTTONS
                .iter()
                .find(|(_, _, code, _)| match button {
                    rdev::Button::Middle => *code == 0,
                    rdev::Button::Unknown(raw) => raw == code,
                    _ => false,
                })
                .map(|(binding, _, _, _)| KeyBinding::Mouse(*binding)),
            _ => None,
        }
    }
}

fn char_to_rdev_key(ch: char) -> Option<rdev::Key> {
    let lowered = ch.to_ascii_lowercase();

    let letter = match lowered {
        'a' => rdev::Key::KeyA,
        'b' => rdev::Key::KeyB,
        'c' => rdev::Key::KeyC,
        'd' => rdev::Key::KeyD,
        'e' => rdev::Key::KeyE,
        'f' => rdev::Key::KeyF,
        'g' => rdev::Key::KeyG,
        'h' => rdev::Key::KeyH,
        'i' => rdev::Key::KeyI,
        'j' => rdev::Key::KeyJ,
        'k' => rdev::Key::KeyK,
        'l' => rdev::Key::KeyL,
        'm' => rdev::Key::KeyM,
        'n' => rdev::Key::KeyN,
        'o' => rdev::Key::KeyO,
        'p' => rdev::Key::KeyP,
        'q' => rdev::Key::KeyQ,
        'r' => rdev::Key::KeyR,
        's' => rdev::Key::KeyS,
        't' => rdev::Key::KeyT,
        'u' => rdev::Key::KeyU,
        'v' => rdev::Key::KeyV,
        'w' => rdev::Key::KeyW,
        'x' => rdev::Key::KeyX,
        'y' => rdev::Key::KeyY,
        'z' => rdev::Key::KeyZ,
        '0' => rdev::Key::Num0,
        '1' => rdev::Key::Num1,
        '2' => rdev::Key::Num2,
        '3' => rdev::Key::Num3,
        '4' => rdev::Key::Num4,
        '5' => rdev::Key::Num5,
        '6' => rdev::Key::Num6,
        '7' => rdev::Key::Num7,
        '8' => rdev::Key::Num8,
        '9' => rdev::Key::Num9,
        other => {
            return PUNCTUATION_KEYS
                .iter()
                .find(|(candidate, _)| *candidate == other)
                .map(|(_, key)| *key)
        }
    };

    Some(letter)
}

fn rdev_key_to_binding(key: rdev::Key) -> Option<KeyBinding> {
    if let Some((named, _, _, _)) = NAMED_KEYS
        .iter()
        .find(|(_, _, candidate, _)| *candidate == key)
    {
        return Some(KeyBinding::Named(*named));
    }

    let ch = match key {
        rdev::Key::KeyA => 'a',
        rdev::Key::KeyB => 'b',
        rdev::Key::KeyC => 'c',
        rdev::Key::KeyD => 'd',
        rdev::Key::KeyE => 'e',
        rdev::Key::KeyF => 'f',
        rdev::Key::KeyG => 'g',
        rdev::Key::KeyH => 'h',
        rdev::Key::KeyI => 'i',
        rdev::Key::KeyJ => 'j',
        rdev::Key::KeyK => 'k',
        rdev::Key::KeyL => 'l',
        rdev::Key::KeyM => 'm',
        rdev::Key::KeyN => 'n',
        rdev::Key::KeyO => 'o',
        rdev::Key::KeyP => 'p',
        rdev::Key::KeyQ => 'q',
        rdev::Key::KeyR => 'r',
        rdev::Key::KeyS => 's',
        rdev::Key::KeyT => 't',
        rdev::Key::KeyU => 'u',
        rdev::Key::KeyV => 'v',
        rdev::Key::KeyW => 'w',
        rdev::Key::KeyX => 'x',
        rdev::Key::KeyY => 'y',
        rdev::Key::KeyZ => 'z',
        rdev::Key::Num0 => '0',
        rdev::Key::Num1 => '1',
        rdev::Key::Num2 => '2',
        rdev::Key::Num3 => '3',
        rdev::Key::Num4 => '4',
        rdev::Key::Num5 => '5',
        rdev::Key::Num6 => '6',
        rdev::Key::Num7 => '7',
        rdev::Key::Num8 => '8',
        rdev::Key::Num9 => '9',
        other => {
            return PUNCTUATION_KEYS
                .iter()
                .find(|(_, candidate)| *candidate == other)
                .map(|(ch, _)| KeyBinding::Char(*ch))
        }
    };

    Some(KeyBinding::Char(ch))
}

impl fmt::Display for KeyBinding {
    /// The canonical spelling — what goes in `config.toml` and what the UI shows.
    ///
    /// Space renders as "Space", never as a blank button.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyBinding::Char(ch) => write!(f, "{ch}"),
            KeyBinding::Named(named) => {
                let name = NAMED_KEYS
                    .iter()
                    .find(|(candidate, _, _, _)| candidate == named)
                    .map(|(_, name, _, _)| *name)
                    .unwrap_or("?");
                write!(f, "{name}")
            }
            KeyBinding::Mouse(button) => {
                let name = MOUSE_BUTTONS
                    .iter()
                    .find(|(candidate, _, _, _)| candidate == button)
                    .map(|(_, name, _, _)| *name)
                    .unwrap_or("?");
                write!(f, "{name}")
            }
            KeyBinding::Unknown(raw) => write!(f, "{raw}"),
        }
    }
}

impl From<char> for KeyBinding {
    fn from(ch: char) -> Self {
        if ch == ' ' {
            return KeyBinding::Named(NamedKey::Space);
        }
        KeyBinding::Char(ch.to_ascii_lowercase())
    }
}

impl Serialize for KeyBinding {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for KeyBinding {
    /// Accepts any string, and a bare character for configs written by hand.
    ///
    /// Deliberately infallible for strings: a binding we do not recognise must
    /// not take the rest of the config file down with it.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct BindingVisitor;

        impl de::Visitor<'_> for BindingVisitor {
            type Value = KeyBinding;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a key name such as \"z\", \"Space\", \"F1\" or \"Mouse4\"")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<KeyBinding, E> {
                Ok(KeyBinding::parse(value))
            }

            fn visit_char<E: de::Error>(self, value: char) -> Result<KeyBinding, E> {
                Ok(KeyBinding::from(value))
            }
        }

        deserializer.deserialize_str(BindingVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_shapes_a_player_would_write() {
        assert_eq!(KeyBinding::parse("z"), KeyBinding::Char('z'));
        assert_eq!(KeyBinding::parse("Z"), KeyBinding::Char('z'));
        assert_eq!(KeyBinding::parse("0"), KeyBinding::Char('0'));
        assert_eq!(KeyBinding::parse("Space"), KeyBinding::Named(NamedKey::Space));
        assert_eq!(KeyBinding::parse("space"), KeyBinding::Named(NamedKey::Space));
        assert_eq!(KeyBinding::parse("F1"), KeyBinding::Named(NamedKey::F1));
        assert_eq!(KeyBinding::parse("f12"), KeyBinding::Named(NamedKey::F12));
        assert_eq!(
            KeyBinding::parse("Mouse4"),
            KeyBinding::Mouse(MouseBinding::Mouse4)
        );
        assert_eq!(
            KeyBinding::parse("mouse5"),
            KeyBinding::Mouse(MouseBinding::Mouse5)
        );
        assert_eq!(
            KeyBinding::parse("Numpad7"),
            KeyBinding::Named(NamedKey::Numpad7)
        );
    }

    /// `e.key` for the space bar is a one-character string, not the word.
    #[test]
    fn a_literal_space_becomes_the_named_key() {
        assert_eq!(KeyBinding::parse(" "), KeyBinding::Named(NamedKey::Space));
        assert_eq!(KeyBinding::from(' '), KeyBinding::Named(NamedKey::Space));
        assert_eq!(KeyBinding::parse(" ").to_string(), "Space");
    }

    #[test]
    fn aliases_normalise_to_the_canonical_spelling() {
        assert_eq!(KeyBinding::parse("esc").to_string(), "Escape");
        assert_eq!(KeyBinding::parse("pgup").to_string(), "PageUp");
        assert_eq!(KeyBinding::parse("xbutton1").to_string(), "Mouse4");
        assert_eq!(KeyBinding::parse("num3").to_string(), "Numpad3");
    }

    #[test]
    fn an_unrecognised_binding_is_kept_verbatim_rather_than_failing() {
        let binding = KeyBinding::parse("Ctrl+Shift+Q");

        assert!(binding.is_unknown());
        assert_eq!(binding.to_string(), "Ctrl+Shift+Q");
        assert!(binding.pressable().is_none());
        assert!(!binding.is_interceptable());
    }

    #[test]
    fn round_trips_through_toml_without_changing_meaning() {
        for raw in ["z", "Space", "F5", "Mouse4", "Numpad0", "-", "Ctrl+Q"] {
            let binding = KeyBinding::parse(raw);
            let encoded = toml::to_string(&toml::value::Table::from_iter([(
                "key".to_string(),
                toml::Value::try_from(&binding).unwrap(),
            )]))
            .unwrap();
            let decoded: toml::Value = toml::from_str(&encoded).unwrap();
            let back: KeyBinding = decoded["key"].clone().try_into().unwrap();

            assert_eq!(back, binding, "round trip changed {raw}");
        }
    }

    /// The whole point of issue #20: one bad key must not discard the file.
    #[test]
    fn a_bad_binding_does_not_fail_deserialization() {
        #[derive(Deserialize)]
        struct Holder {
            slot0: KeyBinding,
            port: u16,
        }

        let holder: Holder = toml::from_str("slot0 = \"Mouse9\"\nport = 3000\n").unwrap();

        assert!(holder.slot0.is_unknown());
        assert_eq!(holder.port, 3000, "the rest of the document survived");
    }

    #[test]
    fn pressable_covers_every_named_key_and_mouse_button() {
        for (named, name, _, _) in NAMED_KEYS {
            assert!(
                KeyBinding::Named(*named).pressable().is_some(),
                "{name} has no enigo mapping"
            );
        }
        for (button, name, _, _) in MOUSE_BUTTONS {
            assert!(
                KeyBinding::Mouse(*button).pressable().is_some(),
                "{name} has no enigo mapping"
            );
        }
    }

    #[test]
    fn interception_covers_every_named_key_and_mouse_button() {
        for (named, name, _, _) in NAMED_KEYS {
            assert!(
                KeyBinding::Named(*named).is_interceptable(),
                "{name} has no rdev mapping"
            );
        }
        for (button, name, _, _) in MOUSE_BUTTONS {
            assert!(
                KeyBinding::Mouse(*button).is_interceptable(),
                "{name} has no rdev mapping"
            );
        }
    }

    #[test]
    fn mouse4_and_mouse5_map_to_the_windows_x_buttons() {
        assert_eq!(
            KeyBinding::parse("Mouse4").rdev_button(),
            Some(rdev::Button::Unknown(1))
        );
        assert_eq!(
            KeyBinding::parse("Mouse5").rdev_button(),
            Some(rdev::Button::Unknown(2))
        );
        assert_eq!(
            KeyBinding::parse("Mouse3").rdev_button(),
            Some(rdev::Button::Middle)
        );
    }

    #[test]
    fn matches_and_rebuilds_from_grab_events() {
        let mouse4 = KeyBinding::parse("Mouse4");
        let event = rdev::EventType::ButtonPress(rdev::Button::Unknown(1));

        assert!(mouse4.matches_event(&event));
        assert_eq!(KeyBinding::from_event(&event), Some(mouse4));

        let space = KeyBinding::parse("Space");
        let event = rdev::EventType::KeyPress(rdev::Key::Space);

        assert!(space.matches_event(&event));
        assert_eq!(KeyBinding::from_event(&event), Some(space));

        // Left and right stay unbindable — Dota needs them.
        assert_eq!(
            KeyBinding::from_event(&rdev::EventType::ButtonPress(rdev::Button::Left)),
            None
        );
    }

    #[test]
    fn a_char_binding_is_matched_case_insensitively_by_the_grab_layer() {
        // The UI used to uppercase what it captured; configs are lowercase.
        assert!(KeyBinding::parse("Z").matches_event(&rdev::EventType::KeyPress(rdev::Key::KeyZ)));
    }
}
