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
    '--angle',
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

test('every application command is ACL-listed and only main receives its permission', async () => {
  const read = (path: string) => Bun.file(new URL(path, import.meta.url)).text();
  const [build, registry, app] = await Promise.all([
    read('../../src-tauri/build.rs'),
    read('../../src-tauri/app_commands.rs'),
    read('../../src-tauri/src/lib.rs'),
  ]);
  const commands = [...registry.matchAll(/"([a-z_]+)"/g)].map((m) => m[1]).sort();
  const block = app.match(/tauri::generate_handler!\[([\s\S]*?)\]/)?.[1];
  expect(block).toBeDefined();
  const registered = block!
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean)
    .sort();
  expect(commands).toEqual(registered);
  expect(new Set(commands).size).toBe(commands.length);
  expect(build).toContain('AppManifest::new().commands(app_commands::COMMANDS)');
  expect(capabilities.webviews).toEqual(['main']);
  expect('windows' in capabilities).toBe(false);
  expect('remote' in capabilities).toBe(false);
  const allowed = capabilities.permissions.filter((p) => p.startsWith('allow-')).sort();
  expect(allowed).toEqual(commands.map((c) => `allow-${c.replaceAll('_', '-')}`).sort());
  expect(app).toContain('web::allows_native_commands(invoke.message.webview_ref().label())');
  expect(app).toContain('web::hide_on_close(');
});

test('the service viewer cannot create permissive windows or inject remote controls into the engine', async () => {
  const source = await Bun.file(new URL('../../src-tauri/src/web.rs', import.meta.url)).text();
  expect(source).toContain('https://www.fractos.app/es/radio');
  expect(source).toContain('.incognito(true)');
  expect(source).toContain('.on_navigation(move |url| service.allows(url))');
  expect(source).toContain('NewWindowResponse::Deny');
  expect(source).toContain('.on_download(|_, _| false)');
  expect(source).not.toContain('.initialization_script(');
  expect(source).not.toContain('.eval(');
  expect(source).not.toContain('audio_action');
  expect(source).toMatch(/window\s*\.add_child\(/);
  expect(source).not.toContain('WebviewWindowBuilder');
  const app = await Bun.file(new URL('../../src-tauri/src/lib.rs', import.meta.url)).text();
  expect(app).not.toContain('get_webview_window(');
});
