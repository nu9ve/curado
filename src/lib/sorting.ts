import { keyLabel } from './music';
import type { DeviceTrack, Track } from './types';

export const SORT_COLUMNS = [
  'title',
  'artist',
  'format',
  'genre',
  'duration',
  'bpm',
  'key',
] as const;
export type SortColumn = (typeof SORT_COLUMNS)[number];
export interface TrackSort {
  column: SortColumn;
  descending: boolean;
}
/** One click sorts, the next reverses, the next goes back to the list's own order. */
export function nextSort(current: TrackSort | null, column: SortColumn): TrackSort | null {
  if (current?.column !== column) return { column, descending: false };
  return current.descending ? null : { column, descending: true };
}
export function parseSort(value: unknown): TrackSort | null {
  const saved = value as Partial<TrackSort> | null | undefined;
  return saved && SORT_COLUMNS.includes(saved.column as SortColumn)
    ? { column: saved.column as SortColumn, descending: saved.descending === true }
    : null;
}
type SortValue = string | number | null | undefined;
export function libraryValue(track: Track, column: SortColumn): SortValue {
  if (column === 'title') return track.title;
  if (column === 'artist') return track.artist;
  if (column === 'format') return track.path.split('.').pop();
  if (column === 'duration') return track.duration;
  if (column === 'bpm') return track.music?.bpm;
  // By wheel position, the order that helps when mixing in key.
  if (column === 'key') return track.music?.key ? keyLabel(track.music.key, 'camelot') : null;
  return null;
}
export function deviceValue(track: DeviceTrack, column: SortColumn): SortValue {
  if (column === 'title') return track.title;
  if (column === 'artist') return track.artist;
  if (column === 'genre') return track.genre;
  if (column === 'duration') return track.duration;
  if (column === 'bpm') return track.bpm;
  if (column === 'key') return track.key;
  return null;
}
const missing = (value: SortValue) =>
  value === null || value === undefined || value === '' || value === 0;
/** A new list, so a playlist's saved order is never touched. Unknown values stay last either way. */
export function sortTracks<T>(
  tracks: T[],
  sort: TrackSort | null,
  value: (track: T, column: SortColumn) => SortValue,
): T[] {
  if (!sort) return tracks;
  const direction = sort.descending ? -1 : 1;
  return [...tracks].sort((a, b) => {
    const x = value(a, sort.column);
    const y = value(b, sort.column);
    if (missing(x) || missing(y)) return Number(missing(x)) - Number(missing(y));
    if (typeof x === 'number' && typeof y === 'number') return (x - y) * direction;
    // Numeric, so 10A comes after 9A.
    return (
      String(x).localeCompare(String(y), undefined, { numeric: true, sensitivity: 'base' }) *
      direction
    );
  });
}
