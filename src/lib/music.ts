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
