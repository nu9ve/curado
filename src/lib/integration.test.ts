import { expect, test } from 'bun:test';
import { DEFAULT_THEME, palette } from './theme';
import mac from '../../src-tauri/tauri.macos.conf.json';
import capabilities from '../../src-tauri/capabilities/default.json';
test('every CSS variable is a palette token or a declared structural variable', async () => {
  const css = await Bun.file(new URL('../styles.css', import.meta.url)).text();
  const tokens = palette(DEFAULT_THEME).tokens;
  const declared = new Set([...css.matchAll(/(--[\w-]+)\s*:/g)].map((m) => m[1]));
  // Set inline by components (TrackArt, sliders, waveform overlays, knobs).
  const inline = [
    '--t',
    '--o',
    '--p',
    '--turn',
    '--art-mix',
    '--art-angle',
    '--art-x',
    '--art-y',
    '--zone',
  ];
  for (const match of css.matchAll(/var\((--[\w-]+)/g))
    expect(match[1] in tokens || declared.has(match[1]) || inline.includes(match[1])).toBe(true);
  // Texture variables are generated at runtime and must have a CSS fallback.
  for (const name of ['--tex-accent-x', '--tex-grain', '--tex-half'])
    expect(declared.has(name)).toBe(true);
  for (const [key, value] of Object.entries(tokens)) expect(css).toContain(`${key}: ${value};`);
  expect(css).not.toMatch(/#cbea99/i);
});
test('macOS uses integrated native controls with scoped drag/theme permissions', () => {
  const win = mac.app.windows[0];
  expect(win.titleBarStyle).toBe('Overlay');
  expect(win.hiddenTitle).toBe(true);
  expect(win.trafficLightPosition.x).toBeGreaterThan(0);
  expect(capabilities.permissions).toContain('core:window:allow-start-dragging');
  expect(capabilities.permissions).toContain('core:window:allow-set-theme');
  expect(capabilities.permissions).not.toContain('shell:default');
  // Mini player: only these window setters, no generic webview/window creation.
  const windowSetters = capabilities.permissions.filter((p) => p.startsWith('core:window:allow-'));
  expect(windowSetters.sort()).toEqual(
    [
      'core:window:allow-set-always-on-top',
      'core:window:allow-set-background-color',
      'core:window:allow-set-min-size',
      'core:window:allow-set-size',
      'core:window:allow-set-theme',
      'core:window:allow-start-dragging',
    ].sort(),
  );
  expect(capabilities.permissions.some((p) => p.includes('webview'))).toBe(false);
});
