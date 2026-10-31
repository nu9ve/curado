import { expect, test } from 'bun:test';
import { DEFAULT_KEYS } from './keyboard';
import { MIXER_TARGETS, mixerMove, mixerTarget } from './mixer-keys';
import type { Audio } from './types';

const audio = {
  master: 0.75,
  sampler: 1,
  beatFx: [
    { kind: 1, on: false, beats: 1, depth: 0.5 },
    { kind: 13, on: true, beats: 32, depth: 1 },
  ],
  slots: [
    { high: 1, mid: 1, low: 1, trim: 1, color: 0, gain: 0.8, colorFx: 0, colorParam: 0.5 },
    { high: 1, mid: 2, low: 1, trim: 1, color: 0, gain: 1.48, colorFx: 6, colorParam: 0.5 },
  ],
} as unknown as Audio;
const move = (code: string, arrow: string, shift = false) =>
  mixerMove(mixerTarget(code)!, arrow, shift, audio);

test('each target has one key, and a chord fires nothing else', () => {
  expect(new Set(MIXER_TARGETS.map((t) => t.code)).size).toBe(MIXER_TARGETS.length);
  const taken = DEFAULT_KEYS.filter((b) => b.alt).map((b) => b.code);
  for (const target of MIXER_TARGETS) expect(taken).not.toContain(target.code);
});

test('every action is one the engine accepts', async () => {
  const engine = await Bun.file('crates/audio-engine/src/lib.rs').text();
  for (const arrow of ['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'])
    for (const target of MIXER_TARGETS) {
      const action = mixerMove(target, arrow, false, audio);
      if (action) expect(engine).toContain(`"${action.kind}"`);
    }
});

test('arrows mean top, silent and center, on the hand of each deck', () => {
  expect(move('KeyZ', 'ArrowDown')).toEqual({ slot: 0, kind: 'low', value: 0 });
  expect(move('KeyZ', 'ArrowUp')).toEqual({ slot: 0, kind: 'low', value: 2 });
  expect(move('KeyZ', 'ArrowLeft')).toEqual({ slot: 0, kind: 'low', value: 1 });
  expect(move('Slash', 'ArrowRight')).toEqual({ slot: 1, kind: 'low', value: 1 });
  expect(move('KeyS', 'ArrowDown')?.value).toBe(-1);
  expect(move('KeyG', 'ArrowLeft')).toEqual({ slot: 0, kind: 'master', value: 0.75 });
});

test('the top of a fader by keyboard is unity, and Shift steps to its real end', () => {
  expect(move('KeyX', 'ArrowUp')?.value).toBe(1);
  expect(move('Period', 'ArrowUp', true)?.value).toBe(1.5);
  expect(move('KeyX', 'ArrowDown', true)?.value).toBeCloseTo(0.74);
  expect(move('Semicolon', 'ArrowUp', true)?.value).toBe(2);
});

test('effects are chosen with the sides and switched with up and down', () => {
  expect(move('KeyD', 'ArrowUp')).toEqual({ slot: 0, kind: 'beatFxOn', value: 1 });
  expect(move('KeyD', 'ArrowDown')).toEqual({ slot: 0, kind: 'beatFxOn', value: 0 });
  expect(move('KeyD', 'ArrowRight')).toEqual({ slot: 0, kind: 'beatFx', value: 2 });
  expect(move('KeyD', 'ArrowLeft')).toEqual({ slot: 0, kind: 'beatFx', value: 0 });
  expect(move('KeyE', 'ArrowUp')).toEqual({ slot: 0, kind: 'colorFx', value: 1 });
  expect(move('KeyE', 'ArrowRight')).toEqual({ slot: 0, kind: 'colorFx', value: 1 });
  expect(move('KeyI', 'ArrowRight')).toEqual({ slot: 1, kind: 'colorFx', value: 1 });
  expect(move('KeyI', 'ArrowLeft')).toEqual({ slot: 1, kind: 'colorFx', value: 5 });
  expect(move('KeyI', 'ArrowUp')).toBeNull();
  expect(move('KeyI', 'ArrowDown')).toEqual({ slot: 1, kind: 'colorFx', value: 0 });
  expect(move('KeyC', 'ArrowRight')).toEqual({ slot: 0, kind: 'beatFxBeats', value: 2 });
  expect(move('Comma', 'ArrowUp')).toEqual({ slot: 1, kind: 'beatFxBeats', value: 32 });
});
