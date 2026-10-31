//! Bounded, offline musical analysis. Never runs in the audio callback.
use crate::beat;
use serde::{Deserialize, Serialize};

/// Stored estimates older than this are recalculated; manual corrections are always kept.
pub const ANALYSIS_VERSION: u32 = 2;

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
    /// Seconds of any bar's first beat. A position rather than a beat index, so it stays
    /// correct when BPM or offset are edited.
    #[serde(default)]
    pub downbeat: f64,
    /// Seconds of the first audible sound: where an unsaved cue starts.
    #[serde(default)]
    pub first_sound: f64,
}
impl MusicAnalysis {
    pub fn is_current(&self) -> bool {
        self.manual || self.version >= ANALYSIS_VERSION
    }
    pub fn valid(&self) -> bool {
        self.bpm
            .is_none_or(|b| b.is_finite() && (40.0..=300.0).contains(&b))
            && self.beat_offset.is_finite()
            && self.beat_offset >= 0.
            && self.downbeat.is_finite()
            && self.downbeat >= 0.
            && self.first_sound.is_finite()
            && self.first_sound >= 0.
            && self.key.is_none_or(|k| k.tonic < 12)
            && self.bpm_confidence.is_finite()
            && (0.0..=1.0).contains(&self.bpm_confidence)
            && self.key_confidence.is_finite()
            && (0.0..=1.0).contains(&self.key_confidence)
    }
}

pub fn analyze(samples: &[[f32; 2]], sample_rate: u32) -> MusicAnalysis {
    let stride = (sample_rate / 11025).max(1) as usize;
    let rate = sample_rate as f64 / stride as f64;
    let energy = samples.iter().fold([0.; 2], |mut e, f| {
        e[0] += (f[0] + f[1]).powi(2) as f64;
        e[1] += (f[0].powi(2) + f[1].powi(2)) as f64;
        e
    });
    // Avoid treating antiphase stereo as silence.
    let use_left = energy[0] < energy[1] * 0.01;
    // Single precision: this buffer is over 150 MiB on an hour-long track.
    let mono: Vec<f32> = samples
        .chunks(stride)
        .map(|chunk| {
            chunk
                .iter()
                .map(|f| if use_left { f[0] } else { (f[0] + f[1]) * 0.5 })
                .sum::<f32>()
                / chunk.len() as f32
        })
        .collect();
    let first_sound = beat::first_sound(samples, sample_rate);
    let grid = beat::estimate(samples, sample_rate, use_left, &mono, rate, first_sound);
    let (key, key_confidence) = key(&mono, rate);
    MusicAnalysis {
        version: ANALYSIS_VERSION,
        bpm: grid.map(|g| g.bpm),
        beat_offset: grid.map_or(0., |g| g.offset),
        bpm_confidence: grid.map_or(0., |g| g.confidence),
        key,
        key_confidence,
        manual: false,
        downbeat: grid.map_or(0., |g| g.downbeat),
        first_sound,
    }
}

fn key(mono: &[f32], rate: f64) -> (Option<MusicalKey>, f32) {
    const N: usize = 8192;
    if mono.len() < N {
        return (None, 0.);
    }
    let mut re = vec![0.; N];
    let mut im = vec![0.; N];
    let mut chroma = [0.; 12];
    // Overlap only pays on short clips; a whole track has plenty of windows without it.
    let hop = if (mono.len() as f64) < rate * 90. {
        N / 2
    } else {
        N
    };
    for window in mono.windows(N).step_by(hop) {
        for i in 0..N {
            re[i] = window[i] as f64
                * (0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / N as f64).cos());
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

pub(crate) fn fft(re: &mut [f64], im: &mut [f64]) {
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
    fn old_estimates_update_and_invalid_positions_are_rejected() {
        let estimate = MusicAnalysis {
            version: 1,
            ..Default::default()
        };
        assert!(!estimate.is_current());
        assert!(MusicAnalysis {
            manual: true,
            ..estimate
        }
        .is_current());
        for invalid in [f64::NAN, -1.] {
            assert!(!MusicAnalysis {
                downbeat: invalid,
                ..estimate
            }
            .valid());
            assert!(!MusicAnalysis {
                first_sound: invalid,
                ..estimate
            }
            .valid());
        }
    }
    #[test]
    fn silence_is_unknown() {
        assert_eq!(analyze(&vec![[0.; 2]; 48000], 48000).key, None);
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
