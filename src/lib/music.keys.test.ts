import { expect, test } from 'bun:test';
import { keyColor, keyLabel, keyMatch, outputKey, transposeKey } from './music';

const minor = (tonic: number) => ({ tonic, minor: true });
const major = (tonic: number) => ({ tonic, minor: false });

test('the difference is the shorter way, with the other one known', () => {
  // A minor (8A) to E minor (9A): up seven or down five.
  expect(keyMatch(minor(9), minor(4))).toEqual({ shift: -5, other: 7, state: 'near' });
  expect(keyMatch(minor(4), minor(9))).toEqual({ shift: 5, other: -7, state: 'near' });
  expect(keyMatch(minor(0), minor(6))?.shift).toBe(6);
  expect(keyMatch(minor(9), minor(9))?.state).toBe('same');
  expect(keyMatch(null, minor(9))).toBeNull();
});

test('applying it always lands on the same wheel number', () => {
  for (const from of [...Array(12).keys()].flatMap((t) => [minor(t), major(t)]))
    for (const to of [...Array(12).keys()].flatMap((t) => [minor(t), major(t)])) {
      const match = keyMatch(from, to)!;
      for (const shift of [match.shift, match.other]) {
        const number = (label: string) => parseInt(label, 10);
        expect(number(keyLabel(transposeKey(from, shift), 'camelot'))).toBe(
          number(keyLabel(to, 'camelot')),
        );
      }
    }
});

test('relative keys already fit, and clashing ones say so', () => {
  expect(keyMatch(minor(9), major(0))?.state).toBe('same');
  expect(keyMatch(major(0), minor(9))?.state).toBe('same');
  expect(keyMatch(minor(9), minor(10))?.state).toBe('far');
});

test('what sounds counts: transposition, and tempo without key lock', () => {
  const slot = (semitones: number, keyLock: boolean, rate = 1) => ({
    music: { key: minor(9) },
    rate,
    tuning: { semitones, keyLock },
  });
  expect(outputKey(slot(2, true))?.tonic).toBe(11);
  expect(outputKey(slot(0, true, 2))?.tonic).toBe(9);
  expect(outputKey(slot(0, false, 2 ** (1 / 12)))?.tonic).toBe(10);
});

test('a color for each wheel number, with black text readable on all of them', () => {
  const contrast = (css: string) => {
    const [h, s, l] = css.match(/[\d.]+/g)!.map(Number);
    const channel = (n: number) => {
      const k = (n + h / 30) % 12;
      const v =
        l / 100 -
        (s / 100) * Math.min(l / 100, 1 - l / 100) * Math.max(-1, Math.min(k - 3, 9 - k, 1));
      return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
    };
    return (0.2126 * channel(0) + 0.7152 * channel(8) + 0.0722 * channel(4) + 0.05) / 0.05;
  };
  const all = [...Array(12).keys()].flatMap((t) => [keyColor(minor(t))!, keyColor(major(t))!]);
  expect(new Set(all).size).toBe(24);
  for (const color of all) expect(contrast(color)).toBeGreaterThan(5);
  expect(keyColor(null)).toBeUndefined();
});
