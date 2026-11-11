import { expect, test } from 'bun:test';
import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
import base from '../src-tauri/tauri.conf.json';
import windows from '../src-tauri/tauri.windows.conf.json';
import { build, contrastReport, woffToSfnt } from './installer-art';
import { context, render } from './installer-check';

const root = resolve(import.meta.dir, '..');
const dir = resolve(root, 'src-tauri', 'windows');
const nsis = windows.bundle.windows.nsis;
const text = (name: string) => Bun.file(resolve(dir, name)).text();
const template = await text('installer.nsi');
const kit = await text('installer-kit.nsh');
const paletteNsh = await text('palette.nsh');

test('installer art, colors and font are the ones the generator makes', async () => {
  const { files } = await build();
  for (const [name, bytes] of files) {
    const committed = await Bun.file(resolve(dir, name)).bytes();
    expect(Buffer.from(committed).equals(Buffer.from(bytes))).toBe(true);
  }
  const size = (bytes: Uint8Array) => {
    const view = new DataView(bytes.buffer, bytes.byteOffset);
    return [view.getInt32(18, true), view.getInt32(22, true), view.getUint16(28, true)];
  };
  // Modern UI's sizes at 100 %, 24-bit.
  expect(size(files.get('sidebar.bmp')!)).toEqual([164, 314, 24]);
  expect(size(files.get('header.bmp')!)).toEqual([150, 57, 24]);
});

test('installer text keeps the app contrast on both surfaces', () => {
  const report = contrastReport();
  expect(report.text).toBeGreaterThanOrEqual(7);
  expect(report.text2).toBeGreaterThanOrEqual(7);
  expect(report.muted).toBeGreaterThanOrEqual(4.5);
  expect(report.accentText).toBeGreaterThanOrEqual(4.5);
  expect(report.signature).toBeGreaterThanOrEqual(4.5);
});

test('the display font is rebuilt whole from its web container', async () => {
  const woff = await Bun.file(
    resolve(root, 'node_modules/@fontsource/jersey-15/files/jersey-15-latin-400-normal.woff'),
  ).bytes();
  const { sfnt, family } = woffToSfnt(woff);
  const view = new DataView(sfnt.buffer);
  expect(family).toBe('Jersey 15');
  expect(view.getUint32(0)).toBe(0x00010000);
  expect(sfnt.length).toBe(new DataView(woff.buffer, woff.byteOffset).getUint32(16));
  expect(await text('palette.nsh')).toContain(`!define NU_DISPLAY_FONT "${family}"`);
});

test('the installer is per user, bilingual and points at files that exist', () => {
  expect(nsis.installMode).toBe('currentUser');
  expect(template).toContain('RequestExecutionLevel user');
  expect(nsis.languages).toEqual(['Spanish', 'English']);
  expect(nsis.displayLanguageSelector).toBe(false);
  expect(windows.bundle.targets).toEqual(['nsis']);
  for (const path of [
    nsis.template,
    nsis.installerHooks,
    nsis.installerIcon,
    nsis.sidebarImage,
    nsis.headerImage,
  ])
    expect(existsSync(resolve(root, 'src-tauri', path))).toBe(true);
  expect(base.bundle.publisher).toBe('nu9ve');
});

test('the installer never ends a running app', () => {
  // Upstream's CheckIfAppIsRunning force-closes the app, without asking in silent mode.
  for (const source of [template, kit]) {
    expect(source).not.toMatch(/!insertmacro CheckIfAppIsRunning/);
    expect(source).not.toMatch(/RmShutdown|KillProcess|taskkill/i);
  }
  expect(template.match(/!insertmacro NU_REQUIRE_APP_CLOSED/g)?.length).toBe(3);
});

test('/DEMO returns before anything is written', () => {
  const section = (name: string) => {
    const start = template.indexOf(`Section ${name}\n`);
    return template.slice(start, template.indexOf('SectionEnd', start));
  };
  const writes = /SetOutPath|File |WriteReg|CreateShortcut|ExecWait|NSISdl|WriteUninstaller/;
  for (const name of ['WebView2', 'Install']) {
    const body = section(name);
    const guard = body.indexOf('${If} $NuDemo = 1');
    expect(guard).toBeGreaterThan(0);
    expect(body.slice(guard, body.indexOf('${EndIf}', guard))).toContain('Return');
    expect(body.slice(0, guard)).not.toMatch(writes);
  }
  for (const name of [
    'CuradoWelcomeLeave',
    'CuradoFinishLeave',
    '.onInstSuccess',
    'PageReinstall',
  ]) {
    const start = template.indexOf(`Function ${name}\n`);
    const body = template.slice(start, template.indexOf('FunctionEnd', start));
    expect(body.slice(0, body.indexOf('$NuDemo = 1'))).not.toMatch(writes);
    expect(body).toContain('$NuDemo = 1');
  }
});

test('every installer text exists in Spanish and English', () => {
  const defined = (language: string) =>
    new Set(
      [
        ...template.matchAll(
          new RegExp(`^LangString (curado\\w+) \\$\\{LANG_${language}\\}`, 'gm'),
        ),
      ].map((m) => m[1]),
    );
  const spanish = defined('SPANISH');
  const english = defined('ENGLISH');
  expect([...spanish].sort()).toEqual([...english].sort());
  const used = new Set([...template.matchAll(/\$\((curado\w+)\)/g)].map((m) => m[1]));
  for (const name of used) expect(spanish.has(name)).toBe(true);
  for (const name of spanish) expect(used.has(name)).toBe(true);
});

test('lines set in the display font fit one line of the page', () => {
  // About 293 px at 100 %, and Jersey 15 at 16 pt advances about 9 px a character.
  const display = ['curadoHello', 'curadoHelloUpdate', 'curadoHelloSame', 'curadoDoneText'];
  const lines = [...template.matchAll(/^LangString (curado\w+) \$\{LANG_\w+\} "(.*)"$/gm)].filter(
    (m) => display.includes(m[1]),
  );
  expect(lines.length).toBe(display.length * 2);
  for (const [, , line] of lines) expect(line.length).toBeLessThanOrEqual(27);
});

test('choosing uninstall on the same version ends there instead of installing again', () => {
  const start = template.indexOf('Function PageLeaveReinstall');
  const body = template.slice(start, template.indexOf('FunctionEnd', start));
  expect(body).toMatch(
    /\$\{If\} \$R0 = 0\s+\$\{AndIf\} \$WixMode <> 1\s+Quit\s+\$\{EndIf\}\s+reinst_done:/,
  );
});

test('upstream behaviour the template must keep', () => {
  for (const hook of ['PREINSTALL', 'POSTINSTALL', 'PREUNINSTALL', 'POSTUNINSTALL'])
    expect(template).toContain(`!insertmacro NSIS_HOOK_${hook}`);
  for (const option of ['"/P"', '"/NS"', '"/UPDATE"', '"/R"', '"/ARGS"'])
    expect(template).toContain(`$CMDLINE ${option}`);
  // The library is removed only by the uninstaller's check box, never by an install.
  expect(template.match(/RmDir \/r "\$(LOCAL)?APPDATA\\\$\{BUNDLEID\}"/g)?.length).toBe(2);
  expect(template.indexOf('$DeleteAppDataCheckboxState = 1')).toBeLessThan(
    template.indexOf('RmDir /r "$APPDATA\\${BUNDLEID}"'),
  );
});

test('the kit stays app-agnostic and readable by makensis without a byte-order mark', () => {
  expect(/^[\x09\x0a\x20-\x7e]*$/.test(kit)).toBe(true);
  expect(kit.toLowerCase()).not.toContain('curado');
  expect(paletteNsh).toMatch(/^[\x09\x0a\x20-\x7e]*$/);
});

test('the template renders with exactly the variables Tauri provides', () => {
  const rendered = render(
    template,
    context(root, resolve(root, 'target/release/nsis/x64'), 'app.exe'),
  );
  expect(rendered).not.toContain('{{');
  expect(rendered).toContain('!define MANUFACTURER "nu9ve"');
  expect(rendered).toContain('!insertmacro MUI_LANGUAGE "Spanish"');
  expect(() => render('{{not_a_tauri_variable}}', {})).toThrow();
});
