/** Theme math is pure: usable by tests, Canvas and the CSS token generator. */
export type ThemeMode = 'dark' | 'light' | 'system';
export type ThemeVariant = 'neutral' | 'tinted' | 'contrast';
export interface ThemeSettings {
  version: 1;
  mode: ThemeMode;
  variant: ThemeVariant;
  base: string;
}
export interface HSV {
  h: number;
  s: number;
  v: number;
}
export const DEFAULT_THEME: ThemeSettings = {
  version: 1,
  mode: 'dark',
  variant: 'neutral',
  base: '#d66154',
};
export const COLOR_PRESETS = [
  { name: 'Coral', value: '#d66154' },
  { name: 'Ámbar', value: '#dca04b' },
  { name: 'Salvia', value: '#92af82' },
  { name: 'Océano', value: '#548bbf' },
  { name: 'Lavanda', value: '#9b7acd' },
  { name: 'Rosa', value: '#ce759e' },
];
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
function accessibleAccent(base: string, bg: string, target: number) {
  if (contrast(base, bg) >= target) return base;
  const endpoint = luminance(bg) < 0.3 ? '#ffffff' : '#111111';
  for (let i = 1; i <= 100; i++) {
    const color = mix(base, endpoint, i / 100);
    if (contrast(color, bg) >= target) return color;
  }
  return endpoint;
}
export function validateTheme(value: unknown): ThemeSettings {
  if (!value || typeof value !== 'object') return { ...DEFAULT_THEME };
  const v = value as Partial<ThemeSettings>;
  if (v.version !== undefined && v.version !== 1) return { ...DEFAULT_THEME };
  const base = typeof v.base === 'string' ? normalizeHex(v.base) : null;
  return {
    version: 1,
    mode: ['dark', 'light', 'system'].includes(v.mode ?? '') ? v.mode! : DEFAULT_THEME.mode,
    variant: ['neutral', 'tinted', 'contrast'].includes(v.variant ?? '')
      ? v.variant!
      : DEFAULT_THEME.variant,
    base: base ?? DEFAULT_THEME.base,
  };
}
export function nativeWindowTheme(settings: ThemeSettings): 'dark' | 'light' | null {
  return settings.mode === 'system' ? null : settings.mode;
}
export function palette(settings: ThemeSettings, systemDark = false) {
  const dark = settings.mode === 'dark' || (settings.mode === 'system' && systemDark);
  const high = settings.variant === 'contrast';
  const tint = settings.variant === 'tinted' ? 0.16 : settings.variant === 'neutral' ? 0.025 : 0;
  const surface = (neutral: string) => mix(neutral, settings.base, tint);
  const bg = surface(dark ? (high ? '#090909' : '#141313') : high ? '#ffffff' : '#f6f3f2');
  const panel = surface(dark ? '#1c1a19' : '#ffffff');
  const text = dark ? '#f5f1ee' : '#261e1c';
  const accent = settings.base;
  const accentText = accessibleAccent(accent, panel, 4.5);
  const secondary = hsvToHex({
    h: (hexToHsv(accent).h + 185) % 360,
    s: dark ? 0.35 : 0.62,
    v: dark ? 0.85 : 0.55,
  });
  return {
    dark,
    bg,
    panel,
    text,
    accent,
    secondary,
    tokens: {
      '--accent': accent,
      '--accent-text': accentText,
      '--accent-ink': readableInk(accent),
      '--accent-hover': mix(accent, dark ? '#ffffff' : '#111111', 0.12),
      '--accent-soft': mix(panel, accent, high ? 0.24 : 0.13),
      '--bg': bg,
      '--surface': panel,
      '--surface-sidebar': surface(dark ? '#181615' : '#eeebe9'),
      '--surface-input': surface(dark ? '#141211' : '#f8f6f5'),
      '--surface-raised': surface(dark ? '#272321' : '#e8e3e0'),
      '--surface-hover': surface(dark ? '#332d2a' : '#ded6d1'),
      '--text': text,
      '--text-secondary': dark ? (high ? '#e2d8d2' : '#c0b4ac') : high ? '#362a25' : '#62514a',
      '--text-muted': accessibleAccent(
        dark ? (high ? '#c4b5ac' : '#a09288') : high ? '#4c3c33' : '#78685f',
        panel,
        high ? 7 : 4.6,
      ),
      '--line': dark ? (high ? '#ffffff4d' : '#ffffff13') : high ? '#30251d66' : '#36281e24',
      '--line-strong': dark ? '#ffffff38' : '#36281e48',
      '--deck-b': secondary,
      '--deck-b-ink': readableInk(secondary),
      '--deck-b-soft': mix(panel, secondary, 0.12),
      '--danger': dark ? '#f0af9d' : '#a33c31',
      '--danger-bg': dark ? '#3a2420' : '#fff0eb',
      '--danger-line': dark ? '#cc7d6355' : '#a33c3155',
      '--success': dark ? '#a9caa0' : '#3c6c36',
      '--visual-bg': surface(dark ? '#191716' : '#e9e2de'),
      '--overlay': '#00000099',
      '--shadow': dark ? '#00000066' : '#36281e1f',
      '--art-a': mix(panel, accent, dark ? 0.32 : 0.2),
      '--art-b': mix(panel, secondary, dark ? 0.3 : 0.2),
      '--art-c': mix(panel, '#ac8855', dark ? 0.32 : 0.22),
      '--art-d': mix(panel, '#9b7acd', dark ? 0.32 : 0.22),
      '--range-track': mix(panel, accent, dark ? 0.3 : 0.25),
      '--meter-end': dark ? '#e0b06b' : '#a76820',
    } satisfies Record<string, string>,
  };
}
