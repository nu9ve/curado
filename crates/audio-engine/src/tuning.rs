//! Offline pitch preparation. The renderer only swaps immutable PCM and scalar state.
use crate::{decode::Clip, music::MusicAnalysis};
use serde::Serialize;
use std::sync::{
    atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering::Relaxed},
    Arc,
};

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tuning {
    pub rate: f32,
    pub semitones: i8,
    pub key_lock: bool,
}
impl Default for Tuning {
    fn default() -> Self {
        Self {
            rate: 1.,
            semitones: 0,
            key_lock: true,
        }
    }
}
impl Tuning {
    pub fn factor(self) -> f32 {
        2_f32.powf(self.semitones as f32 / 12.) / if self.key_lock { self.rate } else { 1. }
    }
}
#[derive(Clone)]
pub struct TunedClip {
    pub clip: Arc<Clip>,
    pub load_ticket: u64,
    pub revision: u64,
    pub tuning: Tuning,
}

/// Same-length pitch render; speed remains a separate renderer parameter.
/// Cancel between blocks, discard both latency halves, preserve stereo phase.
pub fn prepare(
    source: &Arc<Clip>,
    tuning: Tuning,
    cancelled: impl Fn() -> bool,
) -> Result<Arc<Clip>, String> {
    if cancelled() {
        return Err("Ajuste sustituido por una petición más reciente".into());
    }
    if (tuning.factor() - 1.).abs() < 1e-6 {
        return Ok(source.clone());
    }
    let mut stretch = signalsmith_stretch::Stretch::new(2, 2048, 256);
    stretch.set_transpose_factor(tuning.factor(), None);
    let latency = stretch.input_latency() + stretch.output_latency();
    let mut samples = Vec::new();
    samples
        .try_reserve_exact(source.samples.len())
        .map_err(|_| "Memoria insuficiente para preparar tonalidad")?;
    samples.resize(source.samples.len(), [0.; 2]);
    let silence = [0_f32; 1024];
    let mut output = [0_f32; 1024];
    let mut offset = 0;
    while offset < source.samples.len() + latency {
        if cancelled() {
            return Err("Ajuste sustituido por una petición más reciente".into());
        }
        let count = 512.min(source.samples.len() + latency - offset);
        let input = if offset < source.samples.len() {
            source.samples[offset..(offset + count).min(source.samples.len())].as_flattened()
        } else {
            &silence[..count * 2]
        };
        // The final source block can be shorter; keep input/output lengths equal.
        let count = input.len() / 2;
        stretch.process(input, &mut output[..count * 2]);
        for i in 0..count {
            if let Some(destination) = (offset + i)
                .checked_sub(latency)
                .filter(|n| *n < samples.len())
            {
                samples[destination] = [output[i * 2], output[i * 2 + 1]];
            }
        }
        offset += count;
    }
    Ok(Arc::new(Clip {
        id: source.id,
        cues: source.cues.clone(),
        sample_rate: source.sample_rate,
        waveform: source.waveform.clone(),
        title: source.title.clone(),
        artist: source.artist.clone(),
        music: source.music,
        samples,
    }))
}

pub(crate) struct TuningMeter {
    rate: AtomicU32,
    semitones: AtomicU32,
    key_lock: AtomicBool,
}
impl Default for TuningMeter {
    fn default() -> Self {
        Self {
            rate: AtomicU32::new(1_f32.to_bits()),
            semitones: AtomicU32::new(0),
            key_lock: AtomicBool::new(true),
        }
    }
}
impl TuningMeter {
    pub fn store(&self, t: Tuning) {
        self.rate.store(t.rate.to_bits(), Relaxed);
        self.semitones.store(t.semitones as i32 as u32, Relaxed);
        self.key_lock.store(t.key_lock, Relaxed);
    }
    pub fn read(&self) -> Tuning {
        Tuning {
            rate: f32::from_bits(self.rate.load(Relaxed)),
            semitones: self.semitones.load(Relaxed) as i8,
            key_lock: self.key_lock.load(Relaxed),
        }
    }
}
#[derive(Default)]
pub(crate) struct MusicMeter {
    bpm: AtomicU64,
    offset: AtomicU64,
    confidence: AtomicU32,
    key: AtomicU32,
    key_confidence: AtomicU32,
    manual: AtomicBool,
    downbeat: AtomicU64,
    first_sound: AtomicU64,
}
impl MusicMeter {
    pub fn store(&self, a: MusicAnalysis) {
        self.bpm.store(a.bpm.unwrap_or(0.).to_bits(), Relaxed);
        self.offset.store(a.beat_offset.to_bits(), Relaxed);
        self.confidence.store(a.bpm_confidence.to_bits(), Relaxed);
        self.key.store(
            a.key
                .map_or(0, |k| 1 + k.tonic as u32 + if k.minor { 12 } else { 0 }),
            Relaxed,
        );
        self.key_confidence
            .store(a.key_confidence.to_bits(), Relaxed);
        self.manual.store(a.manual, Relaxed);
        self.downbeat.store(a.downbeat.to_bits(), Relaxed);
        self.first_sound.store(a.first_sound.to_bits(), Relaxed);
    }
    pub fn read(&self) -> MusicAnalysis {
        let bpm = f64::from_bits(self.bpm.load(Relaxed));
        let key = self.key.load(Relaxed);
        MusicAnalysis {
            version: crate::music::ANALYSIS_VERSION,
            bpm: (bpm > 0.).then_some(bpm),
            beat_offset: f64::from_bits(self.offset.load(Relaxed)),
            bpm_confidence: f32::from_bits(self.confidence.load(Relaxed)),
            key: (key > 0).then(|| crate::music::MusicalKey {
                tonic: ((key - 1) % 12) as u8,
                minor: key > 12,
            }),
            key_confidence: f32::from_bits(self.key_confidence.load(Relaxed)),
            manual: self.manual.load(Relaxed),
            downbeat: f64::from_bits(self.downbeat.load(Relaxed)),
            first_sound: f64::from_bits(self.first_sound.load(Relaxed)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn octave_transpose_changes_frequency_without_changing_duration_or_stereo() {
        let source = Arc::new(Clip {
            id: 1,
            cues: Default::default(),
            sample_rate: 12000,
            samples: (0..24000)
                .map(|i| {
                    let x = (std::f32::consts::TAU * 220. * i as f32 / 12000.).sin() * 0.3;
                    [x, x * 0.5]
                })
                .collect(),
            waveform: vec![],
            title: None,
            artist: None,
            music: Default::default(),
        });
        let tuned = prepare(
            &source,
            Tuning {
                semitones: 12,
                ..Default::default()
            },
            || false,
        )
        .unwrap();
        assert_eq!(source.samples.len(), tuned.samples.len());
        let part = &tuned.samples[6000..18000];
        let crossings = part
            .windows(2)
            .filter(|p| p[0][0] <= 0. && p[1][0] > 0.)
            .count();
        assert!((crossings as i32 - 440).abs() < 3, "{crossings}");
        assert!(part.iter().all(|f| (f[0] * 0.5 - f[1]).abs() < 0.001));
        let locked = prepare(
            &source,
            Tuning {
                rate: 1.25,
                ..Default::default()
            },
            || false,
        )
        .unwrap();
        let mut crossings = 0;
        let mut last = locked.samples[6000][0];
        for frame in 1..12000 {
            let position = 6000. + frame as f64 * 1.25;
            let i = position as usize;
            let frac = position.fract() as f32;
            let value = locked.samples[i][0] * (1. - frac) + locked.samples[i + 1][0] * frac;
            if last <= 0. && value > 0. {
                crossings += 1;
            }
            last = value;
        }
        assert!(
            (crossings - 220_i32).abs() < 3,
            "key lock at 1.25x: {crossings}"
        );
        assert!(prepare(
            &source,
            Tuning {
                semitones: 1,
                ..Default::default()
            },
            || true
        )
        .is_err());
    }
}
