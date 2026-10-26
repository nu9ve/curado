import { mkdir, mkdtemp, rename, rm, lstat } from 'node:fs/promises';
import { homedir } from 'node:os';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(fileURLToPath(new URL('..', import.meta.url)));
function run(args: string[]) {
  const result = Bun.spawnSync(args, { cwd: root, stdout: 'inherit', stderr: 'inherit' });
  if (result.exitCode !== 0) throw new Error(`Falló ${args[0]} (código ${result.exitCode})`);
}
function identifier(bundle: string) {
  const result = Bun.spawnSync(
    [
      '/usr/libexec/PlistBuddy',
      '-c',
      'Print :CFBundleIdentifier',
      join(bundle, 'Contents/Info.plist'),
    ],
    { stdout: 'pipe', stderr: 'pipe' },
  );
  if (result.exitCode !== 0) throw new Error(`Bundle no válido: ${bundle}`);
  return result.stdout.toString().trim();
}
async function exists(path: string) {
  try {
    await lstat(path);
    return true;
  } catch (e) {
    if ((e as NodeJS.ErrnoException).code === 'ENOENT') return false;
    throw e;
  }
}

/** Copy/verify first, then rename with rollback. Never touch app data or music. */
export async function promoteBundle(staged: string, destination: string, backup: string) {
  const previous = await exists(destination);
  if (previous) await rename(destination, backup);
  try {
    await rename(staged, destination);
  } catch (error) {
    if (previous) await rename(backup, destination);
    throw error;
  }
}

async function main() {
  if (process.platform !== 'darwin') throw new Error('El instalador local actual es para macOS.');
  const flags = new Set(process.argv.slice(2));
  if ([...flags].some((f) => !['--no-build', '--check'].includes(f)))
    throw new Error('Uso: bun run desktop:install [--no-build] [--check]');
  const applications = join(homedir(), 'Applications');
  const destination = join(applications, 'curado.app');
  const source = join(root, 'target/release/bundle/macos/curado.app');
  const config = await Bun.file(join(root, 'src-tauri/tauri.conf.json')).json();
  if (!flags.has('--no-build') && !flags.has('--check')) run(['bun', 'run', 'desktop:build']);
  if (identifier(source) !== config.identifier)
    throw new Error('El bundle compilado tiene otro identificador.');
  run(['/usr/bin/codesign', '--verify', '--deep', '--strict', source]);
  if (await exists(destination)) {
    if ((await lstat(destination)).isSymbolicLink())
      throw new Error('El destino es un enlace simbólico; no se reemplazó.');
    if (identifier(destination) !== config.identifier)
      throw new Error('Hay otra app en el destino; no se reemplazó.');
  }
  const processes = Bun.spawnSync(['/bin/ps', '-ax', '-o', 'command='], {
    stdout: 'pipe',
    stderr: 'pipe',
  });
  if (processes.exitCode !== 0)
    throw new Error('No se pudo comprobar si la app instalada sigue abierta.');
  if (
    processes.stdout
      .toString()
      .split('\n')
      .some((p) => p.trim().startsWith(join(destination, 'Contents/MacOS/curado')))
  )
    throw new Error('Cierra curado instalada antes de actualizarla; no se detuvo su audio.');
  if (flags.has('--check')) {
    console.log(`Bundle y destino comprobados. Instalación prevista: ${destination}`);
    return;
  }
  await mkdir(applications, { recursive: true });
  const stage = await mkdtemp(join(applications, '.curado-install-'));
  const staged = join(stage, 'curado.app');
  const backup = join(stage, 'previous.app');
  let promoted = false;
  try {
    run(['/usr/bin/ditto', '--noqtn', source, staged]);
    run(['/usr/bin/codesign', '--verify', '--deep', '--strict', staged]);
    const executable = 'Contents/MacOS/curado';
    const digest = async (path: string) =>
      new Bun.CryptoHasher('sha256')
        .update(await Bun.file(join(path, executable)).arrayBuffer())
        .digest('hex');
    if ((await digest(source)) !== (await digest(staged)))
      throw new Error('La copia no coincide con el binario compilado.');
    await promoteBundle(staged, destination, backup);
    promoted = true;
    run([
      '/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister',
      '-f',
      destination,
    ]);
    console.log(
      `curado instalada y registrada: ${destination}\nBinario SHA-256: ${await digest(destination)}\nSpotlight/Raycast pueden tardar en actualizar su índice. No se abrió la app.`,
    );
  } finally {
    // A failed rollback keeps the previous app recoverable instead of deleting it.
    if (promoted || !(await exists(backup))) await rm(stage, { recursive: true, force: true });
    else console.error(`Copia anterior conservada en ${backup}`);
  }
}
if (import.meta.main)
  await main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  });
