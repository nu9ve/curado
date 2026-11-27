//! Offline musical analysis over a whole track: a constant beat grid (BPM, offset, downbeat and how
//! clearly it holds), an optional tempo map for music that drifts, the first audible sound, and the key.
//! Bounded, deterministic and never for the audio callback. Origin and known limits: README.md.

pub mod beat;
pub mod music;

pub use beat::Grid;
pub use music::{
    ANALYSIS_VERSION, Evaluation, MusicAnalysis, MusicalKey, analyze, evaluate, follow_tempo,
};
