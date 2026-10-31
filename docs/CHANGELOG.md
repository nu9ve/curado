# Change history

## 2026-10-30 · Keyboard scheme, Release FX, X-Pad, Touch Preview and panels

Redid the keyboard by hand and modifier with badges on the console, arrows through any list and Space for the whole mix. Added Release FX, X-Pad, Touch Preview on a cue bus, estimated sections and panels above the console. See [Status](STATUS.md) and ADR-026. Lesson: a Space key is only global if a clicked button does not keep it, which the check for visible focus now handles.

## 2026-10-30 · Console as an RX3, measured key, tempo maps and damaged files

Rebuilt **cabina** from the official RX3 diagram with always-present, playable platters. Completed the 14 Beat FX, added MIDI toggles, the new controls in the classic view, cues at 0:00, a 60 Hz position stream, beat maps and decode limits with fuzzing. The key now always answers and agrees more with rekordbox. See [Status](STATUS.md), ADR-024 and ADR-025. Lessons: a maintainer's eye catches what tests cannot, because a console that fit and worked was still the wrong console; and measuring stops bad ideas, because a tempo detector that passed its tests fired on nearly half of a real library, and a phase change that looked better on tuning data did nothing on the rest.

## 2026-10-30 · Fixed mixing screen and channel effects

Mix became a fixed screen with the console on the left, a collapsible side panel with library, tools and keyboard, and a compact sidebar rail. Added per-channel Sound Color FX, Beat FX on CH 1, CH 2 and master, trim, beat loop, beat jump and roll pads, slip, reverse, scratch, vinyl brake, eight samples and auto gain. Replaced the EQ with Linkwitz-Riley isolators, filters with 24 dB slopes and linear interpolation with windowed sinc. See [Status](STATUS.md), ADR-022 and ADR-023. Lessons: an existing test protected what mattered, because attack tests caught that even a flat crossover smears a kick, which led to bypassing it when centered; and a measurement needs its control case, because 40 ms spikes that looked like the effects also appeared with none.

## 2026-10-30 · Export to rekordbox.xml

Added **To rekordbox** in the library: a `rekordbox.xml` with cues, loops, hand-corrected grids and keys and curado's playlists, written only to a file chosen in a native dialog. Tracks with nothing to say are left out, unknown fields are omitted, and USB tracks are relocated to their original when exactly one file matches by name and size. Colors and labels no longer follow a cue that moved. Not yet imported into rekordbox. See [Status](STATUS.md) and ADR-021. Lesson: measuring early changed the risk, because only 77 of 2,802 USB tracks have a certain original in the places searched here.

## 2026-10-30 · Connected drives and rekordbox USB

Connected drives appear in the sidebar. A rekordbox USB shows its playlists, and its tracks load with their grids, keys and hot cues; any drive can be browsed for audio. Read only: edits stay in curado. The engine now has eight hot cues that can be loops. See [Status](STATUS.md) and ADR-020. Lesson: real data corrects assumptions, because the maintainer's own cues showed rekordbox's bar numbers were reliable and curado's bars better than earlier agreement figures suggested.

## 2026-10-30 · Long tracks and real-time key lock

Raised the per-track limit from 96 MiB to about 90 minutes under a shared 3 GiB limit, kept in RAM rather than streamed after measuring track lengths and decode speed. Moved key lock into the callback, so tempo and key apply at once and the offline preparation worker is gone. Fixed a crash in the echo that predates this work. See [Status](STATUS.md) and ADR-019. Lessons: a plan is a hypothesis, because streaming looked necessary until decoding an hour took five seconds; and a test double can hide what it tests, because the first allocator hook crashed from its own thread-local access and could have passed without measuring anything.

## 2026-10-30 · Whole-track beat analysis

Replaced the 90-second estimate with three-band onsets, epoch-folded tempo over the whole track, a grid placed just before the real attack, bars and first-sound detection. Added bar lines, **compás aquí** and `bun run music:eval`, which compares with stored rekordbox grids. On tracks not used for tuning, BPM within ±0.02 went from 14.1 % to 61.0 % and phase within 5 ms from 4.4 % to 40.4 %. Bars barely changed. See [Status](STATUS.md) and ADR-018. Lessons: measure the premise before fixing it, because trimming MP3 delay looked like the correction and moved grids away from rekordbox; and a test tolerance is not evidence, because synthetic kicks passed while real tracks did not.

## 2026-10-30 · Clean transport and limiter

Audited the engine and fixed the transport and output findings. Loop boundaries, jumps, pause and PCM swaps crossfade through a second read voice, and cues and play pre-roll 3 ms so their attack stays at full gain. Quantize uses the nearest beat when saving and never moves a saved cue; sync takes the short way to the master's beat, nudge works while synced and a paused follower keeps its position. A −1 dBTP look-ahead true-peak limiter replaces the ±0.98 clamp, with a LIM lamp in both mix layouts. Adds 3 ms to cue and play attacks and 1.6 ms of output latency at 48 kHz. Measured, not yet listened to; see [Status](STATUS.md) and ADR-017. Lesson: the old ±20 ms grid test tolerance hid a 13–25 ms bias, so tolerances need to reflect what is audible.

## 2026-10-29 · Shared console pads and keyboard map

Each console deck can switch its existing performance pad surface between its three hot cues and curado's four shared sample slots. Empty sample pads load the selected library track; Shift-click replaces a loaded sample. Pad modes are temporary frontend state and do not alter transport or shortcuts. Moved the reactive keyboard map into deck B with pointed/last-action descriptions. Hardware pad modes and extra cues remain pending; visual, interaction and listening review remain pending.

## 2026-10-29 · Integrated deck controls

Moved the curado effects, musical controls, linked transport and sample bank from the detached side/below panels into the console chassis below the relevant deck pads, following the maintainer's screenshots. Collapsed the keyboard map and full mixer in console view to avoid an oversized section below. Existing action and audio contracts are unchanged; visual and interaction review remain pending.

## 2026-10-29 · Console side controls

Responded to the maintainer's screenshot showing unused space around the fitted console. Per-deck curado filters, echo and musical controls, linked transport and the existing sample bank now occupy side rails in wide views and reflow below the console at smaller widths; the full mixer remains expandable. Existing native actions and pending hardware functions are unchanged. Visual and interaction review remain pending.

## 2026-10-29 · Console viewport fitting

Responded to maintainer screenshots showing clipped console controls and empty stretched decks. Fixed the hardware surface proportions, positioned the pads below the jogs rather than at the stretched bottom, corrected the master meter height, and fit the surface to the available sheet size with a minimum scale and scrolling fallback. No audio actions or DSP changed. Visual and pointer interaction verification remain pending.

## 2026-10-29 · RX3-inspired console refinement

Replaced the first console arrangement with a dedicated CSS-drawn dark hardware surface: central screen, channel strips and FX sections, side jogs, transport and eight pads per deck. Existing functionality uses the native action contract; unimplemented controls remain visibly dimmed and accessibly labelled as coming soon. Curado's filters, echo and musical controls remain available in an expandable section. This is not a photograph, exact replica or new DSP. Visual, listening and hardware review remain pending.

## 2026-10-29 · Reactive Metal scenes

Added a faceted gateway with particles, circular waveform/spectrum and depth-tested low-poly spectral city. Added temporary scene, glow, sensitivity, speed and color controls; hide/restore, fullscreen and spring camera interactions. A bounded on-demand master capture feeds a worker-side 2,048-point FFT and smoothed 24-bin spectrum; no FFT or allocation runs in the callback. Updated both READMEs and architecture. See STATUS for validation and limits.

## 2026-10-29 · Alternate console view

Added a locally persisted curado/console layout selector in Mix. The console uses the existing deck, mixer, waveform and action contracts in an all-in-one-inspired surface, preserving theme colors and horizontally scrolling at narrow widths. No engine or DSP behavior changed. MIXER-VIEWS records the visual limits and a dedicated audio, routing and player-function backlog; visual and listening review remain pending.

## 2026-10-29 · Mixer view research

Reviewed the current mixer, deck, audio and input contracts against the XDJ-RX3, DJM-750MK2, DJM-A9 and CDJ-3000X. Added a proposed two-deck console direction and a source-linked capability gap matrix in MIXER-VIEWS. This is research and documentation only; no mixer implementation, app launch, visual or listening test.

## 2026-10-29 · Metal visual variations

Added selectable orbit, tunnel and waveform-ribbon scenes alongside the existing terrain, with accent-based gradients and adjustable procedural glow. The native helper retains its bounded 30 FPS and audio snapshot boundary; controls are temporary and the integrated canvas is unchanged. No FFT or spectrum analysis was added. Compilation and validation details, including remaining visual checks, are recorded in STATUS.

## 2026-10-29 · English UI in progress

Added a persisted frontend language preference, Spanish fallback, initial English translations for major UI and action help, and corrected outdated audio settings copy. English remains explicitly marked in progress: native messages/menus and other strings still need coverage. A browser preview verified switching and persistence; native demo mix and Metal viewer captures were added, plus an English settings browser preview. No listening test; full screenshot and localization review remains pending.

## 2026-10-29 · Interface review and README navigation

- Added matching English and Spanish maps of the app's main views and clarified the behavior of SYNC, analysis and web audio.
- Inspected the installed native interface with explicit user authorization. No listening test or new screenshot; the live session and its library were left unchanged.
- Recorded the localization scope and outdated audio-settings text in STATUS. English UI support remains unimplemented.

## 2026-10-26 · English docs and a bilingual welcome

Reworked the public README around listening/mixing/first use, with reciprocal English/Spanish links, the user's development mixer capture and nu9ve.xyz credit. Added SETUP with agent prompts, prerequisite checks, installation/update steps, platform distinctions and troubleshooting. Translated technical docs and AGENTS into English while retaining Spanish maintainer communication and existing UI labels. Documentation-only work is explicitly exempt from rebuilding/reinstalling an unchanged app; required checks still apply.

Added LICENSING: MIT recommendation for commercial-fork freedom, free/donation/paid distribution options, Apache/MPL/GPL comparison and unresolved owner/contribution/brand/dependency obligations. No project license, metadata grant, price or payment setup applied. Existing dependency notices are not a complete distribution audit. Format scripts include README.es.md; 73 local links/images checked. Evidence in STATUS; no new app launch or visual/audio test.

## 2026-10-26 · Radio inside the main window

Replaced external service windows with a child WebView integrated in radio/web. React reserves/updates area for layout/scroll/resize; Rust validates bounds. Leaving, mini or service switch closes it/stops web audio. Settings/playlist dialogs hide temporarily without stopping. Serialized operations avoid recreation after teardown/switch races.

Same Tauri 2.12.5, new unstable feature for add_child (ADR-012). Capability selects only main WebView, not whole window. Tray/single-instance query Window to work with children. Isolation/navigation policy and separate audio retained; no HTML iframe/local CSP change. Bounds/permissions/child/main recovery contracts tested; actual visual/remote playback unverified. STATUS records evidence.

## 2026-10-26 · fractos radio and isolated web explorer

Owner confirmed fractos uses YouTube/SoundCloud iframes. Initial radio/web launcher opened separate reusable windows with explicit close-to-stop; UI/help explained independent audio outside decks/master/FX/sync/panic. No media-source integration/fractos modification. This external-window approach was superseded by the integrated browser above.

HTTPS exact-host policy, incognito, no popups/downloads; close-to-tray main-only. AppManifest explicitly registered 26 commands, main-only ACL plus handler label rejection. Contracts compare registry/handler/capability and URL policy. CSP/core/native audio unchanged, no new dependency. No app launch or remote playback/login test; evidence/limits in STATUS.

## 2026-10-26 · BPM/key, quantize, sync, FX and local installation

Offline `music.rs` analyzes ≤90 s: attack/autocorrelation/refinement BPM/offset and FFT/chroma/profiles major/minor key. Versioned per-ID settings, no schema change, save on import/load, editable/reanalyzable. Library letter/Camelot BPM/key; decks add master/sync/tempo/default-Q/beat loops/transpose/key lock/key sync/grid-key editor.

Renderer retains clock: beat-boundary loops, tempo/phase following and SYNC+Q follower cue at next beat. Context/panic cancel waiting cues. Signalsmith Stretch 0.1.3 MIT worker prepares pitch, no callback FFT/C++/allocation, preserves frames/compensates latency. Coalesced requests, stale cancellation and ticket/revision validation; block-end ack also for rejection. Old audio/transport continue until result applies; explicit errors/retry.

Mixer adds independent HP and beat echo with delay interpolation/sample-rate smoothing/denormal cleanup, eight-second preallocated delay. EQ/ceiling remain basic, not mastering DSP.

`desktop:install`: default build, per-user destination, identity/signature/hash verification, staging/rollback, LaunchServices, no automatic launch/quit. AGENTS/docs updated, MIT notices bundled, exact dependency rationale ADR-014, install ADR-015. At that session: 22 TS/38 Rust tests and callback/music probes; no physical audio/visual/real-corpus certification. Streaming/global memory remain structural priorities.

## 2026-10-26 · Context help and reliable A/B transport

Requested fixed expandable hints and diagnosis of silent B on rapid cues. Screenshot showed B paused. Two regressions reproduced old `Cue` seeking without starting after pause/EOF. Changed to atomic jump+play per slot without interrupting other deck. Seek/nudge preserve pause; out-of-loop jumps disable loop.

Linked A+B play/pause/cue actions use one queue entry/mixer buttons, without BPM/sample/gain/master changes. Deck transport/routing diagnostics and 0:00 initial cue labels; keyboard-accessible detailed waveform.

Queue uses atomic load+play and single-trigger repeat-one; revision-tagged prefetch ignores stale errors/results and bounds failed-file retries. `ContextHelp` delegates hover/focus, exact identifiers/current remaps and fallback labels. Deck/theme tone, reserved footer expansion/notice relocation; help Escape closes without panic. No new dependencies/permissions.

Tests: EOF cue, independent L/R retriggers at 44.1/48/96 kHz, samples/transport/shared PCM, loops/queue bounds/allocator; TS exact cues/mappings/diagnostics/disclosure markup. No app launch/Computer Use/physical audio. Five major deliveries still open, later musical foundation recorded separately.

## 2026-10-25 · Base 0.1

Started from empty directory: independent Rust audio workspace, Tauri 2 shell, React/Vite/Bun UI. Symphonia/CPAL, six slots (two decks/four samples), basic DSP, validated controls/bounded queues, worker reclamation, parameter smoothing/manual transport transitions.

SQLite versioned/transactional schema, referenced library, favorites/playlists, validated import, remappable keys, MIDI Learn, tray, manual device recovery, native playback queue; 30 FPS integrated visual and AppKit/MetalKit helper with reduced telemetry.

Lessons: Bun/Rust/Xcode enabled small local ARM64 bundle; chosen CPAL baseline does not imply native PipeWire/ASIO/JACK. Services need explicit lock order; snapshots release output before MIDI/queue. Never free last PCM reference in callback. Streaming/global memory next. Session evidence in [Status](STATUS.md); release requirements in [Quality](QUALITY.md).

### Coral, themes and native integration

Exact `#d66154`, light/dark/system, neutral/tonal/high-contrast, presets/hex/HSV. Semantic tokens, adaptive contrast, initial cache/versioned SQLite; Canvas/Metal share accent. Geometric icon regeneration. macOS Overlay/hidden title keeps native controls.

Shared atomic track cues/worker autosave, load-ack tickets, visible queue repeat and native persisted UI/keyboard/MIDI mode policy. v1→v2 preserves references/prevents ID reuse. Swift module cache moved into OUT_DIR for reproducible builds independent of global caches.

User stopped visual testing; later changes verified by code/build/tests. Contrast/window flags are not visual review. Bundle resource seal/ad hoc signing fixed while respecting configured identities. Desktop crate emits only rlib, avoiding huge static artifacts that exhausted disk. Final tests/probes in STATUS.

## 2026-10-25 · Redesign, three-band waveforms and coordination

User requested performance/coordination/style/usability review. Lowercase **curado** branding.

Engine/services: new decoding-pass `analysis.rs` produces `[peak, low, mid, high]` at 50 bins/s using EQ cuts. SQLite v3 waveform BLOB/backfill, raw `track_waveform` IPC, no peaks in library listing. Queue prefetch at 15 s, repeat-one seek. MIDI EQ unity corrected to CC64 (previous center −2.4 dB and could not reach +6 dB). Removed duplicate queue disable.

UI: continuous chrome/elevated sheet, flat sections; keyboard-shaped console with central knobs/faders/dB meters/crossfader, A/B platters/vertical pitch, stacked detail waveforms, samples/clickable keymap. Key feedback/HUD, static overview/transformed overlays/detail tiles, mini view, Settings Escape.

Theme v2: five styles, type/texture/waveform controls, user color; generated Bayer textures/local Fontsource 5.3.0 fonts. Shared pixel ring/axle logo/UI/SVG/icon. Added SESSIONS S1–S10, ADR-009…012 and public README. Lessons: split static drawing/moving overlays; analysis fits decode pass; contrast tests both materials. No visual inspection during this redesign by preference.

### Authorized first visual review

Later authorized web-preview review at 1100/1440 px using `bun run dev`, no audio. Fixed drag text selection with `-webkit-user-select`/preventDefault/manual pointerdown focus. Chrome grain changed from 2 px/0.28 opacity to 1 px/0.10. Mixer made smooth; physical knob caps/LED glow/value-on-touch; fader grip; platter/pitch above full-width cue/transport (previously squeezed at 1440). Native app with tracks/other four styles still unreviewed.
