import { harmony, keyMatch } from './music';
import type { MusicalKey, Track } from './types';

/** A sample is any piece with this tag: a mark on it, not a separate library. */
export const SAMPLE_TAG = 'sample';
export type PieceView = 'track' | 'loop' | 'stem' | 'sample';
export interface PieceFilter {
  query: string;
  kind: PieceView | null;
  tags: string[];
  maxDuration: number | null;
  favorite: boolean;
}
export const NO_FILTER: PieceFilter = {
  query: '',
  kind: null,
  tags: [],
  maxDuration: null,
  favorite: false,
};
/** Steps of "short enough to be a sample" in the quick select, in seconds. */
export const SAMPLE_LENGTHS = [null, 5, 15, 30] as const;
export const isSample = (track: Track) => track.tags.includes(SAMPLE_TAG);
export function matchesPiece(track: Track, filter: PieceFilter) {
  if (filter.kind === 'sample' ? !isSample(track) : filter.kind && track.kind !== filter.kind)
    return false;
  if (filter.favorite && !track.favorite) return false;
  if (filter.maxDuration !== null && track.duration > filter.maxDuration) return false;
  if (!filter.tags.every((tag) => track.tags.includes(tag))) return false;
  const query = filter.query.trim().toLocaleLowerCase();
  if (!query) return true;
  const text = `${track.title} ${track.artist} ${track.tags.join(' ')}`.toLocaleLowerCase();
  return query.split(/\s+/).every((word) => text.includes(word));
}
/** The tags in a list, most used first, for choosing with a touch. */
export function tagCounts(tracks: Track[]): [string, number][] {
  const counts = new Map<string, number>();
  for (const track of tracks)
    for (const tag of track.tags) counts.set(tag, (counts.get(tag) ?? 0) + 1);
  return [...counts].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
}
/** What can go on the pads: marked, or short enough, and with something chosen on top. */
export function quickSamples(
  tracks: Track[],
  options: {
    marked: boolean;
    maxDuration: number | null;
    tag: string | null;
    /** Only what came from this track, as its stems and loops. */
    from: number | null;
    /** Only what mixes with this key. */
    key: MusicalKey | null;
  },
) {
  return tracks
    .filter(
      (track) =>
        !track.device &&
        (!options.marked || isSample(track)) &&
        (options.maxDuration === null || track.duration <= options.maxDuration) &&
        (!options.tag || track.tags.includes(options.tag)) &&
        (options.from === null || track.parent === options.from || track.id === options.from) &&
        (!options.key ||
          (track.music?.key
            ? keyMatch(track.music.key, options.key)?.state !== 'far' &&
              harmony(track.music.key, options.key) !== 'clash'
            : false)),
    )
    .sort(
      (a, b) =>
        Number(b.favorite) - Number(a.favorite) ||
        a.duration - b.duration ||
        a.title.localeCompare(b.title),
    );
}
/** The first empty pad, for where the next choice goes. */
export function nextEmptyPad(ids: number[]) {
  const empty = ids.findIndex((id) => !id);
  return empty < 0 ? 0 : empty;
}
