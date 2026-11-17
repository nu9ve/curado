//! What a MIDI controller sends and how curado reads it, independent of Tauri and the engine.
//!
//! - [`message`]: three bytes into a note, a CC or a pitch bend.
//! - [`detect`]: a gesture (one control moved by hand) into what kind of control it is.
//! - [`input`]: reading bound controls while playing: relative encoders, the noise gate.
//! - [`preset`]: known controllers, bundled as JSON.
//!
//! The exploration flow these pieces support is described in `docs/MIDI.md`.

pub mod detect;
pub mod input;
pub mod message;
pub mod preset;

pub use detect::{classify, Detected, Explorer, Seen};
pub use input::{Encoding, Gate, Mode};
pub use message::{Key, Kind, Message};
pub use preset::{Control, Preset, PresetAction};
