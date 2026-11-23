//! Selected CoreMIDI messages from the DDJ-SB on 2026-11-22. The fixture retains the
//! original timestamps and values for fader, jog, pad, FX and browse checks.

use curado_midi::detect::{classify, parse_log, replay};
use curado_midi::input::{Encoding, Mode};
use curado_midi::message::{Key, Kind};
use curado_midi::preset::for_port;
use std::collections::BTreeSet;

const CAPTURE: &str =
    include_str!("../../../vendor/nu9ve-core/rust/nu9ve-midi/tests/fixtures/ddj-sb-2026-11-22.txt");

fn key(kind: Kind, channel: u8, number: u8) -> Key {
    Key {
        kind,
        channel,
        number,
    }
}

#[test]
fn coremidi_capture_covers_fader_jogs_and_buttons() {
    let events = parse_log(CAPTURE);
    let find = |kind, channel, number, lsb: Option<u8>| {
        let control = key(kind, channel, number);
        let selected: Vec<_> = events
            .iter()
            .filter(|e| {
                e.message.key == control
                    || lsb.is_some_and(|lsb| e.message.key == key(Kind::Cc, channel, lsb))
            })
            .copied()
            .collect();
        classify(&selected, &BTreeSet::new(), None)
            .unwrap_or_else(|| panic!("{} not found", control.label()))
    };

    let fader = find(Kind::Cc, 0, 0x13, Some(0x33));
    assert_eq!(
        (fader.mode, fader.lsb, fader.bits),
        (Mode::Absolute, Some(0x33), 14)
    );
    for deck in 0..2 {
        for encoder in [0x21, 0x23] {
            assert_eq!(
                find(Kind::Cc, deck, encoder, None).mode,
                Mode::Relative {
                    encoding: Encoding::Offset64
                }
            );
        }
        assert_eq!(find(Kind::Note, deck, 0x35, None).mode, Mode::Button);
    }
    assert_eq!(find(Kind::Note, 1, 0x0b, None).mode, Mode::Button);
    assert_eq!(find(Kind::Note, 7, 0x02, None).mode, Mode::Button);
}

#[test]
fn preset_agrees_with_coremidi_gestures_and_browse_press() {
    let preset = for_port("PIONEER DDJ-SB").unwrap();
    let events = parse_log(CAPTURE);
    let found = replay(&events, 10_000, 400);
    for (_, d) in &found {
        let control = preset
            .profile
            .control_by_key(d.key)
            .unwrap_or_else(|| panic!("{} is missing from the preset", d.summary()));
        assert_eq!(control.mode, d.mode, "{}", control.label);
        assert_eq!(control.lsb, d.lsb, "{}", control.label);
        assert!(control.verified, "{} was captured", control.label);
    }

    let find = |kind, channel, number| {
        let control = key(kind, channel, number);
        found
            .iter()
            .map(|(_, d)| d)
            .find(|d| d.key == control)
            .unwrap_or_else(|| panic!("{} not found", control.label()))
    };
    assert_eq!(find(Kind::Note, 1, 0x0b).mode, Mode::Button);
    assert_eq!(find(Kind::Note, 6, 0x41).mode, Mode::Button);
    assert_eq!(find(Kind::Note, 6, 0x46).mode, Mode::Button);
    assert_eq!(
        find(Kind::Cc, 6, 0x40).mode,
        Mode::Relative {
            encoding: Encoding::TwosComplement
        }
    );
    let fx = find(Kind::Cc, 5, 0x02);
    assert_eq!(fx.lsb, Some(0x22));
    assert_eq!(
        fx.mirrors,
        vec![key(Kind::Cc, 5, 0x04), key(Kind::Cc, 5, 0x06)]
    );
}
