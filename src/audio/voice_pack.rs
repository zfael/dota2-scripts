//! Voice pack discovery and per-event sound resolution.
//!
//! A voice pack is a directory of audio files named after alert event keys —
//! `power_rune.wav`, `wisdom_rune.wav`, and so on. Selecting a pack points every
//! event at spoken callouts in one action, instead of setting seven paths by hand.
//!
//! Packs are **not committed**: they are generated locally or supplied by the
//! user. `scripts/generate-voice-pack.ps1` produces one using the Windows speech
//! synthesiser; better-sounding packs can be generated with a hosted TTS service
//! and dropped in the same way. See `docs/features/objective-alerts.md`.

use std::path::{Path, PathBuf};

/// Directory holding voice packs, relative to the working directory.
pub const VOICE_PACK_DIR: &str = "assets/voice";

/// Extensions a pack file may use, in preference order.
const EXTENSIONS: [&str; 2] = ["wav", "mp3"];

/// Resolve which audio file should play for an event, if any.
///
/// Precedence, most specific first:
/// 1. An explicit per-event `sound_file` — always wins, so a single override is
///    never silently replaced by a pack selection.
/// 2. A file for this event in the selected voice pack.
/// 3. `None`, meaning the caller should use the built-in synthesised cue.
///
/// Only paths that actually exist are returned, so a pack missing one event
/// falls back to that event's generated cue rather than going silent.
pub fn resolve_sound_path(
    event_key: &str,
    sound_file: &str,
    voice_pack: &str,
    packs_root: &Path,
) -> Option<PathBuf> {
    if !sound_file.is_empty() {
        return Some(PathBuf::from(sound_file));
    }

    if voice_pack.is_empty() {
        return None;
    }

    let pack_dir = packs_root.join(voice_pack);
    EXTENSIONS
        .iter()
        .map(|extension| pack_dir.join(format!("{event_key}.{extension}")))
        .find(|candidate| candidate.is_file())
}

/// Names of the packs available under `packs_root`.
///
/// Returns an empty list rather than an error when the directory is absent —
/// having no packs is the normal state, not a failure.
pub fn list_packs(packs_root: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(packs_root) else {
        return Vec::new();
    };

    let mut packs: Vec<String> = entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();

    packs.sort();
    packs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn pack_with(root: &Path, pack: &str, files: &[&str]) {
        let dir = root.join(pack);
        fs::create_dir_all(&dir).unwrap();
        for file in files {
            fs::write(dir.join(file), b"not really audio").unwrap();
        }
    }

    #[test]
    fn no_pack_and_no_override_uses_the_generated_cue() {
        let temp = TempDir::new().unwrap();
        assert_eq!(
            resolve_sound_path("power_rune", "", "", temp.path()),
            None
        );
    }

    #[test]
    fn an_explicit_sound_file_wins_over_a_selected_pack() {
        let temp = TempDir::new().unwrap();
        pack_with(temp.path(), "voice", &["power_rune.wav"]);

        let resolved =
            resolve_sound_path("power_rune", "C:/custom/horn.wav", "voice", temp.path());

        assert_eq!(resolved, Some(PathBuf::from("C:/custom/horn.wav")));
    }

    #[test]
    fn a_pack_file_is_found_by_event_key() {
        let temp = TempDir::new().unwrap();
        pack_with(temp.path(), "voice", &["wisdom_rune.wav"]);

        let resolved = resolve_sound_path("wisdom_rune", "", "voice", temp.path());

        assert_eq!(resolved, Some(temp.path().join("voice/wisdom_rune.wav")));
    }

    #[test]
    fn wav_is_preferred_over_mp3() {
        let temp = TempDir::new().unwrap();
        pack_with(temp.path(), "voice", &["stack.wav", "stack.mp3"]);

        let resolved = resolve_sound_path("stack", "", "voice", temp.path());

        assert_eq!(resolved, Some(temp.path().join("voice/stack.wav")));
    }

    #[test]
    fn mp3_is_used_when_there_is_no_wav() {
        let temp = TempDir::new().unwrap();
        pack_with(temp.path(), "voice", &["tormentor.mp3"]);

        let resolved = resolve_sound_path("tormentor", "", "voice", temp.path());

        assert_eq!(resolved, Some(temp.path().join("voice/tormentor.mp3")));
    }

    #[test]
    fn an_event_missing_from_a_pack_falls_back_to_its_generated_cue() {
        let temp = TempDir::new().unwrap();
        // Pack covers power runes but not bounty runes.
        pack_with(temp.path(), "voice", &["power_rune.wav"]);

        assert!(resolve_sound_path("power_rune", "", "voice", temp.path()).is_some());
        assert_eq!(
            resolve_sound_path("bounty_rune", "", "voice", temp.path()),
            None
        );
    }

    #[test]
    fn a_selected_pack_that_does_not_exist_falls_back_rather_than_failing() {
        let temp = TempDir::new().unwrap();
        assert_eq!(
            resolve_sound_path("power_rune", "", "missing-pack", temp.path()),
            None
        );
    }

    #[test]
    fn packs_are_listed_alphabetically() {
        let temp = TempDir::new().unwrap();
        pack_with(temp.path(), "zulu", &[]);
        pack_with(temp.path(), "alpha", &[]);

        assert_eq!(list_packs(temp.path()), vec!["alpha", "zulu"]);
    }

    #[test]
    fn loose_files_are_not_mistaken_for_packs() {
        let temp = TempDir::new().unwrap();
        pack_with(temp.path(), "voice", &[]);
        fs::write(temp.path().join("readme.txt"), b"hello").unwrap();

        assert_eq!(list_packs(temp.path()), vec!["voice"]);
    }

    #[test]
    fn a_missing_pack_directory_lists_nothing_rather_than_erroring() {
        let temp = TempDir::new().unwrap();
        assert!(list_packs(&temp.path().join("nope")).is_empty());
    }
}
