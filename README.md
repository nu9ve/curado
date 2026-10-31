<p align="center">
  <img src="docs/assets/logo.svg" width="72" height="72" alt="curado">
</p>
<h1 align="center">curado</h1>
<p align="center">Your music. Two decks. Room to play.<br>A desktop music player and mixer, made for your keyboard.</p>
<p align="center"><a href="README.es.md">Español</a> · <a href="docs/SETUP.md">Install with your agent</a> · <a href="docs/README.md">Documentation</a> · Made by <a href="https://nu9ve.xyz">nu9ve</a></p>

Listen to your local collection, try a transition, or play a whole set from your keyboard. **curado** is a music library and a two-deck console laid out like an all-in-one DJ system, with its own audio engine. Your music stays where it is; curado never moves or edits your files.

![The curado console with two loaded decks, an active auto loop and keyboard badges on every control](docs/assets/console.jpg)

_The console with invented tracks. Each pale badge is the key for that control._

## What you get

**A console to play.** Platters you can scratch, hot cues, slip, reverse, beat jump, and pads for cues, loops, rolls, samples, key shifts and Release FX. Save hot cues that loop, hold a **TRIGGER FX** that follows your pressure, and press **AUTO LOOP** for a loop with no IN or OUT to mark, and pick its length from 1/4 to 32 beats, even while it plays.

**Effects on every channel.** Six color effects and sixteen beat effects, including Stutter and Recycler, different per channel and on the master.

**Your keyboard and your hands at once.** Every control shows its key. Keep playing from the keyboard while dragging a fader or turning a knob, remap anything, or use MIDI Learn. Hold Option to reach the whole mixer with arrows. On a Force Touch trackpad, controls tap as they move and a harder press moves them more finely.

**Your rekordbox USB as it is.** Playlists, covers, grids, keys and hot cues appear when you plug it in. curado only reads it, and it can send your cues back through a `rekordbox.xml`.

|                                                                                                 |                                                                                     |
| ----------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| ![Console and library side by side, with covers in the track list](docs/assets/mix-library.jpg) | ![Player with cover, waveform, reactive visual and library](docs/assets/player.jpg) |
| **Mix:** one fixed screen, with the library, tools and keyboard map in a side panel.            | **Player:** one deck, a queue with repeat, and a visual that follows the music.     |
| ![The console in the gothic style with a violet accent](docs/assets/console-gothic.jpg)         | ![Appearance settings with styles, themes and colors](docs/assets/settings.jpg)     |
| **Your color in every style:** curado, pixel, gothic, y2k and minimal.                          | **Settings:** appearance, audio output, keyboard and MIDI.                          |

_These show the real interface in a browser with invented tracks and a stand-in for the audio engine. [How they are made](docs/README.md#pictures)._

<details>
<summary>The Metal soundscape</summary>

![curado Metal soundscape](docs/assets/visual-metal.jpg)

_A separate window on macOS with seven scenes, including a spectral city and a circular oscilloscope. This picture shows an earlier terrain scene._

</details>

## Everything it does

- **Listen:** search your library, sort it by any column, save favorites and playlists, play a queue with repeat, or switch to an always-on-top mini player. Covers come from rekordbox or the file, with a dithered pattern otherwise.
- **Mix:** two decks and a choice of console or curado's classic layout, eight hot cues per track, beat loops, tempo and phase sync, three-band EQ, filters, trim, crossfader curves and eight samples.
- **Match keys at a glance:** each deck's key in color on the screen, with the semitones to the other deck, one press away.
- **Prepare:** BPM, grid, bars and key estimated over the whole track, in letter notation and Camelot. Correct any of them by hand. Key lock and ±12-semitone transposition. Tracks whose tempo moves can follow their own beats.
- **Headphones:** with an output of four or more channels, 3 and 4 become a cue bus with mix and level, plus Touch Preview for the selected track.
- **Extra surfaces:** two XY pads with eight modes, by finger, pointer or, if you want, by camera, which is off until you ask and stays on your Mac. Each hand drives its side, nearness is a third axis, and hands and face together run three effects.
- **Bring your drive:** connected drives appear in the sidebar. A rekordbox USB shows its playlists; any other drive can be browsed for audio.
- **Make it yours:** five styles, your own color, light/dark/system themes and waveform options.
- **Visualize:** a reactive visual in the player and, on macOS, a Metal window with seven scenes. Adjust glow, reactivity, speed and color, drag the camera, H hides controls and F is fullscreen.
- **Explore:** fractos radio and SoundCloud inside the app. Their web audio is separate from the native mixer.

Local listening and mixing work offline. Web services need internet. There is no WebAudio replacement for the native engine.

The interface starts in Spanish; English is in Settings and still has some native messages in Spanish. BPM, key and grid are estimates you can correct.

> **Early development · 0.1.0.** Built and tested automatically on Apple Silicon. Windows and Linux hardware validation and release signing are still pending. See [current status](docs/STATUS.md) and [what to know first](#before-you-start).

## Install with your agent

Open this repository in your coding agent and paste:

```text
Help me set up curado on this computer. Read AGENTS.md and docs/SETUP.md first,
check my operating system and prerequisites, install from the lockfiles,
and run the required checks. On macOS, build and install the local app with
bun run desktop:install. Do not launch it or interrupt an existing audio session.
Report what passed and what still needs manual verification.
```

The [setup guide](docs/SETUP.md) includes prerequisites, platform differences, troubleshooting and local updates. The [agent guide](AGENTS.md) explains the contracts to preserve when changing code. You do not need to read the architecture to try curado.

### Bienvenido · ¿prefieres español?

curado es un espacio para escuchar tu música y jugar con la mezcla. Puedes darle este repo a tu agente y pedirle: **«Lee AGENTS.md y docs/SETUP.md, revisa los requisitos de mi equipo y ayúdame a instalar curado»**. Tienes la bienvenida, los pasos y un prompt completo en el [README en español](README.es.md).

## Install it yourself

This repository provides a source build workflow; the documentation does not promise a signed public download. You need [Bun](https://bun.sh), stable Rust and the [platform prerequisites](docs/PLATFORMS.md). On macOS, use full Xcode plus its command line tools; C++ and libclang are needed for pitch processing.

```sh
git clone https://github.com/nu9ve/curado.git
cd curado
bun install --frozen-lockfile
bun run desktop:install
```

The installer is **macOS only**. It builds and installs `~/Applications/curado.app`, verifies its local signature and registers it for Spotlight/Raycast. It does not launch the app. Quit an installed instance before updating; the installer will not stop it for you. Launcher indexing may take time. Local signing is ad hoc, not notarization.

For a development session on a supported build environment:

```sh
bun run desktop
```

`bun run dev` previews the interface in a browser **without native audio**. Windows and Linux setup/build instructions are in [Platforms](docs/PLATFORMS.md). To update a local Mac build after changing code, run `bun run desktop:install`; use `--no-build` only immediately after a verified build.

## Your first mix

1. Open curado and choose **sesión de prueba** in the sidebar. It generates two short tracks and four samples locally; nothing is downloaded.
2. Switch to **Mezcla**, press `F` and `J` to play each deck, and move the crossfader.
3. Press `B` for a four-beat auto loop and `B` again to leave it. Point to any control to read its hint in the footer.
4. Import your own music when you are ready. Import registers file references; removing library entries never deletes originals.

Each hand plays its own deck, with the same finger for the same job:

| Action         | Deck 1    | Deck 2    | With Shift  | With Control |
| -------------- | --------- | --------- | ----------- | ------------ |
| Play/pause     | `F`       | `J`       |             |              |
| Cue            | `D`       | `K`       |             |              |
| Sync           | `G`       | `H`       | master deck | key lock     |
| Hot cues A–D   | `1 2 3 4` | `7 8 9 0` | save        | delete       |
| Hot cues E–H   | `Q W E R` | `U I O P` | save        | delete       |
| Auto loop      | `B`       | `N`       | shorter     | longer       |
| Loop in / out  | `Z` `X`   | `/` `.`   | ½ / 2×      |              |
| Reloop         | `C`       | `,`       | quantize    |              |
| Nudge          | `A` `S`   | `L` `;`   | one beat    | four beats   |
| Slip           | `V`       | `M`       | reverse     |              |
| Beat FX on/off | `T`       | `Y`       |             |              |

`Space` stops or starts the whole mix. `↑ ↓` moves through the list and `Shift + ← →` loads the selected track into deck 1 or 2. `Control + ← ↓ →` moves the crossfader. `Option` with a hot cue's number fires a sample. `Option` with a control's key and arrows sets the mixer: ↑ top, ↓ silent, ← → center. `Esc` silences the native audio, not web audio. Keys keep working while you drag a control, or right after you touch one. Remap them in **Configuración → Teclado**. See [Controls](docs/CONTROLS.md) for sync, key lock, MIDI and recovery.

## Before you start

- Tracks load fully into memory, about 21 MiB per minute: up to about **90 minutes per track** and 3 GiB in total. Nothing depends on the disk during a set.
- Effects are measured. Playback and mixing have been heard on the maintainer's system, but nothing was compared side by side with the hardware it takes after.
- The cue bus has not been tried on a real 4-channel interface, nor the camera with a real camera, nor touch and pressure on a trackpad. Without four output channels there is no separate headphone cue. Device recovery is manual.
- rekordbox USBs are read only, and USBs with only the newer OneLibrary database show as plain files. Cues reach rekordbox through an XML export that has not been tried in rekordbox yet, so back up its library first. USB tracks with no certain original on the computer arrive there as other tracks.
- BPM, grid and key estimates are measured against rekordbox's and can be wrong: check them before mixing. Tempo-following is opt-in per track because it fires on some steady ones. On a transposed track, very low notes can sit slightly sharp.
- Web playback and login inside the app still need real validation. Web audio bypasses the local master, effects, sync and panic.

These are active priorities, not hidden switches. See [Roadmap](docs/ROADMAP.md) for what comes next.

## Build with us

The sound runs in a Rust engine (CPAL + Symphonia); Tauri and React handle the desktop interface. Decoding and analysis stay outside the audio callback, and everything inside it is preallocated. The engine owns transport and position; the UI displays snapshots. Separate native helpers hold the Metal window and the camera, so neither can reach the audio.

Start with [AGENTS.md](AGENTS.md), then [Setup](docs/SETUP.md) and the [documentation map](docs/README.md). Record evidence and limits with every change. Passing a build is not a listening test.

```sh
bun run check
bun run build
bun run format:check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

## License and credits

Made by [nu9ve](https://nu9ve.xyz). **The project license has not been selected yet.** A public repository alone does not grant a general license to reuse its code. Dependencies retain their own licenses.

The current recommendation is MIT for permissive use, including commercial forks. Free downloads, donations and paid official editions can coexist; no license is applied by this recommendation. See [Licensing](docs/LICENSING.md) for the decision, dependency obligations and release checklist.
