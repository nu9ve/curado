# Appearance, styles and native window

## Visual language

curado combines two materials:

- **Chrome:** sidebar/topbar/footer form a continuous surface, soft gradient with main-color glow top-left and B-color glow bottom-right, plus ordered dither grain. No borders between them.
- **Sheet:** smooth elevated content with radius, shadow and a light edge. Grain versus smoothness and elevation separate navigation/content without cards/borders.

Inside the sheet, use flat sections, spacing and typography instead of making everything a card. Distinct material is reserved for meaningful surfaces, such as mixer hardware. Pixel logo/keycaps/labels/dither coexist with soft gradients on backgrounds, primary buttons, platters and progress, from accent to each style's `--gradient-end`. Later visual refinement made the mixer smooth rather than grainy; see CHANGELOG.

## Settings

**Configuración → Apariencia**:

| Setting    | Options                                     | Notes                                                              |
| ---------- | ------------------------------------------- | ------------------------------------------------------------------ |
| Style      | curado, pixel, gothic, y2k, minimal         | Default material/radii/type/texture; current UI uses Spanish names |
| Base color | Presets, picker, hex, HSV                   | User-owned; style suggests but does not force a color              |
| Scheme     | Dark, light, system                         | Follows OS when selected                                           |
| Variant    | Neutral, tonal, high contrast               | Tonal tints surfaces; contrast reinforces text/edges               |
| Typography | Style default, display, pixel, mono, system | `auto` follows style                                               |
| Texture    | Style default, dither, soft gradients, flat | `auto` follows style                                               |
| Waveform   | Three-band, frequency RGB, mono accent      | Style colors adapt to light mode                                   |

Defaults: **curado**, **#d66154**, dark, neutral, style typography/texture, three-band waveform. Reset restores these exact values.

| Style   | Display            | UI            | Texture | Character                                       |
| ------- | ------------------ | ------------- | ------- | ----------------------------------------------- |
| curado  | Jersey 15          | System        | Dither  | Warm coral→amber, pixel details                 |
| pixel   | Jersey 15          | Pixelify Sans | Dither  | Zero radius, 2 px edges, stepped shadows, no AA |
| gothic  | UnifrakturMaguntia | System        | Soft    | Deep black, vignette, crimson glow, thin edges  |
| y2k     | Michroma           | System        | Soft    | Chrome, diagonal shine, reflective pill buttons |
| minimal | System             | System        | Flat    | Essential color, no glow/grain                  |

Small labels use Pixelify Sans and numbers Geist Mono except with system typography.

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

Pure tests cover 5 styles ×10 colors ×3 modes ×3 variants ×light/dark OS. Chrome/sheet text-accent contrast ≥4.5:1; primary/secondary >7:1 / ≥7:1, muted >4.5:1; visible highs in light waveforms; valid hex and v1→v2 migration.

Math is not a full visual audit. A later authorized web-preview review covered curado at 1100/1440 px without native audio; native loaded decks and other styles remain unreviewed. Current preference prohibits further Computer Use/visual tests without explicit instruction. `color-mix()` and container units need Safari 16+ WebKit; macOS 12 with unupdated Safari 15 may fall back to unmixed backgrounds. Existing README imagery is a development capture, not evidence of current visual verification.
