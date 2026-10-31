# Quality and release criteria

curado 0.1 is a functional local-testing foundation. Production readiness requires evidence for every gate below; no row is assumed complete.

## Gates

| Gate         | Required evidence                                                                                 |
| ------------ | ------------------------------------------------------------------------------------------------- |
| Build        | Reproducible release from lockfiles on three OS; fmt/lint/types/tests passing                     |
| Real time    | No callback alloc/dealloc/locks; load/jitter tests; physical keyboard/MIDI→DAC latency            |
| Stability    | Two-hour sessions, six sources + FX, concurrent import/visuals, no glitches/crashes/memory growth |
| Devices      | Hotplug, no output, default changes, rejected buffers, sleep/wake/recovery                        |
| Files        | Mono/stereo 44.1/48/96 kHz MP3/FLAC/WAV/AAC/ALAC/AIFF/Vorbis; corrupt/large/Unicode/moved paths   |
| Persistence  | Restart, migrations, backup/restore, corrupt DB, full disk, missing references                    |
| Input        | ES/US layouts, focus/text, conflicts/repeats, real three-OS MIDI/dense events                     |
| Distribution | macOS/helper signing/notarization, Windows signing, Linux packages, licenses/notices              |
| UX           | Tab/focus, modal Escape, screen reader, contrast, scaling, remap labels and visual style review   |

## Automated checks

```sh
bun run check
bun run build
bun run format:check
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Engine tests cover transport/seek/EOF/loops/panic/crossfader/bounds/rate conversion/backpressure/mode transitions/shared cues. Allocator audit covers zero alloc/dealloc in tested render operations, including clip replacement, seek/filter/cue. WAV tests include mono/stereo/garbage; waveform tests cover 50 bins/s and 60 Hz/9 kHz band separation. SQLite covers relationships, playlist dedup/cascades/settings/v1→v3 migration and non-overwriting waveform backfill. Queue covers advance/repeat/prefetch timing/stale IDs/release.

TypeScript covers five-style palette, v1→v2 theme migration, waveform binary contract, Bayer/logo, window permissions, keyboard uniqueness/Shift/modifier/mode policy, hints, installation rollback and musical notation. Browser contracts scope ACL to the local WebView and check native child wiring; Rust checks URLs/bounds. Musical tests use synthetic clicks/triads and prepared audio. Latest actual counts/results are in STATUS.

## Manual local test plan

Added automated regressions on 2026-10-26 cover cue after EOF without interrupting the other deck; rapid L/R-separated retriggers at 44.1/48/96 kHz; independent samples/transports/shared PCM; atomic A+B/load+play under queue bounds; allocator; bounded failed/stale prefetch. Help checks text/mappings/diagnostics/accessibility markup. These do not validate perceived audio, real WebView interaction or footer appearance.

The following is a **plan**, not a completed test record. Respect current authorization before launching/inspecting the app:

1. Load demo, play A, switch to mix/play B, check both crossfader ends.
2. Save distinct cues; jump; IN/OUT/loop; vary rate/EQ/filter/echo/master.
3. Trigger Z/X/C/V during deck playback, repeat with focus on buttons/inputs.
4. Import 44.1/48 kHz fixtures; verify duration and unchanged pitch across source sample rates.
5. Playlist add/remove/favorite/search; restart persistence; removal must never delete originals.
6. Queue EOF/advance with hidden window; pause must not advance, panic must stop queue.
7. Open/minimize/close/reopen Metal, compare CPU/GPU/audio; close main and use tray.
8. Change output/buffer; failed configuration preserves prior output, success unloads/reloads slots.
9. Real MIDI learn/restart/disconnect/reconnect; inspect dropped/error state.
10. Integrated browser: native panel bounds/focus, resize/footer expansion, dialog hide/restore, switch/leave/mini stopping audio, blocked navigation/popups and player/login compatibility.

11. Connect a rekordbox USB: sidebar entry, playlists in order, load with grid and cues, pads and colors; unplug during playback; plug in another with the same name.

12. Export to rekordbox with its library backed up: import one local and one USB track; cues, loops, colors and corrected grid at the same positions, no duplicate, other fields untouched, and accents and symbols in paths.

16. Touch: taps per control on a Force Touch trackpad and their timing, whether `webkitForce` arrives and a deep press stays comfortable and does not open a lookup, and no effect on audio at the fastest drag.

15. Camera: permission prompt, MANOS and CARA, the light going off on leaving Mix or switching apps and back on return, unplugging a camera, another app holding it, CPU use with and without VER, and whether the XY overlay lines up with a real hand.

14. Keyboard in real use, every badge against its action, keys during a drag in the app's own WebKit, and how many keys a real keyboard takes at once. Release FX by ear. A 4-channel interface: cue, MIXING, LEVEL and Touch Preview apart from the mains. XY on a touchscreen with two hands.

13. Mixing screen in the real app: the console against the hardware, rail and window controls, every style and theme, position stream smoothness and cost. Platter feel: scratch, bend, scroll. Each color and beat effect on each channel and master, tails, EQ kills, roll, slip, reverse and brake on real music, with and without key lock, watching for dropouts. MIDI toggles and roll on a controller. A real variable-tempo track.

## Measurement

Synthetic clicks/triads do not certify musical accuracy, and `bun run music:eval` measures agreement with stored rekordbox grids, not truth. Before claiming professional analysis/sync: known-tempo and variable-tempo corpus, silent intros, human timing, unpitched percussion, ambiguous/modulating keys and every supported rate/transpose. Listen for transient/stereo-phase/bypass quality, loop fades on real material and limiter pumping; measure total memory during preparation/cancel/mixing under load. Music probes measure no DAC/perceived quality.

Process RSS alone is not Tauri's total memory: WebKit/GPU have separate processes. CPU percentage does not prove glitch freedom. Callback budget = frames/sample_rate, 256/48000 ≈5.33 ms. Compare P50/P95/P99/max under load. Measure end-to-end latency by loopback/timestamps: buffer + OS + hardware + scheduling.

Offline tests do not validate DAC/hardware jitter/perception. Do not claim exclusive backend, ASIO, extra RT priority or zero-copy without checking actual configuration. Preserve the historical callback outlier in STATUS; a later faster probe does not erase it.

## Known risks

Interpolation, EQ and filters are measured but not listened to, and effects have not been compared with hardware. Grids have only been compared with rekordbox's, not with listening; about one track in five disagrees on phase by over 50 ms, with no arbiter of who is right. Transport changes crossfade and main parameters smooth, but listening tests remain necessary. A 64-sample scope/RMS is not spectral analysis. Corrupt files can consume decode time before PCM limits: cancellation/timeouts/fuzzing missing. MIDI/native mode share policy, but real controller ergonomics remain unvalidated.

Appearance tests cover math/configuration, not screenshots. The authorized web-preview review covered curado at 1100/1440 px without audio; loaded native decks, alternate styles and later changes remain unreviewed. Further Computer Use/visual tests require explicit authorization. Existing screenshots may be reused in docs without becoming evidence of current validation.

## Release licensing

A license recommendation is not permission applied to code. Before any public paid or free release, resolve [Licensing](LICENSING.md), dependency/source obligations, exact bundled font notices and asset/brand rights. Neither local ad hoc signing nor an English README makes this a public release.
