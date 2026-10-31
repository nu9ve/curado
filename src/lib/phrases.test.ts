import { expect, test } from 'bun:test';
import { findPhrases } from './phrases';
import type { WaveData } from './waveform';

/** Bins of level and bass per eight bars, at 120 BPM: 16 seconds each. */
function track(levels: [number, number][]): WaveData {
  const rate = 50;
  const count = levels.length * 16 * rate;
  const bins = new Uint8Array(count * 4);
  for (let i = 0; i < count; i++) {
    const [level, low] = levels[Math.floor(i / (16 * rate))];
    bins[i * 4] = level;
    bins[i * 4 + 1] = low;
  }
  return { rate, bins, count };
}

test('sections follow energy from intro through chorus to outro', () => {
  const wave = track([
    [60, 40],
    [80, 60],
    [200, 200],
    [200, 200],
    [90, 20],
    [120, 30],
    [200, 200],
    [60, 30],
  ]);
  const phrases = findPhrases(wave, 120, 0, 8 * 16);
  expect(phrases.map((p) => p.kind)).toEqual(['intro', 'chorus', 'down', 'up', 'chorus', 'outro']);
  expect(phrases[0].start).toBe(0);
  expect(phrases[1].start).toBe(32);
  expect(phrases.at(-1)?.end).toBe(128);
});

test('no sections without a tempo or for a track too short to have them', () => {
  const wave = track([
    [200, 200],
    [200, 200],
    [200, 200],
  ]);
  expect(findPhrases(wave, null, 0, 48)).toEqual([]);
  expect(findPhrases(wave, 120, 0, 20)).toEqual([]);
  expect(findPhrases({ rate: 50, bins: new Uint8Array(0), count: 0 }, 120, 0, 48)).toEqual([]);
});

test('a downbeat partway through still lines sections up with its bars', () => {
  const wave = track(Array.from({ length: 6 }, () => [200, 200] as [number, number]));
  const phrases = findPhrases(wave, 120, 4, 96);
  expect(phrases[0].start).toBe(0);
  expect(phrases[0].kind).toBe('chorus');
});
