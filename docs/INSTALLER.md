# The Windows installer

curado's installer for Windows looks and talks like curado, and stays an honest installer: it says what it installs, where and which version before anything happens, needs no administrator, never closes the app for you, and removes cleanly from **Settings → Apps**. It is one `.exe` of about 4 MiB made by Tauri's own NSIS toolchain, with no new dependency. Decision: [ADR-034](DECISIONS.md). Evidence and limits: [Status](STATUS.md). How to build and run it: [curado on Windows](WINDOWS.md#5-build-and-install).

**Not seen on screen yet.** The installer compiles, passes its tests and a silent run; its pages have not been looked at, and no install, update or uninstall has been done with it. See [What is verified](#what-is-verified).

## What the user meets

Two clicks on a first install: **Instalar**, then **¡A darle!**.

| Page     | What it shows                                                                                                                                                                                                                                     | Buttons                   |
| -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------- |
| Welcome  | The mark and the grooves at the left; at the right `curado`, a greeting, and four facts: what (name, version, architecture), where (the folder, with **Cambiar…**), your music (played, never moved or edited) and permissions (no administrator) | **Instalar** · Ahorita no |
| Progress | A block progress bar in the accent; details on request                                                                                                                                                                                            | none while it works       |
| Finish   | Where curado now lives, the first mix in one line, **Abrir curado** and a desktop shortcut, both ticked as upstream does                                                                                                                          | **¡A darle!**             |

- **Updating** takes the same two clicks. The welcome page says "from 0.1.0 to 0.2.0, your library stays", the button says **Actualizar**, and the new version installs over the old one in its own folder.
- **Same version or an older installer** adds Tauri's maintenance page (reinstall or uninstall), themed. Choosing uninstall there removes curado and ends; upstream went on to install it again.
- **curado is open:** the installer says so and waits. It never ends the process, because curado may be playing; see [Behaviour](#behaviour).
- **Uninstalling** says what stays before removing anything: your music always, and your library unless you tick the box.

The window keeps the system title bar and its buttons, as the suite asks on Windows; the caption is dark with the accent as the window border on Windows 11.

## Command line

| Run                               | Does                                                                                                                       |
| --------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| `curado_0.1.0_x64-setup.exe`      | The pages above                                                                                                            |
| `… /DEMO`                         | Walks every page and writes nothing: no files, no registry, no shortcuts, no download. The caption and both pages say DEMO |
| `… /S`                            | Silent install for the current user                                                                                        |
| `… /P`                            | Progress page only                                                                                                         |
| `… /D=C:\some\folder`             | Another folder (last argument, no quotes: NSIS's rule)                                                                     |
| `… /NS`, `/R`, `/ARGS`, `/UPDATE` | Tauri's own switches, unchanged                                                                                            |

Exit codes are NSIS's: `0` installed, `1` cancelled by the user, `2` stopped by the installer, which includes "curado is open" in silent and passive runs. The suite's WinUI Setup uses other numbers (0 installed, 1 failed, 2 arguments, 3 cancelled); curado keeps NSIS's, which package managers already understand.

## Behaviour

| Rule                                      | How                                                                                                                                                                |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Says what, where, which version and who   | Welcome page facts; publisher `nu9ve` in **Settings → Apps** and in the file's properties                                                                          |
| Per user, no administrator                | `installMode: currentUser`, `RequestExecutionLevel user`, `%LOCALAPPDATA%\curado`                                                                                  |
| Never ends a running app                  | `NU_REQUIRE_APP_CLOSED` asks Restart Manager whether the executable is in use and only waits. Upstream's check force-closes the app, without asking in silent mode |
| Checks before writing                     | The welcome page tests that the folder takes files and that curado is closed, before the first file is copied                                                      |
| Your library is not an install's business | Only the uninstaller's check box removes `%APPDATA%\studio.curado.player` and its cache. Your music is never touched                                               |
| High contrast                             | Windows' own colors replace the theme on every control the installer colors; dark buttons and window colors are skipped                                            |
| Keyboard and Narrator                     | Native controls: Enter installs, Esc cancels, facts are real text                                                                                                  |
| Quiet mode for scripts                    | `/S`, with the exit codes above                                                                                                                                    |
| Shows nothing it does not do              | `/DEMO` exists so the pages can be reviewed and pictured without installing                                                                                        |

## Files

| File                                                                                 | What it is                                                                                                                |
| ------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------- |
| `src-tauri/tauri.windows.conf.json`                                                  | NSIS as the Windows bundle, per user, Spanish and English, and the paths below                                            |
| `src-tauri/windows/installer.nsi`                                                    | Tauri's template (tauri-cli 2.12.4) with curado's pages and words. Every change is marked `curado:`                       |
| `src-tauri/windows/installer-kit.nsh`                                                | The theme and the three behaviours as macros. No product name: any nu9ve app can use it unchanged                         |
| `src-tauri/windows/palette.nsh`, `sidebar.bmp`, `header.bmp`, `Jersey15-Regular.ttf` | **Generated** by `scripts/installer-art.ts`. Do not edit                                                                  |
| `src-tauri/windows/nu9ve.png`                                                        | The nu9ve signature as ink coverage; the only image not generated                                                         |
| `scripts/installer-art.ts`                                                           | Colors from `palette(DEFAULT_THEME)`, grain from `dither.ts`, mark from `LOGO_CELLS`, font rebuilt from Fontsource's WOFF |
| `scripts/installer-check.ts`                                                         | Renders the template as Tauri does and compiles it with makensis in seconds                                               |
| `scripts/windows-release.ts`                                                         | Builds the installer and writes its SHA-256 beside it                                                                     |
| `scripts/installer.test.ts`                                                          | The contracts above as tests, part of `bun run check`                                                                     |

```powershell
bun run installer:art      # after changing the theme, the mark or the art
bun run installer:check    # art up to date, and the template compiles without new warnings
bun run windows:release    # the installer, its SHA-256 and whether it is signed
```

**Art.** The sidebar is drawn on a grid of 2×2 pixels, so 150 % and 200 % scale it by whole steps. Colors come only from `palette()`, as [Themes](THEMES.md) asks. Contrast on both surfaces: text 15.3, secondary 8.4, muted 5.4, accent 4.7.

**Font.** Jersey 15 is the app's display font. GDI loads TrueType, not WOFF, so the script rebuilds the `.ttf` that the WOFF contains; the installer registers it for its own process only and removes nothing from the system. Its license is in `licenses/fonts`.

**Template and Tauri upgrades.** The template is upstream's file with marked blocks. After upgrading Tauri, take the new upstream template, compare it with the one this was based on, and carry the differences over; `bun scripts/installer-check.ts --template <file>` compiles any template with this project's values, which tells upstream's warnings from ours. The check's renderer is strict: a variable Tauri does not provide fails there, not in a release.

## How it talks

The installer speaks Spanish first, as the app does, and English on an English system. One voice for both:

1. **Facts first.** What, where, which version, your data and permissions are each said once, plainly.
2. **A button says what it does.** Instalar, Actualizar, Siguiente. At most one playful button per page (Ahorita no, ¡A darle!), and never on something that deletes.
3. **Mexican and light.** Sayings and wordplay: _pásale a lo barrido_, _la toco, pero no la muevo_, _ahí nos vidrios_. Nothing crude, nothing at the user's expense, nothing that must be explained to be understood. When the joke and the fact compete, the fact wins.
4. **Errors say what to do.** A light title (_Algo tronó_), then the next step.
5. **The installer is "I", the user is "tú".** _No puedo escribir en esa carpeta._
6. **English is written, not translated:** the same facts with its own turns of phrase.
7. **Two lines at most.**

Every text has both languages; a test fails if one is missing or unused.

## What is verified

Automatically, on Windows 11 Pro 26200 (2026-11-11):

- makensis 3.13 compiles the rendered template with no warning beyond one upstream already has.
- The installer tests pass: generated files match the generator, sizes and contrast, both languages complete, no force-close anywhere, `/DEMO` returns before any write, upstream's hooks and switches kept.
- A silent `/S /DEMO` run exits `0` and leaves the uninstall entry, the install folder and both shortcuts exactly as they were.

**Not verified:**

- Any page on screen: layout, the font, the dark buttons, the progress bar, the art beside the text. Visual checks wait for the maintainer's word, as [AGENTS.md](../AGENTS.md) says.
- A real install, update, reinstall and uninstall with this installer, and the folder picker.
- The "curado is open" prompt, and exit code `2` in a silent run.
- High contrast, 125 %, 150 % and 200 % scaling, Narrator heard, and Windows 10.
- Signing. The installer is unsigned and SmartScreen will warn; signing is a release gate in [Quality](QUALITY.md).

Two parts lean on behaviour that is not a documented contract, and degrade to a readable installer if it changes: the `DarkMode_Explorer` button theme (otherwise light buttons on the dark bar) and a dialog's background color through `SetCtlColors` (otherwise a gray page behind colored text blocks). Text and background are always set as a pair, so text cannot end up unreadable.

## Using it in another nu9ve app

**A Tauri app** takes four things: `installer-kit.nsh` as it is; `installer-art.ts` with its own mark and palette; `installer.nsi` with the `curado:` words and pages replaced by its own; and the `bundle.windows.nsis` block. `installer-check.ts` works as it is. When the suite's shared repository exists, the kit and the check are what belongs there; the art, the words and the pages are each app's identity.

**A WinUI app with an MSIX package** (nove, nüün) cannot use NSIS for its payload: it installs with `PackageManager`, and its reference is `Nuun.Setup` in `n-creator-utils`. What carries over is everything that is not NSIS: the table in [Behaviour](#behaviour), the voice, `/DEMO` as a labelled demonstration that writes nothing, art generated from the app's palette, and never ending a running app or its background host.

**On macOS** curado installs with `bun run desktop:install`, a development script. A themed disk image is the suite's suggestion for a release and does not exist yet.
