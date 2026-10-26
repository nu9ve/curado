/** Theme math is pure: usable by tests, Canvas and the CSS token generator. */
export type ThemeMode = 'dark' | 'light' | 'system';
export type ThemeVariant = 'neutral' | 'tinted' | 'contrast';
export type ThemeStyle = 'curado' | 'pixel' | 'gothic' | 'y2k' | 'minimal';
export type Typography = 'auto' | 'display' | 'pixel' | 'system' | 'mono';
export type Texture = 'auto' | 'dither' | 'smooth' | 'flat';
export type WaveStyle = 'bands' | 'rgb' | 'mono';
export interface ThemeSettings {
  version: 2;
  mode: ThemeMode;
  variant: ThemeVariant;
  base: string;
  style: ThemeStyle;
  typography: Typography;
  texture: Texture;
  waveform: WaveStyle;
}
export interface HSV {
  h: number;
  s: number;
  v: number;
}
export const DEFAULT_THEME: ThemeSettings = {
  version: 2,
  mode: 'dark',
  variant: 'neutral',
  base: '#d66154',
  style: 'curado',
  typography: 'auto',
  texture: 'auto',
  waveform: 'bands',
};
export const COLOR_PRESETS = [
  { name: 'Coral', value: '#d66154' },
  { name: 'Ámbar', value: '#dca04b' },
  { name: 'Salvia', value: '#92af82' },
  { name: 'Océano', value: '#548bbf' },
  { name: 'Lavanda', value: '#9b7acd' },
  { name: 'Rosa', value: '#ce759e' },
];

interface StyleSeed {
  name: string;
  description: string;
  /** A color that suits the style. Selecting a style never overrides the user's base. */
  suggested: string;
  typography: Exclude<Typography, 'auto'>;
  texture: Exclude<Texture, 'auto'>;
  dark: { chrome: string; sheet: string; text: string };
  light: { chrome: string; sheet: string; text: string };
  secondary: { shift: number; dark: [number, number]; light: [number, number] };
  /** Second stop of the smooth accent gradient. */
  gradient: (accent: string, secondary: string, dark: boolean) => string;
  wave: { low: string; mid: string; high: string };
}
export const STYLES: Record<ThemeStyle, StyleSeed> = {
  curado: {
    name: 'curado',
    description: 'Cálido, vanguardista: gradientes suaves y detalles pixel.',
    suggested: '#d66154',
    typography: 'display',
    texture: 'dither',
    dark: { chrome: '#110e0e', sheet: '#1b1716', text: '#f6f0ec' },
    light: { chrome: '#e4dcd7', sheet: '#fbf8f6', text: '#241b19' },
    secondary: { shift: 185, dark: [0.38, 0.86], light: [0.62, 0.52] },
    gradient: (a, _s, dark) => mix(a, dark ? '#f2b36d' : '#c9772f', 0.5),
    wave: { low: '#2d6cff', mid: '#ff9e2c', high: '#f8f2e8' },
  },
  pixel: {
    name: 'pixel',
    description: 'Bordes duros, bloques y dither: una consola de 8 bits.',
    suggested: '#e0584f',
    typography: 'pixel',
    texture: 'dither',
    dark: { chrome: '#0d0d12', sheet: '#17171f', text: '#f1efe6' },
    light: { chrome: '#d9d4c3', sheet: '#f4f0e2', text: '#1d1b16' },
    secondary: { shift: 160, dark: [0.55, 0.9], light: [0.7, 0.5] },
    gradient: (_a, s) => s,
    wave: { low: '#3d7bff', mid: '#ffb02e', high: '#ffffff' },
  },
  gothic: {
    name: 'gótico',
    description: 'Negro profundo, carmesí y tipografía gótica para sesiones nocturnas.',
    suggested: '#a3263f',
    typography: 'display',
    texture: 'smooth',
    dark: { chrome: '#09070a', sheet: '#141015', text: '#f0e8e6' },
    light: { chrome: '#ddd3c6', sheet: '#f5efe5', text: '#21171a' },
    secondary: { shift: 230, dark: [0.42, 0.78], light: [0.55, 0.45] },
    gradient: (a, _s, dark) => mix(a, dark ? '#4b1660' : '#6b2a7a', 0.55),
    wave: { low: '#7b3cff', mid: '#e0384f', high: '#efe4da' },
  },
  y2k: {
    name: 'y2k',
    description: 'Cromo, brillo y burbujas: el futuro de hace veinticinco años.',
    suggested: '#7c8cff',
    typography: 'display',
    texture: 'smooth',
    dark: { chrome: '#0e1120', sheet: '#181d31', text: '#f1f4ff' },
    light: { chrome: '#d8ddea', sheet: '#f8f9fd', text: '#161a2b' },
    secondary: { shift: 120, dark: [0.5, 0.95], light: [0.7, 0.6] },
    gradient: (_a, s, dark) => mix(s, '#ffffff', dark ? 0.25 : 0.05),
    wave: { low: '#4f7cff', mid: '#ff62d0', high: '#e6fcff' },
  },
  minimal: {
    name: 'mínimo',
    description: 'Plano y silencioso. Sin texturas; solo el color esencial.',
    suggested: '#d66154',
    typography: 'system',
    texture: 'flat',
    dark: { chrome: '#121212', sheet: '#1b1b1b', text: '#f2f2f2' },
    light: { chrome: '#e9e9e9', sheet: '#ffffff', text: '#1c1c1c' },
    secondary: { shift: 180, dark: [0.25, 0.8], light: [0.45, 0.5] },
    gradient: (a, _s, dark) => mix(a, dark ? '#ffffff' : '#000000', 0.15),
    wave: { low: '#5b7fd6', mid: '#d8a35c', high: '#eeeeee' },
  },
};
export const STYLE_IDS = Object.keys(STYLES) as ThemeStyle[];
const MODES: ThemeMode[] = ['dark', 'light', 'system'];
const VARIANTS: ThemeVariant[] = ['neutral', 'tinted', 'contrast'];
const TYPOGRAPHY: Typography[] = ['auto', 'display', 'pixel', 'system', 'mono'];
const TEXTURES: Texture[] = ['auto', 'dither', 'smooth', 'flat'];
const WAVES: WaveStyle[] = ['bands', 'rgb', 'mono'];

const clamp = (n: number, min = 0, max = 1) =>
  Math.min(max, Math.max(min, Number.isFinite(n) ? n : min));
export function normalizeHex(value: string): string | null {
  const s = value.trim().replace(/^#/, '');
  if (/^[0-9a-f]{3}$/i.test(s))
    return `#${[...s]
      .map((c) => c + c)
      .join('')
      .toLowerCase()}`;
  return /^[0-9a-f]{6}$/i.test(s) ? `#${s.toLowerCase()}` : null;
}
export function rgb(hex: string): [number, number, number] {
  const s = normalizeHex(hex) ?? DEFAULT_THEME.base;
  return [1, 3, 5].map((i) => parseInt(s.slice(i, i + 2), 16) / 255) as [number, number, number];
}
function hexOf(values: number[]) {
  return `#${values
    .map((n) =>
      Math.round(clamp(n) * 255)
        .toString(16)
        .padStart(2, '0'),
    )
    .join('')}`;
}
/** 8-digit hex with alpha, for hairlines and glows. */
export function alpha(hex: string, a: number) {
  return `${normalizeHex(hex) ?? DEFAULT_THEME.base}${Math.round(clamp(a) * 255)
    .toString(16)
    .padStart(2, '0')}`;
}
export function hexToHsv(hex: string): HSV {
  const [r, g, b] = rgb(hex),
    max = Math.max(r, g, b),
    min = Math.min(r, g, b),
    d = max - min;
  let h = 0;
  if (d)
    h =
      (max === r ? (g - b) / d + (g < b ? 6 : 0) : max === g ? (b - r) / d + 2 : (r - g) / d + 4) *
      60;
  return { h, s: max ? d / max : 0, v: max };
}
export function hsvToHex({ h, s, v }: HSV): string {
  const hue = (((Number.isFinite(h) ? h : 0) % 360) + 360) % 360;
  const saturation = clamp(s),
    value = clamp(v);
  const c = value * saturation,
    x = c * (1 - Math.abs(((hue / 60) % 2) - 1)),
    m = value - c;
  const values =
    hue < 60
      ? [c, x, 0]
      : hue < 120
        ? [x, c, 0]
        : hue < 180
          ? [0, c, x]
          : hue < 240
            ? [0, x, c]
            : hue < 300
              ? [x, 0, c]
              : [c, 0, x];
  return hexOf(values.map((n) => n + m));
}
export function mix(a: string, b: string, t: number) {
  const aa = rgb(a),
    bb = rgb(b);
  return hexOf(aa.map((v, i) => v + (bb[i] - v) * clamp(t)));
}
export function luminance(hex: string) {
  const [r, g, b] = rgb(hex).map((v) => (v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}
export function contrast(a: string, b: string) {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}
export function readableInk(base: string) {
  return contrast(base, '#111111') >= contrast(base, '#ffffff') ? '#111111' : '#ffffff';
}
/** Moves `base` toward black/white until it reaches `target` contrast on every background. */
function accessibleOn(base: string, backgrounds: string[], target: number) {
  const ok = (c: string) => backgrounds.every((bg) => contrast(c, bg) >= target);
  if (ok(base)) return base;
  const endpoint = luminance(backgrounds[0]) < 0.3 ? '#ffffff' : '#111111';
  for (let i = 1; i <= 100; i++) {
    const color = mix(base, endpoint, i / 100);
    if (ok(color)) return color;
  }
  return endpoint;
}
const pick = <T extends string>(value: unknown, allowed: T[], fallback: T): T =>
  allowed.includes(value as T) ? (value as T) : fallback;
/** Accepts v1 (pre-styles) and v2 preferences; anything else falls back to the default. */
export function validateTheme(value: unknown): ThemeSettings {
  if (!value || typeof value !== 'object') return { ...DEFAULT_THEME };
  const v = value as Partial<Omit<ThemeSettings, 'version'>> & { version?: number };
  if (v.version !== undefined && v.version !== 1 && v.version !== 2) return { ...DEFAULT_THEME };
  const base = typeof v.base === 'string' ? normalizeHex(v.base) : null;
  return {
    version: 2,
    mode: pick(v.mode, MODES, DEFAULT_THEME.mode),
    variant: pick(v.variant, VARIANTS, DEFAULT_THEME.variant),
    base: base ?? DEFAULT_THEME.base,
    style: pick(v.style, STYLE_IDS, DEFAULT_THEME.style),
    typography: pick(v.typography, TYPOGRAPHY, DEFAULT_THEME.typography),
    texture: pick(v.texture, TEXTURES, DEFAULT_THEME.texture),
    waveform: pick(v.waveform, WAVES, DEFAULT_THEME.waveform),
  };
}
export function nativeWindowTheme(settings: ThemeSettings): 'dark' | 'light' | null {
  return settings.mode === 'system' ? null : settings.mode;
}
/** Structural choices resolved from the style when the user leaves them on auto. */
export function resolved(settings: ThemeSettings) {
  const seed = STYLES[settings.style];
  return {
    typography: settings.typography === 'auto' ? seed.typography : settings.typography,
    texture: settings.texture === 'auto' ? seed.texture : settings.texture,
  };
}
export function palette(settings: ThemeSettings, systemDark = false) {
  const seed = STYLES[settings.style] ?? STYLES.curado;
  const dark = settings.mode === 'dark' || (settings.mode === 'system' && systemDark);
  const high = settings.variant === 'contrast';
  const tint = settings.variant === 'tinted' ? 0.16 : settings.variant === 'neutral' ? 0.03 : 0;
  const surface = (neutral: string) => mix(neutral, settings.base, tint);
  const n = dark ? seed.dark : seed.light;
  const deepen = (c: string, t: number) => mix(c, dark ? '#000000' : '#ffffff', t);
  const chrome = surface(high ? deepen(n.chrome, 0.5) : n.chrome);
  const panel = surface(high ? deepen(n.sheet, 0.45) : n.sheet);
  const bg = chrome;
  const text = high ? (dark ? '#ffffff' : '#000000') : n.text;
  const accent = settings.base;
  const accentText = accessibleOn(accent, [panel, chrome], 4.5);
  const hue = hexToHsv(accent).h;
  const [secS, secV] = dark ? seed.secondary.dark : seed.secondary.light;
  const secondary = hsvToHex({ h: (hue + seed.secondary.shift) % 360, s: secS, v: secV });
  const gradient = seed.gradient(accent, secondary, dark);
  const raised = mix(panel, text, dark ? 0.07 : 0.05);
  // Light sheets cannot show white highs: bands darken toward the text color.
  const wave = dark
    ? seed.wave
    : {
        low: mix(seed.wave.low, text, 0.15),
        mid: mix(seed.wave.mid, text, 0.2),
        high: mix(seed.wave.high, text, 0.82),
      };
  return {
    dark,
    bg,
    panel,
    chrome,
    text,
    accent,
    secondary,
    gradient,
    wave,
    tokens: {
      '--accent': accent,
      '--accent-text': accentText,
      '--accent-ink': readableInk(accent),
      '--accent-hover': mix(accent, dark ? '#ffffff' : '#111111', 0.12),
      '--accent-soft': mix(panel, accent, high ? 0.26 : 0.14),
      '--accent-glow': alpha(accent, dark ? 0.42 : 0.3),
      '--gradient-end': gradient,
      '--bg': bg,
      '--chrome': chrome,
      '--chrome-2': mix(chrome, accent, dark ? 0.07 : 0.06),
      '--chrome-line': alpha(text, dark ? 0.06 : 0.08),
      '--surface': panel,
      '--sheet-2': mix(panel, chrome, 0.35),
      '--sheet-edge': alpha(text, dark ? (high ? 0.3 : 0.08) : high ? 0.4 : 0.1),
      '--sheet-shadow': dark ? '#000000a6' : '#3a281e33',
      '--surface-sidebar': chrome,
      '--surface-input': mix(panel, chrome, dark ? 0.55 : 0.3),
      '--surface-raised': raised,
      '--surface-hover': mix(panel, text, dark ? 0.12 : 0.09),
      '--text': text,
      '--text-secondary': accessibleOn(mix(text, panel, high ? 0.12 : 0.28), [panel, chrome], 7),
      '--text-muted': accessibleOn(mix(text, panel, 0.45), [panel, chrome], high ? 7 : 4.6),
      '--line': alpha(text, dark ? (high ? 0.32 : 0.08) : high ? 0.4 : 0.12),
      '--line-strong': alpha(text, dark ? 0.24 : 0.3),
      '--deck-b': secondary,
      '--deck-b-ink': readableInk(secondary),
      '--deck-b-soft': mix(panel, secondary, 0.14),
      '--deck-b-glow': alpha(secondary, dark ? 0.4 : 0.3),
      '--danger': dark ? '#f0af9d' : '#a33c31',
      '--danger-bg': dark ? '#3a2420' : '#fff0eb',
      '--danger-line': dark ? '#cc7d6355' : '#a33c3155',
      '--success': dark ? '#a9caa0' : '#3c6c36',
      '--visual-bg': mix(panel, chrome, 0.6),
      '--overlay': dark ? '#000000a6' : '#1a120f66',
      '--shadow': dark ? '#00000080' : '#36281e26',
      '--art-a': mix(panel, accent, dark ? 0.42 : 0.3),
      '--art-b': mix(panel, secondary, dark ? 0.38 : 0.28),
      '--range-track': mix(panel, text, dark ? 0.14 : 0.12),
      '--meter-end': dark ? '#e8b86b' : '#a76820',
      '--wave-low': wave.low,
      '--wave-mid': wave.mid,
      '--wave-high': wave.high,
      '--wave-dim': alpha(panel, 0.62),
    } satisfies Record<string, string>,
  };
}
