import { expect, test } from 'bun:test';
import { isSample, matchesPiece, nextEmptyPad, NO_FILTER, quickSamples, tagCounts } from './pieces';
import type { Track } from './types';

const piece = (id: number, over: Partial<Track> = {}) =>
  ({
    id,
    title: `T${id}`,
    artist: 'A',
    path: `/${id}.wav`,
    duration: 100,
    sampleRate: 44100,
    favorite: false,
    music: null,
    device: false,
    kind: 'track',
    tags: [],
    parent: null,
    stem: null,
    ...over,
  }) as Track;

test('a filter narrows by kind, mark, tags, length and words', () => {
  const stem = piece(1, { kind: 'stem', tags: ['dub'], title: 'Night Vocals' });
  const sample = piece(2, { tags: ['sample', 'dub'], duration: 3, favorite: true });
  expect(matchesPiece(stem, NO_FILTER)).toBe(true);
  expect(matchesPiece(stem, { ...NO_FILTER, kind: 'stem' })).toBe(true);
  expect(matchesPiece(stem, { ...NO_FILTER, kind: 'loop' })).toBe(false);
  expect(matchesPiece(sample, { ...NO_FILTER, kind: 'sample' })).toBe(true);
  expect(matchesPiece(stem, { ...NO_FILTER, kind: 'sample' })).toBe(false);
  expect(matchesPiece(stem, { ...NO_FILTER, tags: ['dub'] })).toBe(true);
  expect(matchesPiece(sample, { ...NO_FILTER, tags: ['dub', 'nope'] })).toBe(false);
  expect(matchesPiece(sample, { ...NO_FILTER, maxDuration: 5 })).toBe(true);
  expect(matchesPiece(stem, { ...NO_FILTER, maxDuration: 5 })).toBe(false);
  expect(matchesPiece(stem, { ...NO_FILTER, favorite: true })).toBe(false);
  // Words in any order, across title and tags.
  expect(matchesPiece(stem, { ...NO_FILTER, query: 'dub night' })).toBe(true);
  expect(matchesPiece(stem, { ...NO_FILTER, query: 'dub day' })).toBe(false);
  expect(isSample(sample)).toBe(true);
});

test('tags are offered by how much they are used', () => {
  const list = [
    piece(1, { tags: ['b', 'a'] }),
    piece(2, { tags: ['b'] }),
    piece(3, { tags: ['c'] }),
  ];
  expect(tagCounts(list)).toEqual([
    ['b', 2],
    ['a', 1],
    ['c', 1],
  ]);
});

test('the quick select finds marked or short pieces, favorites and shortest first', () => {
  const list = [
    piece(1, { tags: ['sample'], duration: 20 }),
    piece(2, { tags: ['sample'], duration: 4 }),
    piece(3, { duration: 2 }),
    piece(4, { tags: ['sample'], duration: 60, favorite: true }),
    piece(5, { tags: ['sample'], duration: 1, device: true }),
    piece(6, { kind: 'stem', parent: 1, duration: 200 }),
  ];
  const ask = { marked: true, maxDuration: null, tag: null, from: null, key: null };
  const ids = (options: typeof ask | object) =>
    quickSamples(list, { ...ask, ...options }).map((t) => t.id);
  expect(ids({})).toEqual([4, 2, 1]);
  expect(ids({ marked: false, maxDuration: 5 })).toEqual([3, 2]);
  expect(ids({ marked: false, from: 1 })).toEqual([1, 6]);
  expect(ids({ marked: false, tag: 'sample', maxDuration: 30 })).toEqual([2, 1]);
});

test('in key leaves out what clashes and what has no key', () => {
  const music = (tonic: number, minor: boolean) =>
    ({ key: { tonic, minor } }) as unknown as Track['music'];
  const list = [
    piece(1, { music: music(9, true) }), // A minor, the same.
    piece(2, { music: music(4, true) }), // E minor, a neighbour.
    piece(3, { music: music(10, true) }), // B flat minor, far.
    piece(4),
  ];
  const found = quickSamples(list, {
    marked: false,
    maxDuration: null,
    tag: null,
    from: null,
    key: { tonic: 9, minor: true },
  }).map((t) => t.id);
  expect(found.sort()).toEqual([1, 2]);
});

test('the next pad is the first empty one', () => {
  expect(nextEmptyPad([5, 0, 0])).toBe(1);
  expect(nextEmptyPad([5, 6])).toBe(0);
});
