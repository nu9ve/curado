# curado status

## Covers, haptics and camera over the XY · 2026-10-31

The maintainer asked for covers in the track lists, haptics by kind of control with finer movement under pressure, and a third camera state inside the XY surfaces. See [ADR-028](DECISIONS.md) and [Controls](CONTROLS.md).

- **Covers:** lists, device tables, the player and the jog display show the track's cover, or a dithered pattern from its id. A rekordbox picture comes first, then one embedded in the file.
- **Haptics:** knobs, sliders, dials, Beat FX steps, the XY grid and platters tap by the kind of passage, with a **toques** switch.
- **Pressure:** a harder press moves knobs and sliders up to 4× more finely, and multiplies with Shift.
- **Camera:** **XY** shows each side of the picture faintly inside its surface, and each hand now owns its half of the view.
- **A test flaw fixed:** two decode tests shared one temporary file and could fail each other. It predates this work and showed up as a flaky run.

**Automatically tested:** the artwork table and id lookup, choosing the larger picture, a path outside the drive or not an image being refused, the path kept and forgotten across imports, an embedded picture read from a tagged MP3, damaged and coverless files, cover cache order, what each passage feels like, the pressure factor, each hand's half, and the overlay's placement against the point it drives.

**Real USB, read only:** of 2,800 exported tracks, 1,131 name a picture, 1,120 of those files exist, and all 1,120 are JPEG. Reading them took under 6 seconds in all.

**Not tried:** taps and pressure on a Force Touch trackpad, including whether WKWebView delivers `webkitForce`; the overlay with a real camera, where its stretched picture may or may not feel natural; and how any of this looks. By the standing preference no visual review was done, so the jog display's cover, the new **toques** button and the art in device rows are checked in code only.

Verification: `bun run check` passed (84 tests), `bun run build`, `cargo fmt --all -- --check`, `cargo test --workspace` (192 tests, 8 more ignored that need real data) and `cargo clippy --workspace --all-targets -- -D warnings` passed. The macOS `.app` built, links AppKit and passed ad hoc signature verification.

**Installation blocked:** curado was open, so the installed copy was left alone and its audio untouched. It is still the previous build (`ec455cd…`). To update once it is closed, run `bun run desktop:install --no-build`.

**Next:** feel the taps and decide their strength and density, try pressure and the overlay on real hardware, then the earlier hardware list in [Quality](QUALITY.md).

## Press fix, layout review and camera tracking · 2026-10-30

The maintainer reported controls jumping when pressed, needing a second click. Positions used `transform`, which the press style and key feedback replace. They now use `translate`; with a press applied to each of the 167 console buttons in a preview, none moved. Dial names no longer collide, labels stay in capitals under the gothic style, and SYNC and MASTER no longer touch.

Added optional hand and face tracking for the XY surfaces, in a helper process. See [ADR-027](DECISIONS.md) and [Controls](CONTROLS.md).

**Automatically tested:** with a stand-in process, a stop ends it at once, a dead one restarts, a denied one does not, a new start replaces the old, preview requests arrive. Gesture thresholds, mirroring, smoothing and release. **In a preview with simulated frames:** no action without a pinch, moves while pinching, neutral after losing sight, stop on blur, start on focus, stop on off, and the preview image goes when paused.

**Not tried:** a real camera, permission prompt, Vision's accuracy or CPU cost, and whether pinch and leaning in feel usable. Swift only compiled.

Verification: `bun run check` passed (77 tests), `bun run build`, `cargo fmt --all -- --check`, `cargo test --workspace` (187 tests, 7 more ignored that need real data) and `cargo clippy --workspace --all-targets -- -D warnings` passed. The macOS `.app` built with the camera usage statement in its `Info.plist`, passed ad hoc signature verification and was installed and registered without launching; executable SHA-256 `ec455cd71fb7ea456cc6e0bf4d71b68e01599b00b4cacf3fc5121e04bf817d19`. Next: the maintainer should turn on MANOS and say whether it responds and how the camera light behaves when switching apps.

## Keyboard scheme, Release FX, X-Pad, Touch Preview and panels · 2026-10-30

Every **cabina** control with a key shows it on a badge, and the keyboard was redone by hand and modifier. The rest of the console gained Release FX, X-Pad with an FX bank, Touch Preview on a new cue bus, sections on the overview, and panels above it: samples, two XY surfaces and connections. See [ADR-026](DECISIONS.md) and [Controls](CONTROLS.md).

**Checked in a browser preview with a simulated engine:** layout, badges, and each key sends the right action, including list movement, loading and toggles.

**Automatically tested, not listened to:** every Release FX, the cue bus on 4 channels with the fader closed, preview routing on 2 and 4 channels, the global toggle, the allocator audit over all of it.

**Not tried:** a real 4-channel interface, audio of any Release FX, touch on a real screen. Phrases are unvalidated: the USB has no rekordbox phrases in 400 files checked. Keys do not follow the pad mode. Face and hand tracking do not exist.

Verification: `bun run check` passed (70 tests), `bun run build`, `cargo fmt --all -- --check`, `cargo test --workspace` (181 tests, 7 more ignored that need real data) and `cargo clippy --workspace --all-targets -- -D warnings` passed. The macOS `.app` built, passed ad hoc signature verification and was installed and registered without launching; executable SHA-256 `06ff23a3cde558a4361dc408df978296e3db6697814bb1bbe75251a8f8aed21b`. Next: the maintainer should learn the keyboard in real use and say what feels wrong, then try Release FX by ear.

## Console as an RX3, measured key, tempo maps and damaged files · 2026-10-30

The maintainer authorized reviewing the screen, and rejected the console as not resembling the real RX3. **cabina** was rebuilt from the official hardware diagram, with control positions measured from it and platters always present. See [ADR-024, ADR-025](DECISIONS.md) and [Controls](CONTROLS.md).

**Checked in a browser preview with a simulated engine and data, at 1000×700 to 1906×1075:** the console fits with no page scroll, no clipped controls; dragging the platter's top sends scratch actions, its edge and a scroll gesture send bends. That proves layout and messages. It is not the real app, real audio or a real jog feel.

**New and automatically tested, not listened to:** Slip Roll, Pitch, Spiral, Helix and Vinyl Brake, for the hardware's 14 Beat FX; delete cue, crossfader curve, KEY SHIFT, KEYBOARD and PAD FX pads; cues at 0:00; slip, reverse, Beat FX on/off and roll on MIDI, not tried on a controller; the new controls in the classic view; a 60 Hz position stream, IPC cost not measured; beat maps from rekordbox and on request, which sync follows within 15 ms in a synthetic test.

**Measured on the maintainer's USB, read only, 986 tracks against rekordbox:**

| Measure | Before | After |
| --- | --- | --- |
| Tracks given a key | 25 % | 100 % |
| Key exact, tracks not used for tuning | 31 % | 41 % |
| Key a fifth away | 26 % | 15 % |
| Exact at confidence ≥ 0.3 | | 54 % |
| Same tempo as rekordbox | 58 % | 58 % |
| Phase within 50 ms, same tempo | 67 % | 67 % |
| Half a beat off, same tempo | 11 % | 11 % |

Key and phase are agreement, not truth, and this USB's keys may come from other tools. **Tempo and phase did not improve:** five alternatives were tried and none held up on tracks not used for tuning, so nothing changed there except a **½ beat** button. Of the rest, about 28 % of tempos are off by a small ratio, mostly at low confidence. rekordbox has no variable grid on any of 989 tracks here, so its import path has only synthetic tests. curado's own detector fired on 27 % even when tightened, so it runs only on request and its accuracy is unknown.

**Damaged files:** 1,500 mutations each of real WAV, FLAC and MP3, 60 of whole M4A files, 300 of `export.pdb` and 20,000 of analysis files: no crash, and the slowest decode took under a second. AIFF was not really exercised, since cut copies do not open and its files were too large to mutate whole; other formats and longer runs remain. Time limit, panic containment and skipping damaged frames are in.

Limits: no listening, real jog or hardware test. Release FX, X-Pad, Touch Preview, phrase display, memory cues in the transport, mic, AUX, booth and headphones do not exist, and their controls are dimmed. The console fits by scaling, so its text is small on small windows. Sync on a moving tempo has only synthetic evidence.

Verification: `bun run check` passed (57 tests), `bun run build`, `cargo fmt --all -- --check`, `cargo test --workspace` (174 tests, 7 more ignored that need real data) and `cargo clippy --workspace --all-targets -- -D warnings` passed, including the allocator audit over every effect. The macOS `.app` built (14 MiB), passed ad hoc signature verification and was installed and registered without launching; executable SHA-256 `09373d8e5cfe04c633b38329e262e74230027834e08df03a9ebe732d7b4c4b9a`. `export.pdb` on the USB kept its date. Next: the maintainer should try **cabina** with real tracks: the platter above all, then each effect by ear. The rest is in [Roadmap](ROADMAP.md).

## Fixed mixing screen, channel effects and cleaner audio · 2026-10-30

The maintainer confirmed by ear that USB tracks play and control well, then asked for a better mixing layout and the RX3's missing functions, and approved the audio quality list. See [ADR-022, ADR-023](DECISIONS.md) and [Controls](CONTROLS.md).

**Layout (implemented, never seen on screen).** Mix is one fixed screen: console left, with a draggable divider and a collapsible side panel holding library, tools and keyboard; track information with tempo and key against the master; a source selector; a compact sidebar rail, on by default in Mix. Lists scroll inside themselves. No visual test was run, so proportions, overlaps, the rail near the window controls and small knobs may need adjusting.

**Functions (automatically tested, not listened to).** Six Sound Color FX per channel and nine Beat FX on independent CH 1, CH 2 and master units; trim; beat loop, beat jump and roll pad modes; loop 1/2X and 2X; slip; reverse; platter scratch and tempo bend; vinyl brake; tempo range; eight samples with a shared level; TAP; TAG TRACK; auto gain. These effects were not compared with an RX3, and their names describe intent rather than equivalence.

**Audio quality (measured, not listened to).**

| Measurement | Result |
| --- | --- |
| EQ, band killed, tone inside that band | under −35 dB, was shallow one-pole |
| EQ flat through the crossovers | within ±0.1 dB, 30 Hz to 15 kHz |
| EQ centered | sample-exact, attacks included |
| LP/HP two octaves past cutoff | −42 to −54 dB, a 24 dB/octave slope |
| Sinc at 0.97× on a 15 kHz tone | within ±0.2 dB, everything else under −70 dB |
| 20 kHz tone at 1.5× (it would fold to 18 kHz) | under −60 dB |
| Loudness of a full-scale 997 Hz tone at 44.1, 48 and 96 kHz | within 0.3 LU of the standard |

**Callback cost on this Mac** (`fx_budget`, normal-priority thread at 48 kHz, 44.1 kHz files, ten voices sounding, three runs and repeats):

| Case | Median | p99 |
| --- | --- | --- |
| No effects, 256 frames | 5.4 % | 6.7 % |
| Tempo moved (sinc), 256 frames | 10 % | 12 % |
| Three reverbs, EQ, filters, 256 frames | 11 % | 13–27 % |
| The same, 64 frames | 6.5 % | 28–39 % |
| The same with key lock, 256 frames | 11 % | 13–25 % |

Single blocks reached 30–45 ms in some runs, in every case including no effects, at different moments, and not in other runs of the same case. That points to the OS preempting a normal-priority thread rather than to this work, but it was not proven on the real-time audio thread, so the maintainer should watch for dropouts.

Limits: the classic layout has no new controls; use **cabina**. MIDI Learn gained continuous controls only, so slip, reverse and Beat FX on/off have no MIDI toggle. Headphone cue, SOURCE, booth, Slip Roll, Pitch, Spiral and Helix do not exist. Effects keep no state between launches, except auto gain. A cue edited here keeps a cached color until its track loads again. Scratching responds to 60 Hz pointer events, with no hardware jog tried. Reverse has not been tried on real music.

Verification: `bun run check` passed (52 tests), `bun run build`, `cargo fmt --all -- --check`, `cargo test --workspace` (156 tests, 4 more ignored that need a real USB; `real_import` passes on this one) and `cargo clippy --workspace --all-targets -- -D warnings` passed, including the allocator audit, which now covers every effect and the new transport modes. The macOS `.app` built, passed ad hoc signature verification and was installed and registered without launching; executable SHA-256 `515e0503f7d72e67a4778b9d7313c3cda88d552ec748b92e9b60b2e33d57dc23`. `export.pdb` on the USB kept its date. Next: the maintainer should try the screen and rail, then listen to each effect, EQ kills, roll, slip, scratch and brake on real tracks, including with key lock. The rest of the priorities remain in [Roadmap](ROADMAP.md).

## Export to rekordbox.xml · 2026-10-30

**To rekordbox** in the library saves a `rekordbox.xml` with cues and loops set here, and grids and keys corrected by hand, plus curado's playlists, in a file the user chooses. See [ADR-021](DECISIONS.md) and [Controls](CONTROLS.md). **It has not been imported into rekordbox:** only the maintainer can confirm that it imports, matches tracks and does not disturb the rest.

Measured on the maintainer's USB, in an in-memory library with one cue added to 200 tracks and only reads on the drive: a valid file for `xmllint`, with counts, ids, playlist references, sizes and loop ends checked by a script against the disk, and every location resolving to an existing file, in 1.4 s. `export.pdb` kept its date.

**Finding originals is the main limit.** Of 2,802 USB tracks, searching `~/Music` and the other drive (144,018 entries, 4.9 s) found 77 with one file of the same name and size, 27 with several copies, 229 with the same name and another size, and 2,469 with no match. Most of this USB's originals are not on those two locations, so most edited USB tracks would keep their USB path, and rekordbox would see them as other tracks. Tracks edited from the local library do not have this problem.

Limits: behavior on import comes from third-party documentation. Percent-encoding of locations, classic key names (this USB's are Camelot and Open Key) and accented paths are unverified with rekordbox. A hot cue exactly at 0:00 cannot be exported, because zero still means unset. Comments, memory cues edited here and active loops do not exist or are not sent. Windows paths are written but unverified. No visual test: the button has not been seen on screen. Cached cue colors in the interface may stay until the track is loaded again.

Verification: `bun run check` passed (44 tests), `bun run build`, `cargo fmt --all -- --check`, `cargo test --workspace` (117 tests, 4 more ignored that need a real USB) and `cargo clippy --workspace --all-targets -- -D warnings` passed. The macOS `.app` built (14.04 MiB), passed ad hoc signature verification and was installed and registered without launching; executable SHA-256 `ab5a9a74947c21e174c1f098c68c12cf0641b71ead4c675f46cc6f28727649b3`. Next: the maintainer should back up rekordbox's library, export, and import one or two tracks with known cues into rekordbox; what happens there decides whether location encoding or field choices need changing.

## Connected drives and rekordbox USB · 2026-10-30

Connected drives now appear in the sidebar. A rekordbox USB lists its playlists, tracks load with rekordbox's grid, key and hot cues, and any drive can be browsed for audio. See [ADR-020](DECISIONS.md) and [Architecture](ARCHITECTURE.md). **Read only:** nothing is written to a device, and cues set here stay in curado.

Tested against the maintainer's real USB, a FAT32 stick also holding PDFs, games and an Engine DJ library:

| Measured on the real USB              | Result                                                 |
| ------------------------------------- | ------------------------------------------------------ |
| Library read                          | 2,800 tracks, 51 playlists, 3,782 entries in 14–17 ms  |
| Broken references                     | 0 playlist entries, 0 audio files, 0 analysis files    |
| Imported into a test database         | 2,780 grids, 2,771 keys, 157 hot cues, 7 of them loops |
| Key notations understood              | All, including Open Key (`8d`, `10m`)                  |
| Top-level folders shown when browsing | 4 with audio; PDFs, games and `PIONEER` hidden         |
| Imported tracks per load              | About 13 ms each from this USB, plus the decode        |

**Corrections to the previous sessions.** rekordbox's bar numbers are reliable: 120 of 135 of the maintainer's hot cues on its grid fall on its beat 1, or 89 %. Calling that numbering a weak reference was wrong. curado's bars also do better than the 58.5 % agreement figure suggested: 87 of 102 cues on our grid fall on our beat 1, or 85 %. On the 99 tracks with hot cues our tempo was within ±0.1 BPM on 88 %, against 69.5 % overall, and our grid was a median 3.7 ms from rekordbox's. These are 99 tracks the maintainer prepared, so they may be easier than the rest. Local analysis files do hold no cues, as stated: a bug in that earlier check was found and rechecked over 6,000 files.

Also new: eight hot cues in the engine, which can be loops, with older saved cues preserved; colors, comments and memory-cue marks from rekordbox; a scan of drives every 2 s on its own worker; a load to a slot replacing any older one.

Limits: no visual, listening or hardware test; the sidebar, browser and pad colors have not been seen on screen, and the 8-pad layout in the classic deck may need adjusting. Memory cues are only marks. Variable-tempo grids keep one tempo. Playlists cannot be queued or used for sample pads yet. A USB with only OneLibrary shows as plain files. The USB files were only read: `export.pdb` kept its modification date through all of this. Mount detection on Windows and Linux is written but unverified. rekordbox has not been shown a USB touched by curado, because curado does not write to one.

Verification: `bun run check` passed (42 tests), `bun run build`, `cargo fmt --all -- --check`, `cargo test --workspace` (104 tests, 3 more ignored that need a real USB and pass on this one) and `cargo clippy --workspace --all-targets -- -D warnings` passed. The macOS `.app` built (13.90 MiB), passed ad hoc signature verification and was installed and registered without launching; executable SHA-256 `741cae941dc7ae4a8d4657ec7c45d1b083fd4e86f785de2e6eacdc8367031619`. Next: the maintainer should open the USB in curado, load tracks from a playlist, check cues against rekordbox and try unplugging during playback. Then decide between `rekordbox.xml` export, which is what lets cues set here reach rekordbox, and DSP quality.

## Long tracks and real-time key lock · 2026-10-30

Long tracks now load and key lock works in real time; see [ADR-019](DECISIONS.md) and [Architecture](ARCHITECTURE.md).

**Long tracks, in RAM rather than streaming.** The 96 MiB per-track cap became 2 GiB, about 90 minutes at 48 kHz, under a 3 GiB limit shared by all live clips. Streaming was planned, but measurements did not justify it: 0.15 % of the maintainer's 31,477 tracks exceed an hour, and a real 60-minute MP3 decoded and analyzed in 5.1 s. Allocation failures are now errors instead of aborts. A newer load for a slot cancels the older one within a packet (0.1 ms in the probe), and imports release the decode gate per file.

A 60-minute MP3 at 44.1 kHz, 192 kbps, end to end in a release build: 1,211 MiB of PCM, peak process memory 1,495 MiB, 200 seeks across the track on both decks, a loop near the end and key lock on A, with no clipping and a slowest callback of 1,441 µs against 5,333 µs. The probe is not a real audio thread or a listening test.

**Key lock, in the callback rather than prepared.** Tempo and key now apply in the block they arrive, so sync no longer waits about 3 s or keeps the old rate meanwhile, and a moving tempo fader is followed. Up to 384 MiB of source and prepared clips and the `curado-tuning` worker are gone.

| Measured on tones and synthetic kicks                     | Result                                              |
| --------------------------------------------------------- | --------------------------------------------------- |
| Key lock at 0.8–1.25× on 440 Hz; a 44.1 kHz file at 1.06× | Within 0.5 Hz, applied in the same block            |
| Key lock at +6 % from 55 Hz to 3.5 kHz                    | 0 cents                                             |
| Transposing, 440 Hz and up                                | Sharp by 5 cents or less                            |
| Transposing, 110–220 Hz                                   | Sharp by up to 15–20 cents                          |
| Transposing, pure 55 Hz tone                              | Sharp by up to 41 cents, 73 at −12 semitones        |
| Kick energy, first 5 ms, under key lock at ±6 %           | −0.14 to −0.42 dB                                   |
| Hot cue onto a kick with key lock, first 5 ms             | Within 1.5 dB                                       |
| Attack timing against position                            | −0.5 ms at 0.94×, −0.9 ms at 1.06×, −2.5 ms at 1.2× |
| Sample steps entering, leaving, looping and pausing       | Within 3× a 55 Hz tone's own slope                  |

Callback cost, offline, 256 frames at 48 kHz, 60 s: six sources with FX and no key lock measured P50 44 µs, P99 58–105 µs, max 135–254 µs, the same as before. With both decks key-locked and looping every second, six interleaved runs measured P50 177 µs and P99 750 µs. Five peaked at about 1.5 ms; one reached 5,465 µs, over the budget, and did not recur. The probe is not a real-time thread, but that outlier is unexplained. Peaks come from starting a voice, about 1 ms each. On this Mac two simultaneous starts fit buffers of about 128 frames or more; 64 frames could overrun. Not a hardware measurement.

**A crash found and fixed.** The echo could compute a read position equal to the buffer length from `f32` rounding, panicking inside the callback. The new key-lock tests hit it at 0.8×; it predates this work and could have closed the app during a set.

Limits: no listening test of key lock, which is the main unknown, and transposition of bass notes may be audible. Files over about 90 minutes are refused with an explanation, and 7 tracks in this library exceed that. Loading a very long track takes seconds, with only the existing loading state to show for it. Hour-long waveforms in the UI have not been seen, and their tiles take about 70 MiB per deck by calculation. Import has no progress or cancel yet. Corrupt-file and decode time limits remain pending. Linux and Windows behavior of the C++ allocation audit, which uses a macOS hook, is unverified. x86 denormals inside Signalsmith are unreviewed; Apple Silicon has no such penalty.

Verification: `bun run check` passed (36 tests), `bun run build`, `cargo fmt --all -- --check`, `cargo test --workspace` (86 tests) and `cargo clippy --workspace --all-targets -- -D warnings` passed, including the allocator audit, which now covers C++ with key lock, transposition, loops, cues, pause and a load over pitched audio, and proves its hook fires. The macOS `.app` built (13.55 MiB), passed ad hoc signature verification and was installed and registered without launching; executable SHA-256 `1d7044df9fbe421d306c0963ecee1e482db6d032ffbebdff5f82bb2ea0d31094`. **No listening, visual, playback-device or hardware test was performed.** Next: the maintainer should try a long track, move the tempo with key lock on, sync two decks and transpose bass-heavy music; then decide between DSP quality and rekordbox import.

## Whole-track beat analysis · 2026-10-30

A new analysis in `beat.rs` replaces the 90-second estimate; see [Architecture](ARCHITECTURE.md) and [ADR-018](DECISIONS.md). It measures onsets in three bands, finds tempo over the whole track by epoch folding, places the grid 1 ms before the real attack, and adds bars and first-sound detection. Waveforms show bars, the grid editor has **compás aquí**, and an unsaved cue now starts at the first sound. Estimates from the old analysis update when a track loads; manual corrections stay.

`bun run music:eval` is a new benchmark against the grids rekordbox stored locally. Thresholds were tuned on one hash-split set of 740 tracks; these results come from the other 731, run once at the end ([results](grid-benchmark-2026-10-30.json)):

| Agreement with rekordbox                 | Before       | After          |
| ---------------------------------------- | ------------ | -------------- |
| BPM within ±0.02                         | 14.1 %       | 61.0 %         |
| BPM within ±0.1                          | 24.9 %       | 69.5 %         |
| BPM within ±0.5                          | 44.0 %       | 76.7 %         |
| Half or double tempo                     | 6.8 %        | 6.1 %          |
| Other ratio (3:2, 4:3)                   | 10.1 %       | 2.3 %          |
| Other wrong tempo                        | 35.2 %       | 10.7 %         |
| No BPM                                   | 3.8 %        | 4.1 %          |
| Phase within 5 ms, tempo within ±0.1     | 4.4 %        | 40.4 %         |
| Phase within 20 ms, tempo within ±0.1    | 30.8 %       | 63.6 %         |
| Phase within 50 ms, tempo within ±0.1    | 66.5 %       | 77.2 %         |
| Median phase difference                  | +19.6 ms     | −1.8 ms        |
| Grid drift over a track, 90th percentile | 179.6 ms     | 32.2 ms        |
| Same bar start                           | 57.3 %       | 58.5 %         |
| Analysis time, median / maximum          | 112 / 261 ms | 261 / 3,439 ms |

Phase rows compare different populations (182 and 508 tracks), because far more tracks now have the right tempo. Confidence is now useful: tracks under 10 % had the right tempo 41 % of the time, and 87–95 % from 35 % up.

Limits, as measured:

- **Phase:** about 23 % of right-tempo tracks still differ from rekordbox by over 50 ms, many by a half or quarter beat, even at high confidence (80 % within 50 ms from 35 % up). Nothing here shows which grid is right. Measuring bass rise cannot settle it, because this method picks the strongest rise by design.
- **Bars:** agreement barely changed, and raising the weight of backbeat energy lowered it. Bars are display-only and sync still works per beat. A later check against real hot cues found them better than this figure suggests; see the section above.
- **Half and double:** still 6.1 %. Rules follow this library's rekordbox convention, so drum and bass can read at half tempo. ×2 fixes it.
- **Scope:** one constant grid per track. Most tracks in the benchmark are MP3; WAV and FLAC samples are small and no M4A or AIFF track was compared. Key now uses the whole track, but its accuracy remains unmeasured on real music.

**Correction to the earlier audit:** it treated the 25 ms of MP3 encoder delay as an error to trim. Measured against stored grids, trimming moves ours about 25 ms away from rekordbox's, whose lines sit at the attack on the untrimmed timeline, so decoding keeps it. That also avoided migrating saved cues, and rekordbox positions should import without a time shift. That conclusion rests on MP3; other formats need checking when import is built.

Verification: `bun run check` passed (36 tests), `bun run build`, `cargo fmt --all -- --check`, `cargo test --workspace` (77 tests) and `cargo clippy --workspace --all-targets -- -D warnings` passed. The macOS `.app` built (13.57 MiB), passed ad hoc signature verification and was installed and registered without launching; executable SHA-256 `f97e7b549104b44f0812b343d84b0197b2cf124d37a1576b7e5fd946984a6998`. **No listening, visual or hardware test was performed**; the bar lines and editor have not been seen on screen. Next: the maintainer should load tracks and check that grids land on kicks, especially where confidence is high; a rekordbox XML export would give real cues to test bars and import against.

## Clean transport and limiter · 2026-10-30

An audit requested by the maintainer measured the engine with synthetic audio of known timing and with 200 tracks of their local library against the beat grids rekordbox had stored, read-only and without opening `master.db`. This session fixes what it found in transport and output; analysis and DSP findings remain open below. See [ADR-017](DECISIONS.md) and [Architecture](ARCHITECTURE.md).

Implemented: a second read voice crossfades loop boundaries, jumps, pause and PCM swaps; cues and play pre-roll 3 ms so their target sounds at exact unity gain; SET CUE, IN, OUT and beat loops use the nearest beat; quantize never moves a saved cue on recall; sync takes the short way to the master's beat; nudge works on a synced deck; a paused follower keeps its position; a synced Q cue lands on the master beat including its pre-roll. A stereo-linked 1.5 ms look-ahead limiter with 4× true-peak detection replaces the ±0.98 clamp at a −1 dBTP ceiling. A **LIM** lamp in both mix layouts shows the reduction, with limited frames and hard clipping in Settings. Master and crossfader smoothing now derive from the sample rate.

| Measurement                                     | Before                | After                       |
| ----------------------------------------------- | --------------------- | --------------------------- |
| Largest sample step, looping sustained 55 Hz    | 0.589                 | 0.007 (ordinary fade slope) |
| Hot cue from pause on a kick, first 2 / 2–5 ms  | −17.7 / −5.3 dB       | −0.0 / −0.0 dB              |
| Hot cue while playing on a kick, first 2 ms     | −6.8 dB               | −0.0 dB                     |
| Two 0 dBFS decks, LOW +6 dB on both             | 4.4 % samples clipped | 0 clipped, peak 0.891       |
| Worst spur, 55 Hz sine ×3 over / 220 Hz ×3 over | −11.1 / −10.5 dBc     | −29.1 / −60.1 dBc           |
| Sync with the follower 50 ms from aligned       | moved −450 ms         | moved +50 ms                |
| +20 ms nudge on a synced follower               | no effect             | +20 ms                      |
| SET CUE pressed 5 ms before the beat at 10 s    | stored 9.5 s          | stored 10 s                 |
| Off-grid cue at 10.3 s recalled with Q on       | jumped to 10 s        | plays 10.3 s                |

The first two rows use perfect grids. Periodic synthetic loops hide the click even without a fade, so the loop result needs real material. Costs: 3 ms between a cue or play command and its target, plus a constant 1.6 ms of limiter latency at 48 kHz. Interleaved runs of `render_probe` on the same host, 8 each: previous engine P50 ≈37.7 µs, P99 54–95 µs, max 144–237 µs; this engine P50 ≈44 µs, P99 61–110 µs, max 109–731 µs, against 5,333 µs. Two earlier runs of this engine, taken right after compiling, showed max 24.9 and 40.2 ms; they did not recur in the 16 interleaved runs, and the cause is unknown like the historical outlier below. Not a hardware measurement.

Open findings from the audit:

- **Analysis and decoding offsets:** resolved or corrected in the section above.
- **Capacity:** the 96 MiB cap rejects 25–32 % of that library.
- **DSP:** a 44.1 kHz file on a 48 kHz output loses 3.4 dB at 15 kHz with an image at −11.6 dBc. LOW kill reaches only −11.7 dB at 60 Hz. LP/HP are 6 dB/octave.
- **Sync:** key lock keeps the follower at its old rate for about 3 s on a four-minute track, and phase has no continuous correction.
- **Cues:** an unsaved cue is still indistinguishable from one saved at 0:00, which needs the cue data model planned with rekordbox import.

Verification: `bun run check` passed (35 tests), `bun run build`, `cargo fmt --all -- --check`, `cargo test --workspace` (66 tests) and `cargo clippy --workspace --all-targets -- -D warnings` passed, including the allocator audit over loop fades, pause fades, PCM replacement and panic. The macOS `.app` built (13.58 MiB), passed ad hoc signature verification and was installed and registered without launching; executable SHA-256 `a26f7675237906a7006fabc759ebc0b1e2c6cff729a9c6256361918ba323ce09`. **No listening, visual, playback-device or hardware test was performed.** Next: the maintainer should listen to loops on real tracks, hot cues, pause and a hot mix for pumping, and check the LIM lamp in both layouts.

## Integrated console controls · 2026-10-29

Maintainer screenshots showed that the separate controls still occupied space below the console even though the deck chassis had room under the pads. Per-deck curado LP/HP/echo, beat division and musical controls now sit inside their respective decks below the pads; linked A/B transport is in deck A and the shared sample bank is in deck B. The console's keyboard map and full mixer are collapsed but remain accessible. Actions, engine and unavailable hardware controls are unchanged. The beatgrid editor and dynamic status messages may increase the console's height; fit and small-window scrolling remain in place.

Verification: `bun run check` passed (30 tests), `bun run build`, `cargo fmt --all -- --check`, `cargo test --workspace` (44 tests) and `cargo clippy --workspace --all-targets -- -D warnings` passed. The macOS `.app` built (13.56 MiB), passed ad hoc signature verification, and was installed and registered without launching; executable SHA-256 `d711da1190293e38c575fc55434ce7888d8797e2e4c270a42fd393e11a18dcbd`. No new visual, pointer, playback or accessibility verification was performed. Next: inspect both decks with loaded tracks, beatgrid editing, samples, themes and small windows with explicit visual-test authorization; effects and hardware functionality remain in [Mixer views](MIXER-VIEWS.md).

## Console side controls · 2026-10-29

The maintainer's screenshot of the fitted console showed unused space on both sides. Wide sheets now place per-deck curado LP/HP/echo and beat division, musical controls, linked A/B transport and the existing sample bank beside the console; narrower sheets reflow these controls below it. The full curado mixer remains expandable and the keyboard map remains below. These controls use the existing native action contract; hardware FX placeholders remain unavailable and no DSP was changed.

Verification: `bun run check` passed (30 tests), `bun run build`, `cargo fmt --all -- --check`, `cargo test --workspace` (44 tests) and `cargo clippy --workspace --all-targets -- -D warnings` passed. The macOS `.app` built (13.56 MiB), passed ad hoc signature verification and was installed and registered without launching; executable SHA-256 `4801371e323da1550c1f10c2e9caf50ca74d3b4f8b698e32fa6bae65b99d0e5c`. No new visual, pointer, playback or accessibility verification was performed. Next: inspect wide and narrow layouts, scaling and live control behavior with explicit visual-test authorization; preserve the separate audio backlog in [Mixer views](MIXER-VIEWS.md).

## Console viewport fitting · 2026-10-29

Maintainer-supplied screenshots showed the console extending beyond the initial viewport and its decks stretching into empty space. The hardware surface now keeps fixed proportions, fits the available sheet width and initial height with resize observation, puts pads directly below the jogs, and corrects the master meter height. The scale is capped at 120% and floored at 65%; smaller available spaces retain scrolling. Existing audio actions and pending controls are unchanged.

Verification: `bun run check` passed (30 tests, including fit and small-window fallback), `bun run build`, `cargo fmt --all -- --check`, `cargo test --workspace` (44 tests) and `cargo clippy --workspace --all-targets -- -D warnings` passed. The macOS `.app` built (13.56 MiB) and its ad hoc signature verified. Installation was blocked because the installed curado was running; it was not interrupted. Close the installed app and run `bun run desktop:install --no-build` while this bundle remains current. The supplied images show the earlier layout; no visual, pointer, playback or accessibility verification of this change was performed. Next: verify fitting and interactive controls in windowed and fullscreen sizes with explicit visual-test authorization, including small windows and accessibility scaling.

## RX3-inspired console refinement · 2026-10-29

The alternate console now has a dedicated dark hardware-like surface with a central waveform display, load buttons, vertical mixer channels, side jogs, transport, eight pads per deck and reference FX/headphone controls. Existing controls dispatch the same native actions; unavailable controls are dimmed and explicitly labelled coming soon, without audio actions. Only three hot cues work on each deck; the jog toggles playback, CUE invokes hot cue 1, and MASTER TEMPO uses curado's key lock. Original curado filters, echo, beat divisions and musical controls remain in an expandable section. The surface is CSS-drawn, not a photograph or exact RX3 replica. See [Mixer views](MIXER-VIEWS.md) for the capabilities and dedicated DSP backlog.

Verification: `bun run check` passed (28 tests, including unavailable control labeling), `bun run build`, `cargo fmt --all -- --check`, `cargo test --workspace` (44 tests) and `cargo clippy --workspace --all-targets -- -D warnings` passed. The macOS `.app` built (13.56 MiB), passed ad hoc signature verification, and was installed and registered without launching; executable SHA-256 `13b26cd94119b055959c8e567a2e2c42618caa9a4f453dbc3aa4b7aae6c28bbc`. No visual, playback, listening or physical hardware comparison was performed. Next: with explicit authorization, inspect layout accuracy, scrolling, readability, keyboard/focus and theme behavior, then iterate; implement effects and monitoring in a separate audio session.

## Reactive Metal scenes · 2026-10-29

The macOS Metal helper now has seven scenes: the earlier terrain, orbit, tunnel and ribbons plus a faceted gateway with particles, circular waveform/spectrum and depth-tested low-poly spectral city. A native panel controls scene, glow, sensitivity, speed and color. H hides/restores the panel and toolbar, F toggles native fullscreen, R resets the camera; dragging the visual moves it with a spring return. Preferences are temporary. Glow affects the screen-shader scenes, not terrain or city.

The audio callback captures at most one requested, preallocated 2,048-sample mono master window; a worker computes a Hann-windowed FFT and smooths 24 logarithmic bins before sending them to the helper. No callback allocation, locking or FFT. Captures stop when the helper closes; minimizing skips drawing but not capture. This is approximate visualization, not a calibrated meter: resolution is 23.4 Hz at 48 kHz and 46.9 Hz at 96 kHz, low bins can overlap, stereo anti-phase can cancel in mono and updates can be slower than 20 Hz. Web audio remains outside the engine.

Verification: 27 TypeScript and 44 Rust tests passed, including synthetic 100 Hz/1 kHz/8 kHz FFT, capture cancellation and callback allocator coverage with capture enabled. `bun run build`, cargo formatting and clippy passed. Swift typechecked; all three Metal pipelines compiled on the local GPU without launching the app. The ARM64 `.app` built (13.54 MiB), passed ad hoc signature verification and was installed and registered without launching; executable SHA-256 `449a67e5e36e804731d96a156259002f0275b3aa0ad83918973bf1d73543403f`. A six-source offline CPU probe at 48 kHz/256 frames with capture requested every 10 blocks measured P50 34.62 µs, P95 36.33 µs, P99 39.00 µs, max 98.92 µs against 5,333 µs; zero clipping. A separate no-capture run measured P99 43.92 µs, max 215.12 µs. Host scheduling and run variation prevent attributing the difference to capture; neither run validates hardware output or GPU performance. Next: with explicit visual-test authorization, inspect all scenes, controls, fullscreen, camera, spectrum response and GPU load on real playback; test accessibility and additional sample rates.

## Alternate console view · 2026-10-29

Mix now has a persisted **curado / cabina** selector. The console arranges the existing two decks around a central waveform screen and mixer, with hardware-inspired jogs, pads and faders; user theme and accent still apply. The same components and native actions handle both layouts, and switching does not change session mode. A narrow viewport scrolls horizontally. This is an initial RX3-inspired surface, not an exact equipment layout; the platter still toggles playback, and FX, headphone routing and jog behavior are unchanged. [Mixer views](MIXER-VIEWS.md) records the dedicated audio and hardware backlog.

Verification: `bun run check` passed (27 tests, including layout storage fallback), `bun run build`, `cargo fmt --all -- --check`, `cargo test --workspace` (42 tests) and `cargo clippy --workspace --all-targets -- -D warnings` passed. The macOS `.app` built (13.51 MiB), its ad hoc signature verified, and `desktop:install --no-build` installed and registered it without launching; executable SHA-256 `c5e03d8538a7c025b38e42b1d43eb5684268985b5217529eac27a11804a8531e`. No visual, listening or hardware review was performed. Next: inspect both layouts, scrolling, themes, accessibility and switching during playback with explicit visual-test authorization; use that feedback to refine proportions and equipment-specific arrangement.

## Mixer console research · 2026-10-29

[Mixer views](MIXER-VIEWS.md) records the current two-deck architecture, a proposed themed 2D console with depth, and gaps relative to the RX3, DJM-750MK2, DJM-A9 and CDJ-3000X. Four sample slots are not four mixer channels; independent headphone cue, continuous jog and full hardware effects remain absent. No layout or DSP was implemented, and no visual, listening or hardware test was performed. Next: choose the first console direction and supported control set, then prototype using existing actions without changing engine ownership; retain streaming and the native position clock as structural priorities.

Verification: `bun run check` passed (26 tests), `bun run build`, `cargo fmt --all -- --check`, `cargo test --workspace` (42 tests), `cargo clippy --workspace --all-targets -- -D warnings` and formatting checks for the edited documents passed. This documentation-only session did not rebuild or install the app.

## Metal visual variations · 2026-10-29

The separate macOS Metal window now offers terrain, orbit, tunnel and ribbons, with a native selector and a glow slider for the three new scenes. The palette follows the user accent. RMS and the 64-sample scope animate the scenes; no FFT or frequency bands were added. Selection and glow reset when the window closes; the integrated canvas is unchanged. The shader uses bounded procedural effects, not a bloom post-process, and retains the separate worker and 30 FPS cap.

Verification: Swift typecheck passed; Metal compiled all shader functions and both render pipelines on the local GPU without opening the app. `bun run check` passed (26 tests), `bun run build`, `cargo fmt --all -- --check`, `cargo test --workspace` (42 tests) and `cargo clippy --workspace --all-targets -- -D warnings` passed. The ARM64 `.app` built (13.51 MiB) and its ad hoc signature verified. Installation was blocked because the installed app was running; it was not stopped or updated. No visual, listening or performance review was performed. Next: close the running app and run `bun run desktop:install --no-build` while this bundle is current; with explicit authorization, inspect all scenes, resizing, accessibility and GPU load with real playback. Consider persistent preferences and true spectral analysis only as separate work.

## Localization in progress · 2026-10-29

The frontend has an explicit Spanish/English selector in Appearance, with Spanish as the fallback and a versioned localStorage preference. The selected language updates document `lang` and the subscribed UI without restarting audio. Main navigation, library, decks, settings, appearance, keyboard labels and action help have initial English translations; the option is explicitly marked in progress. Native errors, menus, visual window title, some dynamic strings and complete coverage/a11y review remain. The audio settings description now reflects sync and beat echo. No new dependency or native permission was added.

Browser preview on macOS: language changed immediately and persisted across reload; empty player and mix in English and the pixel/light appearance were inspected. Native ARM64 `.app` built, ad hoc signature verified and installed in `~/Applications/curado.app`; the installer found no running installed instance. An explicitly authorized native review loaded the local demo and captured [mix in gothic](assets/mix-gothic-es.jpg) and the [Metal viewer](assets/visual-metal.jpg), and switched the native settings to English. [Settings preview](assets/settings-en-preview.jpg) is a browser capture. Visual inspection is not a listening test. Native UI automation later timed out while preparing another theme, so a complete cross-view, bilingual and multi-theme visual review remains pending.

Verification: `bun run check` passed (26 tests), `bun run build`, `cargo fmt --all -- --check`, `cargo test --workspace` (42 tests) and `cargo clippy --workspace --all-targets -- -D warnings` passed. Installed executable SHA-256 `7a500b9157e8ff1e71eee1e0626f6db15c15b564572156539c09d210cef1d206`. Next: translate and test native errors, menus and remaining dynamic strings; verify switching in the installed app and complete the screenshot set before removing the in-progress label.

The native review selected English and attempted a theme change before UI automation timed out; the final appearance preference could not be confirmed or restored. The review loaded the demo into the deck slots and did not start playback.

## Interface and localization review · 2026-10-29

The installed native app was opened for an explicitly authorized interface review. The accessibility tree exposed the player, a populated two-deck mix view, musical controls, samples, keyboard map and library; this is an interface inspection, not a listening or hardware test. The active session was left alone. No new README screenshots were captured: the visible library contains the user's tracks, and English localization is not yet implemented. Existing screenshots remain historical.

At this initial review, the UI embedded Spanish strings throughout React components, contextual help, keyboard labels and native errors, with no language preference. A complete English/Spanish implementation must cover dynamic and accessible text, native errors and menus as well as persistence and language switching, then be verified across the views. The outdated audio settings copy was corrected in the subsequent work above; full localization is still pending.

Verification for this documentation-only change: `bun run check` passed (24 tests), `bun run build`, `cargo fmt --all -- --check`, `cargo test --workspace` (42 tests), and `cargo clippy --workspace --all-targets -- -D warnings` passed. No application rebuild or installation was required. Next: implement and test localization before capturing a clean demo session in both languages and replacing the historical README image.

Updated **2026-10-26**, America/Hermosillo. Version **0.1.0**: local testing foundation with estimated BPM/key, editable beatgrid, sync/quantize, worker-prepared transposition and an integrated isolated web browser. The production matrix in QUALITY is incomplete.

## Resume here

1. Read this document, ARCHITECTURE, ROADMAP and [Sessions](SESSIONS.md). Installation starts in [Setup](SETUP.md).
2. **No Computer Use/visual app testing** without explicit authorization. An authorized 2026-10-25 review covered only curado-style web preview at 1100/1440 px, no audio/tracks. Loaded native decks and other styles remain unreviewed.
3. Always lowercase **curado**; exact initial color **#d66154**, style **curado**. Technical docs are English; [README.es.md](../README.es.md) is the Spanish welcome. App UI remains Spanish; maintainer communication stays Spanish.
4. Next structural priority: S1 streaming/global memory. Next UX priority: S2 native position clock for smooth playheads/platters without breaking invariant 2.

## Documentation session · 2026-10-26

English technical docs and AGENTS, friendly English README and reciprocal Spanish README, user-supplied mixer development capture, nu9ve.xyz credit, linked agent-assisted setup/troubleshooting. No new screenshot capture, app launch, visual/audio test or native behavior change. Documentation-only sessions need not rebuild/reinstall an unchanged app; required automated checks still run.

[Licensing](LICENSING.md) recommends MIT for the maintainer's stated maximum freedom/commercial-fork preference and explains free downloads, donations and paid editions. **No project license applied.** Owner/year, explicit choice, contribution/brand/asset rules and full dependency notices/source obligations remain unresolved. Signalsmith MIT notices exist; Symphonia declares MPL-2.0; exact font notices/full locked-tree audit remain release work. No price/payment provider selected.

Verification: `bun run check` **24 TS tests /11,574 assertions**, `cargo test --workspace --locked` **42 Rust tests** (27 engine +15 app), `bun run build`, cargo fmt/clippy with -D warnings and documentation formatting/diff checks passed. A local link/image check passed **73 references across 16 Markdown files**. Format scripts now include README.es.md. No native rebuild/reinstall was needed; the installed binary below remains the latest app build.

## Integrated browser session · 2026-10-26

**radio / web** contains a native child WebView in main, initially fractos, replacing the external-window implementation below. Sidebar/topbar/footer stay local. React reserves bounds with ResizeObserver/resize/scroll. Settings/playlist dialogs hide it temporarily while audio continues. Leaving the view, entering mini, stopping or switching service closes it/stops web playback; returning opens a new session.

Capability uses **webviews: [main]**, not window selection (which would include children). Handler label validation, fixed URLs/exact-host policy, incognito and popup/download blocking remain. Tray/single-instance use Window rather than WebviewWindow, which no longer represents main with children. No generic JS WebView creation permission or local CSP change.

Resolved Tauri 2.12.5 enables `unstable` for `Window::add_child`, with no dependency/lockfile update (ADR-012). Audio stays outside native mixer, no script injection. Code/contracts only; actual layout/focus/cookies/login/playback unverified.

Evidence: `bun run check` **24 TS tests**; `cargo test --workspace --locked` **42 Rust** (27 engine +15 app); clippy -D warnings, formatting/diff checks passed. Frontend and ARM64 `.app` **13.48 MiB**, verified ad hoc signature, not notarized. Installed/registered `/Users/nu9ve/Applications/curado.app`; executable SHA-256 `6244aa4761012a314267bce70b140fc43bae827a5aeebdecbb8538d14c685182`. Not launched; Spotlight/Raycast launch/indexing unverified.

## Previous web-launcher session · 2026-10-26

Historical implementation, superseded by integrated browser above. Fixed fractos URL `https://www.fractos.app/es/radio`: public HTTP redirects to www/HTTPS and returns 200. Owner confirmed YouTube/SoundCloud iframes, no PCM/native mixing. Separate service windows reused/closed explicitly, preserving local decks/queue; native master/FX/sync/panic affected local audio only.

Fixed URLs/exact HTTPS hosts including widgets, incognito, popup/download blocking; explicit 26-command AppManifest/main-only ACL and handler label guard. CSP/core unchanged. **24 TS /41 Rust** (27 engine +14 app), clippy/format/diff/frontend/native build passed. ARM64 **13.44 MiB**, ad hoc verified. Installed/registered SHA-256 `0996a7b8750239d93a3c1114133c9be6130c0b51ace2f13d09f0d26279a7d178`, not launched.

HTTP 200/policy tests did not validate player/login/cookies on any real WebView. Popups/external auth blocked, session temporary. Next potential integration is iframe transport/volume protocol with fractos; native EQ/BPM/key/sync needs direct authorized sources/streaming.

## Musical session · 2026-10-26

Offline BPM/offset/major-minor key, library/deck letter/Camelot notation, editable BPM/offset/key/reanalysis. New imports analyze ≤90 s; older tracks analyze on load. Versioned per-ID settings preserve manual corrections without schema migration/original music edits.

Default Q; 1/2/4/8/16-beat loops align grid/avoid partial tails; seconds fallback without grid. Native master BPM/phase sync. SYNC+Q follower cues execute per sample at next beat and cancel on panic/linked pause/context changes. ±12-semitone pitch, key lock and one-shot tonic/relative key sync use Signalsmith Stretch 0.1.3 MIT worker-prepared PCM. Previous audio continues until ready, validated ticket/revision batches acknowledge application/rejection. HP and smoothed beat echo with preallocated 8 s delay per slot.

`desktop:install` default build/install to `~/Applications/curado.app`, `--no-build` reuses verified bundle, `--check` validates without installing. Identity/signature/hash, staged replacement/rollback and LaunchServices, without launch/forced quit. Indexing/launcher behavior remains distinct from registration.

### Evidence

- **22 TS tests** including 24 Camelot keys/transposition and installer rollback in a temporary directory.
- **38 Rust tests** (27 engine +11 app): 90/120/124/150 BPM clicks within ±0.1 BPM, offsets ±20 ms; major/minor triads across 12 roots; sync tempo/phase/loop/deferred cue/panic; 220→440 Hz transpose; key lock at 1.25×; stereo/frame count; stale tuning/retry/same-ID reload; HP/beat delay/SQLite/allocator new-command coverage.
- Frontend/ARM64 app/ad hoc signature passed. Wrapper/Stretch/Linear MIT notices bundled. C++/libclang build only; no callback C++/FFT/new permissions/CSP.
- Installed **13.31 MiB** app, executable `c1bd4b0d85e532f13b375f29a32f433c642509473a600bc899490de975f7cd68`, not launched. Superseded by browser build above.
- Offline six-source/FX probe: P50 **36.67 µs**, P95 **39.38 µs**, P99 **51.29 µs**, max **147.25 µs** against 5333.33 µs budget; zero clipping. Does not erase historical outlier or validate physical output.
- Synthetic music probe, 30 s/48 kHz: analysis **22.76 ms**, preparation at 1.25×/+2 semitones **457.12 ms**, estimated **119.943 BPM** for 120, 1,440,000 frames preserved, **23,040,000 PCM bytes** source+result. Not total RSS/DAC latency. See [musical probe](musical-probe-2026-10-26.json), `music:probe`.

Limits: no real-music/controller/listening verification. Heuristics do not detect variable tempo/downbeats/modulations; indicators not calibrated probabilities. No loop crossfade. Full-clip tuning can take seconds and add up to 192 MiB originals +192 MiB results beyond existing clips, without global budget. 96 MiB per-file cap remains. No long-track continuous timestretch/automatic recovery/complete production matrix.

## Implemented

| Area             | Scope                                                                                              |
| ---------------- | -------------------------------------------------------------------------------------------------- |
| Stack            | Bun/React/TS/Vite/Tauri 2; independent Rust engine                                                 |
| Playback         | CPAL/Symphonia, mono/stereo, two decks/four one-shots, seek/EOF                                    |
| Mixer            | Equal-power, master, basic EQ, LP/HP, beat echo, tempo/nudge/IN-OUT loops                          |
| Musical control  | Estimated BPM/key, editable grid, beat loops, native sync, real-time pitch/key lock                |
| Waveforms        | Decode-time offline 3-band 50 bins/s, schema-v3 BLOB/binary IPC, overview/detail 8 s               |
| Cues             | Three per track, shared atomics, SQLite autosave/restore                                           |
| Library          | Referenced files/folders, metadata/search/favorites/playlists; peak-free listings                  |
| Queue            | Rust worker, native advance/repeat off-one-all, 15 s prefetch, repeat-one seek; not gapless        |
| Modes            | Persisted player/mix, shared UI/keyboard/MIDI native policy                                        |
| Input            | Remappable physical keys/animated control-keycap-HUD, clickable keymap, MIDI Learn (EQ unity CC64) |
| UI               | Continuous chrome/elevated sheet, A-mixer-B console, platters/knobs/faders/dB LEDs                 |
| Views            | Full and always-on-top 460×132 mini with size restoration                                          |
| Appearance       | Five styles, light/dark/system, HSV/user accent, variants/type/texture/waveforms                   |
| Brand            | Lowercase name, shared pixel c-ring/axle SVG/UI/geometric icon                                     |
| OS               | Tray/menu, main close-to-hide when tray exists, manual output/buffer recovery                      |
| Data             | SQLite WAL v3, tested migrations v1→v2→v3, stable IDs/originals preserved                          |
| Help             | Fixed hover/focus footer, exact remaps/details/deck transport-routing diagnostics                  |
| Linked transport | One-command A+B play/pause/cue; atomic queue load+play, revisioned bounded prefetch failures       |
| Web              | Isolated internal service browser; independent audio, real player compatibility pending            |
| Setup            | Verified Mac local installer; linked agent/manual instructions; no auto-launch                     |

## Previous footer/A-B transport session

Reported B silence reproduced in code: old cue moved position but did not start a paused/ended slot. Regressions failed before fix. Jump+play now one command per slot without affecting the other. No device-sharing race was established: both decks already sum in renderer with independent positions/playback/gains.

Linked block-boundary actions tested with separate L/R signals at 44.1/48/96 kHz, running samples and 64-command bound. Queue load+play cannot partially accept one of two actions. Prefetch revision rejects old session outcomes; failed prefetch does not retry at 20 Hz (one final advance attempt, reset with new queue).

Footer uses exact actions/current remaps, generic label/title fallback; expansion reserves sheet height, help Escape does not panic, mini unchanged. Markup/code verification only, no visual/listening evidence. Five major deliveries remained open; later musical foundation completes neither streaming/session/recovery nor production gates.

### Evidence · 2026-10-26

- **20 TS tests**, 11,459 assertions: exact action/cue/remaps/diagnostics/mode/accessibility markup.
- **26 Rust tests** (16 engine +10 app): allocator with A/B+sample, linked cues/load+play/EOF/shared PCM/stale-failed prefetch.
- Clippy, format (including cargo fmt), diff passed. Frontend **329.87 KB JS /104.76 KB gzip**, **51.08 KB CSS**. No new dependencies/permissions.
- ARM64 app **13.10 MiB**, ad hoc/signature passed, no launch/notarization.
- Two offline runs in [render-probe-2026-10-26.json](render-probe-2026-10-26.json). After build: P50 **16.75 µs**, P95 **18.21 µs**, P99 **63.37 µs**, max **52,988.25 µs**, zero clipping. **Max exceeds 5333.33 µs callback budget.** Cause unknown, host not isolated, wall-time measurement. Not proof of physical dropouts or continuity. Scheduling/load/physical output measurements remain per QUALITY.

## Automated baseline · 2026-10-25

- **15 TS tests**, 4 files, 11,404 assertions: palette 5 styles ×10 colors ×modes/variants/chrome/sheet; v1→v2 theme, tokens/CSS, exact window permissions, keyboard mode, waveform binary, Bayer/logo.
- **19 Rust tests** (10 engine +9 app): allocator, 60 Hz/9 kHz bands and 50 bins/s at 44.1/48 kHz, v1→v3/backfill, prefetch/repeat-one, theme upgrade.
- Clippy/fmt/Prettier passed; frontend **318 KB JS /101 KB gzip**, **48 KB CSS**, ~120 KB woff2.
- ARM64 `.app` **13.09 MiB**, ad hoc `codesign --verify --deep --strict`; CFBundleName curado, stable identifier.
- Unchanged offline render probe: P50 **16.75 µs**, P95 **17.38 µs**, P99 **18.12 µs**, max **101.46 µs** /5333 µs, zero clipping. [Baseline probe](render-probe.json).

Unverified: alternate four styles/light mode, real waveform readability, mini behavior, 20 Hz WebView console performance, long real-file decode/analysis; three-OS CI configured without verified execution.

## User-visible compatibility notes

- Hot cue/CUE jumps **and plays** from pause/EOF; Shift saves without start, seek retains pause. Out-of-loop jumps disable loop. Default cues show 0:00.
- Existing DB upgrades to v3; old app versions reject newer schema. Make a consistent backup before rollback needs.
- Pre-v3 tracks have flat waveform until first deck load fills it.
- App name is `curado.app`; quit old instance before opening updated one. Old `Curado Visuals.app` helper cache can be removed manually.

## Important limits

- Tracks load fully into RAM: about 90 minutes per track and 3 GiB in total, including prefetch.
- Linear playback interpolation and basic EQ; transposition can sit sharp on very low notes.
- UI position/detail/platters at 20 Hz, 50 ms steps (S2).
- MIDI feedback not mirrored in UI (S7); manual MIDI connect, no Clock/relative encoders/LEDs/physical controller verification.
- One stereo output, no independent headphone cue, ASIO/JACK/native PipeWire.
- Manual device recovery, no media keys/Now Playing/MPRIS/SMTC/video/plugins.
- Remote browser audio outside engine; compatibility/login/layout pending.
- `color-mix()`/container units require Safari 16+ WebKit.

## Implementation lessons

Waveform analysis fits the decode cleaning/copy pass without extra PCM retention, useful for streaming. Peak-free library listings avoid large IPC payloads. Repainting/resizing canvas on every snapshot dominated UI cost; static drawing plus transformed overlays removes that work. Web Animations give per-key feedback without React renders/reflow. These implementation gains do not remove the measured scheduling outlier or missing hardware evidence.
