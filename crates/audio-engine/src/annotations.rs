use std::sync::atomic::{
    AtomicU64,
    Ordering::{Acquire, Relaxed, Release},
};
/// Hot cues A to H, the same as rekordbox and CDJs.
pub const HOT_CUES: usize = 8;
/// Shared by decks loading the same track. Updates never allocate or lock.
#[derive(Debug)]
pub struct CueState {
    values: [AtomicU64; HOT_CUES],
    /// Where a cue's loop ends, or 0 for a plain cue.
    ends: [AtomicU64; HOT_CUES],
    revision: AtomicU64,
}
impl Default for CueState {
    fn default() -> Self {
        Self::new([0.; HOT_CUES], [0.; HOT_CUES])
    }
}
fn stored(seconds: f64) -> u64 {
    if seconds.is_finite() && seconds >= 0. {
        seconds
    } else {
        0.
    }
    .to_bits()
}
impl CueState {
    pub fn new(values: [f64; HOT_CUES], ends: [f64; HOT_CUES]) -> Self {
        Self {
            values: values.map(|v| AtomicU64::new(stored(v))),
            ends: ends.map(|v| AtomicU64::new(stored(v))),
            revision: AtomicU64::new(0),
        }
    }
    pub fn get(&self, index: usize) -> f64 {
        self.values
            .get(index)
            .map_or(0., |v| f64::from_bits(v.load(Acquire)))
    }
    pub fn end(&self, index: usize) -> f64 {
        self.ends
            .get(index)
            .map_or(0., |v| f64::from_bits(v.load(Acquire)))
    }
    /// Saving a position makes it a plain cue, even where a loop used to be.
    pub fn set(&self, index: usize, seconds: f64) {
        if let Some(value) = self.values.get(index) {
            if seconds.is_finite() && seconds >= 0. {
                value.store(seconds.to_bits(), Release);
                self.ends[index].store(0_f64.to_bits(), Release);
                self.revision.fetch_add(1, Release);
            }
        }
    }
    /// Replaces every cue at once, for a track re-imported while it is known to this session.
    pub fn replace(&self, values: [f64; HOT_CUES], ends: [f64; HOT_CUES]) {
        for index in 0..HOT_CUES {
            self.values[index].store(stored(values[index]), Release);
            self.ends[index].store(stored(ends[index]), Release);
        }
        self.revision.fetch_add(1, Release);
    }
    pub fn revision(&self) -> u64 {
        self.revision.load(Relaxed)
    }
    pub fn snapshot(&self) -> (u64, [f64; HOT_CUES], [f64; HOT_CUES]) {
        let mut result = (0, [0.; HOT_CUES], [0.; HOT_CUES]);
        for _ in 0..3 {
            let revision = self.revision.load(Acquire);
            result = (
                revision,
                std::array::from_fn(|i| self.get(i)),
                std::array::from_fn(|i| self.end(i)),
            );
            if revision == self.revision.load(Acquire) {
                break;
            }
        }
        result
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn cues(values: &[f64]) -> [f64; HOT_CUES] {
        std::array::from_fn(|i| values.get(i).copied().unwrap_or(0.))
    }
    #[test]
    fn validates_and_tracks_changes() {
        let c = CueState::new(cues(&[f64::NAN, -1., 2.]), cues(&[]));
        assert_eq!(c.snapshot().1, cues(&[0., 0., 2.]));
        c.set(1, 3.25);
        assert_eq!(c.snapshot().0, 1);
        assert_eq!(c.snapshot().1, cues(&[0., 3.25, 2.]));
        c.set(HOT_CUES, 5.);
        c.set(0, f64::INFINITY);
        assert_eq!(c.revision(), 1);
    }
    #[test]
    fn saving_over_a_loop_makes_a_plain_cue() {
        let c = CueState::new(cues(&[1.]), cues(&[3.]));
        assert_eq!(c.end(0), 3.);
        c.set(0, 2.);
        assert_eq!(c.end(0), 0.);
        c.replace(cues(&[4.]), cues(&[6.]));
        assert_eq!((c.get(0), c.end(0)), (4., 6.));
        assert_eq!(c.revision(), 2);
    }
}
