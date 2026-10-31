# curado: continuing the project

Desktop audio application. Always write the name in lowercase: **curado**. Before changing code, read `docs/README.md`, `docs/STATUS.md`, `docs/SESSIONS.md` and the relevant sections of `docs/ARCHITECTURE.md`. Verified state takes precedence over product vision. Communicate with the maintainer in Spanish; write code, technical names and technical documentation in English. Keep `README.es.md` in Spanish and aligned with the English README. The app's existing Spanish UI is not translated by this documentation policy.

## Map

- `crates/audio-engine/`: Tauri-independent Rust core, decoding, DSP and CPAL.
- `src-tauri/src/`: SQLite library, commands, lifecycle, MIDI and playback queue.
- `src-tauri/native/CuradoVisuals.swift`: AppKit + MetalKit window, compiled only on macOS.
- `src/`: React UI, command bridge, keyboard layout, telemetry, themes/styles (`lib/theme.ts`), waveforms (`lib/waveform.ts`) and keyboard feedback (`lib/feedback.ts`).
- `crates/audio-engine/src/analysis.rs`: offline three-band waveform analysis during decoding, never in the callback.
- `scripts/preview/`: stand-in native side for documentation pictures and input checks, run only on request.
- `docs/`: decisions, contracts, priorities, tests and change history. Start installation work with `docs/SETUP.md`; licensing remains a maintainer decision documented in `docs/LICENSING.md`.

## User preferences

- Initial/main color: `#d66154`, light/dark/system themes and HSV customization. Styles: curado/pixel/gothic/y2k/minimal; the user retains their color within every style.
- Visual direction: flat, no cards or borders by default; continuous sidebar/topbar/footer chrome with dither grain, content on a smooth elevated sheet; pixel details and soft gradients; visual knobs, vertical faders and platters with animated keyboard feedback too.
- Do not use Computer Use or run visual app tests without a new explicit instruction. Verify compilation, automated tests and code contracts. Reusing an existing screenshot for documentation is not a new visual app test.
- macOS uses an integrated header with native controls; do not restore a separate title strip.

## Invariants

1. Audio callback: no locks, disk, network, logging, IPC, allocations or clip destruction. Preallocate buffers; bound commands and work per callback. PCM reclamation goes through `retired` to a worker.
2. The engine owns position and transport. UI displays snapshots; no timers inventing progress. MIDI uses the same action contract.
3. Never add WebAudio as a silent substitute for the engine. The browser previews the interface only.
4. Library entries reference files; removing tracks/playlists never deletes originals. Load registered IDs, not arbitrary paths supplied through IPC.
5. Compilation does not make a feature “tested.” Distinguish implemented, automatically tested, checked on Mac, pending hardware and pending other OS.
6. New DSP requires reviewing automation/smoothing, latency, buffers, bypass, denormals and callback budget. Mode changes require reviewing transport, queue, samples, focus and keyboard.
7. Keep CSP and permissions minimal. No remote scripts in the trusted frontend, exposed shell commands or generic JS filesystem access. Remote service WebViews remain isolated from native commands.
8. Do not routinely update dependencies. Record version and reason in decisions; retain both lockfiles.

## Closing a session

- Update `docs/STATUS.md` with evidence, limits and concrete next steps.
- Update `docs/CHANGELOG.md`; adjust architecture/decisions/contracts when changed.
- Run `bun run check`, `bun run build`, `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`.
- For packaging, Swift, bridge or lifecycle changes, build `.app` and test relevant contracts in code. Do not launch or visually inspect it while the preference above remains in effect.
- After verified app changes, install locally with `bun run desktop:install` (or `bun run desktop:install --no-build` immediately after building). Destination: `~/Applications/curado.app`, registered for Spotlight/Raycast. Never force an open instance to quit or interrupt audio; record a blocked update. Do not launch automatically. Documentation-only edits do not change the installed application and do not require rebuilding/reinstalling its unchanged bundle.
- Do not claim production readiness without meeting `docs/QUALITY.md`.
- Do not commit, publish, select a project license or modify the user's music without instruction.
