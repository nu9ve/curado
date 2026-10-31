import { expect, test } from 'bun:test';
import {
  angleDelta,
  BEAT_DIVISIONS,
  BEAT_FX,
  beatLabel,
  COLOR_FX,
  JUMP_PADS,
  jogSeconds,
  LOOP_PADS,
  nextDivision,
  nextTempoRange,
  ROLL_PADS,
  samplePadIntent,
  tapTempo,
} from './console-pads';

test('sample pads trigger the eight shared sample slots and load only the selected track', () => {
  for (let i = 0; i < 8; i++) {
    expect(samplePadIntent(i, true, 23, false, false)).toEqual({ kind: 'trigger', slot: i + 2 });
    expect(samplePadIntent(i, true, 23, true, false)).toEqual({
      kind: 'load',
      slot: i + 2,
      id: 23,
    });
    expect(samplePadIntent(i, false, 23, false, false)).toEqual({
      kind: 'load',
      slot: i + 2,
      id: 23,
    });
  }
  expect(samplePadIntent(8, true, 23, false, false)).toBeNull();
  expect(samplePadIntent(0, false, null, false, false)).toBeNull();
  expect(samplePadIntent(0, true, null, true, false)).toBeNull();
  expect(samplePadIntent(0, true, 23, false, true)).toBeNull();
});

test('division steps stay in range and read as a musician would write them', () => {
  expect(nextDivision(1, 1)).toBe(2);
  expect(nextDivision(1, -1)).toBe(0.75);
  expect(nextDivision(32, 1)).toBe(32);
  expect(nextDivision(0.0625, -1)).toBe(0.0625);
  expect(nextDivision(3, 1)).toBe(1);
  expect(BEAT_DIVISIONS.map(beatLabel)).toEqual([
    '1/16',
    '1/8',
    '1/4',
    '1/2',
    '3/4',
    '1',
    '2',
    '4',
    '8',
    '16',
    '32',
  ]);
  expect(nextTempoRange(0.06)).toBe(0.1);
  expect(nextTempoRange(0.5)).toBe(0.06);
});

test('platter angles take the short way across the wrap', () => {
  expect(angleDelta(3, -3)).toBeCloseTo(2 * Math.PI - 6);
  expect(angleDelta(-3, 3)).toBeCloseTo(6 - 2 * Math.PI);
  expect(angleDelta(0.1, 0.3)).toBeCloseTo(0.2);
  expect(jogSeconds(2 * Math.PI)).toBeCloseTo(1.8);
  expect(jogSeconds(Math.PI, 2)).toBeCloseTo(1.8);
});

test('tap tempo needs a few taps, averages them and restarts after a pause', () => {
  expect(tapTempo([])).toBeNull();
  expect(tapTempo([0, 500])).toBeNull();
  expect(tapTempo([0, 500, 1000])).toBe(120);
  expect(tapTempo([0, 500, 1000, 5000, 5400, 5800])).toBe(150);
  expect(tapTempo([0, 10, 20])).toBeNull();
  const many = Array.from({ length: 20 }, (_, i) => i * 400);
  expect(tapTempo(many)).toBe(150);
});

test('every pad value is one the engine accepts', () => {
  const loops = [0.0625, 0.125, 0.25, 0.5, 1, 2, 4, 8, 16, 32];
  for (const value of [...LOOP_PADS, ...ROLL_PADS]) expect(loops).toContain(value);
  for (const value of JUMP_PADS) expect(Math.abs(value)).toBeLessThanOrEqual(64);
  expect(LOOP_PADS).toHaveLength(8);
  expect(ROLL_PADS).toHaveLength(8);
  expect(JUMP_PADS).toHaveLength(8);
  expect(COLOR_FX).toHaveLength(6);
  expect(BEAT_FX).toHaveLength(9);
});
