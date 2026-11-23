//! Versioned physical controller profiles. Profile channels are one-based, as in MIDI manuals.

use crate::{Key, Kind, Mode};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeSet;

pub const PIONEER_DDJ_SB_V1: &str = include_str!("../../../data/midi/pioneer-ddj-sb-v1.json");

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Control {
    /// Stable even if the label or wire address is corrected.
    pub id: String,
    pub label: String,
    pub kind: Kind,
    /// 1–16, as manuals write it.
    pub channel: u8,
    #[serde(deserialize_with = "number")]
    pub number: u8,
    #[serde(
        default,
        deserialize_with = "optional_number",
        skip_serializing_if = "Option::is_none"
    )]
    pub lsb: Option<u8>,
    pub mode: Mode,
    /// Minimum drift from the last accepted value, as a fraction of normalized travel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadband: Option<f64>,
    /// This address was observed on hardware. Its gesture can still be unidentified.
    #[serde(default)]
    pub verified: bool,
    #[serde(default = "identified_by_default")]
    pub identified: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
    /// ID of the primary control when this address repeats its output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror_of: Option<String>,
}

fn identified_by_default() -> bool {
    true
}

impl Control {
    /// Convert to the runtime key, which uses zero-based channels.
    pub fn key(&self) -> Key {
        Key {
            kind: self.kind,
            channel: self.channel.saturating_sub(1),
            number: self.number,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Profile {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub vendor: String,
    /// Substrings matched against a native port name, ignoring case.
    pub ports: Vec<String>,
    pub evidence: String,
    #[serde(default)]
    pub notes: Vec<String>,
    pub controls: Vec<Control>,
}

impl Profile {
    pub fn from_json(text: &str) -> Result<Self, String> {
        let profile: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        profile.validate()?;
        Ok(profile)
    }

    pub fn matches(&self, port: &str) -> bool {
        let port = port.to_lowercase();
        self.ports
            .iter()
            .any(|alias| port.contains(&alias.to_lowercase()))
    }

    pub fn control(&self, id: &str) -> Option<&Control> {
        self.controls.iter().find(|c| c.id == id)
    }

    /// Find a control by its primary address.
    pub fn control_by_key(&self, key: Key) -> Option<&Control> {
        self.controls.iter().find(|c| c.key() == key)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err(format!(
                "unsupported MIDI profile version {}",
                self.schema_version
            ));
        }
        if !valid_id(&self.id)
            || self.ports.is_empty()
            || self.ports.iter().any(|p| p.trim().is_empty())
        {
            return Err(format!(
                "{}: expected a profile ID and nonempty port aliases",
                self.id
            ));
        }
        let mut ids = BTreeSet::new();
        let mut keys = BTreeSet::new();
        for control in &self.controls {
            let error = |reason| format!("{} · {}: {reason}", self.id, control.id);
            if !valid_id(&control.id) || !ids.insert(&control.id) {
                return Err(error("invalid or duplicate control ID"));
            }
            if !(1..=16).contains(&control.channel) || control.number > 0x7f {
                return Err(error("invalid MIDI address"));
            }
            if control.kind == Kind::PitchBend && control.number != 0 {
                return Err(error("pitch bend must use number 0"));
            }
            if !keys.insert(control.key()) {
                return Err(error("duplicate primary address"));
            }
            if let Some(lsb) = control.lsb {
                if control.kind != Kind::Cc || lsb > 0x7f {
                    return Err(error("LSB must be a CC numbered 0–127"));
                }
                if !keys.insert(Key {
                    number: lsb,
                    ..control.key()
                }) {
                    return Err(error("duplicate LSB address"));
                }
            }
            if let Some(deadband) = control.deadband
                && (control.mode != Mode::Absolute || !(0.0..=1.0).contains(&deadband))
            {
                return Err(error(
                    "deadband must be between 0 and 1 for an absolute control",
                ));
            }
            if let Some(id) = &control.mirror_of {
                let primary = self
                    .control(id)
                    .filter(|c| c.id != control.id && c.mirror_of.is_none());
                if primary.is_none() {
                    return Err(error("mirror must refer to another primary control"));
                }
            }
        }
        Ok(())
    }
}

fn valid_id(id: &str) -> bool {
    id.split('-').all(|part| {
        !part.is_empty()
            && part
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
    })
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
