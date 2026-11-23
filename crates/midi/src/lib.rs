//! curado's controller presets and shared MIDI reader, independent of Tauri and the engine.

pub mod pickup;
pub mod preset;

pub use nu9ve_midi::{
    detect, input, message, Detected, Encoding, Explorer, Gate, Key, Kind, Message, Mode, Seen,
};
