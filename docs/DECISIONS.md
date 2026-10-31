# Technical decisions

## ADR-019 · Long tracks in RAM and key lock in the callback — accepted (2026-10-30)

The maintainer asked for long tracks and real-time key lock. Both decisions came from measuring first, and the first replaces the streaming plan in S1 and ADR-003.

**In RAM, with a global limit, instead of streaming.** In the maintainer's library of 31,477 tracks, 25–32 % exceeded the old 96 MiB cap, but only 1.5 % exceed 10 minutes, 0.15 % an hour and 0.02 % 90 minutes. Decoding is far faster than expected: an hour-long MP3 took 5.1 s including analysis, using 1.2 GiB. Streaming would save memory on a tiny fraction of tracks in exchange for the failure a DJ can least accept: a callback waiting on a disk or USB stick. In RAM, playback cannot underrun from storage, a pulled drive does not stop the set, seeks and cues are instant and the callback's rules stay simple. Accepted costs: about 21 MiB per minute, a 3 GiB total, and a wait of seconds on very long files. Reconsider for tracks beyond about 90 minutes or machines without the memory.

**Key lock inside the callback, reversing ADR-014.** Preparing a whole clip made sync wait about 3 s on a four-minute track, kept up to 384 MiB extra and could not follow a moving tempo. Running on a worker with a ring buffer was rejected: jumps would have to flush and refill it, a non-real-time thread can miss deadlines, and timing would depend on scheduling. The callback is viable because Signalsmith allocates nothing after creation and costs 70–140 µs per deck per 256-frame block, and because PCM in RAM lets a voice read ahead so its latency is not heard.

- **Window 4096 at 48 kHz:** it kept kicks within 0.42 dB, against 0.56 dB for 2048 and about 1 dB for 3072 or the library's default preset window, and halved transposition errors. Accepted cost: about 1 ms per start.
- **Resample before processing:** correcting a sample-rate difference by transposing added pitch error to almost every track, because most of the library is 44.1 kHz.
- **Hidden warm-up of half a window** instead of a longer pre-roll, so hot cues keep their 3 ms and their attack. Accepted cost: the start spike. Decks at 64 frames could overrun on two simultaneous starts.
- **No stored crossfade for the mode switch beyond the existing voices:** two voices per deck cover every jump.

## ADR-018 · Whole-track grid, folding and an untrimmed timeline — accepted (2026-10-30)

The maintainer asked for accurate tempo, kick and bar detection compatible with rekordbox libraries. Choices were made against stored rekordbox grids, on a tuning set separate from the reported one.

- **Keep encoder delay:** the audit assumed trimming MP3 delay was right. Measuring both showed rekordbox's lines at the attack on the untrimmed timeline, and trimmed grids 25 ms off. Keeping it needs no cue migration and lets rekordbox positions match. Reconsider for real gapless playback, which could trim at playback without moving the timeline.
- **Epoch folding over autocorrelation alone:** a whole track resolves tempo far beyond one window, and survives breakdowns because it adds evidence instead of tracking. It costs about twice the analysis time, and more on long files, which is acceptable in a worker and bounded by limits on attacks, candidates and search steps.
- **Folded band rise over circular mean:** a mean is pulled between kick and off-beat. Requiring bass and broadband to rise together keeps a louder bass note or hi-hat from taking the grid.
- **1 ms before the attack:** the maintainer's concern was cutting kicks. rekordbox's lines sit about 2 ms later, which is close enough to mix, and starting slightly early cannot cut a transient.
- **Follow rekordbox's half/double convention:** it is the only reference available and it is what this maintainer's library uses. Accepted cost: drum and bass can read at half tempo, fixed with ×2.
- **`downbeat` as a position:** a beat index would break whenever BPM or offset are edited. No schema change: it lives in the versioned JSON, and `ANALYSIS_VERSION` updates old estimates on load while keeping manual ones.
- **Rejected for now:** dynamic grids for variable tempo, which need a new data model and evidence; bar-aware sync, while bars are this uncertain; decrypting `master.db` for reference data.

## ADR-017 · Pre-roll crossfades, exact cues and a true-peak limiter — accepted (2026-10-30)

An audit requested by the maintainer confirmed loop clicks, softened attacks and hard clipping. Replacing the old `gain × envelope` ramp and held-sample transition with a second read voice costs little and fixes the cause rather than the symptom.

- **Pre-roll over a shorter fade:** a fade on the destination audio always attenuates its attack; the old path lost 17.7 dB in the first 2 ms from pause. Starting 3 ms early delivers the target at unity. Accepted cost: 3 ms more between a command and the attack it targets. Loop boundaries have no such cost because both voices are scheduled ahead.
- **Raised cosine over equal power:** equal power adds up to 3 dB on correlated audio, and loop material is often correlated. A brief dip on uncorrelated audio is preferred to adding level before a limiter.
- **Exact cues:** quantize re-snapping on recall destroyed off-grid cues and would destroy imported ones. Q now applies when setting; a synced follower still uses its nearest grid line at trigger time, because staying in phase is what the user asked for there, without changing the saved cue.
- **Nudge beats sync:** realigning undid every nudge, which removed the only by-ear correction for a wrong grid. A nudge now lasts until the next realignment. No stored offset was added; revisit with continuous sync.
- **Limiter over soft clipping:** a look-ahead limiter stays transparent below the ceiling and distorts far less above it (55 Hz at ×3: −29 dBc against −11 dBc for the old clamp). Accepted cost: 1.6 ms latency at 48 kHz and about 6 µs per 256-frame block. −1 dBTP over the previous ±0.98 leaves room for converter overs. The default gain staging stayed unchanged: defaults do not reach the limiter with two 0 dBFS decks. Reconsider ceiling, release and gain staging after listening tests.

## ADR-016 · English documentation and bilingual entry points — accepted (2026-10-26)

The maintainer requested English documentation, a friendly README, a Spanish welcome/README, agent-assisted setup, mixer imagery and a nu9ve.xyz credit. English is the technical documentation language; README.es.md remains Spanish, with reciprocal top links. Existing Spanish UI labels are retained in usage instructions; this is not UI localization. SETUP is the user/agent installation path; AGENTS retains engineering invariants and Spanish communication with the maintainer.

Reuse the user-provided mixer capture, labeled as a development session rather than current release verification. No new visual test or app launch is implied. Documentation-only sessions run required checks but need not rebuild/reinstall an unchanged native app.

The maintainer prefers maximum freedom, including commercial forks, and is considering free downloads, donations and paid editions. LICENSING recommends MIT and compares Apache/MPL/GPL and commercial implications; **no project license is applied**. Holder/year, explicit license selection, contribution/brand/asset terms and complete dependency notices/source obligations remain separate decisions. Do not treat a recommendation as a license grant.

## ADR-014 · Editable musical analysis, native sync and prepared pitch — pitch superseded by ADR-019 (2026-10-26)

Requested: BPM, letter/Camelot key, transposition, key lock/sync, default quantize and synchronized loops. Offline analysis is bounded to 90 s, using attack autocorrelation/refinement and chroma/[Krumhansl-Kessler profiles documented by Humdrum](https://extras.humdrum.org/man/keycor/). Synthetic success is not product accuracy. Offset/BPM/key are editable and stored by ID in settings, without migration or original-tag changes. Camelot follows its [creator's reference](https://mixedinkey.com/camelot-wheel/0/); letter/Camelot represent the same key.

New exact dependency **signalsmith-stretch =0.1.3**: MIT Rust wrapper and C++ library. [Official API/license](https://github.com/Signalsmith-Audio/signalsmith-stretch); notices in `licenses/signalsmith-stretch` and bundle resources. Cargo.lock includes transitives; existing dependencies were not routinely updated and bun.lock stays. C++/libclang/bindgen are build requirements.

Prepare equal-length/sample-rate PCM outside callback with input/output latency compensation. This avoids callback FFT/C++/allocations while clips remain fully loaded; playback rate stays native. Accepted costs: preparation delay, extra source/result memory and quality pending corpus/listening. It is neither continuous pitch DSP nor streaming; S1 must move to budgeted incremental preparation with latency measurement.

Engine owns Q/tempo/phase/deferred cues; React timers never sync audio. Sync does not start transport or convert major/minor mode. One-shot key sync selects nearest compatible tonic/relative. Generation/ticket/revision validate batches; even rejected batches acknowledge so current work can retry.

## ADR-015 · Local installer and LaunchServices — accepted (2026-10-26)

Requested: leave an updated local app after app changes for Spotlight/Raycast. `desktop:install` builds by default, installs at `~/Applications/curado.app` without sudo, verifies identifier/signature/executable hash, stages before replacing, renames with rollback and registers LaunchServices. No preferences/library/original changes, automatic launch or forced quit. It refuses an open installed app to avoid replacing playing resources. Indexing/manual launch are separate from copy/registration checks. Documentation-only edits do not change that binary.

## ADR-013 · Playing hot cues and linked transport — accepted (2026-10-26)

B stayed silent when retriggered while A played. `Cue` sought but preserved `playing=false` after pause/EOF; two regressions failed before the fix. Hot cue now jumps+plays in one command. Save cue and seek retain distinct contracts. Jumping outside loop disables it.

Deck state was already independent and summed in one renderer. Separate devices/streams would not fix cue semantics and would complicate clocks. Explicit `playBoth`/`pauseBoth`/`cueBoth` share one queue entry, including at the 64-command callback boundary. No routing/volume changes; linked transport itself is not beat sync. Physical playback/latency still need hardware tests.

Queue load+play and repeat each use one entry to avoid partial acceptance. Session revision invalidates old prefetch successes/errors. Failures allow one early attempt and one on advance, not repeated decode every tick.

Context help uses exact action IDs/current mappings with label/title fallback, not visual proximity. Text is maintained in code; help sends no engine controls.

## ADR-001 · Rust + Tauri 2 + React + Bun — accepted

Audio/OS services need native access; React serves library/decks. Bun manages packages/tools, not audio runtime. Tauri uses system WebView. UI is not AppKit/WinUI; menus/tray/audio/Metal helper use native APIs.

Reconsider JUCE/C++ when immediate professional routing/plugins/controller SDKs or available DSP justify FFI cost. Assess component licenses/maturity before commercial distribution; one C++ DSP library is not a reason to migrate the whole project.

References: [Tauri architecture](https://v2.tauri.app/concept/architecture/), [commands](https://v2.tauri.app/develop/calling-rust/), [tray](https://v2.tauri.app/learn/system-tray/).

## ADR-002 · CPAL 0.16 and Symphonia 0.5.5 — accepted for beta

Known bounded APIs were chosen. Newer releases do not imply their features exist here; lockfile determines resolution. CPAL 0.16 uses CoreAudio/WASAPI/default ALSA; optional ASIO/JACK are off. PipeWire may work via ALSA/Pulse compatibility, not an advertised native backend.

Symphonia `all` enables included codecs, not every codec. App validates mono/stereo. AAC-LC depends on profile/file; no HE-AAC/Opus/DRM promise. Library capability alone does not give app gapless behavior. Symphonia's MPL-2.0 distribution obligations need review independently of the project's future license.

Update after fixture/API/latency/OS tests. [CPAL 0.16](https://docs.rs/cpal/0.16.0/cpal/), [Symphonia 0.5.5](https://github.com/pdeljanov/Symphonia/tree/v0.5.5).

## ADR-003 · Bounded preloaded PCM — superseded by ADR-019

Simple cue/seek/loops, no callback IO. 96 MiB prevents unbounded reads; about 262 s at 48 kHz stereo float32, half at 96 kHz. Costs: rejecting long files and no strict total budget. Streaming/windowed buffers around cues are next; keep preload for short samples.

## ADR-004 · Native musical state, snapshot UI — accepted

Renderer owns transport/DSP; SQLite persists data; React owns navigation, not clock. MIDI/tray do not depend on UI. Focused keyboard events/direct IPC retain accessibility/portability. If latency measurements fail, add native per-OS capture/timestamps aligned to device clock. Current shortcuts are not sample accurate.

## ADR-005 · SQLite and referenced files — accepted

Transactions/relationships with explicit initial migration. No copying all music, tag edits or original deletion. Moving a file breaks its reference; repair/file watching pending. Cues/mode persist, deck loops/positions/state remain temporary. v2 AUTOINCREMENT prevents deleted IDs from identifying another file and inheriting cues.

## ADR-006 · Metal separated from audio — accepted for macOS prototype

AppKit/MTKView demonstrates native GPU rendering and UI isolation, with 20 Hz reduced telemetry. GPU renders images; Metal is not low-latency DSP optimization. Do not move short-block effects to GPU without measuring synchronization/transfers/shared Apple Silicon load.

Before distribution, use signed in-bundle helper. Consider wgpu→Metal/D3D/Vulkan parity with same telemetry contract. [MetalKit](https://developer.apple.com/documentation/metalkit).

## ADR-007 · Honest state and quality gates — accepted

Compilation proves neither live stability, perfect isolation nor physical controller support. Record verifiable state/limits per feature. Local testing is available; production readiness requires QUALITY.

## ADR-008 · Semantic color and integrated header — accepted

User chose `#d66154` and light/dark/variant/HSV controls. One pure TypeScript palette feeds CSS/Canvas; Metal receives reduced RGB. Separate exact accent from readable text tone. SQLite preferences/local first-paint cache never enter engine.

macOS keeps native controls with Overlay/hidden title integrated into header, without HTML system-button replicas. User requests no further Computer Use/visual tests unless explicitly authorized; config/code/build tests are not visual review.

## ADR-009 · Local fonts through Fontsource — accepted

Requested Jersey 15 and pixel lettering. CSP disallows remote fonts, so bundle exact **5.3.0** `@fontsource/*`: Jersey 15 display, Pixelify Sans labels (requested “pixel basel” unavailable on Google Fonts/Fontsource), Geist Mono numbers, UnifrakturMaguntia gothic display, Michroma y2k. Latin subset covers Spanish, ~120 KB woff2. All OFL 1.1: include exact family notices on distribution. General UI remains system font for legibility/native feel. Reconsider for other alphabets/variable fonts.

## ADR-010 · Offline binary three-band waveform — accepted

Compute in existing decoding pass, 50 bins/s `[peak, low, mid, high]` u8 using EQ splits; schema-v3 BLOB/raw ArrayBuffer IPC. Rejected: FFT/bin (cost/memory without visible gain here), float JSON (~10× bytes), peaks in `get_library` (poor scaling). Club colors: bass blue/mids amber/highs white; optional RGB/mono. Revisit with streaming/beatgrid reuse (S1/S4).

## ADR-011 · Token-based styles — accepted

curado/pixel/gothic/y2k/minimal are `palette()` seeds plus `data-style` rules; user keeps their color. Typography/texture independently default to style. Theme v2 SQLite, v1 migrates in TS/Rust. Theme-generated Bayer textures, no external images. Arbitrary user CSS skins rejected for CSP/injection/layout risk.

## ADR-012 · Isolated integrated web browser — accepted, foundation implemented

User requested internal browsing after external windows. A native child WebView reserves space in React, avoiding reliance on providers permitting HTML framing under local origin. Fixed URLs: `https://www.fractos.app/es/radio` and SoundCloud Discover. Owner confirms fractos embeds YouTube/SoundCloud; audio remains outside engine/mixer/sync/panic.

Resolved **tauri 2.12.5** gates `Window::add_child` behind **unstable**. Enable feature without version/lockfile changes. Future minor APIs may change; review contract on upgrade. React controls logical bounds validated by Rust. Actual size/focus/player compatibility remains untested. Switching/stopping/leaving closes child; dialogs temporarily hide it without stopping. Tray/single-instance use Window for multi-WebView compatibility.

HTTPS exact service/widget hosts, incognito, no popups/downloads; local CSP unchanged. AppManifest enumerates commands for generated ACL. **webviews: [main]**, with no `windows`, grants only local frontend (window scope would include children). Additional label rejection in handler. No remote grant/generic creation command. Permissions regenerate from app_commands.rs; tests compare registry/handler/capability and URL/bounds policy, not actual playback/cookies/login.

Iframe volume/transport mixing needs a fractos protocol and its own session. It supplies no PCM for native EQ/sync; direct authorized sources/native streaming are separate work. No injection or embed-audio extraction. [Tauri capabilities](https://v2.tauri.app/security/capabilities/), [SoundCloud Widget API](https://developers.soundcloud.com/docs/api/html5-widget). See S5.
