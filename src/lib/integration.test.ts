import { expect, test } from 'bun:test';
import { DEFAULT_THEME, palette } from './theme';
import mac from '../../src-tauri/tauri.macos.conf.json';
import capabilities from '../../src-tauri/capabilities/default.json';
test('every CSS color token exists and initial CSS matches the default theme', async () => {
  const css = await Bun.file(new URL('../styles.css', import.meta.url)).text();
  const tokens = palette(DEFAULT_THEME).tokens;
  for (const match of css.matchAll(/var\((--[\w-]+)\)/g)) expect(match[1] in tokens).toBe(true);
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
});
