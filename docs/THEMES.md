# Appearance, styles and native window

## Visual language

curado combines two materials:

- **Chrome:** sidebar/topbar/footer form a continuous surface, soft gradient with main-color glow top-left and B-color glow bottom-right, plus ordered dither grain. No borders between them.
- **Sheet:** smooth elevated content with radius, shadow and a light edge. Grain versus smoothness and elevation separate navigation/content without cards/borders.

Inside the sheet, use flat sections, spacing and typography instead of making everything a card. Distinct material is reserved for meaningful surfaces, such as mixer hardware. Pixel logo/keycaps/labels/dither coexist with soft gradients on backgrounds, primary buttons, platters and progress, from accent to each style's `--gradient-end`. Later visual refinement made the mixer smooth rather than grainy; see CHANGELOG.

## Settings

**Configuración → Apariencia**:

| Setting        | Options                                            | Notes                                                              |
| -------------- | -------------------------------------------------- | ------------------------------------------------------------------ |
| Style          | curado, pixel, gothic, y2k, minimal                | Default material/radii/type/texture; current UI uses Spanish names |
| Base color     | Presets, picker, hex, HSV                          | User-owned; style suggests but does not force a color              |
| Scheme         | Dark, light, system                                | Follows OS when selected                                           |
| Variant        | Neutral, tonal, high contrast                      | Tonal tints surfaces; contrast reinforces text/edges               |
| Typography     | Style default, display, pixel, mono, system        | `auto` follows style                                               |
| Texture        | Style default, dither, soft gradients, flat        | `auto` follows style                                               |
| Waveform       | Three-band, frequency RGB, mono accent, energy map | The energy map blends band colors with each deck accent            |
| Waveform scale | Original time, follows tempo                       | Changes the detailed view and its seek geometry                    |

Defaults: **curado**, **#d66154**, dark, neutral, style typography/texture, three-band waveform. Reset restores these exact values.

| Style   | Display            | UI            | Texture | Character                                       |
| ------- | ------------------ | ------------- | ------- | ----------------------------------------------- |
| curado  | Jersey 15          | System        | Dither  | Warm coral→amber, pixel details                 |
| pixel   | Jersey 15          | Pixelify Sans | Dither  | Zero radius, 2 px edges, stepped shadows, no AA |
| gothic  | UnifrakturMaguntia | System        | Soft    | Deep black, vignette, crimson glow, thin edges  |
| y2k     | Michroma           | System        | Soft    | Chrome, diagonal shine, reflective pill buttons |
| minimal | System             | System        | Flat    | Essential color, no glow/grain                  |

Small labels use Pixelify Sans and numbers Geist Mono except with system typography.

The energy map shows high, mid and low intensity from the existing three-band
analysis. The default **Follows tempo** scale spans eight seconds of
output time. The waveform, beat lines, cues and loops scale together. The
whole-track view and the audio engine keep their source-time positions.

## Implementation

- `src/lib/theme.ts`: RGB/HSV math, v1→v2 validation, `STYLES` seeds, `resolved()` and pure `palette()`. All colors originate here.
- `src/lib/theme-store.ts`: local cache, OS preference, tokens and `data-style`, `data-typography`, `data-texture`, `data-theme`; dither generation; SQLite save with 180 ms debounce.
- `src/lib/dither.ts`: 8×8 Bayer matrix; tiny theme-colored data-URL PNGs for X/Y gradients, grain and 50% fill. CSS scales with `image-rendering: pixelated`. Regenerate only if accent/secondary/text changes.
- `src/styles.css`: structural `:root` fonts/radii/texture fallbacks, then generated palette block from `scripts/sync-theme.ts`, then style/type/texture rules.
- `src/components/ThemePanel.tsx`: style previews with their own palette, typography/texture/waveform selectors and color controls.
- `src-tauri/src/preferences.rs`: Theme v2, v1 row defaults, enum/hex validation.

Add color tokens in `palette()`, never arbitrary component hex. Intentional constants: presets, style seeds and waveform bands. Declare structural variables in CSS; set component `--t`, `--p`, `--turn`, `--art-*`, `--zone` inline. Integration tests check every `var()` has a source. Fonts are local Fontsource/OFL; CSP disallows remote fonts (ADR-009).

## Animation and interaction

Web Animations light `data-action` controls/`data-key` keycaps without React rendering. HUD shows key/action, e.g. “A · Hot cue 2”; mode-blocked keys show unavailable. Mode indicator slides, play breathes, platters follow engine position and views enter softly. `prefers-reduced-motion` disables animation/transitions.

## macOS header

`tauri.macos.conf.json`: `titleBarStyle: Overlay`, `hiddenTitle: true`, native controls at (18, 22). Lowercase brand sits right of controls, aligned with topbar. Brand/topbar form drag region; no HTML replica titlebar/buttons. Windows/Linux keep native decorations.

Mini view uses narrowly allowed `setSize`, `setMinSize`, `setAlwaysOnTop`, remembers/restores previous size.

## Evidence and limits

Pure tests cover five styles, three modes, three variants and both OS appearances. The colors include all 11 bases from the [pinned theme vectors](../vendor/nu9ve-core/data/theme-vectors-v1.json), curado's presets, style suggestions and neutral extremes. Each style matches the expected `accent` and `onAccent` in the 110 cases. Text on an accent fill uses whichever of pure black and white has more contrast.

On the chrome, the sheet and the raised, hover and input surfaces, accent, muted, danger and success text have at least 4.5:1 contrast and primary and secondary text at least 7:1. Danger also holds 4.5:1 on its own background, text holds 7:1 on both selection tints, and deck B holds 3:1 on the sheet with readable text on it ([ADR-036](DECISIONS.md)). The tests also check waveform highs in light mode, valid hex tokens and migration from v1 to v2.

Math is not a full visual audit. A later authorized web-preview review covered curado at 1100/1440 px without native audio; native loaded decks and other styles remain unreviewed. Current preference prohibits further Computer Use/visual tests without explicit instruction. `color-mix()` and container units need Safari 16+ WebKit; macOS 12 with unupdated Safari 15 may fall back to unmixed backgrounds. Existing README imagery is a development capture, not evidence of current visual verification.

## Suite theme integration

The pinned [theme contract](../vendor/nu9ve-core/contracts/nu9ve-theme-v1.md)
maps `canvas`, `surface`, `raised`, `ink`, `muted` and `secondary` to
`--chrome`, `--surface`, `--surface-raised`, `--text`, `--text-muted` and
`--deck-b`. `src/lib/suite-theme.ts` is the TypeScript port of the shared
recipe (`ThemeColor`, `ThemeRoles` and `ThemeSurfaces` from Swift and C#) for the
five common skins and both eras. Since 2026-11-27 it gives every role of the 110
shared vectors and the 44 tinted era vectors exactly, by hex. `SUITE_ROLE_TOKENS`
in `theme.ts` maps every role to its CSS token: `canvas` `--chrome`, `surface`
`--surface`, `raised` `--surface-raised`, `line` `--line`, `ink` `--text`,
`muted` `--text-muted`, `accent` `--accent`, `onAccent` `--accent-ink`,
`accentText` `--accent-text`, `accentSoft` `--accent-soft`, the four states
`--danger`, `--warning`, `--success` and `--info`, `warningSoft`
`--warning-soft` and `secondary` `--deck-b`. Since 2026-11-27 the states start
from the suite's semantic colors mixed 12 % toward the text, then hold 4.5:1 on
every ground (ADR-039). curado's styles keep their own surfaces: the skin
recipe is not applied to them. Waveform, deck and meter colors stay domain roles. Offering Macintosh, Windows 9x or Terminal as
styles is a separate decision: the era option and its `tinted` default only apply
to those skins. The local pixel style is distinct from the common `classicMac` skin.

Keep the current integrated macOS header and native window controls. The frontend
already applies the resolved appearance to Tauri's window and passes the accent
to the Metal helper. Verify the theme boundary there and on Windows while the
same palette supplies the WebView. Preserve the existing coral default, saved
colors and Theme v2 settings through any migration to the common preference
format. Choose a contrast skin with the maintainer before adding one.

## Localization

The frontend uses `tr(es, en)` with Spanish fallback and updates the document
language immediately. Settings → Appearance persists the choice in WebView local
storage. MIDI settings use the same selector. Preset metadata, some dynamic
strings, native menus, dialogs and many Rust errors still need coverage.

Move UI text to stable catalog keys incrementally. Use message identifiers and
parameters at the Rust boundary so the frontend can translate errors without
matching sentences. Native menus, dialogs and bundle resources need their own
locale coverage. Test switching, restart persistence, accessible names and
toolbar truncation in Spanish and English with the original and common skins.
