import { expect, test } from 'bun:test';
import { EMPTY_SLOT } from './types';
import {
  faceMoves,
  faceUnit,
  MASTER_FX_UNIT,
  usesBeatFx,
  xyMoves,
  xyPoint,
  xyRelease,
  XY_MODES,
  zMoves,
  zRelease,
} from './xy';

const fx = { kind: 1, on: false, beats: 1, depth: 0.5, bpm: 0 };

test('every value a point sets is one the engine accepts', () => {
  const limits: Record<string, [number, number]> = {
    color: [-1, 1],
    beatFx: [0, 15],
    colorParam: [0, 1],
    filter: [0, 1],
    highPass: [0, 1],
    echo: [0, 1],
    beatFxDepth: [0, 1],
    beatFxBeats: [0.0625, 32],
    beatFxOn: [0, 1],
    low: [0, 2],
    mid: [0, 2],
    high: [0, 2],
  };
  for (const { mode } of XY_MODES)
    for (const x of [-1, 0, 0.3, 0.5, 0.99, 1, 2])
      for (const y of [-1, 0, 0.5, 1, 2])
        for (const { kind, value } of [...xyMoves(mode, x, y), ...xyRelease(mode)]) {
          const [low, high] = limits[kind];
          expect(value).toBeGreaterThanOrEqual(low);
          expect(value).toBeLessThanOrEqual(high);
        }
});

test('the filter axis is one sweep: low-pass on the left, high-pass on the right', () => {
  const get = (x: number, kind: string) =>
    xyMoves('filter', x, 0).find((m) => m.kind === kind)?.value;
  expect(get(0.25, 'filter')).toBe(0.5);
  expect(get(0.25, 'highPass')).toBe(0);
  expect(get(0.75, 'filter')).toBe(1);
  expect(get(0.75, 'highPass')).toBe(0.5);
});

test('letting go leaves the channel neutral', () => {
  expect(xyRelease('color')).toEqual([{ kind: 'color', value: 0 }]);
  expect(xyRelease('beat')).toEqual([{ kind: 'beatFxOn', value: 0 }]);
});

test('the point comes back from what the engine reports', () => {
  const moved = { ...EMPTY_SLOT, color: 0.5, colorParam: 0.25 };
  expect(xyPoint('color', moved, fx)).toEqual([0.75, 0.25]);
  expect(xyPoint('eq', EMPTY_SLOT, fx)).toEqual([0.5, 0.5]);
  expect(xyPoint('filter', { ...EMPTY_SLOT, highPass: 0.5 }, fx)[0]).toBe(0.75);
});

test('the new surfaces pick their effect and let go of it', () => {
  const kinds = { stutter: 14, recycler: 15, gate: 7, pitch: 10 } as const;
  for (const [mode, kind] of Object.entries(kinds) as [keyof typeof kinds, number][]) {
    const moves = xyMoves(mode, 0.5, 0.5);
    expect(moves[0]).toEqual({ kind: 'beatFx', value: kind });
    expect(moves.at(-1)).toEqual({ kind: 'beatFxOn', value: 1 });
    expect(xyRelease(mode)).toEqual([{ kind: 'beatFxOn', value: 0 }]);
    expect(usesBeatFx(mode)).toBe(true);
  }
  expect(xyMoves('beat', 0.5, 0.5).some((m) => m.kind === 'beatFx')).toBe(false);
  expect(usesBeatFx('color')).toBe(false);
});

test('distance is neutral where the pinch began, and moves both ways from there', () => {
  const value = (z: number, kind: string) =>
    zMoves('filter', 'color', z).find((m) => m.kind === kind)?.value;
  expect(value(0.5, 'filter')).toBe(1);
  expect(value(0.5, 'highPass')).toBe(0);
  expect(value(0.55, 'highPass')).toBe(0);
  expect(value(1, 'highPass')).toBe(1);
  expect(value(0, 'filter')).toBe(0);
  expect(value(0, 'highPass')).toBe(0);
  expect(zMoves('echo', 'color', 1)).toEqual([{ kind: 'echo', value: 1 }]);
  expect(zMoves('echo', 'color', 0)).toEqual([{ kind: 'echo', value: 0 }]);
  expect(zMoves('off', 'color', 1)).toEqual([]);
  expect(zMoves('filter', 'filter', 1)).toEqual([]);
  expect(zRelease('filter', 'color')).toEqual([
    { kind: 'filter', value: 1 },
    { kind: 'highPass', value: 0 },
  ]);
});

test('a face takes a second effect on the channel at work, or the master', () => {
  expect(faceUnit([false, false], ['color', 'color'])).toBe(MASTER_FX_UNIT);
  expect(faceUnit([true, true], ['color', 'color'])).toBe(MASTER_FX_UNIT);
  expect(faceUnit([true, false], ['color', 'color'])).toBe(0);
  expect(faceUnit([false, true], ['eq', 'filter'])).toBe(1);
  expect(faceUnit([true, false], ['stutter', 'color'])).toBe(MASTER_FX_UNIT);
  const depth = (z: number) =>
    faceMoves('stutter', 0.5, 1, z).find((m) => m.kind === 'beatFxDepth')?.value;
  expect(depth(1)).toBe(1);
  expect(depth(0)).toBe(0.5);
});
