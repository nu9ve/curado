//! A real DDJ-SB, captured on Windows on 2026-11-17 (`tests/fixtures`, windows of the full
//! capture): what exploring it finds, and that the bundled preset agrees with it.

use curado_midi::detect::{classify, parse_log, replay};
use curado_midi::input::{normalize, Encoding, Gate, Mode};
use curado_midi::message::{Key, Kind};
use curado_midi::preset::for_port;
use std::collections::BTreeSet;

const CAPTURE: &str = include_str!("fixtures/ddj-sb-2026-11-17.log");

fn key(kind: Kind, channel: u8, number: u8) -> Key {
    Key {
        kind,
        channel,
        number,
    }
}

#[test]
fn exploring_the_capture_finds_each_kind_of_control() {
    let events = parse_log(CAPTURE);
    assert!(events.len() > 3000);
    let found = replay(&events, 3000, 400);
    let find = |k: Key| {
        found
            .iter()
            .map(|(_, d)| d)
            .find(|d| d.key == k)
            .unwrap_or_else(|| panic!("{} not found", k.label()))
    };
    // The untouched tempo fader of deck 2 is noise and never wins a gesture.
    assert!(found.iter().all(|(_, d)| d.key != key(Kind::Cc, 1, 0x00)));

    let fader = find(key(Kind::Cc, 0, 0x13));
    assert_eq!(
        (fader.mode, fader.lsb, fader.bits),
        (Mode::Absolute, Some(0x33), 14)
    );
    let sync = find(key(Kind::Note, 0, 0x58));
    assert_eq!(sync.mode, Mode::Button);
    assert_eq!(
        find(key(Kind::Cc, 0, 0x23)).mode,
        Mode::Relative {
            encoding: Encoding::Offset64
        }
    );
    // The side of the jog was turned while the top still moved, so it never had a gesture of
    // its own; read by itself it is the same kind of encoder.
    let side: Vec<_> = events
        .iter()
        .filter(|e| e.message.key == key(Kind::Cc, 0, 0x21))
        .copied()
        .collect();
    assert!(side.len() > 20);
    let side = classify(&side, &BTreeSet::new(), None).unwrap();
    assert_eq!(
        side.mode,
        Mode::Relative {
            encoding: Encoding::Offset64
        }
    );
    // Pressed one after another, each pad is its own gesture.
    assert_eq!(find(key(Kind::Note, 7, 0x01)).mode, Mode::Button);
    assert_eq!(
        find(key(Kind::Cc, 6, 0x40)).mode,
        Mode::Relative {
            encoding: Encoding::TwosComplement
        }
    );
    assert_eq!(find(key(Kind::Note, 6, 0x46)).mode, Mode::Button);
    let fx = find(key(Kind::Cc, 5, 0x02));
    assert_eq!(fx.lsb, Some(0x22));
    assert_eq!(
        fx.mirrors,
        vec![key(Kind::Cc, 5, 0x04), key(Kind::Cc, 5, 0x06)]
    );
    // Pads, with and without SHIFT.
    assert_eq!(find(key(Kind::Note, 7, 0x00)).mode, Mode::Button);
    assert_eq!(find(key(Kind::Note, 7, 0x11)).mode, Mode::Button);
}

#[test]
fn the_preset_agrees_with_the_capture() {
    let preset = for_port("DDJ-SB MIDI").unwrap();
    let events = parse_log(CAPTURE);
    for (_, d) in replay(&events, 3000, 400) {
        let Some(control) = preset.controls.iter().find(|c| c.key() == d.key) else {
            panic!("{} is missing from the preset", d.summary());
        };
        assert_eq!(control.mode, d.mode, "{}", control.label);
        assert_eq!(control.lsb, d.lsb, "{}", control.label);
        assert!(control.verified, "{} was captured", control.label);
    }
}

#[test]
fn the_gate_keeps_an_untouched_tempo_fader_still() {
    // Deck 2's tempo fader before anyone touched it, and deck 1's while moved by hand.
    let events = parse_log(CAPTURE);
    let run = |channel: u8, from: u64, to: u64| {
        let mut gate = Gate::default();
        let mut msb = 0u16;
        let (mut sent, mut passed) = (0, 0);
        for e in events
            .iter()
            .filter(|e| e.at_ms >= from && e.at_ms < to && e.message.key.channel == channel)
        {
            match (e.message.key.kind, e.message.key.number) {
                (Kind::Cc, 0x00) => msb = e.message.value,
                (Kind::Cc, 0x20) => {
                    sent += 1;
                    let x = normalize(msb << 7 | e.message.value, 14);
                    if gate.pass(x, e.at_ms, 0.008) {
                        passed += 1;
                    }
                }
                _ => {}
            }
        }
        (sent, passed)
    };
    let (sent, passed) = run(1, 0, 60_000);
    assert!(sent > 30, "{sent}");
    assert!(passed * 5 < sent, "noise got through: {passed} of {sent}");
}
