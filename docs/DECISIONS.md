# Technical decisions

## ADR-028 · Covers from rekordbox first, taps per input, a picture under the XY — accepted (2026-10-31)

The maintainer asked for covers in the track lists, haptics by kind of control with finer movement under pressure, and a faint camera image inside the XY surfaces.

- **rekordbox's picture before the file's.** On the maintainer's USB, 2 of 250 sampled MP3s appeared to embed a cover, while 1,131 of 2,800 exported tracks name a picture in `PIONEER/Artwork` and 1,120 of those exist. Those files are small and need no audio file opened. So `export.pdb`'s artwork table comes first and Symphonia's reader of embedded pictures second. The path comes from the stick, so it must resolve inside the drive and be an image.
- **A library id still has its picture.** A track from a drive plays by library id, so `artwork:<id>` keeps the path, refreshed on every import. If the drive is gone there is just no cover. The interface asks by id, never by path.
- **Thumbnails, not originals.** The interface reads covers only as they scroll into view, two at a time so a USB is not kept busy while a track loads, drops what scrolled past, and keeps up to 600 small JPEGs in memory. Tracks without a cover are remembered too. The pattern from the track id, always dithered, is what shows otherwise.
- **`NSHapticFeedbackManager`, without a dependency.** One command over a raw `objc_msgSend`, doing nothing elsewhere. Each input maps its own passages: an end is `generic`, an origin or center `alignment`, a step `level`. Knobs have 24 steps, sliders 16, and dials, Beat FX steps, the XY grid and platter dimples have theirs. Passing notches give way to one every 28 ms, since a fast drag crosses many. **Toques** turns them off.
- **Pressure means finer, up to 4×,** as Shift does, and they multiply. Each move counts by the factor at that moment, so pressing harder never jumps a value. A slider normally follows the pointer, so once fine it moves relatively until released. It reads `webkitForce`, which reports 1 at a click and 2 at a force click.
- **Overlay as a third state.** **VER** shows the picture in the panel, **XY** shows each side faintly inside its surface, and neither is on by default. The picture needs the same lifecycle as **VER**, so nothing else starts it. It is placed so a hand lands under its point, which stretches it horizontally. Being true to the point mattered more than the proportions.
- **A hand owns its half.** Each half of the view is now a whole surface, so a hand reaches all of it without crossing into the other's, and the overlay shows exactly that half. The preview went from 160 to 320 pixels wide, since half of it fills a surface.
- **Not verified:** taps and `webkitForce` were not felt on a Force Touch trackpad, and a Magic Mouse has no haptic hardware, so nothing can happen there. The overlay has not been seen with a real camera. By the maintainer's standing preference, no visual review was done of this work.

## ADR-027 · Camera tracking in a process that can be killed — accepted (2026-10-30)

The maintainer asked for hand and face tracking as an option, with the camera released whenever it is not in view and recovered cleanly.

- **A helper process, like the Metal window.** Stopping kills it, which frees the device whatever capture is doing, and a crash cannot reach the app or audio. Its input closing ends it too, so it cannot outlive curado holding the camera.
- **Apple Vision, on the device.** No dependency, nothing remote. Images never leave the Mac and are not stored. The preview is a small JPEG sent to the interface only while **VER** is on, because a preview nobody looks at is wasted work.
- **On only while asked, on this screen and in front.** Off on every launch, not saved. Leaving Mix, closing the console, hiding the window or switching to another app stops the helper; native code also stops it on losing focus, in case the interface does not get to run. Coming back starts a new one. This means tracking does not run with another app in front, which is the point.
- **Recovery by starting fresh,** with backoff from 1 s to 10 s, when the helper dies, the device disconnects or the session fails. A denied permission or a missing camera does not retry, and the interface says which.
- **A gesture never stays on.** Losing sight for 300 ms, a helper dying, or a stop all release to neutral through the same path as lifting a finger.
- **Gestures:** a hand moves its own side's XY and acts only while pinching, since a hand in view is not a decision; pinch has hysteresis. A face moves XY 1 and acts on leaning in. Both are guesses at what will feel right, not yet tried.
- **Not verified on a real camera:** the lifecycle is tested against a stand-in process, and the interface against simulated frames. Publisher signing with the hardened runtime would need the camera entitlement. An ad hoc rebuild can make macOS ask permission again.

## ADR-026 · Keys by hand, a cue bus, and panels beyond the hardware — accepted (2026-10-30)

- **Each hand is a deck, and a modifier keeps one meaning.** Shift is the second function, Control the destructive or further one, Option the samples. Command stays the system's. The list moves with arrows, including a drive's, since that is where tracks come from. Toggles flip what the engine reports, because a key only knows it was pressed. A saved layout from before is left stored but not used.
- **Keys go through the same actions as everything else,** except moving through a list, which only the interface knows about.
- **Badges read from the bindings,** so remapping updates them, and they come from what a control already declares for feedback.
- **Keys hit hot cues whatever the pad mode.** Making them press the shown pad needs the mode in native code. Left for later.
- **A cue bus on outputs 3 and 4.** Touch Preview means nothing without it, and it also gives the channel CUE, MIXING and LEVEL a function, on any device with 4 or more outputs. A stereo output has nowhere to send it, so preview plays at half level through the speakers and says so, since that is useful at home and dangerous in front of people. **Not tried on a real interface.**
- **Release FX stop the deck,** except Build Up and Ducking, which last while held. Echo Out turns on the channel's echo, cuts the track after half a beat and puts the unit's settings back six seconds later.
- **Phrases are an energy estimate in the interface.** The maintainer's USB has no rekordbox phrase data in 400 files checked, and without it there was nothing to validate against, so they carry no claim to know a real verse or chorus. They come from the waveform already loaded and need no storage.
- **XY surfaces use mouse and touch for now.** Their mapping takes a point from any source. Hand tracking would add a camera, a permission and native Vision code, so it is its own piece of work.

## ADR-025 · Tempo maps, decode limits and a position stream — accepted (2026-10-30)

- **Beat maps next to the one-tempo grid.** A track whose tempo moves carries every beat, from a rekordbox grid with more than 0.1 BPM of spread or, when requested, from analysis that follows the pulse in overlapping windows. Everything local (quantize, loops, jumps, FX clock) reads the beat around the read head. A correction by hand wins, since it is one tempo. Sync matches local tempos and adds a small pull, at most 1 %, because matching speeds alone lets beats wander.
- **curado's own detector only runs when asked, per track.** On 348 tracks of the maintainer's USB it answered yes on 27 %, even after being tightened from 47 %. rekordbox has a constant grid on all of them, so most of those are probably false alarms, and a wrong map would damage a good grid. rekordbox's own variable grids apply automatically.
- **60 Hz position, not extrapolation.** The S2 choice: only the two positions, only while a deck moves, drawn once per screen refresh. Nothing predicts, so invariant 2 holds and the display cannot lead the engine. When reports stop for 120 ms the full state speaks again. IPC cost has not been measured in the running app.
- **Files are outside data.** Decoding stops at 180 s, a decoder panic becomes an error for that file, damaged frames are skipped up to 500 in a row, and a sample rate out of range or changing partway is refused.
- **Key:** measured against rekordbox on 986 tracks of a USB, it now always answers instead of on a quarter of tracks. A filter on correlation was refusing most tracks. Limiting notes to 65 Hz–1 kHz and removing third harmonics took exact agreement on tracks not used for tuning from 31 % to 41–45 %, and fifths from 26 % to 15 %. rekordbox is a reference, and this USB's keys may come from other tools.
- **Phase, not changed.** Five other ways to choose the beat were compared on 524 tracks. The most promising cut half-beat errors on the tuning set but not on the rest in the real pipeline, so it was reverted. About 11 % of same-tempo tracks still disagree by half a beat and 10 % by a quarter, with nothing to say who is right, so the grid editor gained a **½ beat** button.
- **Cues at 0:00 sit a microsecond in.** Zero means "no cue" in every store, in exports and in the interface. A sentinel changes nothing else and needs no migration.

## ADR-024 · The console follows the hardware's plan — accepted (2026-10-30)

The maintainer rejected an arrangement that only resembled an all-in-one, sent the product page, and asked for platters always present and playable. The console now follows the official [hardware diagram](https://downloads.support.alphatheta.com/software_info/all-in-one-dj-systems/XDJ-RX3/XDJ-RX3_HardwareDiagram_rekordbox_E1.pdf), with control positions measured from it.

- **Plan copied, identity not.** Layout, control names and their order follow the hardware, which the maintainer asked for explicitly. Logos, photos and artwork are not used, and the brand printed is curado's. Commercial distribution would still need the brand and design review noted in Mixer views.
- **Fixed coordinates, scaled as a whole,** like the object it follows. Below half size the console scrolls in its panel.
- **Platters always, and they behave like platters.** The top scratches with VINYL on and bends with it off, the edge always bends, a scroll gesture bends, and a stopped record moves under the hand. A click no longer plays or pauses here, since the hardware's platter does not.
- **Pads as printed:** HOT CUE, BEAT LOOP, SLIP LOOP, BEAT JUMP, and with SHIFT or their second label KEYBOARD, KEY SHIFT, PAD FX and SAMPLER. An empty hot cue pad saves, a saved one calls and SHIFT deletes.
- **Sound Color per channel stays,** which the maintainer asked for and the hardware does not do: each button applies to both channels, and its 1 and 2 to one.
- **What has no hardware function here stays visible and labelled unavailable:** mic, AUX, input switches, booth, headphones and channel cue. They need input and output routing that cannot be checked without that equipment.
- **curado's own tools moved to the side panel:** LP/HP, beat echo, grid and key editing, linked transport and the full mixer.
- **Differs on purpose:** MEMORY and DELETE act on hot cues because memory cues are not in the transport yet; KEYBOARD transposes and plays hot cue A; Release FX, X-Pad, Touch Preview and phrase display do not exist.

## ADR-023 · Effects per channel, and a transparent path — accepted (2026-10-30)

The maintainer asked for the RX3's missing effects and functions, with different effects on different channels, plus the audio quality items.

- **Each channel owns its effects.** Hardware shares one Sound Color selection and one Beat FX unit. Here each music channel has its own color effect, and CH 1, CH 2 and master each have a Beat FX unit, so they can differ and run at once. One panel edits the unit it shows.
- **Signal path:** trim, EQ, LP/HP, color, fader, echo, beat, then crossfader, master unit, master, limiter. Color goes before the fader and beat after it, as on a mixer, so a closed fader still lets a tail ring out.
- **Effects implemented:** Space, Dub Echo, Sweep, Noise, Crush, Filter; Delay, Echo, Ping Pong, Spiral, Reverb, Trans, Filter, Flanger, Phaser, Pitch, Slip Roll, Roll, Vinyl Brake, Helix. Original algorithms named for what they do, based on public descriptions. Their sound was not compared with hardware.
- **Real-time rules:** every buffer is written once at creation, so the callback neither allocates nor takes a first-write page fault; work per frame is fixed; the allocator audit covers every effect and transport mode. Selecting Reverb clears about 120 KB, which is bounded and only happens on a command.
- **A centered EQ leaves the path.** Linkwitz-Riley crossovers give a real kill and a flat sum, but even flat they spread an attack in time: transport tests caught a first sample at 0.18 instead of 0.3. So bands at unity bypass them and transients stay sample-exact. Moving a band crossfades them in over about 5 ms. LP and HP bypass the same way.
- **Sinc instead of linear interpolation:** 24 taps, Kaiser window, six tables by read speed so nothing folds back above the new Nyquist. A sample read in place at normal speed is untouched. Tables build before the first callback, and reading them takes no lock.
- **Auto gain defaults to off,** because changing levels unasked would surprise anyone who set them by ear. BS.1770 loudness is measured on the loading worker; it moves a track to −14 LUFS, within ±12 dB, and the limiter still catches peaks.
- **Slip uses a silent shadow position** that advances at normal speed during a loop, reverse, scratch or roll, and the return uses the existing crossfade. Key lock gives way to plain playback in reverse, scratch and brake, since a stretcher cannot follow them, and returns afterwards.
- **Not attempted: headphone cue, SOURCE and booth.** They need a second output and device routing that cannot be verified without that hardware, so these controls stay labelled unavailable.

## ADR-022 · Mixing as a fixed screen — accepted (2026-10-30)

The maintainer showed the console cut off by a scrolling page, with the library far below, and asked for a fixed view with internal scrolling and a compact sidebar.

- **Nothing scrolls but lists.** In Mix the sheet stops scrolling. The console fits its panel through the existing scale, and the panel's lists and tools scroll inside themselves. Below 0.65 scale the console scrolls in its own panel, because controls that small stop being usable.
- **Console left, side panel right,** with a draggable divider. Tabs hold library, tools and keyboard. The panel collapses for a mixing focus. An earlier version also let the platters be hidden; the maintainer asked for them always, so that option is gone (ADR-024).
- **The rail is per mode:** on by default in Mix, off in Player, since mixing needs the room and listening does not. Playlists show two letters because many lists sharing one icon cannot be told apart. A source selector in the panel reaches everything the rail abbreviates, including a drive's nested playlists.
- **The footer's playback row is hidden in Mix,** since the decks already have transport.
- **Preferences stay in local storage,** like the existing layout choice, because they are one viewer's conveniences. Each field falls back on its own if the stored data is old or wrong.

## ADR-021 · rekordbox.xml export, conservative by default — accepted (2026-10-30)

The maintainer wanted cues set in curado to reach rekordbox. `rekordbox.xml` is the documented route, needs no reverse engineering and leaves the decision to import with the user. It has **not been imported into rekordbox yet**: everything below about import behavior comes from third-party documentation and needs the maintainer's confirmation.

- **Destination from a native dialog.** The interface cannot name a path. The file is written through a temporary file and a rename, so a failure leaves any earlier export intact. Nothing is written to a drive unless the user picks it.
- **Only what has something to say.** Third parties report that importing a track overwrites its cues, grid and tags in rekordbox. So the collection holds tracks with cues set or edited here, or a grid corrected by hand, plus tracks in curado playlists. A drive's track nobody edited is left out, because resending it could undo newer work in rekordbox.
- **Estimated grids and keys are not sent.** Only `manual` corrections are. An estimate would replace rekordbox's analysis, and a one-tempo grid would flatten its variable ones.
- **Unknown fields are omitted, not written blank,** because curado never sees comments, genres or ratings. The placeholder artist is not sent either.
- **Stale styles are dropped.** A color or label describes the position rekordbox saved it for, so it goes away once curado moves that cue. The interface applies the same rule.
- **A `curado` folder with a `curado cues` playlist,** so playlists cannot land beside a same-named one in rekordbox, and importing "what changed" does not need the tracks that are only listed.
- **Locating originals.** rekordbox identifies a track by location, and its collection knows the computer's file, not the USB copy. A file with the same name and size under `~/Music` or a drive without a rekordbox export replaces the USB path, only if there is exactly one. Two copies leave no way to know which it uses. Without one, the USB path stays and the report warns that rekordbox will treat it as another track. Searching is bounded by 2 million entries and 120 s, never follows links and skips hidden folders. Comparing names by Unicode normalization changed nothing on 469 non-ASCII names, so exact bytes stay and no dependency was added.
- **Why not read rekordbox's own paths:** `master.db` is encrypted and stays closed. An XML exported by rekordbox would give exact locations, and importing one is the natural next step.
- **Sources:** [official XML format](https://cdn.rekordbox.com/files/20200410160904/xml_format_list.pdf), [pyrekordbox](https://pyrekordbox.readthedocs.io/en/stable/formats/xml.html), [Lexicon](https://www.lexicondj.com/manual/sync-rekordbox-xml), [DJ Tools](https://a-rich.github.io/DJ-Tools-dev-docs/conceptual_guides/rekordbox_collection/), [Mixed In Key](https://mixedinkey.com/rekordbox-cue-points/), [MIXO](https://www.mixo.dj/guides/tracks-to-rekordbox-via-xml). The official format omits `Red`, `Green` and `Blue` on `POSITION_MARK` and lists only three hot cue numbers; those come from third-party descriptions of rekordbox's exports.

## ADR-020 · Read-only device access with its own reader — accepted (2026-10-30)

The maintainer asked to see USB drives in the sidebar, browse their playlists and files, and integrate rekordbox's formats.

- **Read only, for now.** The original request included setting cues from here, but writing a device file has the highest cost of failure in this project: undocumented details decide whether hardware accepts a USB, and a mistake could damage a set's only copy. Edits stay in curado. `rekordbox.xml` is the next step because it is documented and rekordbox itself does the writing.
- **Own reader instead of `rekordcrate`.** The formats needed are small, `rekordcrate` describes itself as under heavy development and would add `binrw` and its tree to code handling untrusted files, against keeping dependencies minimal. It stays the reference to check against. The format knowledge comes from Deep Symmetry's public Crate Digger analysis; review this for interoperability before commercial distribution.
- **`export.pdb`, not `exportLibrary.db`.** OneLibrary needs a SQLCipher key that belongs to rekordbox. rekordbox still wrote both on this USB. A USB with only OneLibrary shows as plain files, which is honest and still playable.
- **Opaque ids instead of paths.** Invariant 4 forbids loading arbitrary paths from IPC. A drive's data is also hostile: `export.pdb` can contain `../` or a link, so everything is resolved natively and checked after following links.
- **Device tracks are not library tracks.** They need a stable id for cues and analysis, but 2,800 tracks should not flood a local library because someone plugged in a USB. A settings mark avoids a schema migration.
- **Eight hot cues in the engine,** so an imported cue is never lost. Keyboard defaults stay at three, to avoid changing a layout people have learned.

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
