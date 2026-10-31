# Dedicated sessions

Topics large enough to deserve a separate objective, evidence and closure, in suggested priority order. On starting one, mark its scope in STATUS as in progress; on completion record evidence in CHANGELOG. Format: context, scope, done when, risks.

## S1 · Long tracks and global memory budget (P0)

**Implemented 2026-10-30 without streaming:** see ADR-019 and Status. A 60-minute track plays, seeks and loops with no callback allocation and a global memory limit; playback touches no disk.

- **Remaining:** import progress and cancellation, a corrupt-file corpus and decode time limits; waveform UI with hour-long tracks, unseen so far.
- **Reconsider streaming when:** tracks over about 90 minutes matter, or target machines cannot spare the memory. Its risks are distant seeks in unindexed codecs, cue latency and dropouts from slow drives.

## S11 · rekordbox interchange (P1)

**USB reading implemented 2026-10-30:** see Status and ADR-020. **XML export implemented 2026-10-30, not yet confirmed in rekordbox:** see ADR-021.

- **Context:** curado reads a rekordbox USB and exports an XML, but cannot read an XML from rekordbox.
- **Scope:** confirm the export, then import `rekordbox.xml`, the documented route that needs no reverse engineering. Writing `export.pdb` and `ANLZ` files comes after, and only to exports curado itself created.
- **Done when:** cues set in curado appear in rekordbox and on a real player, confirmed by the maintainer.
- **Risks:** undocumented details of the formats decide whether hardware accepts a USB; never touching an existing library without a backup.

## S2 · Position clock and smooth playhead (P0/UX)

- **Context:** 20 Hz telemetry moves waveform/platters/playhead in 50 ms steps; UI must not invent progress.
- **Scope:** decide between 60 Hz position-only lightweight telemetry, or callback timestamp/rate snapshots with bounded extrapolation corrected every snapshot. Record ADR; UI must never lead engine by more than one interval.
- **Done when:** 60 FPS detail/platter without measurable drift (extrapolated versus snapshot test), measured IPC cost.
- **Risks:** violating invariant 2 with unbounded extrapolation; WebView load.

## S3 · Audio quality: resampler, EQ and limiter (P0)

**Partly implemented 2026-10-30:** transport/loop crossfades and a true-peak limiter, verified by measurements but not by listening.

- **Context:** linear interpolation and one-pole EQ.
- **Scope:** bandlimited sinc/polyphase resampler, DJ biquad EQ with full kill, bipolar HP/LP. Review automation/smoothing, denormals, latency, bypass and callback budget (invariant 6).
- **Done when:** aliasing tests, new offline P99 probe, no callback allocation.

## S4 · Beatgrid, BPM, key and sync (P1 mixing)

**Foundation implemented 2026-10-26:** offline analysis, editable grid, quantized loops, native sync, letter/Camelot notation and pitch/key lock, now real time. **Whole-track analysis 2026-10-30:** see Status for agreement with stored rekordbox grids. Quality is not complete: listening, ground truth for phase and bars, and variable tempo remain.

- **Context:** three-band waveform exists; rhythmic structure needs validation/refinement.
- **Scope:** offline BPM/onsets in decode worker near `analysis.rs`, persistent grid metadata (currently versioned settings in SQLite, see ARCHITECTURE), visible beat markers, quantized loops/tempo sync. Extend initial key lock to incremental streaming and validate quality.
- **Done when:** known-BPM corpus within ±0.1 BPM, measured with `bun run music:eval` and listening; 1/2/4/8-beat loops without discontinuity on real tracks.

## S5 · Web explorer: SoundCloud/fractos radio (P2)

**Foundation implemented 2026-10-26:** native browser inside main, replacing external windows; closes on leaving/switching; WebView-scoped ACL and URL/bounds tests. Confirmed fractos HTTPS URL returns HTTP 200. Real WebView players remain unverified; no native mixing.

- **Context:** explore external services without leaving the app.
- **Scope:** isolated child WebView with exact-domain policy, no native IPC/files/commands. Capability selects only the `main` WebView, not its entire window. WebKit audio does not enter engine/mixer/recording. See ADR-012.
- **Done when:** contract tests deny remote permissions, preserve local CSP and restrict navigation; real player compatibility needs separate platform evidence.
- **Next:** validate embeds/cookies/login under incognito without popups. Now-playing/transport/volume require a protocol with fractos, whose maintainer confirms YouTube/SoundCloud iframes. Widgets provide no raw PCM source for native decks.
- **Risks:** provider terms, third-party cookies and confusion over audio ownership.

## S6 · Alternate views (P1 UX)

- **Context:** always-on-top 460×132 mini exists; other variants do not.
- **Scope:** compact library/player, focus/stage decks/waveform fullscreen, floating visual, per-view size/position and shortcut.
- **Done when:** transport/queue/samples/focus/keyboard survive changes; size restoration covered by contracts.

## S7 · MIDI feedback in UI (P1)

- **Context:** keyboard actions illuminate controls/HUD; native MIDI actions do not.
- **Scope:** bounded/coalesced `input:action` worker event reusing `feedback.ts`, controller LEDs and relative encoders.
- **Done when:** real mapped controller messages update UI without harming MIDI→engine latency.

## S8 · macOS template tray icon (P1 polish)

- **Context:** menu bar uses colored app icon.
- **Scope:** monochrome `icon_as_template`, playback/now-playing menu state.
- **Done when:** correct light/dark appearance; requires explicit authorization for visual validation.

## S9 · Accessibility and visual audit (P1)

- **Context:** palette contrast is mathematically tested across styles/colors/modes/variants; full visual audit remains unavailable by preference. See STATUS for the limited earlier web-preview review.
- **Scope:** modal focus trap, knob/fader screen-reader behavior, text scaling and all-five-style visual review when authorized.

## S10 · Distribution and executed CI (P0)

- **Context:** CI configured but execution unverified; local ad hoc bundle.
- **Scope:** run all three OS jobs, signed in-bundle Metal helper, notarization, licenses/notices including font OFL, and repository license. See [Licensing](LICENSING.md); recommendation is not an applied license.
