import { describe, expect, test } from 'bun:test';
import {
  COLOR_PRESETS,
  DEFAULT_THEME,
  STYLES,
  STYLE_IDS,
  resolved,
  contrast,
  hexToHsv,
  hsvToHex,
  normalizeHex,
  palette,
  nativeWindowTheme,
  readableInk,
  validateTheme,
} from './theme';
describe('theme contract', () => {
  test('coral is the exact initial color and old/corrupt preferences fall back safely', () => {
    expect(DEFAULT_THEME.base).toBe('#d66154');
    expect(palette(DEFAULT_THEME).accent).toBe('#d66154');
    expect(validateTheme({ base: 'garbage', mode: 'other' })).toEqual(DEFAULT_THEME);
    expect(
      validateTheme({ version: 1, mode: 'light', variant: 'tinted', base: '#548bbf' }),
    ).toEqual({
      ...DEFAULT_THEME,
      mode: 'light',
      variant: 'tinted',
      base: '#548bbf',
    });
    expect(validateTheme({ ...DEFAULT_THEME, version: 3 })).toEqual(DEFAULT_THEME);
    expect(validateTheme({ ...DEFAULT_THEME, style: 'vaporwave' }).style).toBe('curado');
    expect(normalizeHex('#D65')).toBe('#dd6655');
    expect(normalizeHex('#d66154ff')).toBeNull();
    expect(normalizeHex('url(evil)')).toBeNull();
  });
  test('RGB/HSV conversions round trip all presets, extremes and wrapped hues', () => {
    for (const color of [
      ...COLOR_PRESETS.map((c) => c.value),
      '#000000',
      '#ffffff',
      '#ff0000',
      '#00ff00',
      '#0000ff',
    ])
      expect(hsvToHex(hexToHsv(color))).toBe(color);
    expect(hsvToHex({ h: 360, s: 1, v: 1 })).toBe('#ff0000');
    expect(hsvToHex({ h: -120, s: 1, v: 1 })).toBe('#0000ff');
    expect(hsvToHex({ h: NaN, s: Infinity, v: NaN })).toBe('#000000');
  });
  test('every style × color × mode × variant keeps the base color and readable text', () => {
    const bases = [...COLOR_PRESETS.map((c) => c.value), '#000000', '#ffffff', '#555555'];
    for (const style of STYLE_IDS)
      for (const base of [...bases, STYLES[style].suggested])
        for (const mode of ['light', 'dark', 'system'] as const)
          for (const variant of ['neutral', 'tinted', 'contrast'] as const)
            for (const systemDark of [false, true]) {
              const p = palette({ ...DEFAULT_THEME, style, base, mode, variant }, systemDark);
              expect(p.accent).toBe(base);
              expect(contrast(base, readableInk(base))).toBeGreaterThanOrEqual(4.5);
              for (const surface of [p.panel, p.chrome]) {
                expect(contrast(p.tokens['--accent-text'], surface)).toBeGreaterThanOrEqual(4.5);
                expect(contrast(p.text, surface)).toBeGreaterThan(7);
                expect(contrast(p.tokens['--text-muted'], surface)).toBeGreaterThan(4.5);
                expect(contrast(p.tokens['--text-secondary'], surface)).toBeGreaterThanOrEqual(7);
              }
              // Waveform highs must stay visible on the sheet in light mode too.
              expect(contrast(p.wave.high, p.panel)).toBeGreaterThan(1.8);
              expect(
                Object.values(p.tokens).every((v) => /^#[0-9a-f]{6}([0-9a-f]{2})?$/i.test(v)),
              ).toBe(true);
            }
  });
  test('styles resolve typography and texture unless the user overrides them', () => {
    expect(resolved(DEFAULT_THEME)).toEqual({ typography: 'display', texture: 'dither' });
    expect(resolved({ ...DEFAULT_THEME, style: 'minimal' })).toEqual({
      typography: 'system',
      texture: 'flat',
    });
    expect(resolved({ ...DEFAULT_THEME, style: 'pixel', texture: 'smooth' }).texture).toBe(
      'smooth',
    );
  });
  test('system theme follows OS and tonal/high-contrast variations differ', () => {
    expect(palette({ ...DEFAULT_THEME, mode: 'system' }, false).dark).toBe(false);
    expect(palette({ ...DEFAULT_THEME, mode: 'system' }, true).dark).toBe(true);
    expect(palette({ ...DEFAULT_THEME, variant: 'tinted' }).bg).not.toBe(palette(DEFAULT_THEME).bg);
  });
});

test('system mode releases the native window override instead of pinning a scheme', () => {
  expect(nativeWindowTheme({ ...DEFAULT_THEME, mode: 'system' })).toBeNull();
  expect(nativeWindowTheme(DEFAULT_THEME)).toBe('dark');
  expect(nativeWindowTheme({ ...DEFAULT_THEME, mode: 'light' })).toBe('light');
});
