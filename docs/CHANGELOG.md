# Change history

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
