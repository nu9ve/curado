//! Stereo-linked look-ahead limiter with 4x true-peak detection.
//! Buffers are sized at engine creation; `process` does bounded work and never allocates.

/// −1 dBTP, leaving room for converter reconstruction overs.
pub const CEILING: f32 = 0.891;
const LOOKAHEAD_SECONDS: f32 = 0.0015;
const RELEASE_SECONDS: f32 = 0.12;
const TAPS: usize = 12;
/// Interpolated positions between two samples; the samples themselves are read directly.
const PHASES: usize = 3;
/// A sample reaches the centre of the detector's window this many frames after entering it.
const DETECTOR_DELAY: usize = TAPS / 2;

pub struct Limiter {
    history: [[f32; TAPS * 2]; 2],
    history_head: usize,
    kernels: [[f32; TAPS]; PHASES],
    delay: Vec<[f32; 2]>,
    delay_head: usize,
    /// Monotonic queue holding the minimum required gain of the last `window` frames.
    minimum: Vec<(u64, f32)>,
    minimum_front: usize,
    minimum_len: usize,
    tick: u64,
    smoothing: Vec<f32>,
    smoothing_head: usize,
    smoothing_sum: f64,
    envelope: f32,
    release: f32,
}

impl Limiter {
    pub fn new(sample_rate: u32) -> Self {
        let sr = sample_rate as f32;
        let window = ((sr * LOOKAHEAD_SECONDS).ceil() as usize).max(1);
        Self {
            history: [[0.; TAPS * 2]; 2],
            history_head: 0,
            kernels: std::array::from_fn(|phase| interpolation_kernel(phase + 1)),
            delay: vec![[0.; 2]; window - 1 + DETECTOR_DELAY],
            delay_head: 0,
            minimum: vec![(0, 1.); window],
            minimum_front: 0,
            minimum_len: 0,
            tick: 0,
            smoothing: vec![1.; window],
            smoothing_head: 0,
            smoothing_sum: window as f64,
            envelope: 1.,
            release: 1. - (-1. / (sr * RELEASE_SECONDS)).exp(),
        }
    }
    /// Frames between a sample entering `process` and leaving it.
    pub fn latency(&self) -> usize {
        self.delay.len()
    }
    /// Returns the limited frame and the gain applied to it. Below the ceiling the gain is
    /// exactly 1, so the signal only sees the constant delay.
    pub fn process(&mut self, input: [f32; 2]) -> ([f32; 2], f32) {
        let input = input.map(|value| if value.is_finite() { value } else { 0. });
        let peak = self.true_peak(input);
        let required = if peak > CEILING { CEILING / peak } else { 1. };
        let minimum = self.sliding_minimum(required);
        // The tiny snap avoids f32 stalling just under unity, which would never bypass.
        self.envelope = if minimum <= self.envelope || minimum - self.envelope < 1e-3 {
            minimum
        } else {
            self.envelope + (minimum - self.envelope) * self.release
        };
        // The average of a window whose values all respect a peak's minimum also respects it:
        // the gain reaches its target smoothly and is never above it when that peak leaves.
        let window = self.smoothing.len();
        let oldest = std::mem::replace(&mut self.smoothing[self.smoothing_head], self.envelope);
        self.smoothing_sum += self.envelope as f64 - oldest as f64;
        self.smoothing_head = (self.smoothing_head + 1) % window;
        if self.smoothing_head == 0 {
            self.smoothing_sum = self.smoothing.iter().map(|value| *value as f64).sum();
        }
        let gain = ((self.smoothing_sum / window as f64) as f32).min(1.);
        let delayed = std::mem::replace(&mut self.delay[self.delay_head], input);
        self.delay_head = (self.delay_head + 1) % self.delay.len();
        ([delayed[0] * gain, delayed[1] * gain], gain)
    }
    fn true_peak(&mut self, input: [f32; 2]) -> f32 {
        let head = self.history_head;
        for (channel, value) in self.history.iter_mut().zip(input) {
            channel[head] = value;
            channel[head + TAPS] = value;
        }
        self.history_head = (head + 1) % TAPS;
        let start = self.history_head;
        let mut peak = 0_f32;
        for channel in &self.history {
            let window = &channel[start..start + TAPS];
            peak = peak.max(window[DETECTOR_DELAY - 1].abs());
            for kernel in &self.kernels {
                let value: f32 = window.iter().zip(kernel).map(|(a, b)| a * b).sum();
                peak = peak.max(value.abs());
            }
        }
        peak
    }
    fn sliding_minimum(&mut self, value: f32) -> f32 {
        let capacity = self.minimum.len();
        while self.minimum_len > 0 {
            let back = (self.minimum_front + self.minimum_len - 1) % capacity;
            if self.minimum[back].1 < value {
                break;
            }
            self.minimum_len -= 1;
        }
        if self.minimum_len == capacity {
            self.minimum_front = (self.minimum_front + 1) % capacity;
            self.minimum_len -= 1;
        }
        let back = (self.minimum_front + self.minimum_len) % capacity;
        self.minimum[back] = (self.tick, value);
        self.minimum_len += 1;
        if self.tick - self.minimum[self.minimum_front].0 >= capacity as u64 {
            self.minimum_front = (self.minimum_front + 1) % capacity;
            self.minimum_len -= 1;
        }
        self.tick += 1;
        self.minimum[self.minimum_front].1
    }
}

/// Hann-windowed sinc reading `phase / 4` of the way between the two central taps.
fn interpolation_kernel(phase: usize) -> [f32; TAPS] {
    use std::f64::consts::PI;
    let offset = phase as f64 / 4.;
    let raw: [f64; TAPS] = std::array::from_fn(|tap| {
        let x = tap as f64 - (DETECTOR_DELAY - 1) as f64 - offset;
        let window = (PI * x / TAPS as f64).cos().powi(2);
        (PI * x).sin() / (PI * x) * window
    });
    let sum: f64 = raw.iter().sum();
    raw.map(|value| (value / sum) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::{FRAC_PI_4, TAU};

    fn run(limiter: &mut Limiter, input: &[[f32; 2]]) -> Vec<[f32; 2]> {
        input
            .iter()
            .map(|frame| limiter.process(*frame).0)
            .collect()
    }
    /// Independent 16x sinc reconstruction, so the test does not trust the detector it checks.
    fn reconstructed_peak(signal: &[f32]) -> f32 {
        let mut peak = 0_f32;
        for center in 32..signal.len() - 32 {
            for phase in 0..16 {
                let position = center as f64 + phase as f64 / 16.;
                let value: f64 = (center - 31..center + 32)
                    .map(|i| {
                        let x = position - i as f64;
                        let window = (std::f64::consts::PI * x / 64.).cos().powi(2);
                        let sinc = if x.abs() < 1e-9 {
                            1.
                        } else {
                            (std::f64::consts::PI * x).sin() / (std::f64::consts::PI * x)
                        };
                        signal[i] as f64 * sinc * window
                    })
                    .sum();
                peak = peak.max(value.abs() as f32);
            }
        }
        peak
    }

    #[test]
    fn signal_below_the_ceiling_is_only_delayed() {
        for sample_rate in [44100, 48000, 96000] {
            let mut limiter = Limiter::new(sample_rate);
            let input: Vec<_> = (0..4800)
                .map(|i| {
                    let x = (i as f32 * 0.02).sin() * 0.5;
                    [x, -x * 0.7]
                })
                .collect();
            let output = run(&mut limiter, &input);
            let latency = limiter.latency();
            assert!(output[..latency].iter().all(|frame| *frame == [0.; 2]));
            assert_eq!(&output[latency..], &input[..input.len() - latency]);
        }
    }

    #[test]
    fn overs_stay_under_the_ceiling_at_every_sample_rate() {
        for sample_rate in [44100, 48000, 96000] {
            let mut limiter = Limiter::new(sample_rate);
            let hz = 55. / sample_rate as f32;
            let input: Vec<_> = (0..sample_rate as usize)
                .map(|i| {
                    let burst = if i % 12000 < 200 { 4. } else { 1.8 };
                    let x = (TAU * hz * i as f32).sin() * burst;
                    [x, x * 0.5]
                })
                .collect();
            let output = run(&mut limiter, &input);
            let peak = output.iter().fold(0_f32, |max, frame| {
                max.max(frame[0].abs()).max(frame[1].abs())
            });
            assert!(peak <= CEILING, "{sample_rate}: {peak}");
            assert!(peak > CEILING * 0.95, "{sample_rate}: {peak}");
        }
    }

    #[test]
    fn detects_a_peak_that_falls_between_samples() {
        // A sine at a quarter of the rate, sampled at 45°: samples read 0.707, the wave reaches 1.
        let input: Vec<_> = (0..48000)
            .map(|i| {
                let x = (i as f32 * TAU / 4. + FRAC_PI_4).sin();
                [x, x]
            })
            .collect();
        assert!(input.iter().all(|frame| frame[0].abs() < CEILING));
        let mut limiter = Limiter::new(48000);
        let output: Vec<f32> = run(&mut limiter, &input).iter().map(|f| f[0]).collect();
        let before =
            reconstructed_peak(&input[1000..1400].iter().map(|f| f[0]).collect::<Vec<_>>());
        let after = reconstructed_peak(&output[40000..40400]);
        assert!(before > 0.98, "{before}");
        assert!(after <= CEILING * 1.01, "{after}");
    }

    #[test]
    fn gain_is_linked_and_recovers_to_exact_unity() {
        let mut limiter = Limiter::new(48000);
        let mut min_gain = 1_f32;
        for i in 0..4800 {
            let x = (i as f32 * 0.03).sin() * 2.;
            let (frame, gain) = limiter.process([x, x * 0.25]);
            min_gain = min_gain.min(gain);
            if i > 200 && frame[0].abs() > 0.01 {
                assert!((frame[1] / frame[0] - 0.25).abs() < 1e-4);
            }
        }
        assert!(min_gain < 0.5);
        let mut gain = 0.;
        for _ in 0..48000 {
            gain = limiter.process([0.1, 0.1]).1;
        }
        assert_eq!(gain, 1.);
    }

    #[test]
    fn invalid_samples_never_reach_the_output_or_poison_the_gain() {
        let mut limiter = Limiter::new(48000);
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for _ in 0..200 {
                let (frame, gain) = limiter.process([value, 0.2]);
                assert!(frame.iter().all(|v| v.is_finite()) && gain.is_finite());
            }
        }
        for _ in 0..200 {
            assert_eq!(limiter.process([0.2, 0.2]).1, 1.);
        }
    }
}
