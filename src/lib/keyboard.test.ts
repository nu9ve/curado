import { describe, expect, test } from 'bun:test';
import {
  DEFAULT_KEYS,
  LOCAL_KINDS,
  TOGGLE_KINDS,
  allowedInMode,
  bindingLabel,
  matchKey,
  resolveAction,
} from './keyboard';
import { EMPTY_SLOT } from './types';
import type { Audio } from './types';
const event = (code: string, shiftKey = false) => ({
  code,
  shiftKey,
  metaKey: false,
  ctrlKey: false,
  altKey: false,
  repeat: false,
});
describe('keyboard performance contract', () => {
  test('unique physical keys and shifted cues', () => {
    expect(
      new Set(DEFAULT_KEYS.map((b) => `${b.code}:${b.shift}:${!!b.ctrl}:${!!b.alt}`)).size,
    ).toBe(DEFAULT_KEYS.length);
    expect(DEFAULT_KEYS.length).toBeLessThanOrEqual(120);
    expect(matchKey(event('Digit1', true), DEFAULT_KEYS, 'mix')?.action.kind).toBe('setCue');
  });
  test('protects text shortcuts, repeats and player mode', () => {
    expect(matchKey({ ...event('KeyQ'), metaKey: true }, DEFAULT_KEYS, 'mix')).toBeUndefined();
    expect(matchKey({ ...event('KeyQ'), repeat: true }, DEFAULT_KEYS, 'mix')).toBeUndefined();
    expect(matchKey(event('KeyP'), DEFAULT_KEYS, 'player')).toBeUndefined();
    expect(matchKey(event('Escape'), DEFAULT_KEYS, 'player')?.action.kind).toBe('panic');
  });
  test('player mode keeps deck A transport and panic only', () => {
    expect(allowedInMode({ slot: 0, kind: 'toggle', value: 0 }, 'player')).toBe(true);
    expect(allowedInMode({ slot: 0, kind: 'panic', value: 0 }, 'player')).toBe(true);
    expect(allowedInMode({ slot: 0, kind: 'crossfader', value: 1 }, 'player')).toBe(false);
    expect(allowedInMode({ slot: 3, kind: 'trigger', value: 0 }, 'mix')).toBe(true);
    for (const kind of ['playBoth', 'pauseBoth', 'cueBoth']) {
      expect(allowedInMode({ slot: 0, kind, value: 0 }, 'player')).toBe(false);
      expect(allowedInMode({ slot: 0, kind, value: 0 }, 'mix')).toBe(true);
    }
  });
  const pressed = (code: string, mods: Partial<ReturnType<typeof event>> = {}) =>
    matchKey({ ...event(code), ...mods }, DEFAULT_KEYS, 'mix')?.action;
  test('each hand does the same job on its own deck', () => {
    expect(pressed('KeyF')).toMatchObject({ slot: 0, kind: 'toggle' });
    expect(pressed('KeyJ')).toMatchObject({ slot: 1, kind: 'toggle' });
    expect(pressed('KeyD')).toMatchObject({ slot: 0, kind: 'cue', value: 0 });
    expect(pressed('KeyK')).toMatchObject({ slot: 1, kind: 'cue', value: 0 });
    expect(pressed('KeyQ')).toMatchObject({ slot: 0, kind: 'cue', value: 4 });
    expect(pressed('KeyP')).toMatchObject({ slot: 1, kind: 'cue', value: 7 });
    expect(pressed('Slash')).toMatchObject({ slot: 1, kind: 'loopIn' });
    expect(pressed('KeyZ')).toMatchObject({ slot: 0, kind: 'loopIn' });
  });
  test('modifiers keep to one meaning', () => {
    expect(pressed('Digit1', { shiftKey: true })?.kind).toBe('setCue');
    expect(pressed('Digit1', { ctrlKey: true })?.kind).toBe('clearCue');
    expect(pressed('Digit1', { altKey: true })).toMatchObject({ slot: 2, kind: 'trigger' });
    expect(pressed('Digit0', { altKey: true })).toMatchObject({ slot: 9, kind: 'trigger' });
    expect(pressed('Digit1', { metaKey: true })).toBeUndefined();
  });
  test('the bar pauses everything and arrows work the list', () => {
    expect(pressed('Space')?.kind).toBe('toggleBoth');
    expect(pressed('ArrowDown')).toMatchObject({ kind: 'listMove', value: 1 });
    expect(pressed('ArrowUp')).toMatchObject({ kind: 'listMove', value: -1 });
    expect(pressed('ArrowLeft', { shiftKey: true })).toMatchObject({ slot: 0, kind: 'listLoad' });
    expect(pressed('ArrowRight', { shiftKey: true })).toMatchObject({ slot: 1, kind: 'listLoad' });
    expect(pressed('ArrowLeft', { ctrlKey: true })?.kind).toBe('crossfader');
    expect(pressed('ArrowDown', { repeat: true })?.kind).toBe('listMove');
    expect(pressed('KeyF', { repeat: true })).toBeUndefined();
  });
  test('every engine action in the scheme is one the engine accepts', async () => {
    const engine = await Bun.file('crates/audio-engine/src/lib.rs').text();
    for (const { action } of DEFAULT_KEYS) {
      if (LOCAL_KINDS.has(action.kind)) continue;
      expect(engine).toContain(`"${action.kind}"`);
    }
  });
  test('a toggle sends the opposite of what the engine reports', () => {
    const audio = {
      slots: [{ ...EMPTY_SLOT, slip: true }, EMPTY_SLOT],
      beatFx: [{ kind: 1, on: false, beats: 1, depth: 1, bpm: 0 }],
    } as unknown as Audio;
    expect(resolveAction({ slot: 0, kind: 'slip', value: 1 }, audio, 'mix').value).toBe(0);
    expect(resolveAction({ slot: 1, kind: 'slip', value: 1 }, audio, 'mix').value).toBe(1);
    expect(resolveAction({ slot: 0, kind: 'beatFxOn', value: 1 }, audio, 'mix').value).toBe(1);
    expect(resolveAction({ slot: 0, kind: 'cue', value: 3 }, audio, 'mix').value).toBe(3);
    expect(resolveAction({ slot: 0, kind: 'toggleBoth', value: 0 }, audio, 'player').kind).toBe(
      'toggle',
    );
    for (const kind of TOGGLE_KINDS) expect(kind.length).toBeGreaterThan(0);
  });
  test('labels name every modifier', () => {
    expect(
      bindingLabel({ code: 'Digit1', shift: true, action: { slot: 0, kind: 'cue', value: 0 } }),
    ).toBe('⇧ 1');
    expect(
      bindingLabel({
        code: 'KeyA',
        shift: false,
        ctrl: true,
        action: { slot: 0, kind: 'cue', value: 0 },
      }),
    ).toBe('⌃ A');
    expect(
      bindingLabel({
        code: 'Digit1',
        shift: false,
        alt: true,
        action: { slot: 2, kind: 'trigger', value: 0 },
      }),
    ).toBe('⌥ 1');
  });
});
