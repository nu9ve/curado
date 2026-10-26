# Platforms, dependencies and distribution

Start with [Setup](SETUP.md) for the install workflow and agent prompt.

## macOS / Apple Silicon

Bun, stable Rust, full Xcode, CLI tools and macOS SDK. Build uses `xcrun swiftc` for AppKit/MetalKit. Declared minimum: macOS 12; helper matches Rust architecture/deployment target 12. Older OS compatibility is unverified. Current local builds are native ARM64.

Signalsmith Stretch 0.1.3 requires C++ compilation and bindgen/libclang. Full Xcode supplies tools on this Mac; if bindgen cannot locate libclang, set `LIBCLANG_PATH` to the actual LLVM/Xcode library directory. These are build requirements, not runtime services/permissions.

```sh
bun install --frozen-lockfile
bun run desktop
bun run desktop:build
```

Output: `target/release/bundle/macos/curado.app`. Lowercase product name; identifier remains `studio.curado.player`, preserving data/preferences. Build script locally signs ad hoc and verifies resource seal, respecting a configured distribution identity. This is not notarization. Public distribution needs Developer ID, hardened runtime, a signed in-bundle helper, minimum entitlements and notarization. No microphone permission: app only plays. Current focused hotkeys need neither Accessibility nor Input Monitoring.

Data: `~/Library/Application Support/studio.curado.player/curado.sqlite` and `demo/`. Helper cache: `~/Library/Caches/studio.curado.player/curado visuals.app`; older `Curado Visuals.app` caches may be removed manually, not automatically. SQLite can have `-wal`/`-shm`; use a consistent SQLite backup or quit before copying.

Per-user installation/update, without sudo or automatic launch:

```sh
bun run desktop:install
bun run desktop:install --no-build
bun run desktop:install --check
```

Destination: `~/Applications/curado.app`. Script validates signature/identifier, rejects symlink/other-app destinations, verifies copied executable SHA-256, stages then renames with rollback and registers LaunchServices. It refuses to interrupt an open installed app. Identifier/data remain unchanged. Spotlight/Raycast indexing may lag; registration does not prove launch. Quit any other checkout instance before launching installed app or single-instance may focus the old process. `--check` validates the bundle/intended destination without installing; it still checks running-instance safety.

## Windows

Bun, Rust MSVC, Microsoft C++ Build Tools, Windows SDK and WebView2. CPAL uses WASAPI; ASIO is not enabled. midir uses WinMM; Tauri provides tray. Run `bun tauri build` on Windows for packaging; `desktop:build` uses Mac-only `--bundles app` and signing. Local installer is Mac-only.

Signalsmith needs LLVM/libclang for bindgen and C++; its Windows configuration/execution still needs CI/hardware verification. Pending physical checks: shared USB/Bluetooth/interface audio, suspend, device changes, DPI and keyboard layouts. CPAL alone does not establish ASIO support; feature/SDK/build and separate evidence are required.

## Linux

Bun, Rust, C/C++ tools, Tauri GTK/WebKit dependencies and ALSA. Ubuntu 24.04 example:

```sh
sudo apt-get update
sudo apt-get install build-essential pkg-config libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libasound2-dev libclang-dev patchelf
bun install --frozen-lockfile
bun run desktop
```

This build uses ALSA; PipeWire/Pulse compatibility depends on system configuration. CPAL 0.16 native PipeWire/JACK are not enabled. midir uses ALSA MIDI. Tray availability depends on desktop/extensions, especially GNOME. Wayland can restrict global shortcuts; these shortcuts are focused. Validate explicit exit behavior without a working indicator: close-to-hide applies if tray was created.

Integrated visuals use shared UI. Metal button is macOS-only; a cross-platform native renderer (wgpu) remains pending. Multi-WebView behavior in the integrated browser also needs platform verification.

## Cross-platform validation

CI is configured for macOS, Windows and Ubuntu. A workflow file does not prove execution; a CI build does not validate drivers, hotplug, hardware or tray. Record runs/hardware matrix in STATUS/QUALITY.

General requirements: [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/). App does not install drivers/system services. Tauri's integrated child WebView currently requires its `unstable` feature; see ADR-012 before upgrades.

## Icons and fonts

`scripts/icon.py` generates 1024 px icon and `docs/assets/logo.svg` from the same geometry as `src/components/Logo.tsx`: pixel c-ring/axle, coral-to-amber dither, dark squircle. `bun tauri icon <png>` regenerates `src-tauri/icons` for supported targets, without external image-generation dependencies.

Fonts are bundled (`@fontsource/*` 5.3.0, Latin, OFL 1.1), never fetched at runtime. Include each family's exact notices when distributing. Signalsmith notices already live in `licenses/`; this is not a complete dependency/font audit. See [Licensing](LICENSING.md).
