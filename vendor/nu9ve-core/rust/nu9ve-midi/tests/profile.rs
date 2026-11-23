use nu9ve_midi::profile::{PIONEER_DDJ_SB_V1, Profile};
use nu9ve_midi::{Encoding, Key, Kind, Mode};

#[test]
fn the_ddj_sb_profile_keeps_the_address_inventory_and_evidence() {
    let profile = Profile::from_json(PIONEER_DDJ_SB_V1).unwrap();
    assert_eq!(profile.id, "pioneer-ddj-sb");
    assert_eq!(profile.controls.len(), 125);
    assert_eq!(profile.controls.iter().filter(|c| c.verified).count(), 89);
    assert!(profile.matches("PIONEER DDJ-SB MIDI"));
    assert!(profile.matches("pioneer ddj-sb"));
    assert!(!profile.matches("Some Keyboard"));

    for (id, number) in [
        ("browse-press", 0x41),
        ("center-unidentified-note-65", 0x65),
    ] {
        let control = profile.control(id).unwrap();
        assert_eq!(
            control.key(),
            Key {
                kind: Kind::Note,
                channel: 6,
                number
            }
        );
        assert!(control.verified);
        assert_eq!(control.identified, id == "browse-press");
    }
    assert_eq!(
        profile.control("browse-turn").unwrap().mode,
        Mode::Relative {
            encoding: Encoding::TwosComplement
        }
    );
    for deck in ["deck-1", "deck-2"] {
        let tempo = profile.control(&format!("{deck}-tempo")).unwrap();
        assert_eq!(
            (tempo.number, tempo.lsb, tempo.deadband),
            (0x00, Some(0x20), Some(0.008))
        );
    }
    for unit in ["fx1", "fx2"] {
        for suffix in ["copy", "second-copy"] {
            let id = format!("{unit}-level-{suffix}");
            let primary = format!("{unit}-level");
            assert_eq!(
                profile.control(&id).unwrap().mirror_of.as_deref(),
                Some(primary.as_str())
            );
        }
    }
}

#[test]
fn rejects_duplicate_ids_and_overlapping_lsb_addresses() {
    let mut profile = Profile::from_json(PIONEER_DDJ_SB_V1).unwrap();
    profile.controls[1].id = profile.controls[0].id.clone();
    assert!(
        profile
            .validate()
            .unwrap_err()
            .contains("duplicate control ID")
    );

    let mut profile = Profile::from_json(PIONEER_DDJ_SB_V1).unwrap();
    let mut also_uses_lsb = profile.control("deck-1-tempo").unwrap().clone();
    also_uses_lsb.id = "duplicate-tempo-lsb".into();
    also_uses_lsb.number = also_uses_lsb.lsb.take().unwrap();
    profile.controls.push(also_uses_lsb);
    assert!(
        profile
            .validate()
            .unwrap_err()
            .contains("duplicate primary address")
    );
}

#[test]
fn validates_schema_and_accepts_decimal_or_hex_midi_numbers() {
    let text = PIONEER_DDJ_SB_V1.replace("\"schemaVersion\": 1", "\"schemaVersion\": 2");
    assert!(
        Profile::from_json(&text)
            .unwrap_err()
            .contains("unsupported MIDI profile version")
    );
    let text = PIONEER_DDJ_SB_V1.replace("\"0x33\"", "51");
    assert_eq!(
        Profile::from_json(&text)
            .unwrap()
            .control("deck-1-channel-fader")
            .unwrap()
            .lsb,
        Some(0x33)
    );
    let text = PIONEER_DDJ_SB_V1.replace("\"0x33\"", "\"0x80\"");
    assert!(Profile::from_json(&text).is_err());
}
