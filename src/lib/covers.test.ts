import { expect, test } from 'bun:test';
import { coverKey, rememberCover } from './covers';

test('a cover is its own for each track, drive and size', () => {
  const keys = [
    coverKey({ kind: 'track', id: 1 }, 96),
    coverKey({ kind: 'track', id: 1 }, 320),
    coverKey({ kind: 'track', id: 2 }, 96),
    coverKey({ kind: 'device', device: 'USB', track: 1 }, 96),
    coverKey({ kind: 'device', device: 'USB2', track: 1 }, 96),
  ];
  expect(new Set(keys).size).toBe(keys.length);
});

test('the least recently seen cover goes first, and a missing one is remembered too', () => {
  const store = new Map<string, string | null>();
  rememberCover(store, 'a', 'one', 2);
  rememberCover(store, 'b', null, 2);
  rememberCover(store, 'a', 'one again', 2);
  rememberCover(store, 'c', 'three', 2);
  expect([...store.keys()]).toEqual(['a', 'c']);
  expect(store.get('a')).toBe('one again');
  rememberCover(store, 'd', null, 2);
  expect(store.has('d')).toBe(true);
  expect(store.get('d')).toBeNull();
});
