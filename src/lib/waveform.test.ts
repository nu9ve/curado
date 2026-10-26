import { describe, expect, test } from 'bun:test';
import { parseWaveform } from './waveform';
import { bayer } from './dither';
import { LOGO_CELLS } from '../components/Logo';

describe('waveform wire format (mirrors analysis::encode)', () => {
  test('parses header and four bytes per bin, ignoring a trailing partial bin', () => {
    const bytes = new Uint8Array([1, 50, 0, 0, 200, 150, 40, 10, 100, 20, 60, 90, 7]);
    const wave = parseWaveform(bytes.buffer)!;
    expect(wave.rate).toBe(50);
    expect(wave.count).toBe(2);
    expect([...wave.bins.subarray(4, 8)]).toEqual([100, 20, 60, 90]);
  });
  test('rejects empty rows, unknown versions and a zero rate', () => {
    expect(parseWaveform(new ArrayBuffer(0))).toBeNull();
    expect(parseWaveform(new Uint8Array([2, 50, 0, 0, 1, 1, 1, 1]).buffer)).toBeNull();
    expect(parseWaveform(new Uint8Array([1, 0, 0, 0, 1, 1, 1, 1]).buffer)).toBeNull();
  });
});

test('Bayer thresholds cover 64 distinct levels in (0, 1)', () => {
  const values = new Set<number>();
  for (let y = 0; y < 8; y++) for (let x = 0; x < 8; x++) values.add(bayer(x, y));
  expect(values.size).toBe(64);
  expect(Math.min(...values)).toBeGreaterThan(0);
  expect(Math.max(...values)).toBeLessThan(1);
});

test('logo mark is a ring opened to the right with a centre spindle', () => {
  const has = (x: number, y: number) => LOGO_CELLS.some(([cx, cy]) => cx === x && cy === y);
  expect(has(7, 7) && has(8, 8)).toBe(true);
  expect(has(1, 7)).toBe(true);
  expect(has(14, 7)).toBe(false);
  expect(has(0, 0)).toBe(false);
});
