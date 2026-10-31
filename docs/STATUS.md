# curado status

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
- **Bars:** barely changed. Backbeat energy disagrees with rekordbox's numbering, so raising its weight lowered agreement. rekordbox's local files hold no cues to use as an independent check. Treat bars as a first guess; they are display-only and sync still works per beat.
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
- Linear playback interpolation and basic EQ; offline pitch is not continuous low-latency streaming DSP.
- UI position/detail/platters at 20 Hz, 50 ms steps (S2).
- MIDI feedback not mirrored in UI (S7); manual MIDI connect, no Clock/relative encoders/LEDs/physical controller verification.
- One stereo output, no independent headphone cue, ASIO/JACK/native PipeWire.
- Manual device recovery, no media keys/Now Playing/MPRIS/SMTC/video/plugins.
- Remote browser audio outside engine; compatibility/login/layout pending.
- `color-mix()`/container units require Safari 16+ WebKit.

## Implementation lessons

Waveform analysis fits the decode cleaning/copy pass without extra PCM retention, useful for streaming. Peak-free library listings avoid large IPC payloads. Repainting/resizing canvas on every snapshot dominated UI cost; static drawing plus transformed overlays removes that work. Web Animations give per-key feedback without React renders/reflow. These implementation gains do not remove the measured scheduling outlier or missing hardware evidence.
