//! MIDI channel messages and physical controller input, independent of the native port backend.
//!
//! Feed one channel message at a time to [`Message::parse`]. A live learner can feed
//! timestamped messages to [`Explorer`]; a capture can use [`parse_log`] and [`replay`].
//! [`Profile`] describes hardware addresses and encodings. Applications own port access,
//! binding policy and any actions triggered by the decoded values.

pub mod detect;
pub mod input;
pub mod message;
pub mod profile;

pub use detect::{Detected, Event, Explorer, Seen, Want, classify, parse_log, replay};
pub use input::{Encoding, Gate, Mode, combine_cc14, normalize};
pub use message::{Key, Kind, Message};
pub use profile::{Control, Profile};
