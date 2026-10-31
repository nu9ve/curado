# Using curado

The current UI labels are Spanish. This guide explains them in English and preserves labels you need to find on screen. For installation, see [Setup](SETUP.md).

## First session

1. Open your installed `~/Applications/curado.app` or the freshly built `target/release/bundle/macos/curado.app` on macOS.
2. Choose **sesión de prueba** near the bottom of the sidebar. It generates two 32-second tracks and four samples in the app data directory, registers them and loads all six slots. No music is downloaded.
3. Press Play in **Reproductor**. Use **Importar música** or **Carpeta** for your own files.
4. Select a row and load A/B. Double-click loads A, then press Play. Loading stops/resets only the destination slot.
5. Create a playlist with sidebar +; double-click its name to rename. Select a track and use “Añadir a playlist.” “Escuchar selección” plays filtered tracks in order from the first, with repeat off/all/one.
6. Switch to **Mezcla**. In the classic view, select a track and press + on a sample pad. In **cabina**, select **SAMPLER** above either deck's pads and click an empty pad to load the selection; Shift-click a loaded pad to replace it.
7. Player's **Metal** button (macOS only) opens a native landscape; the eye button pauses the integrated visual.
8. The topbar picture-in-picture button enters the always-on-top **mini view**; expand restores full view and previous size.

## Mixing console

Layout follows your keyboard: **A left, B right, mixer center, samples below**.

**cabina** follows the layout of an all-in-one like the RX3. These work:

- **Screen:** waveforms, the deck's time, tempo and key, Beat FX state and divisions to touch; the overview seeks. **SOURCE, BROWSE, TAG LIST, PLAYLIST, SEARCH** act on the side panel and **MENU** opens settings.
- **Selector:** turn it to move through the listed tracks; **LOAD 1/2** loads the selection and **TAG TRACK** marks it as a favorite.
- **Platter:** drag around the record to scratch, or with **VINYL** off to bend the tempo. The edge and a scroll gesture always bend. **JOG ADJUST** sets how far a turn goes; **VINYL SPEED** winds the record down on pause.
- **Deck:** **IN, OUT, RELOOP/EXIT**, **1/2X** and **2X** (in a loop they resize it; otherwise they call the previous or next cue), **SLIP**, **QUANTIZE**, **REV**, **TRACK SEARCH** (previous or next track), **SEARCH** (one beat), **SYNC, MASTER**, tempo range, **MASTER TEMPO** (key lock).
- **Pads:** **HOT CUE** (empty saves, saved calls, SHIFT deletes), **BEAT LOOP**, **SLIP LOOP** (hold to repeat; on release the track returns to where it would have been), **BEAT JUMP**. With **SHIFT**, or the small label under each: **KEYBOARD** (plays hot cue A at a pitch), **KEY SHIFT**, **RELEASE FX**, which pressed again gives **PAD FX** (hold an effect), and **SAMPLER** (eight shared samples; Alt-click stops one, Shift-click replaces it).
- **Release FX:** end a track with Brake or Spin, short or long, Echo Out or Mute, and the deck stops. **BUILD UP** and **DUCKING** last while held.
- **Screen:** three effects to choose at a touch and the **X-PAD**: slide for the division, and the effect runs while a finger is down. The overview shows sections as colored marks: an energy estimate, not a real reading of the song's structure.
- **Mixer:** **TRIM**, EQ, **COLOR**, faders and **CROSSFADER CURVE**. **SOUND COLOR FX** applies to both channels; its **1** and **2** to one, so they can differ. **PARAMETER** sets its character per channel. The knob does nothing at its center.
- **BEAT FX:** **CH SELECT** chooses channel 1, 2 or master, which run independently. Then **BEAT**, **TAP** (Shift-click returns to the track's tempo), the effect dial, **TIME**, **LEVEL / DEPTH** and **ON/OFF**. On PITCH, divisions choose an interval. Delay, Echo, Ping Pong, Spiral, Reverb, Helix, Dub Echo and Space keep a tail. **AUTO GAIN** matches levels to −14 LUFS and is off by default.

**Above the console** there are panels the hardware has no room for: **SAMPLES** with its level, **XY 1** and **XY 2**, and **CONEXIONES** with MIDI activity, the audio output, the cue output and drives. An XY surface moves two parameters at once, one per channel with two fingers on a touchscreen, and returns to neutral on release. Its point also follows MIDI and the knobs. Its modes are COLOR, FILTER, BEAT FX and EQ.

**Camera:** in **CONEXIONES**, **MANOS** or **CARA** drive the XY surfaces; off every time curado opens. Each hand moves the surface on its side and acts while pinching; a face moves XY 1 and acts on leaning in. **VER** shows the picture. It stops by itself when you leave Mix or switch to another app, the camera light goes off, and it comes back on return. Images stay on your Mac.

**Headphones:** with an output of 4 or more channels, 3 and 4 are the cue: channel **CUE**, **MASTER**, **MIXING** and **LEVEL**. Holding the waveform of the selected track in **información** is **TOUCH PREVIEW**: it goes to the headphones, or at half level through the speakers on a stereo output, and says so.

Dimmed controls (mic, AUX, input switches, booth, LINK) have no function: they need other inputs and outputs. **herramientas** in the side panel holds curado's own LP/HP, echo, grid and key editing, linked transport, sample levels and the full mixer. The **curado** layout has the same pad modes, slip, reverse and scratch, with color and beat effects in **herramientas**.

## Mixing screen

In **Mezcla** nothing scrolls but the lists. The console fills the left and adjusts to the space. Drag the divider, or use arrow keys on it, to change widths; double-click resets it. The panel button collapses the side panel for focus,.

The side panel has **información** at the top, which also compares the selected track's tempo and key with the master, and tabs for **biblioteca**, **herramientas** (deck effects, tempo, key, linked transport, samples) and **teclado**. **Fuente** reaches any list, including a drive's playlists.

The button in the header compacts the sidebar to a rail. Mezcla and Reproductor each remember their own choice.

- Two detailed waveforms show 8 s with a centered playhead; click to seek relative to center. Bass is blue, mids amber, highs white; RGB/mono alternatives are in Appearance.
- Each deck has title/remaining time, full waveform with numbered cues/loop, a platter rotating with engine position (click toggles play), eight hot cues, transport, nudge, loop/IN/OUT and an outer vertical pitch control (±50%, double-click resets).
- Per-channel HI/MID/LOW (unity at center), FILTER (LP; rightmost is off), ECHO, dB LED meter and vertical volume fader. Vertical master/meter, panic and horizontal crossfader with ← ↓ → controls. Additional HP and beat-echo controls are described below.
- Clickable keyboard map: A coral, B complementary, samples amber, mixer neutral; keys illuminate on input.

Mixer **A + B** controls play both, pause both or trigger both first cues. One action changes A/B before the same block's first sample. Each retains its own cue/rate; linked transport alone does not set BPM or align beats. Samples, crossfader and a zero master are unaffected.

Deck diagnostics distinguish paused, EOF, playing, zero master/channel, outside crossfader and output error. “Reproduciendo” describes transport, not guaranteed audible signal. Channel meters are before crossfader/master.

Knobs/faders: vertical drag (Shift = fine), wheel, arrows (Shift = fine; Page Up/Down = coarse), Home = default, double-click = reset. Dragging displays local input until control returns to engine snapshots. Keyboard feedback illuminates the control and shows a key/action HUD; blocked mode actions show “no disponible” and send nothing.

## Default keyboard

Each hand plays its own deck, with the same finger for the same job. In **cabina** every control shows its key on a small badge; **teclas** in the toolbar hides them.

| | Left, deck 1 | Right, deck 2 | Shift | Control |
| --- | --- | --- | --- | --- |
| Hot cues A–D | `1 2 3 4` | `7 8 9 0` | save | delete |
| Hot cues E–H | `Q W E R` | `U I O P` | save | delete |
| Play / pause | `F` | `J` | | |
| Cue | `D` | `K` | | |
| Sync | `G` | `H` | master deck | key lock |
| Nudge | `A` `S` | `L` `;` | one beat | four beats |
| Loop in / out | `Z` `X` | `/` `.` | ½ / 2× | |
| Reloop | `C` | `,` | quantize | |
| Slip | `V` | `M` | reverse | |
| Beat FX on/off | `T` | `Y` | | |

- **Space:** stops whatever is playing, or starts both decks. In the player it is the one deck.
- **↑ ↓:** moves through the list on the panel, on a drive too; hold it to run. **Shift + ↑ ↓** jumps eight.
- **Shift + ← →:** loads the selection into deck 1 or 2.
- **Control + ← ↓ →:** crossfader to A, center or B.
- **Option + a hot cue's number:** fires samples 1–8. **Option + B:** master Beat FX. **Escape:** mutes all.
- Command is left to the system. Keys hit hot cues whatever mode the pads are in. **Settings** reassigns any of these, with Shift, Control or Option.

A **hot cue jumps and plays**, including from pause/EOF; retrigger never toggles pause. It sounds exactly where you saved it, at full level, about 3 ms after the command. All eight start at 0:00 and display their time before saving; keys cover the first three, and pads or MIDI Learn reach the rest. A cue saved as a loop also starts looping, marked ↻; pressing one you never saved starts at the track's first sound, on the nearest beat with Q on. Shift-click saves without playing. Waveform seek preserves pause/play; focused waveform arrows seek, Shift for finer steps. Cue/seek/nudge outside a loop disables it; inside preserves it. With BPM, Loop uses selected beats; without a grid it falls back to up to four seconds. Q aligns boundaries to the nearest beat and prevents partial tail loops. Loop boundaries are faded so they do not click, and the attack at the start of the loop stays intact. Nudge changes position, not jog inertia.

Hotkeys use physical codes while the local window is focused. Typing in inputs or moving sliders does not trigger them. Space/Enter on buttons retain accessible activation. **Configuración → Teclado** remaps and detects conflicts. Player disables B/sample hotkeys. After panic, restore master and resume playback yourself. Web content has separate focus and audio.

## BPM, key and harmonic mixing

Import estimates BPM, beat offset, bars and key over the whole track. Library shows original BPM, standard letter notation (C, C♯, Am…) and Camelot. Tracks analyzed by an older version update when loaded, keeping manual corrections. Open **beatgrid / análisis** to edit BPM (×2/÷2), mark “beat aquí” or “compás aquí,” select key or reanalyze. Reanalysis replaces manual corrections; saving never edits original files. Waveforms mark bar starts with a heavier line. Periodicity shows how clearly the track pulses: below about 20% check the grid, because the tempo is often wrong; it is evidence, not an accuracy percentage. Some music has no useful global BPM/key.

- **MASTER** selects reference; **SYNC** on the other deck matches tempo/phase without starting it, taking the shortest way to the master's beat. A paused follower keeps its position and aligns when it starts. Set BPM on master. Manually moving follower speed disables sync. Nudging either deck adjusts the relative phase by ear until a master cue, SYNC or another event realigns it; sync does not correct drift continuously. Rate range is 0.5×–1.5×; absolute tempo accepts 40–300 BPM within that rate range.
- **Q** starts enabled. Saving a cue, IN, OUT and loops use the nearest beat; sizes are 1/2/4/8/16 beats. A cue already saved is never moved by Q. Near EOF, loops move to keep full beats or refuse if none fit. With SYNC+Q, a follower cue waits for the next master beat, shows “en espera” and plays from its nearest beat to keep phase. Panic/linked pause cancel it; silence must not be followed by a deferred restart.
- **Key lock** starts enabled. −/+ transpose semitones without BPM changes; center resets to 0. Displayed key follows applied tuning; without key lock, varispeed contributes cents. **Key sync** matches master tonic or relative key if modes differ, requires master key lock and does not turn minor into major. It is a one-time adjustment, not continuous following.
- Tempo and key changes apply at once, also while syncing. Very low notes can sit slightly sharp when transposed; key lock alone keeps pitch exact.
- **HP** is independent of LP. **ECHO · BEATS** offers 0.25/0.5/0.75/1/2 beats using output BPM, or 120 BPM without analysis. Smoothed delay-time changes may sweep the repeats' pitch.

Loading resets that deck's tempo/transpose/key lock/sync but keeps FX; loading master disables follower sync. Player entry cancels sync/pending cues and returns master to A. Check the grid before mixing: one BPM cannot synchronize variable-tempo music. Tempos near 70–90 and 140–180 BPM can read at half or double, which ×2/÷2 fixes, and detected bars are a first guess. Extreme pitch changes may affect transients/timbre. Real-music validation remains pending.

## USB drives and rekordbox

Connected drives appear in the sidebar under **dispositivos** within a couple of seconds. curado never writes to them: what you change here stays in curado until you export it.

- **rekordbox USB:** select it to see **todas las pistas**, its playlists and **archivos**. Double-click or use A/B to load. The track arrives with rekordbox's BPM, grid, bars, key and hot cues with their colors, and memory cues appear as small marks on the overview. The analysis label then reads **rekordbox**.
- **Any drive:** **archivos** browses folders with audio and hides everything else. Tracks without rekordbox data get curado's own analysis.
- **Unplugging:** whatever is loaded keeps playing, since it is in memory. A new rekordbox export updates cues and grids, except those you edited in curado.

Loading a track here does not add it to your library. Import it on purpose if you want that.

## Sending cues to rekordbox

**A rekordbox** in the library saves a `rekordbox.xml` wherever you choose. It holds tracks with cues or loops set here or grids corrected by hand, plus your playlists, in a `curado` folder. Importing has not been tried in rekordbox yet, so back up its library first and start with one or two tracks:

1. In rekordbox, **Preferences > Advanced > Database > rekordbox xml > Imported Library**: choose the file.
2. **Preferences > View > Layout**: enable **rekordbox xml**, then restart rekordbox if the tree does not show it.
3. In the **rekordbox xml** tree open **curado > curado cues**, select the tracks, right-click and use **Import To Collection**. Importing only a playlist reportedly does not update tracks rekordbox already has.

An import can replace a track's cues and grid in rekordbox, so a track prepared there and also edited here keeps curado's version. The message after exporting says how many USB tracks found their original on this computer; the others would arrive as another track on the USB.

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

The **LIM** lamp in the master section lights while the limiter turns the mix down and shows the reduction in dB. A little on the hits is normal; red (6 dB or more) flattens the mix, so lower channels, EQ or master. **Configuración → Audio** shows backend/sample rate/callbacks, limited frames and hard clipping, which should stay at zero. Output/buffer changes unload all six slots. Start with automatic buffer; test 256/512 only with measurements. Bluetooth adds latency the app cannot remove.

Closing main hides it if tray/menu bar exists. Tray can show, play/pause A, silence native audio or quit. Playback does not survive process exit/system suspend; no daemon/service is installed. If disconnect reports an error, choose an available output and apply/recover. Reopening UI does not reconstruct the engine. Moved files currently need reimporting.

## Appearance

**Configuración → Apariencia** offers curado/pixel/gothic/y2k/minimal styles, light/dark/system, neutral/tonal/high contrast, typography, dither/soft/flat texture, waveform and preset/picker/hex/HSV color. Defaults: curado, `#d66154`. Saves automatically; Escape closes Settings. macOS integrates native header controls/drag area without a separate title strip. See [Themes](THEMES.md).
