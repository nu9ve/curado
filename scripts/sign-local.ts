import { resolve } from 'node:path';
/** A local ARM64 bundle still needs a consistent resource seal. Never replace a publisher identity. */
if (process.platform === 'darwin') {
  const bundle = resolve(process.argv[2] ?? 'target/release/bundle/macos/curado.app');
  const base = await Bun.file('src-tauri/tauri.conf.json').json();
  const mac = await Bun.file('src-tauri/tauri.macos.conf.json').json();
  const identity =
    process.env.APPLE_SIGNING_IDENTITY ??
    mac.bundle?.macOS?.signingIdentity ??
    base.bundle?.macOS?.signingIdentity;
  if (!identity || identity === '-') {
    const sign = Bun.spawnSync(['/usr/bin/codesign', '--force', '--sign', '-', bundle], {
      stdout: 'inherit',
      stderr: 'inherit',
    });
    if (sign.exitCode !== 0) process.exit(sign.exitCode);
  }
  const verify = Bun.spawnSync(['/usr/bin/codesign', '--verify', '--deep', '--strict', bundle], {
    stdout: 'inherit',
    stderr: 'inherit',
  });
  if (verify.exitCode !== 0) process.exit(verify.exitCode);
  console.log('macOS bundle signature verified. Local ad hoc signing does not imply notarization.');
}
