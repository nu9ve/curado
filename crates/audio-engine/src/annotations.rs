use std::sync::atomic::{
    AtomicU64,
    Ordering::{Acquire, Relaxed, Release},
};
/// Shared by decks loading the same track. Updates never allocate or lock.
#[derive(Debug)]
pub struct CueState {
    values: [AtomicU64; 3],
    revision: AtomicU64,
}
impl Default for CueState {
    fn default() -> Self {
        Self::new([0.; 3])
    }
}
impl CueState {
    pub fn new(values: [f64; 3]) -> Self {
        Self {
            values: values
                .map(|v| AtomicU64::new(if v.is_finite() && v >= 0. { v } else { 0. }.to_bits())),
            revision: AtomicU64::new(0),
        }
    }
    pub fn get(&self, index: usize) -> f64 {
        self.values
            .get(index)
            .map_or(0., |v| f64::from_bits(v.load(Acquire)))
    }
    pub fn set(&self, index: usize, seconds: f64) {
        if let Some(value) = self.values.get(index) {
            if seconds.is_finite() && seconds >= 0. {
                value.store(seconds.to_bits(), Release);
                self.revision.fetch_add(1, Release);
            }
        }
    }
    pub fn revision(&self) -> u64 {
        self.revision.load(Relaxed)
    }
    pub fn snapshot(&self) -> (u64, [f64; 3]) {
        let mut result = (0, [0.; 3]);
        for _ in 0..3 {
            let revision = self.revision.load(Acquire);
            let values = std::array::from_fn(|i| self.get(i));
            result = (revision, values);
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
    #[test]
    fn validates_and_tracks_changes() {
        let c = CueState::new([f64::NAN, -1., 2.]);
        assert_eq!(c.snapshot().1, [0., 0., 2.]);
        c.set(1, 3.25);
        assert_eq!(c.snapshot(), (1, [0., 3.25, 2.]));
        c.set(7, 5.);
        c.set(0, f64::INFINITY);
        assert_eq!(c.revision(), 1);
    }
}
