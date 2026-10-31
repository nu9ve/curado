//! Offline beat grid estimation over a whole track. Never runs in the audio callback.
//!
//! Onsets are measured in three bands, because a kick, a clap and a hi-hat say different things
//! about the beat. Autocorrelation finds candidate tempos; phase coherence across the entire
//! track then measures each one with far more resolution than any single window. The result is
//! one constant grid: music whose tempo really wanders gets a low confidence, not a dynamic grid.
use crate::music::fft;
use std::f64::consts::TAU;

/// One octave and a bit: the same convention as most DJ software. Only tempos in 70–90 and
/// 140–180 have a second reading inside the range, so few tracks need the half/double decision.
const MIN_BPM: f64 = 70.;
const MAX_BPM: f64 = 180.;
const TICKS_PER_SECOND: f64 = 200.;
const BEATS_PER_BAR: usize = 4;
const FOLD_BIN_SECONDS: f64 = 0.005;
const MAX_ATTACKS: usize = 8000;
const MAX_TEMPO_CANDIDATES: usize = 8;
/// Below this, folding shows no pulse at all. Measured against stored grids, only one track
/// in five under about 1.05 had the right tempo, so reporting nothing is the honest answer.
const MIN_SHARPNESS: f64 = 1.03;
const MIN_PULSE_CONTRAST: f64 = 1.5;
/// A beating pad measures 0.04; the quietest of 400 real tracks measured 0.30.
const MIN_DYNAMICS: f64 = 0.15;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    pub bpm: f64,
    /// Seconds of the first grid line, within the first beat of the track.
    pub offset: f64,
    /// Seconds of a bar's first beat.
    pub downbeat: f64,
    pub confidence: f32,
}

#[derive(Default, Clone, Copy)]
struct Biquad {
    b: [f64; 3],
    a: [f64; 2],
    state: [f64; 2],
}
impl Biquad {
    fn new(hz: f64, rate: f64, high_pass: bool) -> Self {
        let w = TAU * hz / rate;
        let alpha = w.sin() / std::f64::consts::SQRT_2;
        let a0 = 1. + alpha;
        let cos = w.cos();
        let b = if high_pass {
            [(1. + cos) / 2., -(1. + cos), (1. + cos) / 2.]
        } else {
            [(1. - cos) / 2., 1. - cos, (1. - cos) / 2.]
        };
        Self {
            b: b.map(|v| v / a0),
            a: [-2. * cos / a0, (1. - alpha) / a0],
            state: [0.; 2],
        }
    }
    #[inline]
    fn process(&mut self, x: f64) -> f64 {
        let y = self.b[0] * x + self.state[0];
        self.state[0] = self.b[1] * x - self.a[0] * y + self.state[1];
        self.state[1] = self.b[2] * x - self.a[1] * y;
        y
    }
}

/// Positive change of log energy per tick, each band normalized to a mean of one.
struct Onsets {
    tick: f64,
    low: Vec<f32>,
    mid: Vec<f32>,
    high: Vec<f32>,
    /// Energy per tick, used to find structural changes.
    energy: [Vec<f32>; 3],
}
impl Onsets {
    fn new(mono: &[f64], rate: f64) -> Self {
        let hop = (rate / TICKS_PER_SECOND).round().max(1.) as usize;
        let mut low_pass = Biquad::new(180., rate, false);
        let mut mid_high = Biquad::new(180., rate, true);
        let mut mid_low = Biquad::new(2200., rate, false);
        let mut high_pass = Biquad::new(2200., rate, true);
        let ticks = mono.len() / hop;
        let mut energy = [
            Vec::with_capacity(ticks),
            Vec::with_capacity(ticks),
            Vec::with_capacity(ticks),
        ];
        for chunk in mono.chunks_exact(hop) {
            let mut sum = [0_f64; 3];
            for &x in chunk {
                let low = low_pass.process(x);
                let mid = mid_low.process(mid_high.process(x));
                let high = high_pass.process(x);
                sum[0] += low * low;
                sum[1] += mid * mid;
                sum[2] += high * high;
            }
            for (band, value) in energy.iter_mut().zip(sum) {
                band.push((value / hop as f64) as f32);
            }
        }
        // A 50 Hz wave ripples within a 5 ms window; a low-band window has to span it.
        let low = flux(&smooth(&energy[0], 4));
        let mid = flux(&smooth(&energy[1], 2));
        let high = flux(&energy[2]);
        Self {
            tick: hop as f64 / rate,
            low,
            mid,
            high,
            energy,
        }
    }
    fn len(&self) -> usize {
        self.low.len()
    }
    fn duration(&self) -> f64 {
        self.len() as f64 * self.tick
    }
    /// How much the level moves over 200 ms. Rhythm has to move it; sustained notes that merely
    /// beat against each other look periodic to every other measure here, but do not.
    fn dynamics(&self) -> f64 {
        const SPAN: usize = 40;
        let moved = |band: &[f32]| {
            let smoothed = smooth(band, SPAN);
            let mean =
                smoothed.iter().map(|v| *v as f64).sum::<f64>() / smoothed.len().max(1) as f64;
            let floor = mean * 1e-4 + 1e-15;
            let level = |v: f32| (v as f64 + floor).ln();
            smoothed
                .windows(SPAN + 1)
                .map(|pair| (level(pair[SPAN]) - level(pair[0])).abs())
                .sum::<f64>()
                / smoothed.len().max(1) as f64
        };
        moved(&self.energy[0]).max(moved(&self.energy[1]))
    }
    /// What marks a beat: kicks and backbeats. Hi-hats mostly mark its subdivisions.
    fn beat_strength(&self, n: usize) -> f64 {
        self.low[n] as f64 + 0.8 * self.mid[n] as f64
    }
}
fn smooth(values: &[f32], width: usize) -> Vec<f32> {
    let mut sum = 0_f64;
    values
        .iter()
        .enumerate()
        .map(|(i, value)| {
            sum += *value as f64;
            if i >= width {
                sum -= values[i - width] as f64;
            }
            (sum.max(0.) / width as f64) as f32
        })
        .collect()
}
fn flux(energy: &[f32]) -> Vec<f32> {
    let mean = energy.iter().map(|v| *v as f64).sum::<f64>() / energy.len().max(1) as f64;
    if mean <= 1e-12 {
        return vec![0.; energy.len()];
    }
    // Ignores what is 40 dB under the band's average, so fades and noise floors stay quiet.
    let floor = mean * 1e-4;
    let log: Vec<f64> = energy.iter().map(|v| (*v as f64 + floor).ln()).collect();
    let mut flux: Vec<f32> = (0..log.len())
        .map(|i| {
            if i < 2 {
                0.
            } else {
                (log[i] - log[i - 2]).max(0.) as f32
            }
        })
        .collect();
    let scale = flux.iter().map(|v| *v as f64).sum::<f64>() / flux.len().max(1) as f64;
    if scale > 1e-9 {
        for value in &mut flux {
            *value = (*value as f64 / scale) as f32;
        }
    }
    flux
}

/// Normalized autocorrelation of the onset curve, read at fractional lags.
struct Periodicity {
    values: Vec<f64>,
}
impl Periodicity {
    fn new(curve: &[f64], max_lag: usize) -> Self {
        let n = curve.len();
        let mean = curve.iter().sum::<f64>() / n.max(1) as f64;
        let size = (2 * n).next_power_of_two().max(2);
        let mut re = vec![0.; size];
        let mut im = vec![0.; size];
        for (value, sample) in re.iter_mut().zip(curve) {
            *value = sample - mean;
        }
        fft(&mut re, &mut im);
        for (r, i) in re.iter_mut().zip(im.iter_mut()) {
            *r = *r * *r + *i * *i;
            *i = 0.;
        }
        fft(&mut re, &mut im);
        let zero = re[0].max(1e-12) / n.max(1) as f64;
        let values = (0..=max_lag.min(n.saturating_sub(1)))
            .map(|lag| re[lag] / (n - lag).max(1) as f64 / zero)
            .collect();
        Self { values }
    }
    fn at(&self, lag: f64) -> f64 {
        let index = lag as usize;
        match (self.values.get(index), self.values.get(index + 1)) {
            (Some(a), Some(b)) => a + (b - a) * lag.fract(),
            _ => 0.,
        }
    }
    /// Multiples of the period also repeat, so a real period keeps scoring all along the comb.
    fn comb(&self, period: f64) -> f64 {
        (1..=8).map(|k| self.at(period * k as f64)).sum::<f64>() / 8.
    }
}

/// Strong attacks across the track as `(seconds, weight)`.
fn attacks(onsets: &Onsets) -> Vec<(f64, f64)> {
    let n = onsets.len();
    let mean = (0..n).map(|i| onsets.beat_strength(i)).sum::<f64>() / n.max(1) as f64;
    let mut peaks = Vec::new();
    for i in 1..n.saturating_sub(1) {
        let strength = onsets.beat_strength(i);
        if strength > mean * 1.5
            && strength >= onsets.beat_strength(i - 1)
            && strength > onsets.beat_strength(i + 1)
        {
            peaks.push(((i as f64 + 0.5) * onsets.tick, strength));
        }
    }
    // One loud section must not outvote the rest of the track.
    let mut ordered: Vec<f64> = peaks.iter().map(|p| p.1).collect();
    ordered.sort_by(f64::total_cmp);
    if let Some(cap) = ordered.get(ordered.len() * 95 / 100).copied() {
        for peak in &mut peaks {
            peak.1 = peak.1.min(cap);
        }
    }
    // Bounds the tempo search on hour-long mixes.
    if peaks.len() > MAX_ATTACKS {
        peaks.select_nth_unstable_by(MAX_ATTACKS, |a, b| b.1.total_cmp(&a.1));
        peaks.truncate(MAX_ATTACKS);
    }
    peaks
}
/// How much the attacks agree on one point of the period over the whole track, from 0 to 1.
fn coherence(attacks: &[(f64, f64)], period: f64) -> f64 {
    let (mut re, mut im, mut total) = (0_f64, 0_f64, 0_f64);
    let omega = TAU / period;
    for &(time, weight) in attacks {
        let (sin, cos) = (time * omega).sin_cos();
        re += weight * cos;
        im += weight * sin;
        total += weight;
    }
    (re * re + im * im).sqrt() / total.max(1e-12)
}
/// Epoch folding: attacks wrapped onto one period pile up only when the period is right, and
/// over a whole track even a small error smears them. 1 means flat; there is no upper limit.
fn sharpness(attacks: &[(f64, f64)], period: f64, profile: &mut Vec<f64>) -> f64 {
    let bins = ((period / FOLD_BIN_SECONDS).ceil() as usize).max(2);
    profile.clear();
    profile.resize(bins, 0.);
    let mut total = 0.;
    for &(time, weight) in attacks {
        let position = time.rem_euclid(period) / period * bins as f64;
        let bin = position as usize % bins;
        let fraction = position.fract();
        profile[bin] += weight * (1. - fraction);
        profile[(bin + 1) % bins] += weight * fraction;
        total += weight;
    }
    profile.iter().map(|v| v * v).sum::<f64>() / (total * total).max(1e-12) * bins as f64
}
/// Maximizes folding near `period`. Steps stay fine enough that a step never drifts by more
/// than a tenth of a beat over the whole track.
fn refine(attacks: &[(f64, f64)], period: f64, duration: f64, spread: f64) -> f64 {
    let mut profile = Vec::new();
    let step = (0.1 * period * period / duration.max(period)).max(period * 1e-5);
    // Bounded: an hour-long mix must not take minutes, and its tempo is rarely that steady.
    let steps = ((period * spread / step).ceil() as i32).min(240);
    let step = period * spread / steps.max(1) as f64;
    let mut best = (f64::MIN, period);
    for k in -steps..=steps {
        let candidate = period + k as f64 * step;
        let score = sharpness(attacks, candidate, &mut profile);
        if score > best.0 {
            best = (score, candidate);
        }
    }
    // Golden-section search around the best grid point.
    let ratio = 0.618_033_988_75;
    let (mut a, mut b) = (best.1 - step, best.1 + step);
    for _ in 0..12 {
        let c = b - (b - a) * ratio;
        let d = a + (b - a) * ratio;
        if sharpness(attacks, c, &mut profile) > sharpness(attacks, d, &mut profile) {
            b = d;
        } else {
            a = c;
        }
    }
    (a + b) / 2.
}
/// Prefers a round tempo when the audio cannot tell it apart: most music is made on a clock.
fn round_tempo(attacks: &[(f64, f64)], period: f64) -> f64 {
    let bpm = 60. / period;
    let mut profile = Vec::new();
    let score = sharpness(attacks, period, &mut profile);
    for (rounded, tolerance) in [(bpm.round(), 0.07), ((bpm * 2.).round() / 2., 0.04)] {
        if (rounded - bpm).abs() <= tolerance
            && sharpness(attacks, 60. / rounded, &mut profile) >= score * 0.97
        {
            return rounded;
        }
    }
    (bpm * 100.).round() / 100.
}

/// Which reading of the tempo explains the track best, within the allowed range.
fn choose_tempo(onsets: &Onsets, attacks: &[(f64, f64)]) -> Option<f64> {
    let combined: Vec<f64> = (0..onsets.len())
        .map(|i| onsets.beat_strength(i) + 0.5 * onsets.high[i] as f64)
        .collect();
    let max_lag = (8. * 60. / 50. / onsets.tick) as usize;
    let all = Periodicity::new(&combined, max_lag);
    let mut scores = Vec::new();
    let mut bpm = 50.;
    while bpm <= 240. {
        scores.push((bpm, all.comb(60. / bpm / onsets.tick).max(0.)));
        bpm += 0.25;
    }
    let peak = scores.iter().map(|s| s.1).fold(0., f64::max);
    // A pulse stands out from the other tempos. Beating sustained notes repeat at every lag.
    let mut ordered: Vec<f64> = scores.iter().map(|s| s.1).collect();
    ordered.sort_by(f64::total_cmp);
    if peak < 0.02 || peak < ordered[ordered.len() / 2] * MIN_PULSE_CONTRAST {
        return None;
    }
    let mut candidates: Vec<f64> = Vec::new();
    for i in 1..scores.len() - 1 {
        if scores[i].1 >= peak * 0.4
            && scores[i].1 >= scores[i - 1].1
            && scores[i].1 > scores[i + 1].1
        {
            for ratio in [0.5, 1., 2.] {
                let tempo = scores[i].0 * ratio;
                if (MIN_BPM..MAX_BPM).contains(&tempo)
                    && !candidates.iter().any(|c| (c - tempo).abs() < 0.5)
                {
                    candidates.push(tempo);
                }
            }
        }
    }
    candidates.sort_by(|a, b| {
        all.comb(60. / b / onsets.tick)
            .total_cmp(&all.comb(60. / a / onsets.tick))
    });
    candidates.truncate(MAX_TEMPO_CANDIDATES);
    let duration = onsets.duration();
    let mut best: Option<(f64, f64)> = None;
    for tempo in candidates {
        let period = refine(attacks, 60. / tempo, duration, 0.012);
        let bpm = 60. / period;
        if !(MIN_BPM..MAX_BPM).contains(&bpm) {
            continue;
        }
        let comb = all.comb(period / onsets.tick).max(0.);
        let magnitude = coherence(attacks, period);
        // Comb rejects a reading that is too fast; coherence rejects one that is too slow.
        let distance = (bpm / 125.).log2() / 1.1;
        let score = comb.sqrt() * magnitude * (-0.5 * distance * distance).exp();
        if best.is_none_or(|b| score > b.1) {
            best = Some((period, score));
        }
    }
    let (period, _) = best?;
    Some(half_or_double(onsets, period))
}
/// 70–90 and 140–180 BPM read both ways. Against 197 such tracks, rekordbox's reading was the
/// fast one up to about 165 BPM and beyond that only with a kick on every beat or a clear
/// backbeat. A wrong guess here is a one-click ×2 or ÷2 in the grid editor.
fn half_or_double(onsets: &Onsets, period: f64) -> f64 {
    let bpm = 60. / period;
    let fast = if bpm >= 140. {
        period
    } else if bpm < 90. {
        period / 2.
    } else {
        return period;
    };
    let ticks = fast / onsets.tick;
    let curve = |band: &[f32]| {
        let values: Vec<f64> = band.iter().map(|v| *v as f64).collect();
        Periodicity::new(&values, (ticks * 5.) as usize)
    };
    let backbeat = curve(&onsets.mid);
    if 60. / fast < 165.
        || curve(&onsets.low).at(ticks) >= 0.3
        || backbeat.at(ticks * 2.) >= backbeat.at(ticks * 4.) * 1.1
    {
        fast
    } else {
        fast * 2.
    }
}

/// Folds every band onto one beat and finds where the average beat starts. A kick rises in
/// the bass and across the spectrum at once, so a louder off-beat bass note or a hi-hat, which
/// only rise in one of them, cannot take the grid from it.
fn beat_phase(onsets: &Onsets, period: f64) -> f64 {
    let bins = ((period / FOLD_BIN_SECONDS).round() as usize).max(4);
    let mut curves = [vec![0_f64; bins], vec![0_f64; bins], vec![0_f64; bins]];
    let mut counts = vec![0_f64; bins];
    for n in 0..onsets.len() {
        let time = (n as f64 + 0.5) * onsets.tick;
        let bin = (time.rem_euclid(period) / period * bins as f64) as usize % bins;
        for (curve, band) in curves.iter_mut().zip(&onsets.energy) {
            curve[bin] += band[n] as f64;
        }
        counts[bin] += 1.;
    }
    for curve in &mut curves {
        for (value, count) in curve.iter_mut().zip(&counts) {
            *value /= count.max(1.);
        }
        let peak = curve.iter().copied().fold(1e-12, f64::max);
        for value in curve.iter_mut() {
            *value /= peak;
        }
    }
    let rise = |curve: &[f64], i: usize| -> f64 {
        let ahead = (curve[(i + 1) % bins] + curve[(i + 2) % bins]) / 2.;
        let behind = (curve[(i + bins - 1) % bins] + curve[(i + bins - 2) % bins]) / 2.;
        (ahead - behind).max(0.)
    };
    let score = |i: usize| {
        let low = rise(&curves[0], i);
        let broad = rise(&curves[1], i) + rise(&curves[2], i);
        low * (0.25 + broad) + 0.15 * broad
    };
    let best = (0..bins)
        .max_by(|a, b| score(*a).total_cmp(&score(*b)))
        .unwrap_or(0);
    (best as f64 + 0.5) / bins as f64 * period
}

/// Moves the grid from the energy peak to the real start of the attack, measured at full
/// sample rate. A loop or cue on the grid then keeps the whole transient.
fn attack_correction(
    samples: &[[f32; 2]],
    sample_rate: u32,
    use_left: bool,
    onsets: &Onsets,
    period: f64,
    offset: f64,
) -> f64 {
    let sr = sample_rate as f64;
    let before = (0.035 * sr).min(period * sr * 0.3) as usize;
    let after = (0.012 * sr) as usize;
    let rise = ((0.0015 * sr) as usize).max(2);
    let decay = (-1. / (0.002 * sr as f32)).exp();
    let mut beats: Vec<(f64, f64)> = Vec::new();
    let mut time = offset;
    while time < onsets.duration() {
        let center = (time / onsets.tick) as usize;
        let range = center.saturating_sub(2)..(center + 3).min(onsets.len());
        let strength = range
            .map(|n| onsets.low[n] as f64 + 0.5 * onsets.mid[n] as f64)
            .fold(0., f64::max);
        beats.push((strength, time));
        time += period;
    }
    beats.sort_by(|a, b| b.0.total_cmp(&a.0));
    beats.truncate(64);
    let mut corrections = Vec::with_capacity(beats.len());
    let mut envelope = Vec::with_capacity(before + after + rise);
    for &(_, time) in &beats {
        let center = (time * sr) as usize;
        let Some(start) = center.checked_sub(before) else {
            continue;
        };
        let end = center + after + rise;
        let Some(window) = samples.get(start..end) else {
            continue;
        };
        envelope.clear();
        let mut level = 0_f32;
        for frame in window {
            let value = if use_left {
                frame[0].abs()
            } else {
                (frame[0] + frame[1]).abs() * 0.5
            };
            level = value.max(level * decay);
            envelope.push(level);
        }
        let mut best = (0_f32, 0);
        for n in 0..envelope.len() - rise {
            let gain = envelope[n + rise] - envelope[n];
            if gain > best.0 {
                best = (gain, n);
            }
        }
        let (gain, n) = best;
        // A real attack clearly rises over what was already sounding.
        if gain < 0.02 || envelope[n + rise] < envelope[n] * 1.3 {
            continue;
        }
        let threshold = envelope[n] + gain * 0.1;
        let first = (n..n + rise)
            .find(|&m| envelope[m] >= threshold)
            .unwrap_or(n);
        corrections.push((start + first) as f64 / sr - time);
    }
    if corrections.len() < 4 {
        return 0.;
    }
    corrections.sort_by(f64::total_cmp);
    corrections[corrections.len() / 2]
}

/// Bars. Music usually starts on a bar, so that is the starting belief; kicks on 1, backbeats
/// on 2 and 4 and sections changing on a downbeat have to outweigh it.
fn bar_phase(onsets: &Onsets, period: f64, offset: f64, first_sound: f64) -> usize {
    let beats = ((onsets.duration() - offset) / period) as usize;
    if beats < BEATS_PER_BAR * 2 {
        return 0;
    }
    let peak_near = |curve: &[f32], beat: usize| -> f64 {
        let center = ((offset + beat as f64 * period) / onsets.tick) as usize;
        let range = center.saturating_sub(2)..(center + 3).min(curve.len());
        range.map(|n| curve[n] as f64).fold(0., f64::max)
    };
    let mean_near = |band: &[f32], beat: f64, span: f64| -> f64 {
        let from = (((offset + beat * period) / onsets.tick) as usize).min(band.len());
        let to = (((offset + (beat + span) * period) / onsets.tick) as usize).min(band.len());
        band[from..to].iter().map(|v| *v as f64).sum::<f64>() / (to - from).max(1) as f64
    };
    let mut kick = [0_f64; BEATS_PER_BAR];
    let mut backbeat = [0_f64; BEATS_PER_BAR];
    let mut change = [0_f64; BEATS_PER_BAR];
    let mut changes = Vec::with_capacity(beats);
    for beat in 0..beats {
        kick[beat % BEATS_PER_BAR] += peak_near(&onsets.low, beat);
        // Energy, not onset strength: a kick and a clap both start sharply, but only the clap
        // adds this much above the bass.
        backbeat[beat % BEATS_PER_BAR] += mean_near(&onsets.energy[1], beat as f64, 0.15)
            + mean_near(&onsets.energy[2], beat as f64, 0.15);
        let mut amount = 0.;
        if beat >= BEATS_PER_BAR && beat + BEATS_PER_BAR <= beats {
            for band in &onsets.energy {
                let earlier = mean_near(band, beat as f64 - 4., 4.);
                let later = mean_near(band, beat as f64, 4.);
                amount += ((later + 1e-9) / (earlier + 1e-9)).ln().abs();
            }
        }
        changes.push(amount);
    }
    // Only the peak of a real section change counts; ordinary variation should not vote.
    let mut ordered = changes.clone();
    ordered.sort_by(f64::total_cmp);
    let threshold = ordered[ordered.len() * 9 / 10].max(0.5);
    for (beat, amount) in changes.iter().enumerate() {
        let from = beat.saturating_sub(2);
        let to = (beat + 3).min(changes.len());
        if *amount >= threshold && changes[from..to].iter().all(|other| other <= amount) {
            change[beat % BEATS_PER_BAR] += amount;
        }
    }
    let normalize = |values: [f64; BEATS_PER_BAR]| {
        let total: f64 = values.iter().sum();
        values.map(|v| v / total.max(1e-9))
    };
    let (kick, backbeat, change) = (normalize(kick), normalize(backbeat), normalize(change));
    let first = ((first_sound - offset) / period).round().max(0.) as usize % BEATS_PER_BAR;
    let mut best = (f64::MIN, 0);
    for phase in 0..BEATS_PER_BAR {
        let at =
            |values: &[f64; BEATS_PER_BAR], shift: usize| values[(phase + shift) % BEATS_PER_BAR];
        let snare = at(&backbeat, 1) + at(&backbeat, 3) - at(&backbeat, 0) - at(&backbeat, 2);
        let mut score = at(&kick, 0) + 0.5 * snare + 0.5 * at(&change, 0);
        if phase == 0 {
            score += 0.2;
        }
        if phase == first {
            score += 0.1;
        }
        if score > best.0 {
            best = (score, phase);
        }
    }
    best.1
}

pub fn estimate(
    samples: &[[f32; 2]],
    sample_rate: u32,
    use_left: bool,
    mono: &[f64],
    rate: f64,
    first_sound: f64,
) -> Option<Grid> {
    let onsets = Onsets::new(mono, rate);
    if onsets.duration() < 4. {
        return None;
    }
    let attacks = attacks(&onsets);
    if attacks.len() < 8 || onsets.dynamics() < MIN_DYNAMICS {
        return None;
    }
    let period = 60. / round_tempo(&attacks, choose_tempo(&onsets, &attacks)?);
    let folded = sharpness(&attacks, period, &mut Vec::new());
    if folded < MIN_SHARPNESS {
        return None;
    }
    let phase = beat_phase(&onsets, period);
    let correction = attack_correction(samples, sample_rate, use_left, &onsets, period, phase);
    // A hair before the attack: cutting silence is inaudible, cutting a transient is not.
    let offset = (phase + correction - 0.001).rem_euclid(period);
    let bar = bar_phase(&onsets, period, offset, first_sound);
    Some(Grid {
        bpm: 60. / period,
        offset,
        downbeat: offset + bar as f64 * period,
        // How clearly attacks pile up on one period: the track's own evidence, which measured
        // against stored grids separates right from wrong tempos, but it is not a probability.
        confidence: (1. - 1. / folded).clamp(0., 1.) as f32,
    })
}

/// Seconds of the first sound within 50 dB of the track's peak, ignoring any noise under it.
pub fn first_sound(samples: &[[f32; 2]], sample_rate: u32) -> f64 {
    let peak = samples.iter().fold(0_f32, |max, frame| {
        max.max(frame[0].abs()).max(frame[1].abs())
    });
    let threshold = peak * 0.003_16;
    samples
        .iter()
        .position(|frame| frame[0].abs().max(frame[1].abs()) >= threshold && peak > 0.)
        .map_or(0., |frame| frame as f64 / sample_rate as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music::analyze;

    /// Deterministic audio with every kick's first sample known, like a real production.
    struct Track {
        rate: f64,
        buffer: Vec<f64>,
        noise: u64,
    }
    impl Track {
        fn new(seconds: f64, rate: f64) -> Self {
            Self {
                rate,
                buffer: vec![0.; (seconds * rate) as usize],
                noise: 0x9E37_79B9_7F4A_7C15,
            }
        }
        fn kick(&mut self, time: f64, attack_ms: f64, gain: f64) {
            let start = (time * self.rate) as usize;
            let mut phase = 0.;
            for i in 0..(0.35 * self.rate) as usize {
                let Some(out) = self.buffer.get_mut(start + i) else {
                    break;
                };
                let t = i as f64 / self.rate;
                phase += TAU * (48. + 130. * (-t / 0.03).exp()) / self.rate;
                let envelope = (t / (attack_ms / 1000.)).min(1.) * (-t / 0.11).exp();
                let click = if t < 0.0015 {
                    0.35 * (1. - t / 0.0015)
                } else {
                    0.
                };
                *out += gain * (phase.sin() * envelope + click);
            }
        }
        fn hit(&mut self, time: f64, decay: f64, gain: f64, bright: bool) {
            let start = (time * self.rate) as usize;
            let mut last = 0.;
            for i in 0..(decay * 6. * self.rate) as usize {
                self.noise ^= self.noise << 13;
                self.noise ^= self.noise >> 7;
                self.noise ^= self.noise << 17;
                let n = (self.noise >> 11) as f64 / (1_u64 << 53) as f64 * 2. - 1.;
                let value = if bright { n - last } else { n };
                last = n;
                let Some(out) = self.buffer.get_mut(start + i) else {
                    break;
                };
                *out += gain * value * (-(i as f64) / self.rate / decay).exp();
            }
        }
        fn tone(&mut self, from: f64, to: f64, hz: f64, gain: f64) {
            let (a, b) = ((from * self.rate) as usize, (to * self.rate) as usize);
            let fade = 0.004 * self.rate;
            for i in a..b.min(self.buffer.len()) {
                let shape = ((i - a) as f64 / fade).min(1.) * ((b - i) as f64 / fade).min(1.);
                self.buffer[i] += gain * shape * (TAU * hz * i as f64 / self.rate).sin();
            }
        }
        fn finish(self) -> Vec<[f32; 2]> {
            let peak = self.buffer.iter().fold(1e-9_f64, |m, v| m.max(v.abs()));
            self.buffer
                .iter()
                .map(|v| [(v / peak * 0.97) as f32; 2])
                .collect()
        }
    }
    const FIRST: f64 = 0.25;
    fn beats(seconds: f64, bpm: f64) -> impl Iterator<Item = (usize, f64)> {
        let period = 60. / bpm;
        (0..((seconds - FIRST) / period) as usize).map(move |i| (i, FIRST + i as f64 * period))
    }
    /// Milliseconds our grid sits after the true first sample of the kicks.
    fn late_ms(offset: f64, bpm: f64) -> f64 {
        let period = 60. / bpm;
        let difference = (offset - FIRST).rem_euclid(period);
        (if difference > period / 2. {
            difference - period
        } else {
            difference
        }) * 1000.
    }
    fn house(seconds: f64, rate: f64, bpm: f64) -> Vec<[f32; 2]> {
        let mut track = Track::new(seconds, rate);
        let half = 30. / bpm;
        for (i, time) in beats(seconds, bpm) {
            track.kick(time, 1., 1.);
            track.hit(time + half, 0.03, 0.22, true);
            track.tone(time + half / 2., time + half * 1.5, 55., 0.45);
            if i % 2 == 1 {
                track.hit(time, 0.06, 0.5, false);
            }
        }
        track.finish()
    }
    fn assert_grid(samples: &[[f32; 2]], rate: f64, bpm: f64, label: &str) {
        let analysis = analyze(samples, rate as u32);
        let found = analysis.bpm.unwrap_or_else(|| panic!("{label}: no BPM"));
        assert!(
            (found - bpm).abs() < 0.02,
            "{label}: {found} instead of {bpm}"
        );
        let late = late_ms(analysis.beat_offset, bpm);
        assert!(
            (-4.0..=1.5).contains(&late),
            "{label}: grid {late:.2} ms from the kick's first sample"
        );
    }

    #[test]
    fn four_on_the_floor_lands_just_before_the_real_attack() {
        for rate in [44100., 48000., 96000.] {
            assert_grid(&house(60., rate, 124.), rate, 124., "house");
            let mut solo = Track::new(40., rate);
            for (_, time) in beats(40., 128.) {
                solo.kick(time, 1., 1.);
            }
            assert_grid(&solo.finish(), rate, 128., "solo kick");
        }
    }

    #[test]
    fn a_loud_off_beat_bass_does_not_take_the_grid() {
        let bpm = 126.;
        let half = 30. / bpm;
        let mut track = Track::new(60., 44100.);
        for (_, time) in beats(60., bpm) {
            track.kick(time, 1., 0.8);
            track.tone(time + half, time + half * 1.8, 65., 0.7);
        }
        let analysis = analyze(&track.finish(), 44100);
        assert!((analysis.bpm.unwrap() - bpm).abs() < 0.02, "{analysis:?}");
        assert!(
            late_ms(analysis.beat_offset, bpm).abs() < 8.,
            "{analysis:?}"
        );
    }

    #[test]
    fn half_and_double_readings_follow_the_backbeat() {
        let mut dnb = Track::new(60., 44100.);
        for (i, time) in beats(60., 174.) {
            if i % 2 == 1 {
                dnb.hit(time, 0.07, 0.8, false);
            } else if i % 4 == 0 {
                dnb.kick(time, 1., 1.);
            } else {
                dnb.kick(time + 30. / 174., 1., 0.8);
            }
            dnb.hit(time, 0.012, 0.15, true);
        }
        // Fast or half time are both valid readings here; anything else is a wrong tempo.
        let found = analyze(&dnb.finish(), 44100).bpm.unwrap();
        assert!(
            (found - 174.).abs() < 0.05 || (found - 87.).abs() < 0.05,
            "drum and bass: {found}"
        );
        let mut hiphop = Track::new(60., 44100.);
        for (i, time) in beats(60., 87.) {
            if i % 2 == 0 {
                hiphop.kick(time, 2., 1.);
            } else {
                hiphop.hit(time, 0.09, 0.8, false);
            }
            hiphop.hit(time + 30. / 87., 0.012, 0.12, true);
        }
        let found = analyze(&hiphop.finish(), 44100).bpm.unwrap();
        assert!((found - 87.).abs() < 0.05, "hip-hop: {found}");
    }

    #[test]
    fn intros_without_a_beat_do_not_move_the_grid() {
        let bpm = 128.;
        let mut track = Track::new(70., 44100.);
        track.tone(0., 32., 220., 0.25);
        track.tone(0., 32., 330., 0.2);
        for (_, time) in beats(70., bpm) {
            if time >= 32. {
                track.kick(time, 1., 1.);
            }
        }
        assert_grid(&track.finish(), 44100., bpm, "pad intro");
    }

    #[test]
    fn long_track_resolves_a_tempo_a_short_window_cannot() {
        // 0.1 BPM of error would drift 240 ms over five minutes.
        let bpm = 127.37;
        let analysis = analyze(&house(300., 22050., bpm), 22050);
        let found = analysis.bpm.unwrap();
        assert!((found - bpm).abs() < 0.015, "{found}");
    }

    #[test]
    fn bars_start_at_the_top_of_the_track_unless_a_section_says_otherwise() {
        let bpm = 124.;
        // Claps on 2 and 4 cannot separate beats 1 and 3: the section entrance decides.
        for true_bar in [0, 2] {
            let mut track = Track::new(60., 44100.);
            for (i, time) in beats(60., bpm) {
                track.kick(time, 1., 1.);
                if i % 2 == 1 {
                    track.hit(time, 0.06, 0.7, false);
                }
                if i >= 32 + true_bar {
                    track.tone(time, time + 30. / bpm, 110., 0.7);
                }
            }
            let analysis = analyze(&track.finish(), 44100);
            let period = 60. / analysis.bpm.unwrap();
            let beats_from_true = (analysis.downbeat - (FIRST + true_bar as f64 * period)) / period;
            assert_eq!(
                (beats_from_true.round() as i64).rem_euclid(4),
                0,
                "true bar on beat {true_bar}: {analysis:?}"
            );
        }
        // With no structure to go on, the first sound is the best evidence.
        let mut steady = Track::new(40., 44100.);
        for (_, time) in beats(40., bpm) {
            steady.kick(time, 1., 1.);
        }
        let analysis = analyze(&steady.finish(), 44100);
        assert!(
            (analysis.downbeat - analysis.beat_offset).abs() < 1e-9,
            "{analysis:?}"
        );
    }

    #[test]
    fn no_rhythm_reports_no_tempo() {
        let mut pad = Track::new(30., 44100.);
        pad.tone(0., 30., 220., 0.3);
        pad.tone(0., 30., 277.2, 0.3);
        let sustained = analyze(&pad.finish(), 44100);
        assert_eq!(sustained.bpm, None, "{sustained:?}");
        assert_eq!(analyze(&vec![[0.; 2]; 48000 * 8], 48000).bpm, None);
    }

    #[test]
    fn first_sound_skips_leading_silence_and_noise() {
        let mut samples = vec![[0.001_f32; 2]; 48000];
        samples[..12000].fill([0.; 2]);
        for frame in &mut samples[24000..] {
            *frame = [0.8; 2];
        }
        assert_eq!(first_sound(&samples, 48000), 0.5);
        assert_eq!(first_sound(&vec![[0.; 2]; 100], 48000), 0.);
    }
}
