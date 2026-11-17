//! Known controllers. A preset is a JSON file in `crates/midi/presets`, written the way a MIDI
//! manual reads (channels from 1, numbers in hex), so that a capture from `bun run midi:probe`
//! turns into one by hand. See docs/MIDI.md for the format and how to add a controller.

use crate::input::Mode;
use crate::message::{Key, Kind};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetAction {
    pub slot: usize,
    pub kind: String,
    #[serde(default)]
    pub value: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Control {
    /// What it is called on the hardware.
    pub label: String,
    pub kind: Kind,
    /// 1–16, as manuals write it.
    pub channel: u8,
    #[serde(deserialize_with = "number")]
    pub number: u8,
    #[serde(default, deserialize_with = "optional_number")]
    pub lsb: Option<u8>,
    #[serde(default)]
    pub mode: Mode,
    /// `None`: the control exists and is known, but curado gives it nothing to do yet.
    #[serde(default)]
    pub action: Option<PresetAction>,
    /// For faders whose top sends 0 when curado wants the top to be the most.
    #[serde(default)]
    pub invert: bool,
    /// Relative controls: what one step is worth (seconds for a jog, rows for browse).
    #[serde(default)]
    pub scale: Option<f64>,
    /// Absolute controls: how far (of 0–1) a lone message must move to count. See `Gate`.
    #[serde(default)]
    pub deadband: Option<f64>,
    /// Tempo: ± around 1.
    #[serde(default)]
    pub range: Option<f64>,
    /// Seen on a real unit (true) or taken from someone else's map and still to check.
    #[serde(default)]
    pub verified: bool,
}
impl Control {
    pub fn key(&self) -> Key {
        Key {
            kind: self.kind,
            channel: self.channel.saturating_sub(1),
            number: self.number,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub vendor: String,
    /// Matched against the port name, ignoring case, as parts of it.
    pub ports: Vec<String>,
    /// How the map was checked: when, on what, with which tool.
    pub evidence: String,
    #[serde(default)]
    pub notes: Vec<String>,
    /// Sent once on connecting, like choosing the filter for a knob printed FILTER.
    #[serde(default)]
    pub init: Vec<PresetAction>,
    pub controls: Vec<Control>,
}
impl Preset {
    pub fn matches(&self, port: &str) -> bool {
        let port = port.to_lowercase();
        self.ports.iter().any(|p| port.contains(&p.to_lowercase()))
    }
    /// The controls curado acts on.
    pub fn mapped(&self) -> impl Iterator<Item = (&Control, &PresetAction)> {
        self.controls
            .iter()
            .filter_map(|c| c.action.as_ref().map(|a| (c, a)))
    }
    pub fn validate(&self) -> Result<(), String> {
        let mut keys = BTreeSet::new();
        for control in &self.controls {
            let at = || format!("{} · {}", self.id, control.label);
            if !(1..=16).contains(&control.channel) {
                return Err(format!("{}: channel must be 1–16", at()));
            }
            if control.lsb.is_some() && control.kind != Kind::Cc {
                return Err(format!("{}: only a CC has an LSB", at()));
            }
            if !keys.insert(control.key()) {
                return Err(format!(
                    "{}: {} is listed twice",
                    at(),
                    control.key().label()
                ));
            }
            if let Some(lsb) = control.lsb {
                let low = Key {
                    number: lsb,
                    ..control.key()
                };
                if !keys.insert(low) {
                    return Err(format!("{}: LSB {} is used twice", at(), low.label()));
                }
            }
        }
        Ok(())
    }
}

const BUNDLED: &[&str] = &[include_str!("../presets/pioneer-ddj-sb.json")];

/// Every preset shipped with curado. A file that does not read is a bug the tests catch.
pub fn bundled() -> Vec<Preset> {
    BUNDLED
        .iter()
        .filter_map(|text| serde_json::from_str(text).ok())
        .collect()
}

pub fn for_port(port: &str) -> Option<Preset> {
    bundled().into_iter().find(|p| p.matches(port))
}

pub fn by_id(id: &str) -> Option<Preset> {
    bundled().into_iter().find(|p| p.id == id)
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Number {
    Int(u8),
    Text(String),
}
impl Number {
    fn read<E: serde::de::Error>(self) -> Result<u8, E> {
        let n = match self {
            Number::Int(n) => n,
            Number::Text(text) => {
                let text = text.trim();
                match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
                    Some(hex) => u8::from_str_radix(hex, 16),
                    None => text.parse(),
                }
                .map_err(|_| E::custom(format!("not a MIDI number: {text}")))?
            }
        };
        if n > 0x7f {
            return Err(E::custom(format!("MIDI numbers end at 0x7F: {n}")));
        }
        Ok(n)
    }
}
fn number<'de, D: Deserializer<'de>>(d: D) -> Result<u8, D::Error> {
    Number::deserialize(d)?.read()
}
fn optional_number<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u8>, D::Error> {
    Option::<Number>::deserialize(d)?
        .map(Number::read)
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Encoding;

    #[test]
    fn every_bundled_preset_reads_and_holds_together() {
        assert_eq!(bundled().len(), BUNDLED.len());
        for preset in bundled() {
            preset.validate().unwrap();
            assert!(!preset.ports.is_empty(), "{}", preset.id);
            for (control, _) in preset.mapped() {
                if let Mode::Relative { .. } = control.mode {
                    assert!(control.scale.is_some(), "{}", control.label);
                }
            }
        }
    }

    #[test]
    fn the_ddj_sb_is_found_by_its_port() {
        let preset = for_port("DDJ-SB MIDI").unwrap();
        assert_eq!(preset.id, "pioneer-ddj-sb");
        assert!(for_port("Some Keyboard").is_none());
        let fader = preset
            .controls
            .iter()
            .find(|c| c.label == "Deck 1 channel fader")
            .unwrap();
        assert_eq!(
            (fader.channel, fader.number, fader.lsb),
            (1, 0x13, Some(0x33))
        );
        let browse = preset
            .controls
            .iter()
            .find(|c| c.label == "Browse")
            .unwrap();
        assert_eq!(
            browse.mode,
            Mode::Relative {
                encoding: Encoding::TwosComplement
            }
        );
    }

    #[test]
    fn numbers_read_in_hex_or_decimal() {
        let control: Control = serde_json::from_str(
            r#"{"label":"x","kind":"cc","channel":7,"number":"0x1F","lsb":63}"#,
        )
        .unwrap();
        assert_eq!((control.number, control.lsb), (0x1f, Some(63)));
        assert_eq!(control.key().channel, 6);
        assert!(serde_json::from_str::<Control>(
            r#"{"label":"x","kind":"cc","channel":7,"number":"0x80"}"#
        )
        .is_err());
    }
}
