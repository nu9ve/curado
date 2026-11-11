# curado on Windows within the nu9ve suite

Status: **proposal** (2026-11-09); **section 2 was carried out on 2026-11-11**, further than its phase 1: see [The Windows installer](INSTALLER.md) and [ADR-034](DECISIONS.md). Sections 3 to 6 are still proposals. This page is written from a session in `n-creator-utils`, after reviewing curado, nove and nüün on this Windows 11 machine. It does not change curado's code, decisions or verified state: [Status](STATUS.md) and [curado on Windows](WINDOWS.md) remain the source of truth. Suite rules live in `n-creator-utils`: `docs/contracts/windows-app-v1.md`, `nu9ve-theme-v1.md`, `nu9ve-interaction-v1.md`. The how-to and lessons are in `docs/platforms/windows.md`. On this machine the repositories are siblings under `C:\Users\jose\Code\`.

## 1. Where curado stands

curado is the suite's **Tauri** case on Windows. It builds and passes `cargo test`, `clippy` and `bun run check` on Windows 11, and `bun tauri build` produces NSIS and MSI installers (2026-11-06). It has not been heard on Windows yet. It brings three lessons the suite has adopted:

- The installer must be run by hand. An agent app packaged as MSIX virtualizes its children's `AppData`, so an installer started from it lands in `Packages\…\LocalCache`.
- `.gitattributes` with LF is required; Git for Windows checks out CRLF by default.
- A Python worker must watch its parent process, not a blocking stdin. Otherwise numpy's DLL load hangs.

What it lacks relative to the suite standards is mostly **identity at install time** and **parity of the common skins**. The native window is already right: it keeps the system title bar on Windows by design.

## 2. Installer with identity (windows-app-v1 §9)

**Done 2026-11-11, not seen on screen yet.** What follows is the proposal as written; the outcome is in [The Windows installer](INSTALLER.md). It differs in three ways: the template is curado's own after all, because configuration could not color pages, drop the directory page or stop the default installer from force-closing a running curado; the wordmark is live text in Jersey 15 rather than drawn into a bitmap; and the hooks slot carries the theme kit instead of a hook.

Before that, the NSIS installer was Tauri's default: generic pages, unsigned, `currentUser` to `%LOCALAPPDATA%\curado`. The suite asks that installing feel like the app while staying an honest installer. That means it says what, where and which version; shows real progress; offers Cancel; explains errors in words; works with keyboard and Narrator; has a quiet mode; and asks for admin only when needed.

### Phase 1: brand the NSIS installer (config only, no new dependency)

`src-tauri/tauri.conf.json` → `bundle.windows.nsis`:

| Key                                                      | Proposal                                                                                                    | Why                                                                          |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| `installMode`                                            | `"currentUser"`                                                                                             | What happens today, but stated explicitly; no admin                          |
| `installerIcon`                                          | `icons/icon.ico`                                                                                            | Same mark as the app                                                         |
| `headerImage` (150×57 BMP), `sidebarImage` (164×314 BMP) | Generated from `palette()` with the Bayer 8×8 dither of `dither.ts`, coral `#d66154`, wordmark in Jersey 15 | The «curado» original style at install time; deterministic, no hand-made art |
| `languages` + `displayLanguageSelector`                  | Spanish and English (check the exact names in Tauri's NSIS list)                                            | The suite's base language is Spanish (theme §9)                              |
| `installerHooks` (`.nsh`)                                | Only to show the library location and say that uninstalling keeps it                                        | Honest «what and where»; nothing hidden                                      |
| `template`                                               | Not at first                                                                                                | A custom `.nsi` is maintenance; start with images and hooks                  |

Generate the two BMPs with a script next to `scripts/icon.py` (for example `scripts/installer-art.ts`), from the same palette function. This follows the theme rule that colors come only from `palette()`. Silent mode stays NSIS's `/S`. Signing remains a release gate in [Quality](QUALITY.md).

### Phase 2 (optional): a shaped Setup from the suite base

The suite has a reference Setup: `n-creator-utils`, `native/windows/Nuun.Setup` (WinUI 3, built on `Nu9ve.WinUI`).

- A borderless window cut with `SetWindowRgn` into the app's own outline (nüün uses the SD card of its icon).
- The app's skins, its own − and × at the top right, and dragging by the header.
- `--quiet` with stable exit codes; with no package, a labelled demonstration.

For curado the outline would come from its identity, for instance a record or a cassette, with no third-party marks. Unlike nüün, curado's payload is the NSIS installer. The Setup would therefore run it silently (`/S`), show an **indeterminate** but visible bar (NSIS reports no percentage), and keep NSIS as the owner of uninstalling (Settings → Apps). Decide this together with the maintainer. Phase 1 alone already meets the contract's «should».

## 3. Window on Windows (windows-app-v1 §1–§3)

- **Keep the native title bar.** Do not set `decorations: false` to draw custom buttons. On Windows the caption and its buttons belong to the system and stay on the right, with Snap Layouts.
- **Mode.** Pass curado's resolved mode (`nativeWindowTheme()` in `theme.ts`) to the window theme on Windows too, so the caption follows Dark, Light or System. Verify it in the installed app: «system» must not fall back to a generic theme (theme §4).
- **Mica (optional).** Tauri 2 offers `windowEffects: { effects: ["mica"] }` with a transparent window, without a new dependency. Use it only for a glass look like the one the suite gives Cristal. Fall back to solid when Windows' transparency effects are off. Do not use it for the «curado» dither chrome, which is opaque by design.
- **High contrast.** In WebView2, add a `@media (forced-colors: active)` block that maps the roles to system colors (`Canvas`, `CanvasText`, `Highlight`, `HighlightText`, `GrayText`) and drops dither and gradients. The suite contract makes system colors mandatory in high contrast.
- **Scaling.** 125/150/200 % and the always-on-top mini player are already on the [unverified list](WINDOWS.md#what-is-still-unverified).

## 4. Theme parity (nu9ve-theme-v1)

| Item                                              | curado today                | Suite                                                                                       | Proposal                                                                                                                                               |
| ------------------------------------------------- | --------------------------- | ------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Palette as a pure function with a contrast matrix | Yes (`palette()`, 5×10×3×3) | Same rule                                                                                   | Keep. It is the TypeScript reference                                                                                                                   |
| Shared test vectors                               | No                          | `n-creator-utils/docs/contracts/examples/theme-vectors-v1.json` (110 cases, verified in C#) | When curado adopts the **common** skins, check their roles against the same file. curado's own styles keep their own matrix                            |
| Common skins Macintosh, Windows 9x, Terminal      | Missing                     | The same in every app (theme §5)                                                            | Port the measures from nüün (`ThemeSkin`, `SurfaceChrome`: 1-bit edges, Win9x bevel highlight/light shadow/dark shadow, Terminal phosphor) to `STYLES` |
| Contrast skin                                     | To be asked                 | One opposite of the original                                                                | Ask the maintainer (gothic, y2k, pixel or minimal)                                                                                                     |
| Fonts                                             | Local OFL (Fontsource)      | Packaged OFL, system fallback                                                               | Already right. nüün on Windows still lacks them                                                                                                        |

## 5. Keyboard (nu9ve-interaction-v1)

curado is the suite's most complete keyboard app: a catalog, badges with an Option layer, remapping and a context footer. On Windows, labels already say `Alt` and `Ctrl`. Two items remain:

- Alt combinations reaching WebView2 are unverified. Alt alone may open nothing, because WebView2 has no menu bar, so it does not conflict with KeyTips.
- The suite's common keys on Windows: Ctrl+, for Settings, Ctrl+/ for shortcuts, F1 for Help, F11 for full screen. Check that the keyboard map lists them and that none collides with the two-hand deck layout.

## 6. QA isolation and diagnosis

- Propose a `CURADO_QA_ROOT`, following `NOVE_QA_ROOT` and `NUUN_QA_ROOT`. It would point the library, preferences and caches to a temporary root, so visual checks on Windows never touch `%APPDATA%\studio.curado.player`. This needs a decision because curado's maintainer limits visual tests; the variable only enables them on request.
- When the maintainer asks for a GUI check, the suite's scripts already work on any top-level window: `scripts/windows/capture-window.ps1` (PrintWindow) and `ui-invoke.ps1` (UI Automation, the tree Narrator uses). WebView2 content appears as a separate fragment, which `ui-invoke.ps1` walks node by node.

## 7. Suggested order

1. ~~Phase 1 installer art and config~~ Done 2026-11-11 with its own template; next is looking at it on screen, starting with `/DEMO`.
2. `forced-colors` block and the native window theme on Windows.
3. Common skins with their measures from nüün, then shared vectors for them.
4. Decide on `CURADO_QA_ROOT` and on a phase 2 Setup.

None of these touches the audio callback, the engine or the library. Follow [Quality](QUALITY.md) before calling any of it shipped.
