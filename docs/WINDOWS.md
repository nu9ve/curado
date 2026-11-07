# curado on Windows

Install, build, run and debug curado on Windows 10/11 x64. Start with [Setup](SETUP.md) for the general workflow and [Platforms](PLATFORMS.md) for how each OS differs. This page is the Windows detail: exact prerequisites, commands for PowerShell, which features have a Windows adapter, which are macOS only, and how to debug.

**First verified on 2026-11-06**, Windows 11 Pro 26200, x64, Rust 1.99.1 MSVC, Bun 1.4.4, LLVM 23.1.5, Visual Studio Build Tools 2022 17.14 and WebView2 156. On that machine every automated check passes and `bun tauri build` produces both installers. The app has **not been launched or listened to** on Windows yet; see [What is still unverified](#what-is-still-unverified).

## 1. Prerequisites

| Tool                      | Why                                                  | Install                                                                 |
| ------------------------- | ---------------------------------------------------- | ----------------------------------------------------------------------- |
| Git for Windows           | Source checkout                                      | `winget install --id Git.Git -e`                                        |
| Rust, MSVC toolchain      | Engine and app (`x86_64-pc-windows-msvc`)            | `winget install --id Rustlang.Rustup -e`, then `rustup default stable`  |
| Visual Studio Build Tools | MSVC linker, C++ compiler for Signalsmith and SQLite | Workload **Desktop development with C++** with a **Windows 10/11 SDK**  |
| LLVM (libclang)           | `bindgen` for Signalsmith Stretch                    | `winget install --id LLVM.LLVM -e`                                      |
| Bun 1.4+                  | Frontend, scripts and Tauri CLI                      | `winget install --id Oven-sh.Bun -e`                                    |
| WebView2 Runtime          | The app's WebView                                    | Preinstalled on Windows 11; on Windows 10 install the Evergreen runtime |

Build Tools from the command line, if they are missing:

```powershell
winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

The LLVM folder that Visual Studio Build Tools ships under `VC\Tools\Llvm` usually only holds `clang-format` and `clang-tidy`, without `libclang.dll`. Install LLVM separately as above. curado's build finds `C:\Program Files\LLVM\bin\libclang.dll` on its own; set `LIBCLANG_PATH` only if LLVM lives elsewhere.

**Open a new terminal after installing.** winget changes `PATH`, and a terminal opened before does not see `bun` or `clang`. Check:

```powershell
git --version
rustc -vV          # host: x86_64-pc-windows-msvc
bun --version      # 1.4 or newer
clang --version
Test-Path "C:\Program Files\LLVM\bin\libclang.dll"
```

## 2. Get the source

```powershell
git clone https://github.com/nu9ve/curado.git
cd curado
```

The repository keeps LF line endings through `.gitattributes`, whatever `core.autocrlf` says; Git for Windows sets it to `true` by default. **A checkout made before `.gitattributes` existed** has CRLF files, and `bun run format:check` then fails on every file. Fix it without losing work: commit or stash your changes, then

```powershell
git rm -r --cached -q .
git reset --hard
```

or, with local changes you want to keep, convert only the tracked text files from Git Bash: `git ls-files --eol | awk '$2=="w/crlf"{sub(/^[^\t]*\t/,"");print}' | xargs sed -i 's/\r$//'` and then `git add --renormalize .`.

## 3. Install dependencies and run the checks

```powershell
bun install --frozen-lockfile
bun run check
bun run build
bun run format:check
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The first Rust build compiles Signalsmith's C++ and takes several minutes; later ones are incremental. Keep `Cargo.lock` and `bun.lock` unchanged.

Expected on Windows: about 72 tests in `curado` and 142 in `curado-audio` pass, with a few ignored that need real data. That is fewer than on macOS on purpose: tests that drive a `#!/bin/sh` stand-in process (stems, camera helper), or use Unix symbolic links (drive browsing, rekordbox XML) only exist on Unix. The callback allocation audit runs here for Rust allocations; only its hook on the C++ system allocator is macOS only.

## 4. Develop

```powershell
bun run desktop
```

Starts Vite on `127.0.0.1:1420` and the native app in debug, with a console that shows Rust's output. `bun run dev` alone is a browser preview **without native audio**, as on every OS.

## 5. Build and install

```powershell
bun tauri build
```

Do not use `bun run desktop:build` or `bun run desktop:install`: they are macOS only (`--bundles app`, codesign, LaunchServices). The first `bun tauri build` downloads NSIS and WiX from Tauri's official releases into its cache.

Output, under `target\release\`:

| File                                     | What it is                                                                       |
| ---------------------------------------- | -------------------------------------------------------------------------------- |
| `curado.exe`                             | The app itself, runnable in place for a quick try                                |
| `bundle\nsis\curado_0.1.0_x64-setup.exe` | Per-user installer, no administrator needed, installs to `%LOCALAPPDATA%\curado` |
| `bundle\msi\curado_0.1.0_x64_en-US.msi`  | Per-machine installer for `C:\Program Files\curado`, asks for administrator      |

Prefer the NSIS installer for a personal machine. It adds curado to the Start menu and to **Settings → Apps** for uninstalling. Neither installer is signed, so SmartScreen may warn on first run: **More info → Run anyway**. Signing is a release gate in [Quality](QUALITY.md).

To update, close curado and run the new installer; it replaces the previous version and keeps the library. Do not end the process from Task Manager during a set.

**Run the installer yourself, from Explorer or your own terminal.** Some agent apps on Windows, including the Claude desktop app, are MSIX packages, and Windows redirects what their child processes write under `AppData` into the package's own folder. An installer started from there lands in `%LOCALAPPDATA%\Packages\<app>\LocalCache\Local\curado` instead of `%LOCALAPPDATA%\curado`; it still runs, but other programs may not see it and removing the agent app can remove it too. An agent can build the installer; installing is better done by hand.

## Where curado keeps things

| What                      | Where                                                                   |
| ------------------------- | ----------------------------------------------------------------------- |
| Library and settings      | `%APPDATA%\studio.curado.player\curado.sqlite` (with `-wal` and `-shm`) |
| Demo session, stems, etc. | `%APPDATA%\studio.curado.player\`                                       |
| Caches                    | `%LOCALAPPDATA%\studio.curado.player\`                                  |
| WebView2 profile          | `%LOCALAPPDATA%\studio.curado.player\EBWebView\`                        |

Copy the SQLite file only with curado closed. Uninstalling does not remove these folders, and removing tracks from the library never deletes your music.

## What works on Windows, and how

| Area                     | On Windows                                                                                                                                                                                      |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Audio output             | CPAL over **WASAPI**, shared mode. ASIO is not enabled. Without any output at launch, curado tries the default output every 2 s and starts the engine once one appears.                         |
| Pitch and key lock       | Signalsmith Stretch, compiled with MSVC and bindgen/libclang. Builds and passes the engine tests.                                                                                               |
| MIDI and MIDI Learn      | midir over **WinMM**.                                                                                                                                                                           |
| Keyboard                 | The same keys. **Option is Alt**: Alt with a hot cue's number fires a sample, Alt with a control's key and arrows sets the mixer. Badges and help say `Alt` and `Ctrl`, as the keys print them. |
| Window                   | Native Windows title bar. The integrated header with traffic lights is macOS only, by design.                                                                                                   |
| Tray and single instance | Tauri tray icon in the notification area; a second launch focuses the first.                                                                                                                    |
| Drives and rekordbox USB | Letters `D:` to `Z:` are listed as drives. rekordbox XML writes `file://localhost/C:/…` locations.                                                                                              |
| Moved files and drives   | Size and fingerprint as everywhere; the drive's id is the **volume serial number** (`GetVolumeInformationW`), so a USB that comes back under another letter heals.                              |
| Free space for stems     | `GetDiskFreeSpaceExW`, space available to the user.                                                                                                                                             |
| Stems                    | Works with a Python you choose; see [Stems on Windows](#stems-on-windows).                                                                                                                      |
| Integrated web explorer  | WebView2 child WebView, same isolation and CSP as on macOS.                                                                                                                                     |
| Metal soundscape         | **macOS only.** The button is absent; the player's reactive visual works.                                                                                                                       |
| Camera hands and face    | **macOS only** (Vision). The XY pads work by pointer and finger.                                                                                                                                |
| Force Touch haptics      | **macOS only.** No haptics; controls work without them.                                                                                                                                         |

## Stems on Windows

`scripts/setup-separation.sh` is zsh and Apple Silicon only. On Windows, make an environment with the regular Demucs (PyTorch) and point curado at it:

```powershell
winget install --id Python.Python.3.12 -e
py -3.12 -m venv "$env:APPDATA\studio.curado.player\separation"
& "$env:APPDATA\studio.curado.player\separation\Scripts\python.exe" -m pip install demucs==4.0.1 soundfile
```

curado looks for `separation\Scripts\python.exe` in its data folder by itself; any other Python with Demucs can be chosen in **Configuración → Audio**. The model, about 80 MB, downloads on first use. macOS compresses stems to FLAC with `afconvert`; Windows has no such tool, so stems stay as WAV, about three times larger, and count that way against the space limit. **Not tried on Windows yet**, including the exact Demucs, torch and torchaudio versions.

## Debugging

| Symptom                                                                                                        | What to do                                                                                                                                                                                                                                                                          |
| -------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `bun` or `clang` not recognized right after installing                                                         | Open a new terminal; winget updated `PATH` for new ones only.                                                                                                                                                                                                                       |
| `Unable to find libclang` / `couldn't find any valid shared libraries matching: ['clang.dll', 'libclang.dll']` | Install LLVM (`winget install LLVM.LLVM`). If it is not in `C:\Program Files\LLVM`, set `$env:LIBCLANG_PATH = "<LLVM>\bin"` before building.                                                                                                                                        |
| `link.exe` not found, `cl.exe` not found, missing `windows.h`                                                  | Build Tools lack the C++ workload or a Windows SDK. Add **Desktop development with C++** in the Visual Studio Installer.                                                                                                                                                            |
| `bun run format:check` fails on almost every file                                                              | CRLF checkout from before `.gitattributes`: see [Get the source](#2-get-the-source).                                                                                                                                                                                                |
| `error[E0433]: cannot find unix in os` in a test                                                               | A new test uses a Unix-only API. Gate it with `#[cfg(unix)]`, as the process tests in `stems.rs` and `tracking.rs` are.                                                                                                                                                             |
| The installed app prints nothing                                                                               | Release builds are GUI apps without a console. Run `bun run desktop` for Rust output, or `$env:RUST_BACKTRACE = "1"; .\target\debug\curado.exe` after `cargo build -p curado`.                                                                                                      |
| Inspecting the interface                                                                                       | In `bun run desktop`, right-click → **Inspect** opens WebView2's DevTools. Release builds do not have them.                                                                                                                                                                         |
| Footer says **sin salida de audio**; loading a track says "Sin salida de audio…"                               | Windows has no output device: nothing in the headphone jack and no speakers enabled. Plug something in, or enable it in **Settings → System → Sound**; curado starts the engine within 2 s. Check with `Get-PnpDevice -Class AudioEndpoint`: an output must be `OK`, not `Unknown`. |
| No sound, or the device list is empty                                                                          | Choose the device in **Configuración → Audio**. Check it is the default and not held in exclusive mode by another app (**Sound → Device properties → Advanced → Exclusive mode**).                                                                                                  |
| Crackles                                                                                                       | Raise the buffer in **Configuración → Audio**. Bluetooth and some USB devices resample in Windows' audio engine; a wired interface at 48 kHz is a fairer test.                                                                                                                      |
| A MIDI controller does not appear                                                                              | WinMM lets only one program open a MIDI port. Close other DJ software, then reconnect the controller.                                                                                                                                                                               |
| Alt opens nothing but a key combination does nothing either                                                    | Report it with the exact keys: Alt combinations reaching WebView2 are not verified yet.                                                                                                                                                                                             |
| SmartScreen blocks the installer                                                                               | Unsigned local build: **More info → Run anyway**.                                                                                                                                                                                                                                   |
| Builds are very slow                                                                                           | Windows Defender scans every object file. Excluding the checkout's `target\` folder from real-time scanning speeds it up markedly; it is your security decision.                                                                                                                    |
| The library looks empty after reinstalling                                                                     | Check `%APPDATA%\studio.curado.player\curado.sqlite` exists. The identifier is the same on every OS and version, so it is reused.                                                                                                                                                   |

Report a Windows problem with: Windows version (`winver`), `rustc -vV`, `bun --version`, the full error, the audio device and driver, and whether it happens in `bun run desktop` or only in the installed app.

## What is still unverified

Automated checks and packaging pass; the following needs a person at a Windows machine, recorded in [Status](STATUS.md) when done:

1. Launch the installed app, run **sesión de prueba**, play both decks and move the crossfader.
2. WASAPI with built-in, USB, Bluetooth and a 4-channel interface (cue bus), sample rates other than 48 kHz, unplugging and the default device changing.
3. Sleep and resume during playback.
4. Alt combinations in WebView2, Spanish and US layouts, and many keys at once.
5. A MIDI controller through WinMM.
6. A rekordbox USB under a drive letter, and the same USB coming back under another letter.
7. The integrated web explorer in WebView2: fractos radio, SoundCloud and login.
8. Display scaling at 125 %, 150 % and 200 %, and the always-on-top mini player.
9. C++ allocations in the callback on Windows: the audit sees every Rust allocation here and passes, but its hook on the system allocator, which also sees Signalsmith's C++, is macOS only. x86 denormals inside Signalsmith are unreviewed.
10. Stems with Demucs on PyTorch, end to end.
