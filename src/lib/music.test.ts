import { expect, test } from 'bun:test';
import { mkdtemp, mkdir, readFile, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { keyLabel, transposeKey } from './music';
import { promoteBundle } from '../../scripts/install-local';
test('Camelot covers all 24 keys and transposition preserves mode', () => {
  const codes = new Set<string>();
  for (const minor of [false, true])
    for (let tonic = 0; tonic < 12; tonic++) {
      const key = { tonic, minor };
      codes.add(keyLabel(key, 'camelot'));
      expect(transposeKey(key, 12)).toEqual(key);
      expect(transposeKey(key, -12)).toEqual(key);
      expect(transposeKey(key, 7)?.minor).toBe(minor);
    }
  expect(codes.size).toBe(24);
  expect(keyLabel({ tonic: 0, minor: false }, 'camelot')).toBe('8B');
  expect(keyLabel({ tonic: 9, minor: true }, 'camelot')).toBe('8A');
});
test('installation rolls back the old app if promotion fails', async () => {
  const dir = await mkdtemp(join(tmpdir(), 'curado-install-test-'));
  try {
    const destination = join(dir, 'curado.app'),
      backup = join(dir, 'previous.app');
    await mkdir(destination);
    await writeFile(join(destination, 'identity'), 'old');
    await expect(promoteBundle(join(dir, 'missing.app'), destination, backup)).rejects.toThrow();
    expect(await readFile(join(destination, 'identity'), 'utf8')).toBe('old');
    const staged = join(dir, 'new.app');
    await mkdir(staged);
    await writeFile(join(staged, 'identity'), 'new');
    await promoteBundle(staged, destination, backup);
    expect(await readFile(join(destination, 'identity'), 'utf8')).toBe('new');
    expect(await readFile(join(backup, 'identity'), 'utf8')).toBe('old');
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
});
