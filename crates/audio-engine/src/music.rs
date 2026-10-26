//! Bounded, offline musical analysis. Never runs in the audio callback.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MusicalKey {
    /// C=0, C#=1 ... B=11.
    pub tonic: u8,
    pub minor: bool,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MusicAnalysis {
    pub version: u32,
    pub bpm: Option<f64>,
    pub beat_offset: f64,
    pub bpm_confidence: f32,
    pub key: Option<MusicalKey>,
    pub key_confidence: f32,
    #[serde(default)]
    pub manual: bool,
}
impl MusicAnalysis {
    pub fn valid(&self) -> bool {
        self.bpm
            .is_none_or(|b| b.is_finite() && (40.0..=300.0).contains(&b))
            && self.beat_offset.is_finite()
            && self.beat_offset >= 0.
            && self.key.is_none_or(|k| k.tonic < 12)
            && self.bpm_confidence.is_finite()
            && (0.0..=1.0).contains(&self.bpm_confidence)
            && self.key_confidence.is_finite()
            && (0.0..=1.0).contains(&self.key_confidence)
    }
}

pub fn analyze(samples: &[[f32; 2]], sample_rate: u32) -> MusicAnalysis {
    let limit = samples.len().min(sample_rate as usize * 90);
    let stride = (sample_rate / 11025).max(1) as usize;
    let rate = sample_rate as f64 / stride as f64;
    let energy = samples[..limit]
        .iter()
        .take(sample_rate as usize)
        .fold([0.; 2], |mut e, f| {
            e[0] += (f[0] + f[1]).powi(2) as f64;
            e[1] += (f[0].powi(2) + f[1].powi(2)) as f64;
            e
        });
    // Avoid treating antiphase stereo as silence.
    let mono: Vec<f64> = samples[..limit]
        .chunks(stride)
        .map(|chunk| {
            chunk
                .iter()
                .map(|f| {
                    if energy[0] < energy[1] * 0.01 {
                        f[0] as f64
                    } else {
                        (f[0] as f64 + f[1] as f64) * 0.5
                    }
                })
                .sum::<f64>()
                / chunk.len() as f64
        })
        .collect();
    let (bpm, beat_offset, bpm_confidence) = tempo(&mono, rate);
    let (key, key_confidence) = key(&mono, rate);
    MusicAnalysis {
        version: 1,
        bpm,
        beat_offset,
        bpm_confidence,
        key,
        key_confidence,
        manual: false,
    }
}

fn tempo(mono: &[f64], rate: f64) -> (Option<f64>, f64, f32) {
    let hop = (rate / 200.).round().max(1.) as usize;
    let tick = hop as f64 / rate;
    let mut baseline = 0.;
    let onset: Vec<f64> = mono
        .chunks(hop)
        .map(|c| {
            let rms = (c.iter().map(|v| v * v).sum::<f64>() / c.len() as f64).sqrt();
            baseline += (rms - baseline) * 0.08;
            (rms - baseline).max(0.)
        })
        .collect();
    if onset.len() < (4. / tick) as usize {
        return (None, 0., 0.);
    }
    let total = onset.iter().map(|v| v * v).sum::<f64>();
    if total < 1e-8 {
        return (None, 0., 0.);
    }
    let score = |lag: usize| {
        let a = &onset[..onset.len() - lag];
        let b = &onset[lag..];
        let xy = a.iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
        let xx = a.iter().map(|v| v * v).sum::<f64>();
        let yy = b.iter().map(|v| v * v).sum::<f64>();
        xy / (xx * yy).sqrt().max(1e-12)
    };
    let min_lag = (60. / 180. / tick).ceil() as usize;
    let max_lag = ((60. / 75. / tick).floor() as usize).min(onset.len() / 3);
    let (lag, confidence) = (min_lag..=max_lag)
        .map(|lag| (lag, score(lag)))
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap_or((0, 0.));
    if confidence < 0.25 {
        return (None, 0., confidence as f32);
    }
    let peak = onset.iter().copied().fold(0., f64::max);
    let mut peaks = Vec::new();
    for i in 1..onset.len() - 1 {
        if onset[i] > peak * 0.15 && onset[i] >= onset[i - 1] && onset[i] > onset[i + 1] {
            peaks.push((i as f64 * tick, onset[i]));
        }
    }
    let mut period = lag as f64 * tick;
    let mut anchor = peaks
        .iter()
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map_or(0., |p| p.0);
    // Refine from many attacks instead of rounding BPM to an envelope lag.
    for _ in 0..3 {
        let mut sums = [0.; 5];
        for &(time, weight) in &peaks {
            let beat = ((time - anchor) / period).round();
            if (time - anchor - beat * period).abs() > period * 0.12 {
                continue;
            }
            sums[0] += weight;
            sums[1] += weight * beat;
            sums[2] += weight * time;
            sums[3] += weight * beat * beat;
            sums[4] += weight * beat * time;
        }
        let denominator = sums[0] * sums[3] - sums[1] * sums[1];
        if denominator > 1e-9 {
            period = (sums[0] * sums[4] - sums[1] * sums[2]) / denominator;
            anchor = (sums[2] - period * sums[1]) / sums[0];
        }
    }
    let bpm = 60. / period;
    if !(40.0..=300.0).contains(&bpm) {
        return (None, 0., 0.);
    }
    (
        Some(bpm),
        anchor.rem_euclid(period),
        confidence.clamp(0., 1.) as f32,
    )
}

fn key(mono: &[f64], rate: f64) -> (Option<MusicalKey>, f32) {
    const N: usize = 8192;
    if mono.len() < N {
        return (None, 0.);
    }
    let mut re = vec![0.; N];
    let mut im = vec![0.; N];
    let mut chroma = [0.; 12];
    for window in mono.windows(N).step_by(N / 2) {
        for i in 0..N {
            re[i] = window[i] * (0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / N as f64).cos());
            im[i] = 0.;
        }
        fft(&mut re, &mut im);
        let mag: Vec<f64> = re
            .iter()
            .zip(&im)
            .take(N / 2)
            .map(|(r, i)| (r * r + i * i).sqrt())
            .collect();
        for i in 2..N / 2 - 1 {
            let hz = i as f64 * rate / N as f64;
            if !(65.0..=2500.0).contains(&hz) || mag[i] <= mag[i - 1] || mag[i] < mag[i + 1] {
                continue;
            }
            let a = mag[i - 1].max(1e-12).ln();
            let b = mag[i].max(1e-12).ln();
            let c = mag[i + 1].max(1e-12).ln();
            let delta = (0.5 * (a - c) / (a - 2. * b + c)).clamp(-0.5, 0.5);
            let frequency = (i as f64 + delta) * rate / N as f64;
            let note = 69. + 12. * (frequency / 440.).log2();
            let nearest = note.round();
            let tuning_weight = (1. - (note - nearest).abs() * 2.).max(0.);
            chroma[(nearest as i32).rem_euclid(12) as usize] += mag[i].sqrt() * tuning_weight;
        }
    }
    if chroma.iter().sum::<f64>() < 1e-5 {
        return (None, 0.);
    }
    // Krumhansl-Kessler major/minor profiles, evaluated by Pearson correlation.
    let profiles = [
        [
            6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88,
        ],
        [
            6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17,
        ],
    ];
    let mean = chroma.iter().sum::<f64>() / 12.;
    if chroma.iter().copied().fold(0., f64::max) < mean * 1.4 {
        return (None, 0.);
    }
    let mut candidates = Vec::new();
    for (minor, profile) in profiles.iter().enumerate() {
        let pm = profile.iter().sum::<f64>() / 12.;
        for tonic in 0..12 {
            let mut values = [0.; 3];
            for i in 0..12 {
                let a = chroma[(i + tonic) % 12] - mean;
                let b = profile[i] - pm;
                values[0] += a * b;
                values[1] += a * a;
                values[2] += b * b;
            }
            candidates.push((
                values[0] / (values[1] * values[2]).sqrt().max(1e-12),
                MusicalKey {
                    tonic: tonic as u8,
                    minor: minor == 1,
                },
            ));
        }
    }
    candidates.sort_by(|a, b| b.0.total_cmp(&a.0));
    let confidence = ((candidates[0].0 - candidates[1].0) * 3.).clamp(0., 1.) as f32;
    if candidates[0].0 < 0.45 {
        (None, confidence)
    } else {
        (Some(candidates[0].1), confidence)
    }
}

fn fft(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut width = 2;
    while width <= n {
        let angle = -std::f64::consts::TAU / width as f64;
        let (wi, wr) = angle.sin_cos();
        for start in (0..n).step_by(width) {
            let (mut r, mut m) = (1., 0.);
            for k in 0..width / 2 {
                let (a, b) = (start + k, start + k + width / 2);
                let tr = r * re[b] - m * im[b];
                let ti = r * im[b] + m * re[b];
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
                (r, m) = (r * wr - m * wi, r * wi + m * wr);
            }
        }
        width *= 2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn silence_is_unknown_and_click_tempos_have_a_measured_grid() {
        assert_eq!(analyze(&vec![[0.; 2]; 48000], 48000).key, None);
        for bpm in [90., 120., 124., 150.] {
            let sr = 12000;
            let mut samples = vec![[0.; 2]; sr * 20];
            for beat in 0..(20. * bpm / 60.) as usize {
                let index = ((0.12 + beat as f64 * 60. / bpm) * sr as f64) as usize;
                for i in 0..120.min(samples.len().saturating_sub(index)) {
                    let v = ((i as f32 * 0.4).sin() * (-(i as f32) / 20.).exp()) * 0.8;
                    samples[index + i] = [v, v];
                }
            }
            let a = analyze(&samples, sr as u32);
            assert!((a.bpm.unwrap() - bpm).abs() < 0.1, "{bpm}: {a:?}");
            assert!((a.beat_offset - 0.12).abs() < 0.02, "{a:?}");
        }
    }
    #[test]
    fn detects_major_and_minor_triads_in_all_twelve_roots() {
        for tonic in 0..12 {
            for minor in [false, true] {
                let samples: Vec<_> = (0..24000)
                    .map(|i| {
                        let mut x = 0.;
                        for semitone in [0, if minor { 3 } else { 4 }, 7] {
                            let hz = 261.625565 * 2_f64.powf((tonic + semitone) as f64 / 12.);
                            x += (std::f64::consts::TAU * hz * i as f64 / 12000.).sin() * 0.2;
                        }
                        [x as f32, x as f32]
                    })
                    .collect();
                assert_eq!(
                    analyze(&samples, 12000).key,
                    Some(MusicalKey {
                        tonic: tonic as u8,
                        minor
                    }),
                    "root={tonic} minor={minor}"
                );
            }
        }
    }
}
