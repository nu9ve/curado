//! ITU-R BS.1770-4 loudness with the gating of EBU R 128.
//!
//! - K-weighting: a high shelf (stage 1) and the RLB high-pass (stage 2), each a biquad in f64.
//! - Integrated: 400 ms blocks every 100 ms (75 % overlap), above the absolute gate of −70 LUFS,
//!   then above the relative gate of −10 LU under their mean. Both gates are strict (`>`).
//! - Loudness range (EBU Tech 3342): 3 s blocks every 100 ms with gates of −70 LUFS and −20 LU
//!   (`>=`), then the 95th minus the 10th percentile, as its reference MATLAB code picks them.
//! - Maximum momentary (400 ms) and short-term (3 s) loudness: blocks every 10 ms, so that a
//!   burst lands whole in a block wherever it starts (EBU Tech 3341, cases 10 and 13).
//!
//! A trailing partial 100 ms segment or 10 ms step is dropped.
//!
//! The arithmetic runs in a fixed order so that a planar or an interleaved feed, whole or in
//! chunks of any size, gives the same bits.

use std::f64::consts::PI;
use std::fmt;

/// Lowest and highest sample rates accepted. Below 8 kHz the shelf would sit close to Nyquist.
pub const MIN_SAMPLE_RATE: u32 = 8_000;
pub const MAX_SAMPLE_RATE: u32 = 768_000;
/// BS.1770 weight of the left and right surround channels; every other channel weighs 1.
/// The LFE channel is left out of the measurement by the caller.
pub const SURROUND_WEIGHT: f64 = 1.41;

const ABSOLUTE_GATE: f64 = -70.0;
const RELATIVE_GATE: f64 = -10.0;
const RANGE_RELATIVE_GATE: f64 = -20.0;
const MOMENTARY_SEGMENTS: usize = 4;
const SHORT_TERM_SEGMENTS: usize = 30;
const MOMENTARY_STEPS: usize = 40;
const SHORT_TERM_STEPS: usize = 300;
/// Steps whose momentary and short-term blocks are summed together, so that their sums overlap.
const BATCH: usize = 8;
/// Steps the history holds before it drops the ones no block needs any more.
const HISTORY: usize = 2 * SHORT_TERM_STEPS;

/// A biquad with `a0 = 1`: `y = b0 x + b1 x[-1] + b2 x[-2] - a1 y[-1] - a2 y[-2]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Biquad {
    pub b: [f64; 3],
    /// `a1` and `a2`.
    pub a: [f64; 2],
}

/// The two stages of the K-weighting filter for one sample rate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KWeighting {
    pub shelf: Biquad,
    pub high_pass: Biquad,
}

impl KWeighting {
    /// The coefficients printed in ITU-R BS.1770-4 (tables 1 and 2) for 48 kHz.
    pub const BS1770_48K: KWeighting = KWeighting {
        shelf: Biquad {
            b: [1.53512485958697, -2.69169618940638, 1.19839281085285],
            a: [-1.69065929318241, 0.73248077421585],
        },
        high_pass: Biquad {
            b: [1.0, -2.0, 1.0],
            a: [-1.99004745483398, 0.99007225036621],
        },
    };

    /// The filter for `sample_rate`: the printed coefficients at 48 kHz, derived ones elsewhere.
    pub fn new(sample_rate: u32) -> Result<Self, UnsupportedInput> {
        if !(MIN_SAMPLE_RATE..=MAX_SAMPLE_RATE).contains(&sample_rate) {
            return Err(UnsupportedInput::SampleRate(sample_rate));
        }
        Ok(if sample_rate == 48_000 {
            Self::BS1770_48K
        } else {
            Self::derive(sample_rate as f64)
        })
    }

    /// Bilinear transform of the analogue prototype of both stages, with the constants that
    /// libebur128 fitted to the printed coefficients. At 48 kHz it lands within 1e-9 of them.
    pub fn derive(sample_rate: f64) -> Self {
        let f0 = 1681.974450955533;
        let gain_db = 3.999843853973347;
        let q = 0.7071752369554196;
        let k = (PI * f0 / sample_rate).tan();
        let vh = 10f64.powf(gain_db / 20.0);
        let vb = vh.powf(0.4996667741545416);
        let a0 = 1.0 + k / q + k * k;
        let shelf = Biquad {
            b: [
                (vh + vb * k / q + k * k) / a0,
                2.0 * (k * k - vh) / a0,
                (vh - vb * k / q + k * k) / a0,
            ],
            a: [2.0 * (k * k - 1.0) / a0, (1.0 - k / q + k * k) / a0],
        };
        let f0 = 38.13547087602444;
        let q = 0.5003270373238773;
        let k = (PI * f0 / sample_rate).tan();
        let a0 = 1.0 + k / q + k * k;
        let high_pass = Biquad {
            b: [1.0, -2.0, 1.0],
            a: [2.0 * (k * k - 1.0) / a0, (1.0 - k / q + k * k) / a0],
        };
        Self { shelf, high_pass }
    }
}

/// What a measurement found. `None` where there is not enough signal above the gates:
/// shorter than the block, or silent.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Report {
    pub integrated_lufs: Option<f64>,
    /// The loudest 400 ms block, on a 10 ms grid.
    pub momentary_max_lufs: Option<f64>,
    /// The loudest 3 s block, on a 10 ms grid.
    pub short_term_max_lufs: Option<f64>,
    /// EBU Tech 3342 loudness range, in LU. Needs at least 3 s.
    pub loudness_range_lu: Option<f64>,
}

/// Why a meter could not be built or fed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsupportedInput {
    /// Outside `MIN_SAMPLE_RATE..=MAX_SAMPLE_RATE`.
    SampleRate(u32),
    /// No channels, a weight that is negative or not finite, or a feed with another channel count.
    Channels { expected: usize, found: usize },
    /// Planar channels of different lengths, or interleaved samples that are not whole frames.
    Length { expected: usize, found: usize },
}

impl fmt::Display for UnsupportedInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SampleRate(rate) => write!(
                f,
                "sample rate {rate} Hz is outside {MIN_SAMPLE_RATE}..={MAX_SAMPLE_RATE} Hz"
            ),
            Self::Channels { expected, found } => {
                write!(f, "expected {expected} channels, found {found}")
            }
            Self::Length { expected, found } => {
                write!(f, "expected {expected} samples, found {found}")
            }
        }
    }
}

impl std::error::Error for UnsupportedInput {}

#[derive(Clone, Copy, Debug, Default)]
struct Channel {
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
    z1: f64,
    z2: f64,
    /// Sum of the squared weighted samples of the open 100 ms segment.
    energy: f64,
    /// The same for the open 10 ms step.
    step: f64,
}

impl Channel {
    /// Filters `samples` and adds their squares to the open segment and step. The state lives
    /// in locals for the run, and the order of the arithmetic is that of one sample at a time.
    /// `CHECKED` is false only when the caller has seen that every sample is finite.
    #[inline(always)]
    fn run<const CHECKED: bool>(&mut self, k: &KWeighting, samples: impl Iterator<Item = f32>) {
        let mut state = *self;
        for sample in samples {
            state.add::<CHECKED>(k, sample);
        }
        *self = state;
    }

    /// [`Channel::run`] for two channels in one loop. Their filters do not depend on each
    /// other, so the processor overlaps them; each channel still sees the same operations.
    #[inline(always)]
    fn run_pair<const CHECKED: bool>(
        first: &mut Self,
        second: &mut Self,
        k: &KWeighting,
        samples: impl Iterator<Item = (f32, f32)>,
    ) {
        let (mut a, mut b) = (*first, *second);
        for (x, y) in samples {
            a.add::<CHECKED>(k, x);
            b.add::<CHECKED>(k, y);
        }
        (*first, *second) = (a, b);
    }

    #[inline(always)]
    fn add<const CHECKED: bool>(&mut self, k: &KWeighting, sample: f32) {
        // A sample that is not finite would poison the filter state for good; it reads as silence.
        let x = if !CHECKED || sample.is_finite() {
            sample as f64
        } else {
            0.0
        };
        let s = &k.shelf;
        let y =
            s.b[0] * x + s.b[1] * self.x1 + s.b[2] * self.x2 - s.a[0] * self.y1 - s.a[1] * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        // The high pass has `b = [1, -2, 1]` at every rate (see `KWeighting::derive`). `1 * v` is
        // `v` and `-2 * v` is `-(v + v)`, both exact, so these terms give the bits of the products.
        let h = &k.high_pass;
        let z = y - (self.y1 + self.y1) + self.y2 - h.a[0] * self.z1 - h.a[1] * self.z2;
        self.y2 = self.y1;
        self.y1 = y;
        self.z2 = self.z1;
        self.z1 = z;
        let square = z * z;
        self.energy += square;
        self.step += square;
    }
}

/// A loudness meter fed in chunks. It keeps one `f64` per 100 ms of signal, so `report` can
/// gate the whole programme; feeding does not allocate except to store that number. The maxima
/// are kept as it goes, from the last 3 s of 10 ms steps, eight steps at a time.
#[derive(Clone, Debug)]
pub struct Meter {
    weighting: KWeighting,
    channels: Vec<Channel>,
    weights: Vec<f64>,
    /// Frames in a 100 ms segment.
    segment: usize,
    filled: usize,
    /// Weighted mean square of each closed segment, summed over channels.
    segments: Vec<f64>,
    /// Frames in a 10 ms step.
    step: usize,
    step_filled: usize,
    /// Weighted mean square of the last closed steps, oldest first: at least the
    /// `SHORT_TERM_STEPS` that the blocks of the pending steps need, at most `HISTORY`.
    steps: Vec<f64>,
    steps_closed: usize,
    /// The last closed steps whose blocks are not in the maxima yet, fewer than `BATCH`.
    pending: usize,
    /// Mean square of the loudest momentary and short-term block before the pending steps.
    momentary_max: Option<f64>,
    short_term_max: Option<f64>,
}

impl Meter {
    /// A meter where every channel weighs 1, as for mono and stereo.
    pub fn new(sample_rate: u32, channels: usize) -> Result<Self, UnsupportedInput> {
        Self::with_weights(sample_rate, &vec![1.0; channels])
    }

    /// A meter with one BS.1770 weight per channel, e.g. `[1, 1, 1, SURROUND_WEIGHT, SURROUND_WEIGHT]`
    /// for L, R, C, Ls, Rs.
    pub fn with_weights(sample_rate: u32, weights: &[f64]) -> Result<Self, UnsupportedInput> {
        let weighting = KWeighting::new(sample_rate)?;
        if weights.is_empty() || weights.iter().any(|w| !w.is_finite() || *w < 0.0) {
            return Err(UnsupportedInput::Channels {
                expected: weights.len().max(1),
                found: weights
                    .iter()
                    .filter(|w| w.is_finite() && **w >= 0.0)
                    .count(),
            });
        }
        Ok(Self {
            weighting,
            channels: vec![Channel::default(); weights.len()],
            weights: weights.to_vec(),
            // libebur128's rounding, exact for any rate that is a multiple of 10 Hz.
            segment: (sample_rate as usize + 5) / 10,
            filled: 0,
            segments: Vec::new(),
            step: (sample_rate as usize + 50) / 100,
            step_filled: 0,
            steps: Vec::with_capacity(HISTORY),
            steps_closed: 0,
            pending: 0,
            momentary_max: None,
            short_term_max: None,
        })
    }

    pub fn channels(&self) -> usize {
        self.channels.len()
    }

    /// Forgets everything fed so far.
    pub fn reset(&mut self) {
        self.channels.fill(Channel::default());
        self.filled = 0;
        self.segments.clear();
        self.step_filled = 0;
        self.steps.clear();
        self.steps_closed = 0;
        self.pending = 0;
        self.momentary_max = None;
        self.short_term_max = None;
    }

    /// Feeds one slice per channel, all of the same length.
    pub fn add_planar<C: AsRef<[f32]>>(&mut self, channels: &[C]) -> Result<(), UnsupportedInput> {
        if channels.len() != self.channels.len() {
            return Err(UnsupportedInput::Channels {
                expected: self.channels.len(),
                found: channels.len(),
            });
        }
        let frames = channels[0].as_ref().len();
        if let Some(other) = channels.iter().find(|c| c.as_ref().len() != frames) {
            return Err(UnsupportedInput::Length {
                expected: frames,
                found: other.as_ref().len(),
            });
        }
        let weighting = self.weighting;
        let mut start = 0;
        while start < frames {
            let end = self.run_end(start, frames);
            let (pairs, last) = self.channels.as_chunks_mut::<2>();
            let (feeds, last_feed) = channels.as_chunks::<2>();
            for ([first, second], [a, b]) in pairs.iter_mut().zip(feeds) {
                let (a, b) = (&a.as_ref()[start..end], &b.as_ref()[start..end]);
                let samples = a.iter().copied().zip(b.iter().copied());
                if all_finite(a) && all_finite(b) {
                    Channel::run_pair::<false>(first, second, &weighting, samples);
                } else {
                    Channel::run_pair::<true>(first, second, &weighting, samples);
                }
            }
            if let ([channel], [samples]) = (last, last_feed) {
                let samples = &samples.as_ref()[start..end];
                if all_finite(samples) {
                    channel.run::<false>(&weighting, samples.iter().copied());
                } else {
                    channel.run::<true>(&weighting, samples.iter().copied());
                }
            }
            self.advance(end - start);
            start = end;
        }
        Ok(())
    }

    /// Feeds interleaved frames: `[l0, r0, l1, r1, …]` for stereo.
    pub fn add_interleaved(&mut self, samples: &[f32]) -> Result<(), UnsupportedInput> {
        let n = self.channels.len();
        if !samples.len().is_multiple_of(n) {
            return Err(UnsupportedInput::Length {
                expected: samples.len() / n * n + n,
                found: samples.len(),
            });
        }
        let weighting = self.weighting;
        let frames = samples.len() / n;
        let mut start = 0;
        while start < frames {
            let end = self.run_end(start, frames);
            let run = &samples[start * n..end * n];
            let finite = all_finite(run);
            let mut channel = 0;
            while let [first, second, ..] = &mut self.channels[channel..] {
                let samples = run
                    .chunks_exact(n)
                    .map(|frame| (frame[channel], frame[channel + 1]));
                if finite {
                    Channel::run_pair::<false>(first, second, &weighting, samples);
                } else {
                    Channel::run_pair::<true>(first, second, &weighting, samples);
                }
                channel += 2;
            }
            if let [last] = &mut self.channels[channel..] {
                let samples = run.chunks_exact(n).map(|frame| frame[channel]);
                if finite {
                    last.run::<false>(&weighting, samples);
                } else {
                    last.run::<true>(&weighting, samples);
                }
            }
            self.advance(end - start);
            start = end;
        }
        Ok(())
    }

    /// The end of a run from `start` that closes no segment or step before its last frame.
    fn run_end(&self, start: usize, frames: usize) -> usize {
        frames
            .min(start + self.segment - self.filled)
            .min(start + self.step - self.step_filled)
    }

    /// Counts `frames` fed without crossing a segment or step boundary, and closes what they fill.
    fn advance(&mut self, frames: usize) {
        self.step_filled += frames;
        if self.step_filled == self.step {
            self.close_step();
        }
        self.filled += frames;
        if self.filled == self.segment {
            self.close_segment();
        }
    }

    fn close_step(&mut self) {
        let mut energy = 0.0;
        for (channel, weight) in self.channels.iter_mut().zip(&self.weights) {
            energy += channel.step / self.step as f64 * weight;
            channel.step = 0.0;
        }
        if self.steps.len() == HISTORY {
            // Within capacity: shifting the kept steps down does not allocate.
            let keep = SHORT_TERM_STEPS - 1 + self.pending;
            self.steps.drain(..HISTORY - keep);
        }
        self.steps.push(energy);
        self.steps_closed += 1;
        self.step_filled = 0;
        self.pending += 1;
        if self.pending == BATCH {
            let (momentary, short_term) = self.pending_maxima();
            self.momentary_max = louder(self.momentary_max, momentary);
            self.short_term_max = louder(self.short_term_max, short_term);
            self.pending = 0;
        }
    }

    /// Mean square of the loudest momentary and short-term block that ends at a pending step.
    ///
    /// Each block is summed from its newest step back, one step at a time, and the momentary
    /// sum is the short-term one after its first `MOMENTARY_STEPS` terms. A whole batch is
    /// summed in lockstep: the order within each sum does not change, only that the processor
    /// can work on eight of them at once.
    fn pending_maxima(&self) -> (Option<f64>, Option<f64>) {
        let len = self.steps.len();
        let oldest = len - self.pending;
        // Steps closed when the oldest pending one closed.
        let closed = self.steps_closed + 1 - self.pending;
        if self.pending == BATCH && closed >= SHORT_TERM_STEPS {
            let window: &[f64; SHORT_TERM_STEPS - 1 + BATCH] = self.steps
                [oldest + 1 - SHORT_TERM_STEPS..]
                .try_into()
                .expect("the history holds every step a pending block needs");
            // `window[SHORT_TERM_STEPS - 1 + j]` is pending step `j`.
            let newest = |back: usize, j: usize| window[SHORT_TERM_STEPS - 1 + j - back];
            let mut sums: [f64; BATCH] = std::array::from_fn(|j| newest(0, j));
            for back in 1..MOMENTARY_STEPS {
                for (j, sum) in sums.iter_mut().enumerate() {
                    *sum += newest(back, j);
                }
            }
            let momentary = sums;
            for back in MOMENTARY_STEPS..SHORT_TERM_STEPS {
                for (j, sum) in sums.iter_mut().enumerate() {
                    *sum += newest(back, j);
                }
            }
            let loudest = |sums: [f64; BATCH], length: usize| {
                sums.into_iter()
                    .map(|sum| sum / length as f64)
                    .reduce(f64::max)
            };
            return (
                loudest(momentary, MOMENTARY_STEPS),
                loudest(sums, SHORT_TERM_STEPS),
            );
        }
        (0..self.pending)
            .map(|j| block_means(&self.steps, oldest + j, closed + j))
            .fold((None, None), |(m, s), (momentary, short_term)| {
                (louder(m, momentary), louder(s, short_term))
            })
    }

    fn close_segment(&mut self) {
        let mut energy = 0.0;
        for (channel, weight) in self.channels.iter_mut().zip(&self.weights) {
            energy += channel.energy / self.segment as f64 * weight;
            channel.energy = 0.0;
        }
        self.segments.push(energy);
        self.filled = 0;
    }

    /// Mean power of every block of `length` segments, one per segment.
    fn blocks(&self, length: usize) -> Vec<f64> {
        if self.segments.len() < length {
            return Vec::new();
        }
        self.segments
            .windows(length)
            .map(|block| block.iter().sum::<f64>() / length as f64)
            .collect()
    }

    /// The measurement of everything fed so far.
    pub fn report(&self) -> Report {
        let above_gate = |power: Option<f64>| power.map(lufs).filter(|&l| l > ABSOLUTE_GATE);
        let (momentary, short_term) = self.pending_maxima();
        Report {
            integrated_lufs: integrated(&self.blocks(MOMENTARY_SEGMENTS)),
            momentary_max_lufs: above_gate(louder(self.momentary_max, momentary)),
            short_term_max_lufs: above_gate(louder(self.short_term_max, short_term)),
            loudness_range_lu: loudness_range(&self.blocks(SHORT_TERM_SEGMENTS)),
        }
    }
}

/// Measures planar channels of equal weight in one call.
pub fn measure<C: AsRef<[f32]>>(
    channels: &[C],
    sample_rate: u32,
) -> Result<Report, UnsupportedInput> {
    let mut meter = Meter::new(sample_rate, channels.len())?;
    meter.add_planar(channels)?;
    Ok(meter.report())
}

/// Whether every sample is finite, without stopping early, so that it vectorises.
fn all_finite(samples: &[f32]) -> bool {
    samples
        .iter()
        .fold(true, |finite, s| finite & s.is_finite())
}

fn lufs(power: f64) -> f64 {
    -0.691 + 10.0 * power.max(1e-12).log10()
}

fn integrated(blocks: &[f64]) -> Option<f64> {
    let above: Vec<f64> = blocks
        .iter()
        .copied()
        .filter(|&p| lufs(p) > ABSOLUTE_GATE)
        .collect();
    if above.is_empty() {
        return None;
    }
    let threshold = lufs(above.iter().sum::<f64>() / above.len() as f64) + RELATIVE_GATE;
    let gated: Vec<f64> = above.into_iter().filter(|&p| lufs(p) > threshold).collect();
    if gated.is_empty() {
        return None;
    }
    Some(lufs(gated.iter().sum::<f64>() / gated.len() as f64))
}

/// Mean square of the momentary and short-term block that ends at `steps[newest]`, the step
/// that made `closed` closed steps; `None` for a block that does not fit yet. The sum runs from
/// the newest step back, and the momentary one is its first `MOMENTARY_STEPS` terms.
fn block_means(steps: &[f64], newest: usize, closed: usize) -> (Option<f64>, Option<f64>) {
    let terms = closed.min(SHORT_TERM_STEPS);
    let mut sum = steps[newest];
    let mut momentary = None;
    for back in 1..terms {
        if back == MOMENTARY_STEPS {
            momentary = Some(sum / MOMENTARY_STEPS as f64);
        }
        sum += steps[newest - back];
    }
    if terms == MOMENTARY_STEPS {
        momentary = Some(sum / MOMENTARY_STEPS as f64);
    }
    let short_term = (terms == SHORT_TERM_STEPS).then(|| sum / SHORT_TERM_STEPS as f64);
    (momentary, short_term)
}

fn louder(max: Option<f64>, power: Option<f64>) -> Option<f64> {
    match (max, power) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    }
}

fn loudness_range(blocks: &[f64]) -> Option<f64> {
    let levels: Vec<f64> = blocks
        .iter()
        .map(|&p| lufs(p))
        .filter(|&l| l >= ABSOLUTE_GATE)
        .collect();
    if levels.is_empty() {
        return None;
    }
    let power = levels.iter().map(|l| 10f64.powf(l / 10.0)).sum::<f64>() / levels.len() as f64;
    let threshold = 10.0 * power.log10() + RANGE_RELATIVE_GATE;
    let mut gated: Vec<f64> = levels.into_iter().filter(|&l| l >= threshold).collect();
    gated.sort_by(f64::total_cmp);
    let last = (gated.len() - 1) as f64;
    let low = gated[(last * 0.10).round() as usize];
    let high = gated[(last * 0.95).round() as usize];
    Some(high - low)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(hz: f64, dbfs: f64, seconds: f64, rate: u32) -> Vec<f32> {
        let amplitude = 10f64.powf(dbfs / 20.0);
        (0..(seconds * rate as f64).round() as usize)
            .map(|i| (amplitude * (2.0 * PI * hz * i as f64 / rate as f64).sin()) as f32)
            .collect()
    }

    #[test]
    fn derived_coefficients_reproduce_the_printed_ones_at_48k() {
        let printed = KWeighting::BS1770_48K;
        let derived = KWeighting::derive(48_000.0);
        let pairs = [
            (printed.shelf.b.as_slice(), derived.shelf.b.as_slice()),
            (printed.shelf.a.as_slice(), derived.shelf.a.as_slice()),
            (
                printed.high_pass.b.as_slice(),
                derived.high_pass.b.as_slice(),
            ),
            (
                printed.high_pass.a.as_slice(),
                derived.high_pass.a.as_slice(),
            ),
        ];
        for (printed, derived) in pairs {
            for (p, d) in printed.iter().zip(derived) {
                assert!((p - d).abs() < 1e-9, "printed {p}, derived {d}");
            }
        }
        assert_eq!(KWeighting::new(48_000).unwrap(), printed);
    }

    #[test]
    fn the_high_pass_numerator_is_the_one_the_filter_assumes() {
        // `Channel::add` leaves out the products by 1 and by -2 of the high pass.
        for rate in [
            MIN_SAMPLE_RATE,
            22_050,
            44_100,
            48_000,
            96_000,
            MAX_SAMPLE_RATE,
        ] {
            let k = KWeighting::new(rate).unwrap();
            assert_eq!(k.high_pass.b, [1.0, -2.0, 1.0], "{rate}");
        }
    }

    #[test]
    fn a_stereo_sine_reads_the_same_at_every_rate() {
        for rate in [
            8_000, 22_050, 32_000, 44_100, 48_000, 88_200, 96_000, 192_000,
        ] {
            let tone = sine(1000.0, -23.0, 5.0, rate);
            let report = measure(&[&tone, &tone], rate).unwrap();
            let integrated = report.integrated_lufs.unwrap();
            assert!((integrated + 23.0).abs() < 0.05, "{rate}: {integrated}");
        }
    }

    #[test]
    fn chunks_and_layouts_give_the_same_bits() {
        let left = sine(997.0, -18.0, 4.3, 44_100);
        let right = sine(61.0, -9.0, 4.3, 44_100);
        let whole = measure(&[&left, &right], 44_100).unwrap();
        let mut planar = Meter::new(44_100, 2).unwrap();
        let mut interleaved = Meter::new(44_100, 2).unwrap();
        let mut start = 0;
        for size in [1, 7, 4410, 333, 10_000].iter().cycle() {
            let end = left.len().min(start + size);
            planar
                .add_planar(&[&left[start..end], &right[start..end]])
                .unwrap();
            let frames: Vec<f32> = (start..end).flat_map(|i| [left[i], right[i]]).collect();
            interleaved.add_interleaved(&frames).unwrap();
            start = end;
            if start == left.len() {
                break;
            }
        }
        assert_eq!(planar.report(), whole);
        assert_eq!(interleaved.report(), whole);
        assert!(whole.short_term_max_lufs.is_some() && whole.loudness_range_lu.is_some());
    }

    #[test]
    fn short_silent_and_invalid_input() {
        let short = sine(1000.0, -20.0, 0.39, 48_000);
        assert_eq!(measure(&[&short], 48_000).unwrap(), Report::default());
        let silence = vec![0.0f32; 96_000];
        assert_eq!(measure(&[&silence], 48_000).unwrap(), Report::default());
        let mut poisoned = sine(1000.0, -20.0, 1.0, 48_000);
        poisoned[100] = f32::NAN;
        poisoned[200] = f32::INFINITY;
        assert!(
            measure(&[&poisoned], 48_000)
                .unwrap()
                .integrated_lufs
                .unwrap()
                .is_finite()
        );
        assert_eq!(
            Meter::new(4_000, 1).unwrap_err(),
            UnsupportedInput::SampleRate(4_000)
        );
        assert!(Meter::new(48_000, 0).is_err());
        assert!(Meter::with_weights(48_000, &[1.0, f64::NAN]).is_err());
        let mut meter = Meter::new(48_000, 2).unwrap();
        assert!(meter.add_interleaved(&[0.0; 3]).is_err());
        assert!(
            meter
                .add_planar(&[&[0.0f32; 3][..], &[0.0f32; 2][..]])
                .is_err()
        );
        assert!(meter.add_planar(&[&[0.0f32; 3][..]]).is_err());
    }
}
