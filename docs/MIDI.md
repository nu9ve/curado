# MIDI controllers

How curado finds out what a controller is, maps it, and how a new controller becomes a preset anyone can use. The person's side of it is in [Controls](CONTROLS.md#midi); this page is the method and the contracts. Code: `crates/midi` (messages, detection, presets, no Tauri) and `src-tauri/src/midi.rs` (ports, bindings, engine actions).

## What curado reads

| Message                                     | Read as                                                                                       |
| ------------------------------------------- | --------------------------------------------------------------------------------------------- |
| Note On / Note Off (Note On 0 is a release) | A button: pressed and released                                                                |
| CC 0–127                                    | A button (two states, always a 0 on release), an absolute fader/knob, or a relative encoder   |
| CC _n_ + CC _n_ + 0x20                      | One 14-bit fader or knob: _n_ carries the high 7 bits, _n_ + 0x20 the low 7, sent right after |
| Pitch bend                                  | One 14-bit absolute control (keyboard wheels, some faders)                                    |
| Relative encodings                          | `offset64` (64 still, 65 +1, 63 −1), `twosComplement` (1 +1, 127 −1), `signBit` (1 +1, 65 −1) |

Not read yet: SysEx, MIDI Clock, aftertouch, NRPN and program changes. curado sends nothing back, so controller LEDs stay as the controller leaves them.

## The flow in the app

**Settings → MIDI.**

1. **Choose the port.** A controller curado knows (by the name of its port) is recognised, mapped by its preset if it has no mappings yet, and remembered. From then on curado connects it at launch and whenever it is plugged in again; pulling it out says so and waits.
2. **Explore an unknown one.** _Medir ruido_ listens for 3 s while nothing is touched: controls that send by themselves (a worn fader) are noted and never mistaken for the one you move. Then move one control at a time, letting go before the next. Each gesture shows what it was (`CH 1 · CC 0x13 · fader/knob 14 bits · 0–16383`), and _Ver controles vistos_ lists every control seen, its range, whether it moves by itself and what it does now.
3. **Learn.** Choose an action, press _Aprender_, move the control and let go. curado binds it with the kind it detected, so a 14-bit fader is read at 14 bits and a jog as an encoder, without asking. Learning a control again replaces what it did.
4. **Or follow a guided list.** _Asistente: controlador DJ_ asks for the controls most DJ controllers share (play, cue, sync, faders, tempo, EQ, filter, headphone cue, jog touch and turn, pads 1–4, crossfader, browse, load, headphone mix, master), one at a time; _Saltar_ skips what the unit lacks. _Asistente: teclado o pads_ maps keys or pads to the eight samples and the hot cues.

Bindings are stored per device in SQLite (`settings.midi`), the remembered port in `settings.midi-device`.

## Making a preset for a new controller

Anyone with the controller can do this from a terminal; nothing needs the app.

1. `bun run midi:probe` lists the inputs and says which have a preset. On Windows, close other DJ software first: WinMM lets one program open a port.
2. `bun run midi:probe "<part of the port name>" --log take.log` listens for 5 minutes (`--seconds N`). Touch nothing for the first 3 s. Then move every control once, in a fixed order, pausing about 2 s between controls: transport and buttons per deck, each fader end to end, each knob, the jog touched and turned and then its side, every pad in every pad mode, the same with SHIFT held, then the centre. Note the order: it is how a number gets its name.
3. `bun run midi:probe --replay take.log` explores the capture again and prints one line per gesture, then which of them a bundled preset already knows. Changing the detection and replaying is how its rules were tuned.
4. Write `crates/midi/presets/<vendor>-<model>.json` (format below) and add it to `BUNDLED` in `crates/midi/src/preset.rs`. Mark a control `"verified": true` only if it was in the capture.
5. Keep a few windows of the capture as a fixture in `crates/midi/tests/fixtures` (tens of KB, not the whole take) and a test like `tests/ddj_sb_capture.rs`: every gesture in it must be in the preset with the same kind and LSB.
6. Record the evidence in the preset's `evidence` and its quirks in `notes`; they show in the app.

A preset is matched by parts of the port name (`"ports": ["DDJ-SB"]`). Two units of one model share it.

### Preset format

```json
{
  "id": "pioneer-ddj-sb",
  "name": "DDJ-SB",
  "vendor": "Pioneer DJ",
  "ports": ["DDJ-SB"],
  "evidence": "Captured … on 2026-11-17 with midi_probe …",
  "notes": ["Faders are 14-bit …"],
  "init": [{ "slot": 0, "kind": "colorFx", "value": 6 }],
  "controls": [
    {
      "label": "Deck 1 tempo",
      "kind": "cc",
      "channel": 1,
      "number": "0x00",
      "lsb": "0x20",
      "mode": { "type": "absolute" },
      "action": { "slot": 0, "kind": "rate", "value": 1 },
      "range": 0.1,
      "deadband": 0.008,
      "verified": true
    }
  ]
}
```

- `channel` is 1–16 and numbers may be `"0x13"` or `19`, as a manual writes them.
- `mode`: `button`, `absolute`, or `relative` with an `encoding`. `lsb` makes an absolute control 14-bit.
- `action` is any engine action (`Action::validate` in `crates/audio-engine`) or one of MIDI's own: `jog` (scratches while the top is touched, bends the tempo otherwise), `listMove` and `listLoad` (the library list, handled by the interface). Leave `action` out for a control that exists but does nothing yet; listing it keeps learn and the probe from calling it unknown.
- `scale` (relative): seconds of music per jog step (default 0.0025, that is 720 steps a turn at 33⅓), rows per browse step (1), or the part of 0–1 per step for an endless knob on a parameter (1/128).
- `deadband` (absolute): see the noise gate below. `range` (tempo): ± around 1. `invert`: for a fader whose top sends 0.
- `init`: actions sent once on connecting, such as choosing Filter as the colour effect when the knob is printed FILTER.

## How detection decides

A **gesture** is everything the controller sent while a control was moved. It ends when the controller is quiet for 400 ms, or at once when the last held note is let go and nothing else moved. A held note waits up to 5 s for what it modifies, so SHIFT and a pad is the pad.

`classify` in `crates/midi/src/detect.rs` then:

1. Pairs **14-bit** halves: CC _n_ below 0x20 with CC _n_ + 0x20 arriving within 2 ms after it.
2. Folds **mirrors**: CCs on the same channel that sent exactly the same values at the same time (the DDJ-SB FX knob sends three) go under the lowest number.
3. Decides each control's **kind**. Notes are buttons. A CC with only 0 and 127, or at most two values including 0 in a few messages, is a button. Values repeated back to back, or jumping between the two ends, are an encoder, because absolute controls only send on change; the encoding comes from where the values sit. Anything else is absolute.
4. Drops **too little**: a release with no press, fewer than 3 messages that are not a button, or an absolute control that moved less than 2% in under 10 messages (a worn fader drifting while something else was pressed).
5. Prefers controls not found **noisy**, then, when learning, the kind the action wants (a button for play, a continuous control for a fader), then the control that sent most, then the one touched last.

## Reading while playing

- **Buttons** act on press. `roll`, `releaseFx` and `jogTouch` also act on release. On/off actions (`sync`, `keyLock`, `channelCue`, `slip`, `reverse`, `beatFxOn`, `quantize`, `masterCue`, `autoGain`) flip what the engine reports, so the controller and the screen never disagree.
- **Absolute** values become 0–1 with the centre detent (64 of 127, 8192 of 16383) at exactly 0.5, so EQs, filters and tempo rest on unity. Then each action's range applies (`continuous()` in `midi.rs`).
- **The noise gate.** A message that arrives alone (more than 40 ms after the previous one) only counts when it has moved at least `deadband` from the last value that counted; messages in a movement always count, at full resolution. On the DDJ-SB an untouched tempo fader stepped 48–64 of 16383 about once a second and drifted 5% in three minutes; a hand sends every 2–10 ms. Measured on the full capture: with the preset's 0.8%, 35 of the untouched fader's 152 messages in 160 s got through (the drift, in steps of 0.8%, about 0.16% of tempo at ±10%), and 228 of 230 while it was moved by hand. With 0.2%, 150 of 152 got through. Default: 0.2% for 14-bit, which only quiets a resting fader's last bit; none for 7-bit.
- **Relative**: `jog` sends `jogMove` while the top is touched and `jogBend` otherwise, letting go 60 ms after the last step; `listMove` moves the list; an endless knob on a parameter keeps its own position, starting at the middle.
- `listMove` and `listLoad` reach the interface as a `midi:local` event and go through the same path as the keyboard's arrows. Everything else goes to the engine through the same `Action` contract as the keyboard and the screen.

The MIDI input callback copies three bytes into a lock-free queue of 256 and returns. A worker reads at most 64 messages every 2 ms, which is where all of the above happens. A remembered port is looked for every 2 s.

## Known controllers

| Controller     | Preset           | Evidence                                                                                                                                                       |
| -------------- | ---------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Pioneer DDJ-SB | `pioneer-ddj-sb` | Captured on Windows 11 on 2026-11-17, 27 000 messages, every control moved once. 124 controls listed, 95 mapped. Not played with yet: see [Status](STATUS.md). |

**DDJ-SB in short.** Decks on channels 1 and 2, FX units on 5 and 6, the centre on 7, deck 1 pads on 8 and deck 2 pads on 9. Faders, EQs, filters, crossfader, headphone mix and FX levels are 14-bit. Buttons send Note On 0x7F and then 0x00. Jogs are `offset64`: touching the top is note 0x35, turning it CC 0x23, the side CC 0x21. Browse is CC 0x40 on channel 7, `twosComplement`; its press is note 0x65, not the 0x41 another map lists. Pads are hot cue 0x00–0x03, auto loop 0x10–0x13, manual loop 0x20–0x23 and sampler 0x30–0x33; SHIFT adds 0x08. Master and headphone level are analog. Both tempo faders on the unit tested move by themselves. CC 0x05/0x25 on each deck channel sent a few values with nothing touched and has no known control.

## What is missing

- **LEDs.** Lighting play, cue, sync and the pads needs a MIDI output and per-preset feedback; the DDJ-SB lights a button when its own note is sent with 0x7F.
- **Jog calibration.** The 720 steps a turn is assumed. Record one full turn of each jog with the probe and set `scale`.
- **Soft takeover.** A fader that does not match the engine on connecting jumps the value when first moved.
- **Software SHIFT layers** for controllers that do not change their notes under SHIFT.
- **Identifying a model** by the MIDI identity request (SysEx `F0 7E 7F 06 01 F7`) when port names are generic, and **user presets** saved, exported and shared as the same JSON.
- **Checked on hardware:** nothing has been played from the DDJ-SB yet. Windows only so far; macOS (CoreMIDI) and Linux (ALSA) read through the same midir code.
