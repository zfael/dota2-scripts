//! Audio output for alert cues.
//!
//! Playback lives in Rust rather than the WebView on purpose. The Tauri window's
//! normal state while you are playing Dota is minimised or occluded, and in that
//! state the OS and the WebView runtime may throttle timers and suspend
//! `AudioContext`. An alert engine that is unreliable exactly when it is needed
//! is worse than no alert engine.

use crate::audio::motif::{Motif, SAMPLE_RATE};
use rodio::{OutputStream, OutputStreamHandle, Sink};
use std::sync::{Mutex, OnceLock};
use tracing::warn;

/// Holds the output stream open for the process lifetime.
///
/// `OutputStream` closes the device when dropped, so it must outlive every sink;
/// dropping it per-cue would also make each alert pay device-open latency.
struct AudioOutput {
    _stream: OutputStream,
    handle: OutputStreamHandle,
}

// The stream handle is only ever touched under the mutex below.
unsafe impl Send for AudioOutput {}

fn output() -> Option<&'static Mutex<AudioOutput>> {
    static OUTPUT: OnceLock<Option<Mutex<AudioOutput>>> = OnceLock::new();

    OUTPUT
        .get_or_init(|| match OutputStream::try_default() {
            Ok((stream, handle)) => Some(Mutex::new(AudioOutput {
                _stream: stream,
                handle,
            })),
            Err(e) => {
                // A machine with no audio device is not a reason to fail; the
                // rest of the app must keep working.
                warn!("audio: no output device available, alerts will be silent: {e}");
                None
            }
        })
        .as_ref()
}

/// Play raw mono PCM.
///
/// Returns `false` if there is no usable audio device. Playback is detached: the
/// call returns immediately rather than blocking for the cue's duration.
pub fn play_samples(samples: Vec<f32>) -> bool {
    if samples.is_empty() {
        return false;
    }

    let Some(output) = output() else {
        return false;
    };

    let handle = match output.lock() {
        Ok(output) => output.handle.clone(),
        Err(e) => {
            warn!("audio: output lock poisoned: {e}");
            return false;
        }
    };

    let sink = match Sink::try_new(&handle) {
        Ok(sink) => sink,
        Err(e) => {
            warn!("audio: could not create sink: {e}");
            return false;
        }
    };

    let source = rodio::buffer::SamplesBuffer::new(1, SAMPLE_RATE, samples);
    sink.append(source);
    // Hand ownership to rodio so the cue finishes after this returns.
    sink.detach();

    true
}

/// Render and play a motif.
pub fn play_motif(motif: &Motif, volume: f32) -> bool {
    play_samples(motif.render(volume))
}

/// Play a user-supplied `.wav` / `.mp3`.
///
/// Returns `false` on any failure so the caller can fall back to the built-in
/// cue — a mistyped path should not silently disable an alert.
pub fn play_file(path: &str, volume: f32) -> bool {
    use std::fs::File;
    use std::io::BufReader;

    if path.is_empty() {
        return false;
    }

    let Some(output) = output() else {
        return false;
    };

    let file = match File::open(path) {
        Ok(file) => file,
        Err(e) => {
            warn!("audio: could not open '{path}': {e}");
            return false;
        }
    };

    let source = match rodio::Decoder::new(BufReader::new(file)) {
        Ok(source) => source,
        Err(e) => {
            warn!("audio: could not decode '{path}': {e}");
            return false;
        }
    };

    let handle = match output.lock() {
        Ok(output) => output.handle.clone(),
        Err(e) => {
            warn!("audio: output lock poisoned: {e}");
            return false;
        }
    };

    let sink = match Sink::try_new(&handle) {
        Ok(sink) => sink,
        Err(e) => {
            warn!("audio: could not create sink: {e}");
            return false;
        }
    };

    sink.set_volume(volume.clamp(0.0, 1.0));
    sink.append(source);
    sink.detach();

    true
}
