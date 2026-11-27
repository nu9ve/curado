# Integration with nu9ve/core

curado is a Tauri application with a TypeScript interface and a Rust audio engine.
macOS provides the native window and uses CoreAudio for output and CoreMIDI for
controller input. The Metal visuals run in a separate AppKit helper. The suite's
Swift and C# implementations target their own UI stacks. The shared contracts and
vectors can drive a TypeScript implementation with the same appearance across
curado's platforms.

## Current adoption

| Area                         | In curado                                                                                                                                                                                                                                                           | Next validation                                                                                                                                               |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Pin and gate                 | The [pin](../vendor/nu9ve-core/SOURCE) lists the exact revision. The [path list](../vendor/nu9ve-core.paths) selects the four app/theme/interaction contracts, the theme and era vectors, six Rust crates and the DDJ-SB profile. Pinned at `6d1d85a` (2026-11-27). | `bun run core:check` verifies the copy. Run the sync script with `--strict` to compare the selected content with the local core checkout.                     |
| Files and jobs               | `nu9ve-fs` makes rekordbox XML export durable. `nu9ve-json` is vendored as its dependency. `nu9ve-jobs` supervises the stems worker's process group on macOS and Linux.                                                                                             | Confirm XML interchange in rekordbox. Run a real separation and cancel it (`real_cancel`). Windows keeps its own runner until the suite has one.              |
| MIDI                         | `nu9ve-midi` v0.1.0 and the DDJ-SB v1 hardware profile from `1114e0244f49`. curado supplies the 95 default action mappings.                                                                                                                                         | Verify listening, jog calibration, reconnection and the unidentified CH 7 note 0x65 on hardware.                                                              |
| Theme                        | Five local styles; `suite-theme.ts` gives every shared role exactly in the 110 skin and 44 era vectors. SQLite holds curado's Theme v2.                                                                                                                             | Map the common roles to CSS tokens. See [Appearance](THEMES.md#suite-theme-integration).                                                                      |
| Native shell and interaction | Integrated native macOS controls, system title bars on Windows/Linux, local keyboard catalog, remapping and contextual help.                                                                                                                                        | Review native menus, window lifecycle and accessibility against the pinned contracts.                                                                         |
| Language                     | Spanish fallback and an in-progress English interface, stored in WebView local storage.                                                                                                                                                                             | Cover dynamic UI, accessibility labels, menus and errors in both languages. See [Appearance](THEMES.md#localization).                                         |
| Audio                        | `nu9ve-loudness` measures LUFS offline and `nu9ve-music` analyzes grid and key (`ANALYSIS_VERSION` 3). The real-time limiter is curado's.                                                                                                                           | Listen to auto gain with the new meter (up to about 0.3 dB more reduction). Limiter DSP is separate work.                                                     |
| Volumes and interchange      | Local file identity and relinking, read-only rekordbox access and XML export.                                                                                                                                                                                       | The new device and volume contracts are proposals. Evaluate compatibility and read-only discovery before introducing any suite marker or catalog integration. |

## Suggested delivery order

1. Establish the Mac hardware baseline with [DDJ-SB and LPLBZ](MIDI.md#macos-hardware-check).
   Use the same transport, pads, routing and reconnect checklist on Windows.
2. ~~Shared loudness meter~~ (2026-11-27, ADR-038): the old meter read
   0.07–0.26 LU lower on curado's own comparison. Listen to auto gain.
3. ~~Shared theme roles in TypeScript~~ (2026-11-27). Map them to CSS tokens.
   Add macOS appearance and bilingual coverage with the
   existing Tauri window, then validate the common skins on each OS. Keep the
   interface catalogs and native message keys independent of the theme calculation.
4. Revisit relinking with stable volume identity once the suite supplies a
   shared reader and migration policy. A read-only engine/catalog interface,
   shared stems supervision and multi-machine features can follow independently.

## Shared MIDI handoff

The extraction used the curado working tree based on `bb6f780`, including the
CoreMIDI baseline work from 2026-11-22. The [capture fixture](../vendor/nu9ve-core/rust/nu9ve-midi/tests/fixtures/ddj-sb-2026-11-22.txt)
has SHA-256 `d13ed8b92ab7ec56f68fa5fa99e3f370951e6b616dade2508ce296d41f627651`.
The source is the timestamped fixture that ships with the tests. The full
terminal capture and the earlier untracked Windows capture are unnecessary.

### Core extraction

Core delivered the portable reader and versioned DDJ-SB profile in
`1114e0244f496bb62367e3097935061776e69c97`. The crate documents the
[profile and calibration precedence](../vendor/nu9ve-core/rust/nu9ve-midi/README.md)
and provides a [control ID crosswalk](../vendor/nu9ve-core/rust/nu9ve-midi/ddj-sb-control-ids.md).
The profile records the captured Browse press (CH 7 note `0x41`) and the
unidentified WinMM gesture (CH 7 note `0x65`).

### curado adoption

The pin includes `rust/nu9ve-midi` and `data/midi/pioneer-ddj-sb-v1.json`.
See [Status](STATUS.md#shared-midi-adoption--2026-11-22) for local verification
and the remaining hardware checks.

- Pin that exact commit via `sync-core.sh` and consume its path dependency.
  Keep the `bun run midi:probe` entry point working. Keep native port ownership,
  reconnect, bounded queues, Tauri events, engine validation and actions in
  the curado adapter.
- Map the current default preset to shared control IDs and compare parity
  against the existing 125 control entries and 95 action bindings. In particular,
  preserve both LOAD targets, jog touch/release, encodings, filter initialization
  once per output generation, and Deck B's Mix-mode gating.
- Preserve saved custom bindings, legacy rows and remembered ports in
  `settings.midi` and `settings.midi-device`. A fresh device may get the default
  mapping. A previously customized device keeps its mappings until the person
  explicitly restores the preset. Verify presets, replay, app action tests and
  the frontend learning contract before building and installing locally.
- Track physical verification separately with the [DDJ-SB checklist](MIDI.md#macos-hardware-check).
  The open listening and reconnection checks do not block the offline extraction.

### Mapping profiles across apps

After curado adopts the common reader, validate it with a second app. Use one
hardware profile with multiple mappings identified by app and context. Each
app declares its own action IDs and supported value types. A shared format can
carry those IDs, transforms and user overrides, while the app validates them.
Start with local import/export and explicit profile selection. Device ownership
across running apps and LED feedback need a later decision informed by native
port behavior. No Swift/C# bridge is required until a client needs one.

### Absolute control pickup

`crates/midi/src/pickup.rs` is a candidate for `nu9ve-midi`. It accepts
normalized positions and a target, waits for proximity or a crossing, and
returns the exact target on capture. It keeps waiting if the parameter is
outside the current range. The app decides when to reset pickup, which
normalized range represents the parameter, and how to report the waiting
state. curado additionally rejects stale MIDI tempo commands at the engine
boundary using a revision. Test the primitive with a second parameter or
app before moving it to the shared crate.
