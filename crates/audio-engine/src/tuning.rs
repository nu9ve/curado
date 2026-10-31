//! Pitch settings and the lock-free meters that report them.
use crate::music::MusicAnalysis;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering::Relaxed};

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
    /// Output pitch relative to the file's. With key lock off, speed changes pitch as on vinyl.
    pub fn pitch_ratio(self) -> f32 {
        2_f32.powf(self.semitones as f32 / 12.) * if self.key_lock { 1. } else { self.rate }
    }
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
