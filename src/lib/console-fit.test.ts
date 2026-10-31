import { expect, test } from 'bun:test';
import { consoleScale } from './console-fit';

test('console fits within both dimensions when available space permits', () => {
  for (const [width, height] of [
    [1140, 690],
    [1640, 830],
    [1200, 900],
  ]) {
    const scale = consoleScale(width, height, 1170, 900);
    expect(1170 * scale).toBeLessThanOrEqual(width);
    expect(900 * scale).toBeLessThanOrEqual(height);
  }
  expect(consoleScale(3000, 3000, 1170, 900)).toBe(1.2);
});

test('console keeps a usable minimum with scroll fallback and handles invalid measurements', () => {
  expect(consoleScale(320, 300, 1170, 900)).toBe(0.5);
  expect(consoleScale(0, 300, 1170, 900)).toBe(1);
  expect(consoleScale(500, Number.NaN, 1170, 900)).toBe(1);
});
