# Set up curado

This guide builds curado from source. macOS/Apple Silicon is the locally verified build environment. Windows and Linux have shared source and CI configuration, but recorded hardware/CI validation is still pending. See [Platforms](PLATFORMS.md) for the exact differences and [Status](STATUS.md) for evidence.

## Give this to your agent

Open the repository in your coding agent and paste:

```text
Set up curado on this computer. Read AGENTS.md, docs/SETUP.md, docs/STATUS.md
and docs/PLATFORMS.md before acting. Check the OS, available tools and local
changes. Preserve both lockfiles and existing work. Identify missing prerequisites
and use the normal permission flow before installing system tools.
Install JS dependencies with bun install --frozen-lockfile, then run the checks
in this guide. On macOS, build/install with bun run desktop:install.
Do not launch the app, stop another instance, modify music, commit or publish.
Report the installed path, verification results and remaining manual checks.
```

The same instructions can be requested in Spanish; [README.es.md](../README.es.md) includes a Spanish prompt. A coding agent needs terminal/filesystem access to this checkout. No specific agent vendor is required, and the prompt does not bypass that agent's permission model.

## 1. Get the source and prerequisites

```sh
git clone https://github.com/nu9ve/curado.git
cd curado
```

Already have a checkout? Use it; inspect local changes before updating. Do not reset or overwrite local work to follow this guide.

Required: Git, Bun (the baseline README uses 1.4+), stable Rust/Cargo, the platform's Tauri dependencies, a C++ compiler and libclang for Signalsmith's bindgen step. macOS also needs full Xcode and its CLI tools for AppKit/MetalKit compilation with `xcrun swiftc`.

```sh
bun --version
rustc --version
cargo --version
# macOS only:
xcode-select -p
xcrun --find swiftc
```

Install missing system tools using their official installers/package manager, following [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) and [Platforms](PLATFORMS.md). Do not silently replace toolchains or upgrade project dependencies. Build requirements are not runtime microphone or Accessibility permissions.

## 2. Install dependencies and check the checkout

```sh
bun install --frozen-lockfile
bun run check
bun run build
bun run format:check
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Cargo downloads the versions in `Cargo.lock` as needed. Keep `Cargo.lock` and `bun.lock`. Record failures; a successful build does not validate audio hardware or perceived sound quality.

## 3. Build and install on macOS

```sh
bun run desktop:install
```

This builds the frontend and native app, verifies local signing and installs at `~/Applications/curado.app`. It registers LaunchServices for Spotlight/Raycast without launching. It needs permission to write outside a sandboxed checkout. No sudo is needed for the per-user destination.

If you just completed `bun run desktop:build` and verified its output:

```sh
bun run desktop:install --no-build
```

`bun run desktop:install --check` validates the existing source bundle without installing; see the script output for the actual bundle identity. The build output is `target/release/bundle/macos/curado.app`. Local signing is ad hoc, not Developer ID distribution/notarization.

Quit an installed instance before replacing it. The installer refuses to interrupt it, rejects unexpected identities/symlink destinations and stages a verified copy with rollback. Library and settings keep the same app identifier. An instance launched from another checkout can capture the single-instance activation: quit that instance before opening the installed app yourself.

## 4. Try it when ready

Launch `curado.app` yourself, or search in Spotlight/Raycast once indexed. Choose **sesión de prueba** to generate two short tracks and four samples locally. No music download is needed. First-use steps and keys are in [Controls](CONTROLS.md).

Agents should not launch or visually inspect the app without explicit authorization. A screenshot already in the README is documentation, not proof of testing the current build. The app's visible labels are currently Spanish.

For development, `bun run desktop` starts the native app and dev server. `bun run dev` starts a browser UI preview without native audio. The commands launch interactive software; an agent should respect the user's current launch/test preference.

## Updating your local app

After source changes have passed checks, run `bun run desktop:install` again. Use `--no-build` only for a bundle just built from the intended source. Do not claim an update installed when the installer reports an open app or permission failure. Documentation-only changes do not change the installed binary.

Windows/Linux: use `bun run desktop` for development and `bun tauri build` for native packaging on that OS. The Mac install/signing scripts are not cross-platform installers. See [Platforms](PLATFORMS.md).

## Troubleshooting

| Symptom                                       | Next step                                                                                                                      |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| Missing GTK/WebKit/ALSA, SDK or C++ toolchain | Install the OS prerequisites in Platforms; keep dependency versions unchanged                                                  |
| bindgen cannot find libclang                  | Install the appropriate LLVM/Xcode components; point `LIBCLANG_PATH` at their real library directory if needed                 |
| Build succeeds, app is absent in launcher     | Check `~/Applications/curado.app` and installer registration output; indexing and launch remain separate manual checks         |
| Installer reports curado is running           | Quit it normally, then rerun; do not force-kill an audio session                                                               |
| Browser preview has no audio                  | Use the desktop runtime; do not add a WebAudio fallback                                                                        |
| Long file is rejected                         | Limits are about 90 minutes per track and 3 GiB of decoded audio in total; load a shorter track on the deck holding a long one |
| Output fails after disconnect                 | Select an available device and recover manually in Configuración → Audio; successful reconfiguration unloads slots             |
| Remote player/login does not work             | WebView compatibility remains unverified; external login popups/hosts are restricted. Do not relax local CSP/IPC to fix it     |

## Maintainer handoff

Report commands and results, platform, bundle path/signature/hash if installed, limits and the next manual check. Do not commit, publish, select a license or modify music as part of setup. Code changes follow the full closure rules in [AGENTS.md](../AGENTS.md).
