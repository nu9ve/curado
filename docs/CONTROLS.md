# Using curado

The current UI labels are Spanish. This guide explains them in English and preserves labels you need to find on screen. For installation, see [Setup](SETUP.md).

## First session

1. Open your installed `~/Applications/curado.app` or the freshly built `target/release/bundle/macos/curado.app` on macOS.
2. Choose **sesión de prueba** near the bottom of the sidebar. It generates two 32-second tracks and four samples in the app data directory, registers them and loads all six slots. No music is downloaded.
3. Press Play in **Reproductor**. Use **Importar música** or **Carpeta** for your own files.
4. Select a row and load A/B. Double-click loads A, then press Play. Loading stops/resets only the destination slot.
5. Create a playlist with sidebar +; double-click its name to rename. Select a track and use “Añadir a playlist.” “Escuchar selección” plays filtered tracks in order from the first, with repeat off/all/one.
6. Switch to **Mezcla** for the console. Select a track and press + on a sample pad to assign it.
7. Player's **Metal** button (macOS only) opens a native landscape; the eye button pauses the integrated visual.
8. The topbar picture-in-picture button enters the always-on-top **mini view**; expand restores full view and previous size.

## Mixing console

Layout follows your keyboard: **A left, B right, mixer center, samples below**.

- Two detailed waveforms show 8 s with a centered playhead; click to seek relative to center. Bass is blue, mids amber, highs white; RGB/mono alternatives are in Appearance.
- Each deck has title/remaining time, full waveform with numbered cues/loop, a platter rotating with engine position (click toggles play), three hot cues, transport, nudge, loop/IN/OUT and an outer vertical pitch control (±50%, double-click resets).
- Per-channel HI/MID/LOW (unity at center), FILTER (LP; rightmost is off), ECHO, dB LED meter and vertical volume fader. Vertical master/meter, panic and horizontal crossfader with ← ↓ → controls. Additional HP and beat-echo controls are described below.
- Clickable keyboard map: A coral, B complementary, samples amber, mixer neutral; keys illuminate on input.

Mixer **A + B** controls play both, pause both or trigger both first cues. One action changes A/B before the same block's first sample. Each retains its own cue/rate; linked transport alone does not set BPM or align beats. Samples, crossfader and a zero master are unaffected.

Deck diagnostics distinguish paused, EOF, playing, zero master/channel, outside crossfader and output error. “Reproduciendo” describes transport, not guaranteed audible signal. Channel meters are before crossfader/master.

Knobs/faders: vertical drag (Shift = fine), wheel, arrows (Shift = fine; Page Up/Down = coarse), Home = default, double-click = reset. Dragging displays local input until control returns to engine snapshots. Keyboard feedback illuminates the control and shows a key/action HUD; blocked mode actions show “no disponible” and send nothing.

## Default keyboard

| Action                   | A             | B             |
| ------------------------ | ------------- | ------------- |
| Play/pause               | Q, Space      | P             |
| First cue and play       | A             | L             |
| Hot cues 1/2/3           | 1 / 2 / 3     | 8 / 9 / 0     |
| Save at current position | Shift + 1/2/3 | Shift + 8/9/0 |
| Loop on/off              | W             | O             |
| Loop IN                  | E             | I             |
| Loop OUT                 | R             | U             |
| Nudge −50/+50 ms         | D / F         | J / K         |

Samples 1–4: `Z / X / C / V`. Crossfader left/center/right: `← / ↓ / →`. Stop native decks and zero master: `Esc`.

A **hot cue jumps and plays**, including from pause/EOF; retrigger never toggles pause. All three start at 0:00 and display their time before saving. Shift-click saves without playing. Waveform seek preserves pause/play; focused waveform arrows seek, Shift for finer steps. Cue/seek/nudge outside a loop disables it; inside preserves it. With BPM, Loop uses selected beats; without a grid it falls back to up to four seconds. Q aligns boundaries and prevents partial tail loops. Nudge changes position, not jog inertia.

Hotkeys use physical codes while the local window is focused. Typing in inputs or moving sliders does not trigger them. Space/Enter on buttons retain accessible activation. **Configuración → Teclado** remaps and detects conflicts. Player disables B/sample hotkeys. After panic, restore master and resume playback yourself. Web content has separate focus and audio.

## BPM, key and harmonic mixing

Import estimates BPM, beat offset and key. Library shows original BPM, standard letter notation (C, C♯, Am…) and Camelot. Older tracks analyze on load. Open **beatgrid / análisis** to edit BPM (×2/÷2), mark “beat aquí,” select key or reanalyze. Reanalysis replaces manual corrections; saving never edits original files. Periodicity is analysis evidence, not an accuracy percentage; some music has no useful global BPM/key.

- **MASTER** selects reference; **SYNC** on the other deck matches tempo/phase without starting it. Set BPM on master. Manually moving follower speed disables sync. Rate range is 0.5×–1.5×; absolute tempo accepts 40–300 BPM within that rate range.
- **Q** starts enabled. Cues/IN/OUT and loops use the grid; sizes are 1/2/4/8/16 beats. Near EOF, loops move to keep full beats or refuse if none fit. With SYNC+Q, follower cue waits for next master beat and shows “en espera.” Panic/linked pause cancel it; silence must not be followed by a deferred restart.
- **Key lock** starts enabled. −/+ transpose semitones without BPM changes; center resets to 0. Displayed key follows applied tuning; without key lock, varispeed contributes cents. **Key sync** matches master tonic or relative key if modes differ, requires master key lock and does not turn minor into major. It is a one-time adjustment, not continuous following.
- Tempo/key changes requiring preparation display **preparando** and keep previous audio until ready. Play/cues/mixer remain usable. New requests cancel old ones; failure offers retry. Full-clip preparation takes time and additional memory.
- **HP** is independent of LP. **ECHO · BEATS** offers 0.25/0.5/0.75/1/2 beats using output BPM, or 120 BPM without analysis. Smoothed delay-time changes may sweep the repeats' pitch.

Loading resets that deck's tempo/transpose/key lock/sync but keeps FX; loading master disables follower sync. Player entry cancels sync/pending cues and returns master to A. Check the grid before mixing: one BPM cannot synchronize variable-tempo music. Loops still lack crossfade; extreme pitch changes may affect transients/timbre. Real-music validation remains pending.

## Context footer

The footer stays outside content scroll. Hover or Tab-focus a control to see name/function/current shortcuts. **Más info** expands tips in the footer, reserving sheet height and moving notices. Tone follows deck/theme. Escape inside expanded help closes it and returns focus to its button without panic.

Help preserves the previous control while entering the help panel. Exact `data-action` slot/type/value identifies audio actions; generic controls use label/title and fallback text. Tips are maintained in `src/lib/hints.ts`, associated through `data-help`, not automatically generated. Mini view omits the footer.

## MIDI Learn

**Configuración → MIDI**: choose port/action, **Aprender**, then press a note or move CC. Device/channel/type/number persist; learning the same control replaces its previous action. Connection is manual each launch. Track cues are shared across decks holding the same track.

Supports Note On velocity >0 and absolute CC 0–127; EQ unity is CC 64. No relative encoders, Clock, SysEx, LED feedback or automatic reconnect yet. A CC-to-button mapping triggers on every value ≥64; prefer notes for buttons. Player also blocks B/sample playback through MIDI.

## Radio and web services

Sidebar **radio / web** shows fractos inside curado; fractos/SoundCloud buttons switch the integrated browser. Desktop and internet required. Exact service/widget domains only, incognito, no popups/downloads; some logins/links can be blocked. Actual player compatibility remains unverified.

Players control their own web audio, outside local master/crossfader/FX/sync/Escape. **detener audio web**, switching service, leaving radio/web or entering mini closes the browser and stops it. Returning starts a new session. Settings/playlist dialogs hide it temporarily while audio continues. Local decks/queue retain their state; web focus does not deliver keys to local hotkeys.

## Audio and background behavior

**Configuración → Audio** shows backend/sample rate/callbacks/clipped samples. A growing clipping count indicates excess level; lower channel/master. Output/buffer changes unload all six slots. Start with automatic buffer; test 256/512 only with measurements. Bluetooth adds latency the app cannot remove.

Closing main hides it if tray/menu bar exists. Tray can show, play/pause A, silence native audio or quit. Playback does not survive process exit/system suspend; no daemon/service is installed. If disconnect reports an error, choose an available output and apply/recover. Reopening UI does not reconstruct the engine. Moved files currently need reimporting.

## Appearance

**Configuración → Apariencia** offers curado/pixel/gothic/y2k/minimal styles, light/dark/system, neutral/tonal/high contrast, typography, dither/soft/flat texture, waveform and preset/picker/hex/HSV color. Defaults: curado, `#d66154`. Saves automatically; Escape closes Settings. macOS integrates native header controls/drag area without a separate title strip. See [Themes](THEMES.md).
