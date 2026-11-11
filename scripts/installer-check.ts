/**
 * Renders the Windows installer template the way Tauri does and compiles it with makensis,
 * without building the app: seconds instead of minutes. It needs one earlier
 * `bun tauri build` on this machine, which leaves makensis in Tauri's cache and its helper
 * .nsh files in target/release/nsis.
 *
 *   bun scripts/installer-check.ts          compile; fail on an error or a new warning
 *   bun scripts/installer-check.ts --keep   keep the result in target/installer-check
 *
 * The result is a real installer around target/release/curado.exe when that exists, but
 * it is a check, not a release: it carries no resources. `bun tauri build` makes releases.
 */
import { existsSync, mkdirSync, rmSync } from 'node:fs';
import { join, resolve } from 'node:path';
import base from '../src-tauri/tauri.conf.json';
import windows from '../src-tauri/tauri.windows.conf.json';

type Value = string | string[];

/**
 * The subset of Handlebars the template uses. Strict on purpose: a variable Tauri does
 * not provide is an error here instead of an empty string in a release.
 */
export function render(template: string, context: Record<string, Value>): string {
  const scalar = (name: string) => {
    const value = context[name];
    if (typeof value !== 'string') throw new Error(`Unknown template variable: ${name}`);
    return value;
  };
  const list = (name: string) => {
    const value = context[name];
    if (!Array.isArray(value)) throw new Error(`Unknown template list: ${name}`);
    return value;
  };
  // Nested blocks belong to lists that are maps upstream. Only their empty form is rendered.
  let out = template.replace(/\{\{#each association\.ext[^}]*\}\}[\s\S]*?\{\{\/each\}\}/g, '');
  out = out.replace(
    /\{\{#each ([\w.]+)[^}]*\}\}\n?([\s\S]*?)\{\{\/each\}\}\n?/g,
    (_, name: string, body: string) => {
      const items = list(name);
      if (items.length && /this\.\[|@key|\|/.test(body))
        throw new Error(`The check renders ${name} only when it is empty`);
      return items
        .map((item) => body.replace(/(\\\\)?\{\{this\}\}/g, (_m, esc) => (esc ? '\\' : '') + item))
        .join('');
    },
  );
  out = out.replace(
    /\{\{#if (\w+)\}\}\n?([\s\S]*?)\{\{\/if\}\}\n?/g,
    (_, name: string, body: string) => (scalar(name) ? body : ''),
  );
  out = out.replace(
    /(\\\\)?\{\{(\w+)\}\}/g,
    (_m, esc, name: string) => (esc ? '\\' : '') + scalar(name),
  );
  if (out.includes('{{'))
    throw new Error(
      `Unrendered template syntax: ${out.slice(out.indexOf('{{'), out.indexOf('{{') + 60)}`,
    );
  return out;
}

/** What Tauri passes for this project, read from its configuration files. */
export function context(root: string, helpers: string, binary: string): Record<string, Value> {
  const nsis = windows.bundle.windows.nsis;
  const tauri = (path: string) => resolve(root, 'src-tauri', path);
  return {
    compression: 'lzma',
    signed_plugins_path: '',
    installer_hooks: tauri(nsis.installerHooks),
    manufacturer: base.bundle.publisher,
    product_name: base.productName,
    version: base.version,
    version_with_build: `${base.version}.0`,
    homepage: base.bundle.homepage,
    install_mode: nsis.installMode,
    license: '',
    installer_icon: tauri(nsis.installerIcon),
    sidebar_image: tauri(nsis.sidebarImage),
    header_image: tauri(nsis.headerImage),
    uninstaller_icon: '',
    uninstaller_header_image: '',
    main_binary_name: base.productName,
    main_binary_path: binary,
    bundle_id: base.identifier,
    copyright: '',
    out_file: 'nsis-output.exe',
    arch: 'x64',
    additional_plugins_path: join(nsisHome(), 'Plugins', 'x86-unicode', 'additional'),
    allow_downgrades: 'true',
    display_language_selector: String(nsis.displayLanguageSelector),
    install_webview2_mode: 'downloadBootstrapper',
    webview2_installer_args: '/silent',
    webview2_bootstrapper_path: '',
    webview2_installer_path: '',
    minimum_webview2_version: '',
    uninstaller_sign_cmd: '',
    estimated_size: '16000',
    start_menu_folder: '',
    languages: nsis.languages,
    language_files: nsis.languages.map((language) => join(helpers, `${language}.nsh`)),
    resources_dirs: [],
    resources: [],
    binaries: [],
    file_associations: [],
    deep_link_protocols: [],
    resources_ancestors: [],
  };
}

const nsisHome = () => join(process.env.LOCALAPPDATA ?? '', 'tauri', 'NSIS');

/** Warnings the upstream template already has; anything else is ours and fails the check. */
const UPSTREAM_WARNINGS = [/LangString "choowHowToInstall" is not set/];

if (import.meta.main) {
  const root = resolve(import.meta.dir, '..');
  const makensis = join(nsisHome(), 'makensis.exe');
  const helpers = join(root, 'target', 'release', 'nsis', 'x64');
  if (
    process.platform !== 'win32' ||
    !existsSync(makensis) ||
    !existsSync(join(helpers, 'utils.nsh'))
  ) {
    console.log(
      'Skipped: needs Windows and one earlier `bun tauri build` (makensis and helper files).',
    );
    process.exit(0);
  }
  const out = join(root, 'target', 'installer-check');
  rmSync(out, { recursive: true, force: true });
  mkdirSync(out, { recursive: true });
  for (const name of ['utils.nsh', 'FileAssociation.nsh'])
    await Bun.write(join(out, name), Bun.file(join(helpers, name)));
  const real = join(root, 'target', 'release', `${base.productName}.exe`);
  const binary = existsSync(real) ? real : join(helpers, 'utils.nsh');
  // --template FILE compiles another template with the same values, such as upstream's
  // after a Tauri upgrade, to tell its warnings from ours.
  const other = process.argv.indexOf('--template');
  const template = await Bun.file(
    other > 0
      ? resolve(process.argv[other + 1])
      : join(root, 'src-tauri', windows.bundle.windows.nsis.template),
  ).text();
  // Tauri writes the rendered script with a byte-order mark; makensis reads it as UTF-8.
  await Bun.write(
    join(out, 'installer.nsi'),
    '﻿' + render(template, context(root, helpers, binary)),
  );
  const run = Bun.spawnSync([makensis, '-V2', join(out, 'installer.nsi')], { cwd: out });
  const log = run.stdout.toString() + run.stderr.toString();
  const warnings = log
    .split(/\r?\n/)
    .filter(
      (line) => /^warning \d+:/.test(line) && !UPSTREAM_WARNINGS.some((known) => known.test(line)),
    );
  if (run.exitCode !== 0 || warnings.length) {
    console.error(log.trim());
    console.error(run.exitCode !== 0 ? 'makensis failed.' : `${warnings.length} warning line(s).`);
    process.exit(1);
  }
  const size = Bun.file(join(out, 'nsis-output.exe')).size;
  console.log(`Installer template compiles: ${(size / 1048576).toFixed(2)} MiB, no warnings.`);
  if (process.argv.includes('--keep')) console.log(join(out, 'nsis-output.exe'));
  else rmSync(out, { recursive: true, force: true });
}
