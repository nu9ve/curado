import type { MusicalKey } from './types';
export const NOTES = ['C', 'C♯', 'D', 'E♭', 'E', 'F', 'F♯', 'G', 'A♭', 'A', 'B♭', 'B'];
const MAJOR = [8, 3, 10, 5, 12, 7, 2, 9, 4, 11, 6, 1];
const MINOR = [5, 12, 7, 2, 9, 4, 11, 6, 1, 8, 3, 10];
export function transposeKey(key: MusicalKey | null, semitones: number): MusicalKey | null {
  return key ? { ...key, tonic: (((key.tonic + Math.round(semitones)) % 12) + 12) % 12 } : null;
}
export function keyLabel(key: MusicalKey | null, notation: 'american' | 'camelot' = 'american') {
  if (!key) return '—';
  return notation === 'camelot'
    ? `${(key.minor ? MINOR : MAJOR)[key.tonic]}${key.minor ? 'A' : 'B'}`
    : `${NOTES[key.tonic]}${key.minor ? 'm' : ''}`;
}

/** The key a deck sounds in now, with its transposition and, without key lock, its tempo. */
export function outputKey(slot: {
  music: { key: MusicalKey | null };
  rate: number;
  tuning: { semitones: number; keyLock: boolean };
}) {
  const shift = slot.tuning.semitones + (slot.tuning.keyLock ? 0 : 12 * Math.log2(slot.rate));
  return transposeKey(slot.music.key, shift);
}
/** A place on the Camelot wheel as a color, so neighbours look alike. Major is lighter. */
export function keyColor(key: MusicalKey | null): string | undefined {
  if (!key) return undefined;
  const number = (key.minor ? MINOR : MAJOR)[key.tonic];
  return `hsl(${(number - 1) * 30} 78% ${key.minor ? 68 : 80}%)`;
}
export interface KeyMatch {
  /** Semitones that put this key on the other's wheel number, the shorter way. */
  shift: number;
  /** The same result going the other way, an octave apart. */
  other: number;
  state: 'same' | 'near' | 'far';
}
/** What to transpose `from` by to fit `to`. Across modes, the goal is the relative key. */
export function keyMatch(from: MusicalKey | null, to: MusicalKey | null): KeyMatch | null {
  if (!from || !to) return null;
  const relative = from.minor === to.minor ? 0 : to.minor ? 3 : -3;
  const difference = (((to.tonic + relative - from.tonic) % 12) + 12) % 12;
  const shift = difference > 6 ? difference - 12 : difference;
  const fit = harmony(from, to);
  return {
    shift,
    other: shift > 0 ? shift - 12 : shift + 12,
    state: shift === 0 ? 'same' : fit === 'clash' ? 'far' : 'near',
  };
}

/** Whether beat `n` of a map starts a bar, counting from the beat nearest the downbeat. */
export function isMapDownbeat(beats: number[], n: number, downbeat: number, beatsPerBar = 4) {
  let nearest = 0;
  let distance = Infinity;
  for (let i = 0; i < Math.min(beats.length, 64); i++) {
    const gap = Math.abs(beats[i] - downbeat);
    if (gap < distance) {
      distance = gap;
      nearest = i;
    }
  }
  return (n - nearest) % beatsPerBar === 0;
}

/** Whether a grid line starts a bar. Works from positions, so editing the grid cannot break it. */
export function isDownbeat(time: number, beat: number, downbeat: number, beatsPerBar = 4) {
  const beats = Math.round((time - downbeat) / beat);
  return ((beats % beatsPerBar) + beatsPerBar) % beatsPerBar === 0;
}

export type Harmony = 'same' | 'compatible' | 'clash';
/** The Camelot rule mixes use: the same number or letter one step away on the wheel. */
export function harmony(a: MusicalKey | null, b: MusicalKey | null): Harmony | null {
  if (!a || !b) return null;
  if (a.tonic === b.tonic && a.minor === b.minor) return 'same';
  const number = (key: MusicalKey) => (key.minor ? MINOR : MAJOR)[key.tonic];
  const distance = Math.abs(number(a) - number(b));
  if (a.minor !== b.minor) return distance === 0 ? 'compatible' : 'clash';
  return distance === 1 || distance === 11 ? 'compatible' : 'clash';
}
