# MIDI controllers

How curado finds out what a controller is, maps it, and how a new controller becomes a preset anyone can use. The person's side of it is in [Controls](CONTROLS.md#midi); this page is the method and the contracts. Code: `nu9ve-midi` (messages, detection, physical profiles), `crates/midi` (curado mappings and probe), and `src-tauri/src/midi.rs` (ports, bindings, engine actions).

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
4. Define a versioned hardware profile with stable IDs, port aliases and evidence in nu9ve/core, then pin it. Follow the [profile format](../vendor/nu9ve-core/rust/nu9ve-midi/README.md#perfil-físico-v1). Mark an address verified when it was observed and record whether the gesture was identified.
5. Add the app mappings in `crates/midi/presets/<vendor>-<model>.json` and register the profile and mappings together in `crates/midi/src/preset.rs`. The hardware profile matches substrings of native port names. Two units of one model share the preset.
6. Keep a short timestamped capture with regression tests in core. The curado integration test uses that fixture to check that detected gestures have the same kind and LSB as the preset.

### curado mapping format

```json
{
  "profile": "pioneer-ddj-sb",
  "notes": ["The jog scale assumes 720 steps per turn."],
  "init": [{ "slot": 0, "kind": "colorFx", "value": 6 }],
  "mappings": [
    {
      "control": "deck-1-tempo",
      "action": { "slot": 0, "kind": "rate", "value": 1 },
      "range": 0.1
    }
  ]
}
```

- `control` is a stable ID from the hardware profile. Each mapping has an engine action (`Action::validate` in `crates/audio-engine`) or one of MIDI's own actions. `jog` scratches while the top is touched and bends the tempo otherwise. `listMove` and `listLoad` go to the interface. Unmapped IDs remain part of the hardware profile.
- `scale` sets seconds of music per jog step (default 0.0025), rows per Browse step (1), or the part of 0–1 per step for an endless knob (1/128). `range` sets the tempo range around 1. `invert` reverses an absolute control.
- `deadband` in a default mapping overrides the profile's calibration. The app uses its resolution-based default when both omit it. Saved bindings keep their own values and wire addresses, including rows saved before the profile migration.
- `init` selects mixer defaults, such as Filter for the knobs printed FILTER. It applies when there is audio output and the mode permits the action, once per engine generation for that connection.

## How detection decides

A **gesture** is everything the controller sent while a control was moved. It ends when the controller is quiet for 400 ms, or at once when the last held note is let go and nothing else moved. A held note waits up to 5 s for what it modifies, so SHIFT and a pad is the pad.

`classify` in the [pinned reader](../vendor/nu9ve-core/rust/nu9ve-midi/src/detect.rs) then:

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

| Controller     | Preset           | Evidence                                                                                                              |
| -------------- | ---------------- | --------------------------------------------------------------------------------------------------------------------- |
| Pioneer DDJ-SB | `pioneer-ddj-sb` | WinMM capture on 2026-11-17, CoreMIDI capture on 2026-11-22. 125 controls listed, 95 mapped. See [Status](STATUS.md). |

**DDJ-SB in short.** Decks on channels 1 and 2, FX units on 5 and 6, the centre on 7, deck 1 pads on 8 and deck 2 pads on 9. Faders, EQs, filters, crossfader, headphone mix and FX levels are 14-bit. Buttons send Note On 0x7F and then 0x00. Jogs are `offset64`: touching the top is note 0x35, turning it CC 0x23, the side CC 0x21. Browse turns on CC 0x40 on channel 7 with `twosComplement`. Its press sent note 0x41 in the Mac capture. The Windows capture also included CH 7 note 0x65, whose physical gesture needs reconfirming. Both notes are listed without a default action. Pads are hot cue 0x00–0x03, auto loop 0x10–0x13, manual loop 0x20–0x23 and sampler 0x30–0x33. SHIFT adds 0x08. Master and headphone level are analog. Both tempo faders on the unit tested moved by themselves on Windows. The short idle window on Mac saw no events. CC 0x05/0x25 on each deck channel sent a few values with nothing touched on Windows and has no known control.

### DDJ-SB tempo and auto-loop controls

SHIFT + KEY LOCK sends note `0x60` on the deck channel according to the
[Pioneer MIDI table](https://www.pioneerdj.com/-/media/pioneerdj/software-info/controller/ddj-sb/ddj-sb_list_of_midi_messages_e.pdf).
It cycles the same ±6/10/16/50% range as the console's TEMPO button. The next
movement of a stock tempo fader uses that range. Each deck starts at ±10%.
A learned mapping on the shifted note takes precedence.

**Tempo pickup.** After a range, track or software tempo change, move the
physical fader through the current deck tempo to take control. The crossing
keeps the tempo steady. Subsequent fader motion applies normally. If the tempo
is outside the chosen range, expand the range or gradually drag the on-screen
fader back within it. curado displays the actual percentage even when its cap
sits at the rail edge.

In AUTO LOOP pad mode, press once to start and again to exit. Press a
different length to resize the playing loop. Only the first Note On before
release counts as a press. Existing stock bindings gain this behavior when
read and learned bindings retain their configured actions.

### DDJ-SB v1 coverage

The pinned [physical profile](../vendor/nu9ve-core/data/midi/pioneer-ddj-sb-v1.json)
lists 125 address entries. The [curado preset](../crates/midi/presets/pioneer-ddj-sb.json)
maps 95 of them. The table describes stock assignments and labels features that
need an additional mapping or software behavior. Saved bindings take precedence.

| Control group                             | Works through the current action contract                                                                                                                                                                             | Remaining                                                                                                                                                                                                 |
| ----------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Play, deck CUE and SYNC                   | Play/pause, sync toggle and jump-and-play at hot cue A (first sound if unset).                                                                                                                                        | A separate temporary transport cue with set, return and hold-to-preview behavior needs a new deck action and release semantics.                                                                           |
| Key Lock and tempo                        | Key Lock toggles pitch preservation. The 14-bit fader uses per-deck range, dead band and pickup. SHIFT + KEY LOCK cycles ±6/10/16/50% via the stock Key Lock fallback for note 0x60, outside the 125 profile entries. | Confirm both hardware buttons, the range change, direction, center and pickup. A custom binding on 0x60 wins over the fallback.                                                                           |
| Jog top, touch and rim                    | Touch and rotation are mapped to scratch, and the rim to bend without touch. The screen platter reuses the existing pointer gesture.                                                                                  | Verify scratch/bend and measure a full turn. The unbound VINYL buttons need a shared per-deck jog-mode action for MIDI. Shifted jog seek needs confirmed addresses and a mapping/gesture rule.            |
| SHIFT and pad-mode buttons                | The DDJ-SB sends different pad notes according to mode and SHIFT. Standalone SHIFT and mode notes need no mapping for these pads to work. Screen modes and SHIFT are local to the view.                               | Physical mode/SHIFT indicators in the view need MIDI-to-UI state. Controllers that send the same pad notes under SHIFT need a software layer for that behavior.                                           |
| Hot Cue pads                              | Four per deck. Stock MIDI jumps and plays; SHIFT clears. Screen pads save an empty position or loop and recall a saved cue.                                                                                           | A stock physical pad that saves when empty needs state-dependent dispatch; existing save and recall actions can be reused.                                                                                |
| Auto Loop pads                            | 1/2/4/8 beats. Same length toggles off, a different length resizes. All four shifted pads toggle the current auto loop in curado.                                                                                     | Individual shifted half/double assignments could use the existing `loopScale` action after hardware verification.                                                                                         |
| Manual Loop pads                          | IN, OUT, reloop/exit and halve. SHIFT pads 1, 2 and 4 jump −4/+4 beats and double.                                                                                                                                    | Shifted pad 3 has no default. Loop activation independent of the toggle would need explicit semantics.                                                                                                    |
| Sampler pads                              | Deck A triggers global slots S1–S4; deck B triggers S5–S8. The screen can load or replace a sample from the library.                                                                                                  | The eight shifted notes are unmapped. Sample stop can reuse the existing `pause` action.                                                                                                                  |
| Browse and LOAD                           | Browse rotation moves through the current list and each LOAD selects its corresponding deck. The screen routes through the same list/load functions, including device libraries.                                      | Browse press has a captured address and needs a local MIDI action/binding to open or focus the library. Recheck both physical LOAD buttons.                                                               |
| Mixer EQ, faders and crossfader           | Three-band EQ and channel faders per deck, with shared crossfader.                                                                                                                                                    | Confirm hands-on response on both decks.                                                                                                                                                                  |
| Filter and headphone cue                  | Both 14-bit FILTER knobs drive the channel's bipolar Color FX amount. Factory initialization selects Filter. Channel CUE and headphone mix are mapped, with independent monitoring on a four-channel output.          | Listen to both filter directions and check cue with the fader down. A software equivalent of FILTER FADE would need crossfader-linked DSP; the official MIDI table lists no input message for the button. |
| FX, analog levels and remaining addresses | FX buttons map to Beat FX on/off, slip and reverse; the primary FX level pair maps to depth. Master and headphone level are hardware analog controls.                                                                 | Four FX level mirrors intentionally have no extra binding. Three addresses remain unidentified. LED feedback needs MIDI output.                                                                           |

The manufacturer's expanded pad modes and shifted transport, mixer and FX
gestures include addresses outside v1. Confirm any candidate addresses on
hardware before adding bindings. The shared profile describes the wire
messages; the preset and this view use curado's actions.
[Pioneer hardware diagram](https://www.pioneerdj.com/-/media/pioneerdj/software-info/controller/ddj-sb/ddj-sb_hardwarediagram_serato_dj_pro_lite_e2.pdf/)
and [MIDI message table](https://www.pioneerdj.com/-/media/pioneerdj/software-info/controller/ddj-sb/ddj-sb_list_of_midi_messages_e.pdf/).

### DDJ-SB layout and playback check

1. With RX3 selected, connect the DDJ-SB and verify that DDJ-SB is suggested
   while RX3 stays active. Use Browse, LOAD A/B and Play on the controller
   while RX3 is still visible.
2. With both decks sounding, put one deck in a 4-beat loop and sync it to the
   other. Select DDJ-SB, then switch back to RX3. Confirm uninterrupted audio,
   the same tracks, advancing clocks, preserved loop and sync, and working
   MIDI/custom bindings. Try crossfader or gain while each view is active.
3. On DDJ-SB, check hot cue recall/clear, auto-loop repeat and resize, manual
   loops, both jog surfaces, Filter, channel cue with the channel fader down,
   and SHIFT + KEY LOCK followed by a gentle fader pickup. Check elapsed
   time, BPM and both key notations from a small window.
4. Select DDJ-SB with another mapped controller. Verify that its original
   mappings still drive both decks, then reopen curado to verify the saved
   layout.

These checks require a running app. Physical gestures, sound, headphone
routing and controller exchange require the corresponding hardware. The
automated suite covers layout selection/storage contracts, MIDI capture
replay and dispatch, tempo pickup and loop behavior.

## macOS hardware check

1. Connect the DDJ-SB and LPLBZ. In **Settings → MIDI**, choose **PIONEER DDJ-SB** and confirm the recognised preset and activity. Use the current mappings if already set. Measure noise with both tempo faders untouched, then leave the controls idle for at least a minute while the decks are playing to check for pitch drift.
2. In **Settings → Audio**, select **PIONEER DDJ-SB** and apply with the automatic buffer before loading decks. The stream opens in CoreAudio with four channels at 44.1 kHz on the tested Mac. curado routes master to 1–2 and cue to 3–4. Verify speakers and headphones separately with the hardware levels down first. Turn on channel CUE, close that channel's fader and check that it stays present in headphones only. Try headphone mixing, master cue and Touch Preview. The mixer and headphones must use the same four-channel output for this check.
3. Open **Mix** and start with the generated demo. Test both PLAY/PAUSE and CUE buttons, sync and key lock, faders/EQ/filter/crossfader, and tempo direction and centre detents. Verify SHIFT + KEY LOCK on both decks and move each fader at ±10%, ±16% and ±50%. On each jog compare side bend with touched scratch, then measure one full marked turn per jog at slow speed with the probe. Exercise each pad mode and SHIFT, checking that held rolls and release FX stop on release. Press the same AUTO LOOP pad twice and then a different length while playing.
4. In the library or Mix list, turn Browse both ways and press each LOAD. On LPLBZ, pick a known playlist and load two tracks with known grid or cues. Compare their metadata and listen after loading. Keep XML import into rekordbox as its own check with a backed-up rekordbox library.
5. Pause both decks and unplug/replug only the controller. Verify that MIDI returns with the same mappings. Then explicitly verify audio recovery or reapply the output if required. Relaunch to check the remembered MIDI port. Repeat MIDI learn with the Komplete's MIDI input if an external controller is attached.

The last Mac capture saw the right PLAY/PAUSE and Browse press. The left
PLAY/PAUSE note and right LOAD were absent from that capture. If a control
seems unresponsive in curado, isolate it on the command line with
`bun run midi:probe DDJ-SB --seconds 30`. The 2026-11-22 measurements cover
terminal capture and silent stream acknowledgement. Playback, headphones
and reconnect in the installed app are still open.

curado remembers the MIDI input. Audio starts on the system default at
launch, so choose the Pioneer again if another device is the default.

## What is missing

- **LEDs.** Lighting play, cue, sync and the pads needs a MIDI output and per-preset feedback; the DDJ-SB lights a button when its own note is sent with 0x7F.
- **Jog calibration.** The 720 steps a turn is assumed. Record one full turn of each jog with the probe and set `scale`.
- **Soft takeover for more parameters.** Native pickup now covers absolute MIDI tempo. Other absolute mappings still apply immediately.
- **Software SHIFT layers** for controllers that do not change their notes under SHIFT.
- **Identifying a model** by the MIDI identity request (SysEx `F0 7E 7F 06 01 F7`) when port names are generic, and **user presets** saved, exported and shared as the same JSON.
- **Checked on hardware:** WinMM and CoreMIDI captures exist. Listening and playing from the DDJ-SB in curado, in-app hotplug and ALSA hardware remain open.
