//! Procedural synthesis of short alert cues.
//!
//! Alert cues are generated rather than shipped as audio files. That keeps the
//! binary free of assets, and — more usefully — makes every property of a cue
//! (pitch, pulse count, timbre, length) a config value that can be retuned
//! without regenerating anything.
//!
//! # Why cues are shaped this way
//!
//! Under fight-time load a cue has to be identified pre-attentively — recognised
//! without being thought about. Pitch alone is a weak discriminator, so three
//! orthogonal channels carry meaning:
//!
//! - **Rhythm carries identity.** A countable pulse count is the strongest
//!   discriminator and survives a noisy mix.
//! - **Pitch contour carries direction.** Rising = becoming available,
//!   falling = expiring.
//! - **Timbre carries category.** Economy events are wooden, objectives are
//!   brass, runes are bright bell.
//!
//! Cues are kept short (≤500ms) so they never mask a spell sound.
//!
//! This module is pure DSP over `f32` samples and has no audio-device
//! dependency, so all of it is unit-testable.

/// Output sample rate. Matches the rodio source we feed.
pub const SAMPLE_RATE: u32 = 44_100;

/// Attack/release ramp length.
///
/// Without this the waveform starts and ends at a non-zero sample, which is an
/// instantaneous discontinuity — audible as a click on every cue.
const RAMP_MS: f32 = 3.0;

/// Harmonic character of a cue. See the module docs for what each conveys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Timbre {
    /// Pure fundamental. Soft and unobtrusive.
    Sine,
    /// Inharmonic upper partials, fast decay. Bright and cutting — runes.
    Bell,
    /// Strong fundamental, very fast decay. Dry and percussive — economy events.
    Wood,
    /// Odd harmonics, slower attack. Heavy — map objectives.
    Brass,
}

impl Timbre {
    /// Relative amplitude of each partial, as multiples of the fundamental.
    fn partials(self) -> &'static [(f32, f32)] {
        match self {
            Timbre::Sine => &[(1.0, 1.0)],
            // Inharmonic ratios are what make a bell read as metallic.
            Timbre::Bell => &[(1.0, 1.0), (2.76, 0.34), (5.4, 0.16)],
            Timbre::Wood => &[(1.0, 1.0), (2.0, 0.22), (3.0, 0.10)],
            Timbre::Brass => &[(1.0, 1.0), (2.0, 0.5), (3.0, 0.33), (4.0, 0.15)],
        }
    }

    /// Exponential decay rate. Higher decays faster.
    fn decay(self) -> f32 {
        match self {
            Timbre::Sine => 3.0,
            Timbre::Bell => 4.5,
            Timbre::Wood => 12.0,
            Timbre::Brass => 2.0,
        }
    }
}

/// One pulse of a cue.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tone {
    pub frequency_hz: f32,
    pub duration_ms: u32,
}

impl Tone {
    pub fn new(frequency_hz: f32, duration_ms: u32) -> Self {
        Self {
            frequency_hz,
            duration_ms,
        }
    }
}

/// A complete alert cue.
#[derive(Debug, Clone, PartialEq)]
pub struct Motif {
    pub tones: Vec<Tone>,
    /// Silence between consecutive tones. This is what makes the pulse count
    /// countable rather than heard as one blurred sound.
    pub gap_ms: u32,
    pub timbre: Timbre,
}

impl Motif {
    pub fn new(tones: Vec<Tone>, gap_ms: u32, timbre: Timbre) -> Self {
        Self {
            tones,
            gap_ms,
            timbre,
        }
    }

    /// Total cue length in milliseconds.
    pub fn duration_ms(&self) -> u32 {
        if self.tones.is_empty() {
            return 0;
        }
        let tone_total: u32 = self.tones.iter().map(|t| t.duration_ms).sum();
        let gap_total = self.gap_ms * (self.tones.len() as u32 - 1);
        tone_total + gap_total
    }

    /// Render to mono PCM in `[-1.0, 1.0]`.
    ///
    /// `volume` is clamped to `[0.0, 1.0]`. The result is peak-normalised before
    /// volume is applied, so cues with different harmonic content sit at a
    /// consistent loudness and no single event startles.
    pub fn render(&self, volume: f32) -> Vec<f32> {
        let volume = volume.clamp(0.0, 1.0);
        if self.tones.is_empty() || volume == 0.0 {
            return Vec::new();
        }

        let mut samples: Vec<f32> = Vec::new();
        let gap_samples = ms_to_samples(self.gap_ms);

        for (index, tone) in self.tones.iter().enumerate() {
            if index > 0 {
                samples.extend(std::iter::repeat_n(0.0, gap_samples));
            }
            samples.extend(self.render_tone(tone));
        }

        normalise(&mut samples);
        for sample in samples.iter_mut() {
            *sample *= volume;
        }

        samples
    }

    fn render_tone(&self, tone: &Tone) -> Vec<f32> {
        let total = ms_to_samples(tone.duration_ms);
        if total == 0 {
            return Vec::new();
        }

        let partials = self.timbre.partials();
        let decay = self.timbre.decay();
        let ramp = ms_to_samples(RAMP_MS as u32).max(1).min(total / 2);

        (0..total)
            .map(|index| {
                let t = index as f32 / SAMPLE_RATE as f32;
                let progress = index as f32 / total as f32;

                let mut value = 0.0;
                for &(ratio, amplitude) in partials {
                    value += amplitude
                        * (std::f32::consts::TAU * tone.frequency_hz * ratio * t).sin();
                }

                // Exponential decay gives a struck-instrument shape; the linear
                // ramps at both ends remove the click.
                value *= (-decay * progress).exp();
                value *= edge_ramp(index, total, ramp);
                value
            })
            .collect()
    }
}

fn ms_to_samples(ms: u32) -> usize {
    (SAMPLE_RATE as u64 * ms as u64 / 1000) as usize
}

/// Fade in over the first `ramp` samples and out over the last `ramp`.
fn edge_ramp(index: usize, total: usize, ramp: usize) -> f32 {
    if ramp == 0 {
        return 1.0;
    }
    if index < ramp {
        return index as f32 / ramp as f32;
    }
    let from_end = total.saturating_sub(index + 1);
    if from_end < ramp {
        return from_end as f32 / ramp as f32;
    }
    1.0
}

/// Scale to unit peak so every cue lands at a comparable loudness.
fn normalise(samples: &mut [f32]) {
    let peak = samples.iter().fold(0.0_f32, |max, s| max.max(s.abs()));
    if peak > f32::EPSILON {
        let scale = 1.0 / peak;
        for sample in samples.iter_mut() {
            *sample *= scale;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_blip() -> Motif {
        Motif::new(
            vec![Tone::new(784.0, 90), Tone::new(1046.0, 110)],
            60,
            Timbre::Bell,
        )
    }

    #[test]
    fn duration_sums_tones_and_the_gaps_between_them() {
        // 90 + 60 + 110
        assert_eq!(two_blip().duration_ms(), 260);
    }

    #[test]
    fn a_single_tone_motif_has_no_gaps() {
        let motif = Motif::new(vec![Tone::new(880.0, 120)], 60, Timbre::Wood);
        assert_eq!(motif.duration_ms(), 120);
    }

    #[test]
    fn an_empty_motif_renders_nothing() {
        let motif = Motif::new(Vec::new(), 60, Timbre::Bell);
        assert_eq!(motif.duration_ms(), 0);
        assert!(motif.render(1.0).is_empty());
    }

    #[test]
    fn render_length_matches_the_declared_duration() {
        let motif = two_blip();
        let samples = motif.render(1.0);
        let expected = ms_to_samples(motif.duration_ms());

        // Allow a sample or two of rounding across the three segments.
        assert!(
            (samples.len() as i64 - expected as i64).abs() <= 3,
            "expected ~{expected} samples, got {}",
            samples.len()
        );
    }

    #[test]
    fn output_never_clips() {
        for timbre in [Timbre::Sine, Timbre::Bell, Timbre::Wood, Timbre::Brass] {
            let motif = Motif::new(vec![Tone::new(440.0, 100)], 0, timbre);
            for sample in motif.render(1.0) {
                assert!(
                    (-1.0..=1.0).contains(&sample),
                    "{timbre:?} produced an out-of-range sample: {sample}"
                );
            }
        }
    }

    #[test]
    fn volume_scales_the_peak_and_zero_is_silent() {
        let motif = two_blip();

        let peak_at = |volume: f32| {
            motif
                .render(volume)
                .iter()
                .fold(0.0_f32, |max, s| max.max(s.abs()))
        };

        assert!((peak_at(1.0) - 1.0).abs() < 0.001);
        assert!((peak_at(0.5) - 0.5).abs() < 0.001);
        assert!(motif.render(0.0).is_empty());
    }

    #[test]
    fn volume_is_clamped_rather_than_allowed_to_clip() {
        let peak = two_blip()
            .render(4.0)
            .iter()
            .fold(0.0_f32, |max, s| max.max(s.abs()));

        assert!(peak <= 1.0 + f32::EPSILON, "clamping failed, peak {peak}");
    }

    #[test]
    fn cues_start_and_end_near_silence_so_they_do_not_click() {
        let samples = two_blip().render(1.0);

        assert!(samples[0].abs() < 0.01, "click at start: {}", samples[0]);
        let last = samples[samples.len() - 1];
        assert!(last.abs() < 0.01, "click at end: {last}");
    }

    #[test]
    fn the_gap_between_pulses_is_actually_silent() {
        let motif = two_blip();
        let samples = motif.render(1.0);

        // Sample the middle of the gap that follows the first 90ms tone.
        let gap_middle = ms_to_samples(90 + 30);
        assert_eq!(samples[gap_middle], 0.0);
    }

    #[test]
    fn timbres_are_actually_distinguishable() {
        let render = |timbre| Motif::new(vec![Tone::new(440.0, 120)], 0, timbre).render(1.0);

        let sine = render(Timbre::Sine);
        let bell = render(Timbre::Bell);
        let brass = render(Timbre::Brass);

        assert_ne!(sine, bell);
        assert_ne!(bell, brass);
    }

    #[test]
    fn faster_decaying_timbres_have_less_energy_late_in_the_tone() {
        let energy_tail = |timbre| -> f32 {
            let samples = Motif::new(vec![Tone::new(440.0, 200)], 0, timbre).render(1.0);
            let tail = &samples[samples.len() * 3 / 4..];
            tail.iter().map(|s| s.abs()).sum::<f32>() / tail.len() as f32
        };

        // Wood decays hard and fast; brass sustains.
        assert!(
            energy_tail(Timbre::Wood) < energy_tail(Timbre::Brass),
            "wood should die away faster than brass"
        );
    }
}
