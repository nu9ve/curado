export type SamplePadIntent =
  { kind: 'trigger'; slot: number } | { kind: 'load'; slot: number; id: number } | null;

export const SAMPLE_PADS = 8;
/** Sample pads on either deck address the same eight native one-shot slots. */
export function samplePadIntent(
  index: number,
  loaded: boolean,
  selected: number | null,
  shift: boolean,
  loading: boolean,
): SamplePadIntent {
  if (!Number.isInteger(index) || index < 0 || index >= SAMPLE_PADS || loading) return null;
  const slot = index + 2;
  if (shift || !loaded) return selected === null ? null : { kind: 'load', slot, id: selected };
  return { kind: 'trigger', slot };
}

export type PadMode = 'cue' | 'loop' | 'jump' | 'roll' | 'samples' | 'keyboard' | 'keys' | 'padfx';
export const LOOP_PADS = [0.25, 0.5, 1, 2, 4, 8, 16, 32];
export const ROLL_PADS = [0.0625, 0.125, 0.25, 0.5, 1, 2, 4, 8];
/** Back along the top row, forward along the bottom one. */
export const JUMP_PADS = [-1, -4, -8, -16, 1, 4, 8, 16];
export const BEAT_DIVISIONS = [0.0625, 0.125, 0.25, 0.5, 0.75, 1, 2, 4, 8, 16, 32];
export const COLOR_FX = ['SPACE', 'DUB ECHO', 'SWEEP', 'NOISE', 'CRUSH', 'FILTER'];
/** Semitones on the pads for KEY SHIFT and KEYBOARD. */
export const KEY_PADS = [-3, -2, -1, 0, 1, 2, 3, 4];
/** Beat FX held under the pads: the rhythmic ones work best for a moment. */
export const PAD_FX = [7, 8, 9, 1, 5, 4, 10, 13];
export const BEAT_FX = [
  'DELAY',
  'ECHO',
  'PING PONG',
  'REVERB',
  'FLANGER',
  'PHASER',
  'FILTER',
  'TRANS',
  'ROLL',
  'SLIP ROLL',
  'PITCH',
  'SPIRAL',
  'HELIX',
  'VINYL BRAKE',
];
/** The order printed around the hardware's dial. `BEAT_FX` follows the engine's numbering. */
export const BEAT_FX_ORDER = [0, 1, 2, 11, 3, 7, 6, 4, 5, 10, 9, 8, 13, 12];
export const COLOR_FX_LABELS = COLOR_FX;
export const PITCH_FX = BEAT_FX.indexOf('PITCH');
/** What each division means on PITCH: the engine holds the same list. */
export const PITCH_STEPS = [-12, -7, -5, -3, -2, -1, 1, 2, 5, 7, 12];
export function pitchLabel(beats: number): string {
  const step = PITCH_STEPS[BEAT_DIVISIONS.indexOf(beats)] ?? 1;
  return `${step > 0 ? '+' : ''}${step} st`;
}
export const TEMPO_RANGES = [0.06, 0.1, 0.16, 0.5];

export function beatLabel(beats: number): string {
  if (beats >= 1) return String(beats);
  if (beats === 0.75) return '3/4';
  return `1/${Math.round(1 / beats)}`;
}
/** The next division in a direction, staying at the ends. An unknown one starts from the middle. */
export function nextDivision(current: number, direction: 1 | -1): number {
  const index = BEAT_DIVISIONS.indexOf(current);
  if (index < 0) return 1;
  return BEAT_DIVISIONS[Math.min(BEAT_DIVISIONS.length - 1, Math.max(0, index + direction))];
}
export function nextTempoRange(current: number): number {
  return TEMPO_RANGES[(TEMPO_RANGES.indexOf(current) + 1) % TEMPO_RANGES.length];
}
/** The shortest way around between two angles, in radians, across the point where they wrap. */
export function angleDelta(from: number, to: number): number {
  const turn = 2 * Math.PI;
  return ((((to - from + Math.PI) % turn) + turn) % turn) - Math.PI;
}
/** Seconds of audio in a platter movement, with one turn as a 33⅓ record's 1.8 s. */
export function jogSeconds(radians: number, sensitivity = 1): number {
  return (radians / (2 * Math.PI)) * 1.8 * sensitivity;
}
/**
 * BPM from recent taps, or null until there are enough. A long pause starts over, so a stale
 * tap cannot pull the average. Accepts only what the engine accepts.
 */
export function tapTempo(times: number[]): number | null {
  const recent: number[] = [];
  for (let i = times.length - 1; i >= 0; i--) {
    if (recent.length && recent[0] - times[i] > 2000) break;
    recent.unshift(times[i]);
    if (recent.length === 8) break;
  }
  if (recent.length < 3) return null;
  const bpm = 60000 / ((recent[recent.length - 1] - recent[0]) / (recent.length - 1));
  return bpm >= 40 && bpm <= 300 ? Math.round(bpm * 10) / 10 : null;
}

/** The next or previous saved cue from a position, skipping one the head is sitting on. */
export function neighborCue(cues: number[], position: number, direction: 1 | -1): number | null {
  const saved = cues.filter((cue) => cue > 0).sort((a, b) => a - b);
  const found =
    direction > 0
      ? saved.find((cue) => cue > position + 0.05)
      : saved.reverse().find((cue) => cue < position - 0.05);
  return found ?? null;
}
