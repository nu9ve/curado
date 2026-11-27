//! Loudness measurements that must give the same number in every app of the suite.
//!
//! - [`lufs`]: ITU-R BS.1770 K-weighting with its gating. Integrated loudness, the maximum
//!   momentary (400 ms) and short-term (3 s) loudness, and the EBU Tech 3342 loudness range.
//!   At 48 kHz the filters use the coefficients printed in the standard; at any other rate
//!   they are derived from the analogue prototype as libebur128 does, which reproduces the
//!   printed ones at 48 kHz.
//! - [`true_peak`]: a 4x oversampling true-peak detector with a 12-tap Hann-windowed sinc.
//!   It works sample by sample on fixed arrays, so it can run inside a real-time callback.
//!
//! Vectors: `data/audio/loudness-vectors-v1.json`, checked by `tests/vectors.rs`.

pub mod lufs;
pub mod true_peak;

pub use lufs::{Meter, Report, UnsupportedInput, measure};
pub use true_peak::{TruePeakDetector, to_dbtp, true_peak};
