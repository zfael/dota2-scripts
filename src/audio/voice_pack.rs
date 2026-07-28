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

use crate::config::storage::ConfigPaths;
use std::path::{Path, PathBuf};

/// Voice pack directory relative to the working directory.
///
/// Convenient when running from a checkout, but it does **not** survive
/// launching the exe directly — the working directory then becomes the exe's own
/// folder. It is searched in addition to, not instead of, the LocalAppData
/// location.
pub const RELATIVE_VOICE_PACK_DIR: &str = "assets/voice";

/// Extensions a pack file may use, in preference order.
const EXTENSIONS: [&str; 2] = ["wav", "mp3"];

/// Directories searched for voice packs, in precedence order.
///
/// `%LOCALAPPDATA%\dota2-scripts\assets\voice` first, because it resolves however
/// the app was started, then the working-directory-relative path for checkouts.
pub fn pack_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Ok(paths) = ConfigPaths::detect() {
        roots.push(paths.voice_pack_dir());
    }
    roots.push(PathBuf::from(RELATIVE_VOICE_PACK_DIR));

    roots
}

/// Resolve which audio file should play for an event, if any.
///
/// Precedence, most specific first:
/// 1. An explicit per-event `sound_file` — always wins, so a single override is
///    never silently replaced by a pack selection.
/// 2. A file for this event in the selected pack, searching `roots` in order.
/// 3. `None`, meaning the caller should use the built-in synthesised cue.
///
/// Only paths that actually exist are returned, so a pack missing one event
/// falls back to that event's generated cue rather than going silent. A pack
/// present in several roots may therefore be satisfied per-event from whichever
/// root has that file.
pub fn resolve_sound_path(
    event_key: &str,
    sound_file: &str,
    voice_pack: &str,
    roots: &[PathBuf],
) -> Option<PathBuf> {
    if !sound_file.is_empty() {
        return Some(PathBuf::from(sound_file));
    }

    if voice_pack.is_empty() {
        return None;
    }

    roots.iter().find_map(|root| {
        let pack_dir = root.join(voice_pack);
        EXTENSIONS
            .iter()
            .map(|extension| pack_dir.join(format!("{event_key}.{extension}")))
            .find(|candidate| candidate.is_file())
    })
}

/// Pack names found across every root, merged and de-duplicated.
///
/// A pack of the same name in two roots is listed once; selecting it resolves
/// per-event through [`resolve_sound_path`]. Missing directories are skipped —
/// having no packs is the normal state, not a failure.
pub fn list_packs(roots: &[PathBuf]) -> Vec<String> {
    let mut packs: Vec<String> = roots
        .iter()
        .flat_map(|root| list_packs_in(root))
        .collect();

    packs.sort();
    packs.dedup();
    packs
}

fn list_packs_in(root: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };

    entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect()
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

    fn roots(dirs: &[&TempDir]) -> Vec<PathBuf> {
        dirs.iter().map(|d| d.path().to_path_buf()).collect()
    }

    #[test]
    fn no_pack_and_no_override_uses_the_generated_cue() {
        let temp = TempDir::new().unwrap();
        assert_eq!(
            resolve_sound_path("power_rune", "", "", &roots(&[&temp])),
            None
        );
    }

    #[test]
    fn an_explicit_sound_file_wins_over_a_selected_pack() {
        let temp = TempDir::new().unwrap();
        pack_with(temp.path(), "voice", &["power_rune.wav"]);

        let resolved = resolve_sound_path(
            "power_rune",
            "C:/custom/horn.wav",
            "voice",
            &roots(&[&temp]),
        );

        assert_eq!(resolved, Some(PathBuf::from("C:/custom/horn.wav")));
    }

    #[test]
    fn a_pack_file_is_found_by_event_key() {
        let temp = TempDir::new().unwrap();
        pack_with(temp.path(), "voice", &["wisdom_rune.wav"]);

        let resolved = resolve_sound_path("wisdom_rune", "", "voice", &roots(&[&temp]));

        assert_eq!(resolved, Some(temp.path().join("voice/wisdom_rune.wav")));
    }

    #[test]
    fn wav_is_preferred_over_mp3() {
        let temp = TempDir::new().unwrap();
        pack_with(temp.path(), "voice", &["stack.wav", "stack.mp3"]);

        let resolved = resolve_sound_path("stack", "", "voice", &roots(&[&temp]));

        assert_eq!(resolved, Some(temp.path().join("voice/stack.wav")));
    }

    #[test]
    fn mp3_is_used_when_there_is_no_wav() {
        let temp = TempDir::new().unwrap();
        pack_with(temp.path(), "voice", &["tormentor.mp3"]);

        let resolved = resolve_sound_path("tormentor", "", "voice", &roots(&[&temp]));

        assert_eq!(resolved, Some(temp.path().join("voice/tormentor.mp3")));
    }

    #[test]
    fn an_event_missing_from_a_pack_falls_back_to_its_generated_cue() {
        let temp = TempDir::new().unwrap();
        // Pack covers power runes but not bounty runes.
        pack_with(temp.path(), "voice", &["power_rune.wav"]);

        assert!(resolve_sound_path("power_rune", "", "voice", &roots(&[&temp])).is_some());
        assert_eq!(
            resolve_sound_path("bounty_rune", "", "voice", &roots(&[&temp])),
            None
        );
    }

    #[test]
    fn a_selected_pack_that_does_not_exist_falls_back_rather_than_failing() {
        let temp = TempDir::new().unwrap();
        assert_eq!(
            resolve_sound_path("power_rune", "", "missing-pack", &roots(&[&temp])),
            None
        );
    }

    #[test]
    fn an_earlier_root_wins_when_both_have_the_file() {
        let primary = TempDir::new().unwrap();
        let fallback = TempDir::new().unwrap();
        pack_with(primary.path(), "voice", &["power_rune.wav"]);
        pack_with(fallback.path(), "voice", &["power_rune.wav"]);

        let resolved =
            resolve_sound_path("power_rune", "", "voice", &roots(&[&primary, &fallback]));

        assert_eq!(resolved, Some(primary.path().join("voice/power_rune.wav")));
    }

    #[test]
    fn a_later_root_covers_events_the_earlier_one_is_missing() {
        let primary = TempDir::new().unwrap();
        let fallback = TempDir::new().unwrap();
        pack_with(primary.path(), "voice", &["power_rune.wav"]);
        pack_with(fallback.path(), "voice", &["stack.wav"]);

        let all = roots(&[&primary, &fallback]);

        assert_eq!(
            resolve_sound_path("stack", "", "voice", &all),
            Some(fallback.path().join("voice/stack.wav"))
        );
    }

    #[test]
    fn a_pack_in_only_the_second_root_is_still_found() {
        let empty = TempDir::new().unwrap();
        let fallback = TempDir::new().unwrap();
        pack_with(fallback.path(), "voice", &["tormentor.wav"]);

        let resolved =
            resolve_sound_path("tormentor", "", "voice", &roots(&[&empty, &fallback]));

        assert_eq!(resolved, Some(fallback.path().join("voice/tormentor.wav")));
    }

    #[test]
    fn packs_are_listed_alphabetically() {
        let temp = TempDir::new().unwrap();
        pack_with(temp.path(), "zulu", &[]);
        pack_with(temp.path(), "alpha", &[]);

        assert_eq!(list_packs(&roots(&[&temp])), vec!["alpha", "zulu"]);
    }

    #[test]
    fn packs_from_every_root_are_merged() {
        let first = TempDir::new().unwrap();
        let second = TempDir::new().unwrap();
        pack_with(first.path(), "installed", &[]);
        pack_with(second.path(), "checkout", &[]);

        assert_eq!(
            list_packs(&roots(&[&first, &second])),
            vec!["checkout", "installed"]
        );
    }

    #[test]
    fn a_pack_present_in_both_roots_is_listed_once() {
        let first = TempDir::new().unwrap();
        let second = TempDir::new().unwrap();
        pack_with(first.path(), "voice", &[]);
        pack_with(second.path(), "voice", &[]);

        assert_eq!(list_packs(&roots(&[&first, &second])), vec!["voice"]);
    }

    #[test]
    fn loose_files_are_not_mistaken_for_packs() {
        let temp = TempDir::new().unwrap();
        pack_with(temp.path(), "voice", &[]);
        fs::write(temp.path().join("readme.txt"), b"hello").unwrap();

        assert_eq!(list_packs(&roots(&[&temp])), vec!["voice"]);
    }

    #[test]
    fn a_missing_pack_directory_lists_nothing_rather_than_erroring() {
        let temp = TempDir::new().unwrap();
        assert!(list_packs(&[temp.path().join("nope")]).is_empty());
    }

    #[test]
    fn a_missing_root_does_not_hide_packs_in_the_others() {
        let temp = TempDir::new().unwrap();
        pack_with(temp.path(), "voice", &[]);

        let mixed = vec![PathBuf::from("Z:/definitely/not/here"), temp.path().to_path_buf()];

        assert_eq!(list_packs(&mixed), vec!["voice"]);
    }

    #[test]
    fn default_roots_prefer_local_app_data_over_the_relative_path() {
        let roots = pack_roots();

        assert!(!roots.is_empty());
        assert_eq!(roots.last().unwrap(), Path::new(RELATIVE_VOICE_PACK_DIR));
    }
}
