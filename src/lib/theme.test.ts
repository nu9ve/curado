import { describe, expect, test } from 'bun:test';
import {
  COLOR_PRESETS,
  DEFAULT_THEME,
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
  test('all combinations preserve base color and readable text, even black/white accents', () => {
    for (const base of [...COLOR_PRESETS.map((c) => c.value), '#000000', '#ffffff', '#555555'])
      for (const mode of ['light', 'dark', 'system'] as const)
        for (const variant of ['neutral', 'tinted', 'contrast'] as const)
          for (const systemDark of [false, true]) {
            const p = palette({ ...DEFAULT_THEME, base, mode, variant }, systemDark);
            expect(p.accent).toBe(base);
            expect(contrast(base, readableInk(base))).toBeGreaterThanOrEqual(4.5);
            expect(contrast(p.tokens['--accent-text'], p.panel)).toBeGreaterThanOrEqual(4.5);
            expect(contrast(p.text, p.bg)).toBeGreaterThan(7);
            expect(contrast(p.tokens['--text-muted'], p.panel)).toBeGreaterThan(4.5);
            expect(
              Object.values(p.tokens).every((v) => /^#[0-9a-f]{6}([0-9a-f]{2})?$/i.test(v)),
            ).toBe(true);
          }
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
