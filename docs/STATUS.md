# curado status

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
| Musical control  | Estimated BPM/key, editable grid, beat loops, native sync, prepared pitch/key lock                 |
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

- Preloaded PCM ≤96 MiB/file (~4:22 at 48 kHz stereo); prefetch adds another clip; tuning adds source/result memory.
- Linear playback interpolation, basic EQ/ceiling, no loop crossfade; offline pitch is not continuous low-latency streaming DSP.
- UI position/detail/platters at 20 Hz, 50 ms steps (S2).
- MIDI feedback not mirrored in UI (S7); manual MIDI connect, no Clock/relative encoders/LEDs/physical controller verification.
- One stereo output, no independent headphone cue, ASIO/JACK/native PipeWire.
- Manual device recovery, no media keys/Now Playing/MPRIS/SMTC/video/plugins.
- Remote browser audio outside engine; compatibility/login/layout pending.
- `color-mix()`/container units require Safari 16+ WebKit.

## Implementation lessons

Waveform analysis fits the decode cleaning/copy pass without extra PCM retention, useful for streaming. Peak-free library listings avoid large IPC payloads. Repainting/resizing canvas on every snapshot dominated UI cost; static drawing plus transformed overlays removes that work. Web Animations give per-key feedback without React renders/reflow. These implementation gains do not remove the measured scheduling outlier or missing hardware evidence.
