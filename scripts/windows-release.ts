/**
 * Builds the Windows installer and writes what a release needs beside it: its SHA-256
 * and whether it is signed. It publishes nothing and installs nothing.
 *
 *   bun run windows:release
 *
 * Run the installer yourself, from Explorer or your own terminal: an agent app packaged
 * as MSIX redirects what its child processes write under AppData (docs/WINDOWS.md).
 */
import { createHash } from 'node:crypto';
import { existsSync } from 'node:fs';
import { join, resolve } from 'node:path';
import base from '../src-tauri/tauri.conf.json';

if (process.platform !== 'win32') {
  console.error('windows:release runs on Windows. See docs/PLATFORMS.md for other systems.');
  process.exit(1);
}
const root = resolve(import.meta.dir, '..');
const run = (command: string[]) => {
  const result = Bun.spawnSync(command, { cwd: root, stdio: ['inherit', 'inherit', 'inherit'] });
  if (result.exitCode !== 0) process.exit(result.exitCode ?? 1);
};
// The art and colors must be the ones the theme generates today.
run(['bun', 'scripts/installer-art.ts', '--check']);
run(['bun', 'tauri', 'build']);

const name = `${base.productName}_${base.version}_x64-setup.exe`;
const folder = join(root, 'target', 'release', 'bundle', 'nsis');
const installer = join(folder, name);
if (!existsSync(installer)) {
  console.error(`Expected ${installer}`);
  process.exit(1);
}
const bytes = await Bun.file(installer).bytes();
const sha256 = createHash('sha256').update(bytes).digest('hex');
await Bun.write(join(folder, 'SHA256SUMS.txt'), `${sha256}  ${name}\n`);
const signature = Bun.spawnSync([
  'powershell',
  '-NoProfile',
  '-Command',
  `(Get-AuthenticodeSignature -LiteralPath '${installer}').Status`,
]);
const status = signature.stdout.toString().trim() || 'unknown';

console.log(`
${name}
  size     ${(bytes.length / 1048576).toFixed(2)} MiB
  sha256   ${sha256}
  signed   ${status === 'Valid' ? 'yes' : `no (${status}): SmartScreen will warn; signing is a release gate`}
  folder   ${folder}

Install by hand from Explorer. Other ways to run it:
  /DEMO    walk every page, write nothing
  /S       silent; exits 2 without closing curado if it is open
  /P       progress only
`);
