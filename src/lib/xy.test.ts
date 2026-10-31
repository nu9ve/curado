import { expect, test } from 'bun:test';
import { EMPTY_SLOT } from './types';
import { xyMoves, xyPoint, xyRelease, XY_MODES } from './xy';

const fx = { kind: 1, on: false, beats: 1, depth: 0.5, bpm: 0 };

test('every value a point sets is one the engine accepts', () => {
  const limits: Record<string, [number, number]> = {
    color: [-1, 1],
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
