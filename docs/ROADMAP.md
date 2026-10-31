# Priorities and dependencies

Large work topics have dedicated briefs in [Sessions](SESSIONS.md), S1…S10.

## Agreed delivery sequence · 2026-10-26

Footer help and A/B transport fixed a concrete problem but did not complete musical synchronization. The subsequent musical session adds the sync/grid/key-lock foundation recorded in STATUS. No delivery replaces QUALITY gates.

| Delivery                         | Dependencies                     | Remaining verifiable completion                                                                  |
| -------------------------------- | -------------------------------- | ------------------------------------------------------------------------------------------------ |
| 1 · Long tracks and import       | S1                               | Done in RAM with a global limit; remaining: import progress and cancellation                     |
| 2 · Smooth transport and session | S2 native clock; session storage | Smooth playhead without drift; action acknowledgement; coherent paused deck/loop restoration     |
| 3 · Quality and recovery         | S3; device lifecycle             | Aliasing/discontinuity/limiter/RT measurements; hotplug, sleep/wake, deterministic shutdown      |
| 4 · Everyday player              | Robust library/queue             | Editable queue, shuffle, reorderable playlists, media keys, albums/artwork and moved-file repair |
| 5 · Advanced mixing              | Clock/DSP; S4/routing            | Editable BPM/grid, beat loops and sync; then validated key lock and hardware headphone cue       |

Executed three-OS CI, codec/error corpus, two-hour stress, accessibility and signed distribution must accompany progress. S1 remains the structural priority; raising the PCM limit is not a replacement for a memory budget.

## P0 · Before production stability

1. **Long tracks (S1):** resolved in RAM with a global limit, see ADR-019. Remaining: import progress and cancellation, a corrupt-file corpus and decode time limits. Reconsider streaming only for tracks over about 90 minutes or machines without the memory.
2. **Native session contracts and smooth position clock (S2):** mode/track cues already persist. Finish deck loops/session, transactional transport acknowledgements and paused restoration. Do not persist incoherent snapshot positions.
3. **Audio quality (S3):** loop/jump crossfades and the true-peak limiter exist, pending listening. Bandlimited resampler, isolator EQ, 24 dB filters and auto gain exist, measured. Remaining: listening and comparison with hardware.
4. **OS/device resilience:** hotplug/sleep-wake, controlled reconnect without surprise volume; deterministic thread teardown/join; recoverable DB errors. Measure UI/input/DAC/callback under load.
5. **Distribution/physical testing (S10):** executed three-OS CI, signed in-bundle helper, signing/notarization, dependency/license audit, two-hour stress; explicit Linux exit without usable tray. Project license decision is documented in [Licensing](LICENSING.md), still unapplied.

## P1 · Complete player

- Editable queue/shuffle; next-track prefetch exists. Real gapless depends on the native clock.
- Reorderable playlists, real tags, artwork/albums, file relocation and cancellable incremental scans.
- Theme/style/mode persist; finish navigation state and validated startup output.
- Alternate views (S6): mini exists; compact/focus/stage/floating visual and per-view size persistence remain.
- macOS template tray icon/now-playing (S8).
- Media keys/macOS Now Playing, Windows SMTC, Linux MPRIS; opt-in autostart/file associations.
- Accessibility: modal focus trap/restoration, Escape, audited contrast beyond palette math, dynamic scaling/screen reader.

## P1 · rekordbox and devices

- Try the cue bus on a real interface, keys that press the shown pad, and hand tracking for the XY surfaces.
- USB reading and `rekordbox.xml` export exist (S11). Next: confirm the export in rekordbox, then import an XML from rekordbox, which also gives exact file locations, then writing to a USB only with validation on real players.
- Memory cues in the transport, queueing from a device playlist, tracks across a device and the local library, and Engine DJ libraries.

## P1 · Deeper mixing

- S4 foundation exists: whole-track BPM/key, editable grid and bars, beat loops, native sync/deferred cues and a benchmark against stored rekordbox grids. Need listening, ground truth for phase and bars, a variable-tempo strategy and uncertain-grid handling.
- Real-time key lock and transposition exist. Need listening quality and better transposition of low notes; preserve position and transport contracts.
- Multichannel cue/master output and pair selection; ASIO/JACK/PipeWire per platform require configuration/routing/UI/hardware tests.
- Continuous jog/nudge by dragging platter, timestamped actions, relative MIDI, hotplug, LEDs/UI MIDI feedback (S7).
- Native keyboard capture only if measurements justify it; global shortcuts opt-in with conflict/permission review.

## P2 · Expansion

- Shared wgpu GPU renderer → Metal/D3D/Vulkan; worker FFT/timestamped smoothed musical features; adaptive GPU budget.
- Scene presets, 3D waveform, landscapes, set recording/export with explicit permissions.
- VST/AU hosting only with fault isolation/license review.
- Integrated isolated web browser exists for SoundCloud/fractos (S5/ADR-012), audio outside engine. Validate players/cookies/login and design fractos transport/volume protocol. Native source integration requires authorized streaming.
- Video/network/music-service integration into engine are separate projects, not implied Symphonia/Tauri features.
- Shareable validated theme-v2 export/import and more styles.

Every delivery updates STATUS/CHANGELOG, adds contract evidence and reevaluates affected behavior. Avoid simultaneous DSP/scheduling/visual changes that obscure performance or failure attribution.
