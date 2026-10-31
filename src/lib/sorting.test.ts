import { expect, test } from 'bun:test';
import { libraryValue, nextSort, parseSort, sortTracks } from './sorting';
import type { Track } from './types';

const track = (id: number, title: string, bpm: number | null, tonic = 0, minor = true) =>
  ({
    id,
    title,
    artist: `Artist ${id}`,
    path: `/m/${id}.flac`,
    duration: 100 + id,
    music: bpm === null ? null : { bpm, key: { tonic, minor } },
  }) as unknown as Track;
const ids = (list: Track[]) => list.map((t) => t.id);

test('a column sorts, reverses and goes back', () => {
  const asc = nextSort(null, 'bpm');
  expect(asc).toEqual({ column: 'bpm', descending: false });
  const desc = nextSort(asc, 'bpm');
  expect(desc).toEqual({ column: 'bpm', descending: true });
  expect(nextSort(desc, 'bpm')).toBeNull();
  expect(nextSort(desc, 'title')).toEqual({ column: 'title', descending: false });
});

test('numbers sort as numbers and unknown values stay last both ways', () => {
  const list = [track(1, 'a', 128), track(2, 'b', null), track(3, 'c', 90), track(4, 'd', 124)];
  expect(ids(sortTracks(list, { column: 'bpm', descending: false }, libraryValue))).toEqual([
    3, 4, 1, 2,
  ]);
  expect(ids(sortTracks(list, { column: 'bpm', descending: true }, libraryValue))).toEqual([
    1, 4, 3, 2,
  ]);
});

test('text ignores case, and key follows the wheel', () => {
  const list = [track(1, 'bravo', 120, 9), track(2, 'Alpha', 120, 2), track(3, 'charlie', 120, 4)];
  expect(ids(sortTracks(list, { column: 'title', descending: false }, libraryValue))).toEqual([
    2, 1, 3,
  ]);
  // A minor is 8A, D minor 7A, E minor 9A.
  expect(ids(sortTracks(list, { column: 'key', descending: false }, libraryValue))).toEqual([
    2, 1, 3,
  ]);
  const value = (t: { key: string }) => t.key;
  const wheel = [{ key: '10A' }, { key: '9A' }, { key: '1B' }];
  expect(sortTracks(wheel, { column: 'key', descending: false }, value).map(value)).toEqual([
    '1B',
    '9A',
    '10A',
  ]);
});

test('sorting leaves the original list alone, and no sort is the same list', () => {
  const list = [track(1, 'b', 130), track(2, 'a', 120)];
  const sorted = sortTracks(list, { column: 'title', descending: false }, libraryValue);
  expect(ids(list)).toEqual([1, 2]);
  expect(ids(sorted)).toEqual([2, 1]);
  expect(sortTracks(list, null, libraryValue)).toBe(list);
});

test('a stored sort is checked', () => {
  expect(parseSort({ column: 'bpm', descending: true })).toEqual({
    column: 'bpm',
    descending: true,
  });
  expect(parseSort({ column: 'nope' })).toBeNull();
  expect(parseSort('bpm')).toBeNull();
  expect(parseSort(null)).toBeNull();
});
