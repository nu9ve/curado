import { describe, expect, test } from 'bun:test';
import { DEFAULT_KEYS, matchKey } from './keyboard';
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
    expect(new Set(DEFAULT_KEYS.map((b) => `${b.code}:${b.shift}`)).size).toBe(DEFAULT_KEYS.length);
    expect(matchKey(event('Digit1', true), DEFAULT_KEYS, 'mix')?.action.kind).toBe('setCue');
  });
  test('protects text shortcuts, repeats and player mode', () => {
    expect(matchKey({ ...event('KeyQ'), metaKey: true }, DEFAULT_KEYS, 'mix')).toBeUndefined();
    expect(matchKey({ ...event('KeyQ'), repeat: true }, DEFAULT_KEYS, 'mix')).toBeUndefined();
    expect(matchKey(event('KeyP'), DEFAULT_KEYS, 'player')).toBeUndefined();
    expect(matchKey(event('Escape'), DEFAULT_KEYS, 'player')?.action.kind).toBe('panic');
  });
});
