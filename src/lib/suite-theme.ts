// nu9ve-theme-v1 §2, §3 and §5 · the suite's role recipe in TypeScript, a literal port of
// Nu9veTheme ThemeRoles.swift / ThemeSurfaces.swift and Nu9ve.Core ThemeRoles.cs (nu9ve/core).
// Same formulas, same order of operations and the same hex rounding (nearest, away from zero),
// so the shared vectors give the same hex in Swift, C# and here. curado's own five styles keep
// their palette in theme.ts; this module is the common skins, ready for a window or style that
// adopts them. Pure: no DOM.

export type SuiteSkin = 'glass' | 'classicMac' | 'win9x' | 'terminal' | 'gothic';
export type SuiteEra = 'original' | 'tinted';
export const SUITE_SKINS: readonly SuiteSkin[] = [
  'glass',
  'classicMac',
  'win9x',
  'terminal',
  'gothic',
];

/** Opaque, normalized sRGB. */
export type Rgb = readonly [number, number, number];
const BLACK: Rgb = [0, 0, 0];
const WHITE: Rgb = [1, 1, 1];

export function fromHex(hex: string): Rgb | null {
  const value = hex.trim().replace(/^#/, '');
  if (!/^[0-9a-fA-F]{6}$/.test(value)) return null;
  const rgb = parseInt(value, 16);
  return [((rgb >> 16) & 255) / 255, ((rgb >> 8) & 255) / 255, (rgb & 255) / 255];
}
function hex(value: string): Rgb {
  const color = fromHex(value);
  if (!color) throw new Error(`Invalid hex ${value}`);
  return color;
}
/** Nearest, halves away from zero, as Swift and C# `MidpointRounding.AwayFromZero`. */
function byte(channel: number): number {
  const scaled = channel * 255;
  return Math.sign(scaled) * Math.round(Math.abs(scaled));
}
export function toHex(color: Rgb): string {
  return `#${color
    .map((c) =>
      Math.min(255, Math.max(0, byte(c)))
        .toString(16)
        .toUpperCase()
        .padStart(2, '0'),
    )
    .join('')}`;
}
function luminance([r, g, b]: Rgb): number {
  const linear = (x: number) => (x <= 0.04045 ? x / 12.92 : Math.pow((x + 0.055) / 1.055, 2.4));
  return 0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b);
}
export function contrastOf(a: Rgb, b: Rgb): number {
  const la = luminance(a);
  const lb = luminance(b);
  return (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05);
}
function mixed(a: Rgb, b: Rgb, amount: number): Rgb {
  return [
    a[0] + (b[0] - a[0]) * amount,
    a[1] + (b[1] - a[1]) * amount,
    a[2] + (b[2] - a[2]) * amount,
  ];
}
/** Unchanged when it already reads well; otherwise mixed toward the better text polarity. */
function readable(color: Rgb, backgrounds: readonly Rgb[], minimum = 4.5): Rgb {
  const score = (c: Rgb) =>
    backgrounds.length === 0 ? 21 : Math.min(...backgrounds.map((b) => contrastOf(c, b)));
  if (score(color) >= minimum) return color;
  const target = score(BLACK) >= score(WHITE) ? BLACK : WHITE;
  if (score(target) < minimum) return target;
  let low = 0;
  let high = 1;
  for (let i = 0; i < 20; i++) {
    const mid = (low + high) / 2;
    if (score(mixed(color, target, mid)) >= minimum) high = mid;
    else low = mid;
  }
  return mixed(color, target, high);
}
function onColor(color: Rgb): Rgb {
  return contrastOf(color, BLACK) >= contrastOf(color, WHITE) ? BLACK : WHITE;
}
/** Hue, saturation and value in 0…1. */
function toHsv([r, g, b]: Rgb): [number, number, number] {
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const delta = max - min;
  let hue = 0;
  if (delta > 0) {
    // JS % keeps the dividend's sign, as Swift truncatingRemainder and C# %.
    if (max === r) hue = ((g - b) / delta) % 6;
    else if (max === g) hue = (b - r) / delta + 2;
    else hue = (r - g) / delta + 4;
    hue /= 6;
    if (hue < 0) hue += 1;
  }
  return [hue, max === 0 ? 0 : delta / max, max];
}
function fromHsv(hue: number, saturation: number, value: number): Rgb {
  const h = (hue >= 1 ? 0 : Math.max(0, hue)) * 6;
  const sector = Math.trunc(h);
  const fraction = h - Math.floor(h);
  const p = value * (1 - saturation);
  const q = value * (1 - saturation * fraction);
  const t = value * (1 - saturation * (1 - fraction));
  switch (sector) {
    case 0:
      return [value, t, p];
    case 1:
      return [q, value, p];
    case 2:
      return [p, value, t];
    case 3:
      return [p, q, value];
    case 4:
      return [t, p, value];
    default:
      return [value, p, q];
  }
}
const gray = (level: number): Rgb => [level, level, level];

export function hasEra(skin: SuiteSkin): boolean {
  return skin === 'classicMac' || skin === 'win9x';
}
/** The mode a skin fixes with that era, or null when it follows the person's. */
export function forcedDark(skin: SuiteSkin, era: SuiteEra = 'original'): boolean | null {
  if (era === 'tinted' && hasEra(skin)) return null;
  if (skin === 'classicMac' || skin === 'win9x') return false;
  if (skin === 'gothic' || skin === 'terminal') return true;
  return null;
}
/** The mode a skin starts in when the person has not chosen one for it (§5). */
export function defaultDark(skin: SuiteSkin, era: SuiteEra): boolean | null {
  return forcedDark(skin, era) ?? (skin === 'classicMac' ? false : null);
}

interface Surfaces {
  canvas: Rgb;
  surface: Rgb;
  raised: Rgb;
  line: Rgb;
  preferredInk: Rgb;
}
function tinted(skin: SuiteSkin, dark: boolean, seed: Rgb): Surfaces {
  if (skin === 'classicMac') {
    const paper = dark ? BLACK : WHITE;
    const ink = dark ? WHITE : BLACK;
    const canvas = mixed(seed, paper, 0.9);
    const surface = mixed(seed, paper, dark ? 0.84 : 0.96);
    const raised = mixed(seed, paper, dark ? 0.76 : 0.84);
    const line = readable(mixed(seed, ink, 0.35), [canvas, surface, raised], 3.1);
    return { canvas, surface, raised, line, preferredInk: mixed(seed, ink, dark ? 0.92 : 0.9) };
  }
  if (dark)
    return {
      canvas: mixed(gray(0.16), seed, 0.16),
      surface: mixed(gray(0.2), seed, 0.12),
      raised: mixed(gray(0.27), seed, 0.08),
      line: mixed(gray(0.06), seed, 0.08),
      preferredInk: WHITE,
    };
  return {
    canvas: mixed(gray(200 / 255), seed, 0.22),
    surface: mixed(gray(225 / 255), seed, 0.16),
    raised: mixed(WHITE, seed, 0.06),
    line: mixed(gray(128 / 255), seed, 0.28),
    preferredInk: BLACK,
  };
}
function surfacesFor(skin: SuiteSkin, dark: boolean, seed: Rgb, era: SuiteEra): Surfaces {
  if (era === 'tinted' && hasEra(skin)) return tinted(skin, dark, seed);
  const base = dark ? BLACK : WHITE;
  const blend = (fraction: number) => mixed(seed, base, fraction);
  switch (skin) {
    case 'classicMac':
      return {
        canvas: WHITE,
        surface: WHITE,
        raised: gray(0.93),
        line: BLACK,
        preferredInk: BLACK,
      };
    case 'win9x':
      return {
        canvas: gray(192 / 255),
        surface: gray(223 / 255),
        raised: WHITE,
        line: gray(128 / 255),
        preferredInk: BLACK,
      };
    case 'terminal': {
      const canvas = mixed(hex('#050806'), seed, 0.05);
      const [h, s] = toHsv(seed);
      const phosphor = fromHsv(h, Math.min(s, 0.55), 1);
      return {
        canvas,
        surface: mixed(hex('#0B100C'), seed, 0.06),
        raised: mixed(hex('#121912'), seed, 0.08),
        line: mixed(canvas, phosphor, 0.5),
        preferredInk: mixed(phosphor, WHITE, 0.15),
      };
    }
    case 'gothic': {
      const canvas = mixed(hex('#09070A'), seed, 0.04);
      return {
        canvas,
        surface: mixed(hex('#141015'), seed, 0.04),
        raised: mixed(hex('#1E181F'), seed, 0.05),
        line: mixed(canvas, seed, 0.45),
        preferredInk: hex('#F0E8E6'),
      };
    }
    default:
      return {
        canvas: blend(dark ? 0.91 : 0.8),
        surface: blend(dark ? 0.85 : 0.94),
        raised: blend(0.88),
        line: blend(dark ? 0.64 : 0.57),
        preferredInk: dark ? gray(0.95) : mixed(seed, BLACK, 0.86),
      };
  }
}

/** The semantic colors the state roles start from (§2), before they mix with `ink`. */
export const STATE_COLORS = {
  danger: '#E5372C',
  warning: '#E8870E',
  success: '#2E9E57',
  info: '#2F78D6',
} as const;
/** The mix toward `ink` every state takes before contrast adjusts it. */
export const STATE_INK_MIX = 0.12;

export const ROLE_NAMES = [
  'canvas',
  'surface',
  'raised',
  'line',
  'ink',
  'muted',
  'accent',
  'onAccent',
  'accentText',
  'accentSoft',
  'danger',
  'warning',
  'success',
  'info',
  'warningSoft',
] as const;
export type RoleName = (typeof ROLE_NAMES)[number];
export interface SuiteRoles {
  /** The mode the roles were built in, after the skin's own rule. */
  dark: boolean;
  roles: Record<RoleName, string>;
}

/**
 * The common roles for a skin, the person's mode and base color: the vectors' input path
 * (hex → HSV → roles). A skin with a period look of its own overrides the mode unless its era
 * is tinted.
 */
export function suiteRoles(
  skin: SuiteSkin,
  requestedDark: boolean,
  base: string,
  era: SuiteEra = 'original',
): SuiteRoles {
  const [h, s, v] = toHsv(hex(base));
  return rolesFor(skin, requestedDark, fromHsv(h, s, v), era);
}
/** The same roles from the base color as is, with no HSV round trip (the era vectors' input). */
export function suiteRolesFromBase(
  skin: SuiteSkin,
  requestedDark: boolean,
  base: string,
  era: SuiteEra = 'original',
): SuiteRoles {
  return rolesFor(skin, requestedDark, hex(base), era);
}
function rolesFor(skin: SuiteSkin, requestedDark: boolean, seed: Rgb, era: SuiteEra): SuiteRoles {
  const dark = forcedDark(skin, era) ?? requestedDark;
  const { canvas, surface, raised, line, preferredInk } = surfacesFor(skin, dark, seed, era);
  const backgrounds = [canvas, surface, raised];
  const ink = readable(preferredInk, backgrounds, 7);
  const muted = readable(mixed(surface, ink, 0.68), backgrounds, 4.5);
  const accentText = readable(dark ? seed : mixed(seed, BLACK, 0.48), backgrounds, 4.5);
  let soft = mixed(canvas, seed, 0.32);
  for (let step = 0; step <= 7; step++) {
    const amount = Math.max(0, 0.28 - step * 0.04);
    if (contrastOf(ink, soft) < 7) soft = mixed(canvas, seed, amount);
  }
  const orange = hex(STATE_COLORS.warning);
  let banner = mixed(surface, orange, 0.14);
  for (const amount of [0.12, 0.1, 0.08, 0.06, 0.04, 0.02, 0])
    if (contrastOf(ink, banner) < 7) banner = mixed(surface, orange, amount);
  const state = (value: string, ...extra: Rgb[]) =>
    readable(mixed(hex(value), ink, STATE_INK_MIX), [...backgrounds, ...extra], 4.5);
  const roles: Record<RoleName, Rgb> = {
    canvas,
    surface,
    raised,
    line,
    ink,
    muted,
    accent: seed,
    onAccent: onColor(seed),
    accentText,
    accentSoft: soft,
    danger: state(STATE_COLORS.danger),
    warning: state(STATE_COLORS.warning, banner),
    success: state(STATE_COLORS.success),
    info: state(STATE_COLORS.info),
    warningSoft: banner,
  };
  return {
    dark,
    roles: Object.fromEntries(ROLE_NAMES.map((name) => [name, toHex(roles[name])])) as Record<
      RoleName,
      string
    >,
  };
}
