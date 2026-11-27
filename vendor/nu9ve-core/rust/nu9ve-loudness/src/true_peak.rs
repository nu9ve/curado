//! 4x true-peak detection, as ITU-R BS.1770-4 annex 2 describes it: the sample itself and three
//! positions interpolated between it and the next one with a 12-tap Hann-windowed sinc.
//!
//! [`TruePeakDetector::push`] does a fixed amount of work on fixed arrays: no allocation, no lock,
//! no branch on the signal. It can run inside a real-time callback, one detector per channel.

/// Taps of each interpolation kernel.
pub const TAPS: usize = 12;
/// Interpolated positions between two samples (1/4, 2/4 and 3/4); the sample is read directly.
pub const PHASES: usize = 3;
/// A sample is judged this many pushes after it entered: [`TruePeakDetector::push`] returns the
/// peak around the sample pushed `DELAY` calls earlier.
pub const DELAY: usize = TAPS / 2;

/// One channel's detector.
#[derive(Clone, Debug)]
pub struct TruePeakDetector {
    /// The last `TAPS` samples, written twice so that a window is always one contiguous slice.
    history: [f32; TAPS * 2],
    head: usize,
    kernels: [[f32; TAPS]; PHASES],
}

impl Default for TruePeakDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl TruePeakDetector {
    pub fn new() -> Self {
        Self {
            history: [0.0; TAPS * 2],
            head: 0,
            kernels: std::array::from_fn(|phase| interpolation_kernel(phase + 1)),
        }
    }

    /// Back to silence, as if just built.
    pub fn reset(&mut self) {
        self.history = [0.0; TAPS * 2];
        self.head = 0;
    }

    /// Takes one sample and returns the absolute peak, linear, of the sample pushed [`DELAY`]
    /// calls earlier and of the three positions between it and the one after. A sample that is
    /// not finite reads as silence.
    #[inline]
    pub fn push(&mut self, sample: f32) -> f32 {
        let sample = if sample.is_finite() { sample } else { 0.0 };
        let head = self.head;
        self.history[head] = sample;
        self.history[head + TAPS] = sample;
        self.head = (head + 1) % TAPS;
        let window = &self.history[self.head..self.head + TAPS];
        let mut peak = window[DELAY - 1].abs();
        for kernel in &self.kernels {
            let value: f32 = window.iter().zip(kernel).map(|(a, b)| a * b).sum();
            peak = peak.max(value.abs());
        }
        peak
    }
}

/// Hann-windowed sinc reading `phase / 4` of the way between the two central taps, normalised to
/// unity gain at DC.
fn interpolation_kernel(phase: usize) -> [f32; TAPS] {
    use std::f64::consts::PI;
    let offset = phase as f64 / 4.0;
    let raw: [f64; TAPS] = std::array::from_fn(|tap| {
        let x = tap as f64 - (DELAY - 1) as f64 - offset;
        let window = (PI * x / TAPS as f64).cos().powi(2);
        (PI * x).sin() / (PI * x) * window
    });
    let sum: f64 = raw.iter().sum();
    raw.map(|value| (value / sum) as f32)
}

/// The true peak of whole planar channels, linear: the maximum over every channel, with the
/// signal assumed silent before and after.
pub fn true_peak<C: AsRef<[f32]>>(channels: &[C]) -> f32 {
    let mut peak = 0.0f32;
    let mut detector = TruePeakDetector::new();
    for channel in channels {
        detector.reset();
        for &sample in channel.as_ref() {
            peak = peak.max(detector.push(sample));
        }
        for _ in 0..DELAY {
            peak = peak.max(detector.push(0.0));
        }
    }
    peak
}

/// A linear peak in dBTP; minus infinity for silence.
pub fn to_dbtp(peak: f32) -> f64 {
    20.0 * (peak as f64).log10()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::{FRAC_PI_4, TAU};

    #[test]
    fn finds_a_peak_between_samples() {
        // A sine at a quarter of the rate sampled at 45°: the samples read 0.707, the wave reaches 1.
        let signal: Vec<f32> = (0..4800)
            .map(|i| (i as f32 * TAU / 4.0 + FRAC_PI_4).sin())
            .collect();
        assert!(signal.iter().all(|s| s.abs() < 0.71));
        let peak = true_peak(&[&signal]);
        assert!((0.98..1.02).contains(&peak), "{peak}");
    }

    #[test]
    fn judges_each_sample_after_the_delay() {
        let mut detector = TruePeakDetector::new();
        detector.push(0.5);
        for _ in 1..DELAY {
            detector.push(0.0);
        }
        // DELAY pushes after the impulse, it is the sample being judged.
        assert_eq!(detector.push(0.0), 0.5);
        // Away from the edges a constant reads as itself: each kernel has unity gain at DC.
        let mut detector = TruePeakDetector::new();
        let mut peak = 0.0;
        for _ in 0..2 * TAPS {
            peak = detector.push(0.25);
        }
        assert!((peak - 0.25).abs() < 1e-6, "{peak}");
    }

    #[test]
    fn the_last_samples_are_judged_and_bad_values_ignored() {
        assert_eq!(true_peak(&[&[0.0f32, 0.0, -0.75][..]]), 0.75);
        assert_eq!(
            true_peak(&[&[f32::NAN, f32::INFINITY][..], &[0.5][..]]),
            0.5
        );
        assert_eq!(true_peak::<&[f32]>(&[]), 0.0);
        assert_eq!(to_dbtp(1.0), 0.0);
        assert_eq!(to_dbtp(0.0), f64::NEG_INFINITY);
    }
}
