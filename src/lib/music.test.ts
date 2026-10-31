import { expect, test } from 'bun:test';
import { mkdtemp, mkdir, readFile, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { harmony, isDownbeat, keyLabel, transposeKey } from './music';
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
test('bars follow the stored downbeat position, whichever grid line it falls on', () => {
  const lines = [0, 1, 2, 3, 4, 5, 6, 7, 8].map((n) => 0.125 + n * 0.5);
  expect(lines.filter((time) => isDownbeat(time, 0.5, 1.125)).length).toBe(2);
  expect(isDownbeat(1.125, 0.5, 1.125)).toBe(true);
  expect(isDownbeat(3.125, 0.5, 1.125)).toBe(true);
  expect(isDownbeat(1.625, 0.5, 1.125)).toBe(false);
  // Before the stored downbeat, and with an estimate a few milliseconds off the line.
  expect(isDownbeat(0.125, 0.5, 2.125)).toBe(true);
  expect(isDownbeat(1.125, 0.5, 1.13)).toBe(true);
  // An analysis without bars reads the first grid line as the start of a bar.
  expect(isDownbeat(0.125, 0.5, 0)).toBe(true);
});

test('harmony follows the Camelot wheel, including across 12 and 1', () => {
  const key = (tonic: number, minor: boolean) => ({ tonic, minor });
  expect(harmony(key(9, true), key(9, true))).toBe('same');
  expect(harmony(key(9, true), key(0, false))).toBe('compatible'); // 8A and 8B
  expect(harmony(key(9, true), key(4, true))).toBe('compatible'); // 8A and 9A
  expect(harmony(key(9, true), key(2, true))).toBe('compatible'); // 8A and 7A
  expect(harmony(key(1, true), key(8, true))).toBe('compatible'); // 12A and 1A
  expect(harmony(key(9, true), key(10, true))).toBe('clash');
  expect(harmony(key(9, true), null)).toBeNull();
});
