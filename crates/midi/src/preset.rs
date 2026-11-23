//! curado's action mappings for shared hardware profiles. See docs/MIDI.md.

use nu9ve_midi::profile::{Control, Profile, PIONEER_DDJ_SB_V1};
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresetAction {
    pub slot: usize,
    pub kind: String,
    #[serde(default)]
    pub value: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mapping {
    control: String,
    pub action: PresetAction,
    #[serde(default)]
    pub invert: bool,
    /// Seconds per jog step, rows per browse step, or normalized travel per step.
    pub scale: Option<f64>,
    /// Override the hardware profile's normalized deadband.
    pub deadband: Option<f64>,
    /// Tempo: ± around 1.
    pub range: Option<f64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MappingFile {
    profile: String,
    #[serde(default)]
    notes: Vec<String>,
    #[serde(default)]
    init: Vec<PresetAction>,
    mappings: Vec<Mapping>,
}

#[derive(Clone, Debug)]
pub struct Preset {
    pub profile: Profile,
    pub notes: Vec<String>,
    /// Applied when the audio output and session mode allow each action.
    pub init: Vec<PresetAction>,
    mappings: Vec<Mapping>,
}

impl Preset {
    fn from_json(profile: &str, mappings: &str) -> Result<Self, String> {
        let profile = Profile::from_json(profile)?;
        let file: MappingFile = serde_json::from_str(mappings).map_err(|e| e.to_string())?;
        if file.profile != profile.id {
            return Err(format!("{} mappings refer to {}", profile.id, file.profile));
        }
        let mut ids = BTreeSet::new();
        for mapping in &file.mappings {
            if !ids.insert(&mapping.control) || profile.control(&mapping.control).is_none() {
                return Err(format!(
                    "{}: unknown or duplicate control {}",
                    profile.id, mapping.control
                ));
            }
        }
        let mut notes = profile.notes.clone();
        notes.extend(file.notes);
        Ok(Self {
            profile,
            notes,
            init: file.init,
            mappings: file.mappings,
        })
    }

    /// The physical controls curado acts on, in mapping order.
    pub fn mapped(&self) -> impl Iterator<Item = (&Control, &Mapping)> {
        self.mappings.iter().map(|mapping| {
            let control = self
                .profile
                .control(&mapping.control)
                .expect("validated mapping");
            (control, mapping)
        })
    }
}

const BUNDLED: &[(&str, &str)] = &[(
    PIONEER_DDJ_SB_V1,
    include_str!("../presets/pioneer-ddj-sb.json"),
)];

/// Every preset shipped with curado. A file that does not read is a bug the tests catch.
pub fn bundled() -> Vec<Preset> {
    BUNDLED
        .iter()
        .filter_map(|(profile, mappings)| Preset::from_json(profile, mappings).ok())
        .collect()
}

pub fn for_port(port: &str) -> Option<Preset> {
    bundled().into_iter().find(|p| p.profile.matches(port))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Mode;

    #[test]
    fn every_bundled_preset_reads_and_holds_together() {
        assert_eq!(bundled().len(), BUNDLED.len());
        for preset in bundled() {
            for (control, mapping) in preset.mapped() {
                if let Mode::Relative { .. } = control.mode {
                    assert!(mapping.scale.is_some(), "{}", control.label);
                }
            }
        }
    }

    #[test]
    fn the_ddj_sb_is_found_by_its_port() {
        let preset = for_port("DDJ-SB MIDI").unwrap();
        assert_eq!(preset.profile.id, "pioneer-ddj-sb");
        assert_eq!(preset.profile.controls.len(), 125);
        assert_eq!(preset.mapped().count(), 95);
        assert!(for_port("Some Keyboard").is_none());
    }
}
