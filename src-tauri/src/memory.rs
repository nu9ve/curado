//! A single limit on decoded audio in memory, whatever mix of tracks holds it.
//!
//! Tracks stay fully in RAM by design: during a set nothing depends on a disk or a USB stick, so
//! a slow or unplugged drive cannot cause a dropout. The limit is what keeps that bounded.
use curado_audio::decode::{Clip, CANCELLED, TOO_LARGE};
use std::sync::{Arc, Mutex, Weak};

/// Two hour-long tracks plus samples, on a limit a laptop can afford. Tracks of ordinary length
/// use a small fraction of it.
const TOTAL_BYTES: usize = 3 * 1024 * 1024 * 1024;

#[derive(Default)]
pub struct PcmBudget {
    clips: Mutex<Vec<(Weak<Clip>, usize)>>,
}
impl PcmBudget {
    pub fn track(&self, clip: &Arc<Clip>) {
        let bytes = clip.samples.capacity() * std::mem::size_of::<[f32; 2]>();
        self.clips
            .lock()
            .unwrap()
            .push((Arc::downgrade(clip), bytes));
    }
    /// Counts only clips still alive, including ones waiting for reclamation.
    pub fn available(&self) -> usize {
        let mut clips = self.clips.lock().unwrap();
        clips.retain(|(clip, _)| clip.strong_count() > 0);
        TOTAL_BYTES.saturating_sub(clips.iter().map(|(_, bytes)| bytes).sum())
    }
}
/// Turns the decoder's limit into something a person can act on.
pub fn explain(error: String, room: usize) -> String {
    if error == TOO_LARGE {
        let gib = |bytes: usize| bytes as f64 / (1024. * 1024. * 1024.);
        format!(
            "Esta pista no cabe: quedan {:.1} GiB de {:.0} GiB para audio y cada minuto ocupa unos 21 MiB. Carga una pista más corta en el deck que tenga la pista larga, o usa pistas de hasta unos 90 minutos.",
            gib(room),
            gib(TOTAL_BYTES)
        )
    } else {
        debug_assert!(error != CANCELLED || room <= TOTAL_BYTES);
        error
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn clip(frames: usize) -> Arc<Clip> {
        Arc::new(Clip {
            loudness: None,
            beats: None,
            id: 1,
            cues: Default::default(),
            samples: vec![[0.; 2]; frames],
            sample_rate: 48000,
            waveform: vec![],
            title: None,
            artist: None,
            music: Default::default(),
        })
    }
    #[test]
    fn counts_live_clips_only_and_frees_room_when_they_go() {
        let budget = PcmBudget::default();
        assert_eq!(budget.available(), TOTAL_BYTES);
        let first = clip(1000);
        budget.track(&first);
        let second = clip(500);
        budget.track(&second);
        assert_eq!(budget.available(), TOTAL_BYTES - 1500 * 8);
        let shared = first.clone();
        drop(first);
        assert_eq!(
            budget.available(),
            TOTAL_BYTES - 1500 * 8,
            "still referenced"
        );
        drop(shared);
        assert_eq!(budget.available(), TOTAL_BYTES - 500 * 8);
        drop(second);
        assert_eq!(budget.available(), TOTAL_BYTES);
    }
    #[test]
    fn explains_only_the_memory_limit() {
        assert!(explain(TOO_LARGE.into(), 1 << 30).contains("1.0 GiB"));
        assert_eq!(explain("otro error".into(), 0), "otro error");
    }
}
