import { BEAT_DIVISIONS, BEAT_FX } from './console-pads';
import type { BeatFxState, Slot } from './types';

export type XYMode = 'color' | 'filter' | 'beat' | 'eq' | 'stutter' | 'recycler' | 'gate' | 'pitch';
export const XY_MODES: { mode: XYMode; label: string }[] = [
  { mode: 'color', label: 'COLOR' },
  { mode: 'filter', label: 'FILTER' },
  { mode: 'beat', label: 'BEAT FX' },
  { mode: 'eq', label: 'EQ' },
  { mode: 'stutter', label: 'STUTTER' },
  { mode: 'recycler', label: 'RECYCLE' },
  { mode: 'gate', label: 'GATE' },
  { mode: 'pitch', label: 'PITCH' },
];
/** The modes that play on the channel's Beat FX, and the effect each one picks. */
const BEAT_MODE_KIND: Partial<Record<XYMode, number>> = {
  stutter: BEAT_FX.indexOf('STUTTER'),
  recycler: BEAT_FX.indexOf('RECYCLER'),
  gate: BEAT_FX.indexOf('TRANS'),
  pitch: BEAT_FX.indexOf('PITCH'),
};
export const usesBeatFx = (mode: XYMode) => mode === 'beat' || mode in BEAT_MODE_KIND;
/** What each surface is on now, for a face deciding where it can go. */
export const xyModesNow: [XYMode, XYMode] = ['color', 'color'];
export interface XYMove {
  kind: string;
  value: number;
}
const clamp = (v: number) => Math.max(0, Math.min(1, v));

/**
 * What a point on the surface sets, with x and y from 0 to 1 and y growing upward. Any source of
 * a point can drive this, so a touch, a mouse or later a tracked hand all mean the same.
 */
export function xyMoves(mode: XYMode, x: number, y: number): XYMove[] {
  const [px, py] = [clamp(x), clamp(y)];
  switch (mode) {
    case 'color':
      return [
        { kind: 'color', value: px * 2 - 1 },
        { kind: 'colorParam', value: py },
      ];
    case 'filter':
      return [
        { kind: 'filter', value: px < 0.5 ? px * 2 : 1 },
        { kind: 'highPass', value: px > 0.5 ? (px - 0.5) * 2 : 0 },
        { kind: 'echo', value: py },
      ];
    case 'stutter':
    case 'recycler':
    case 'gate':
    case 'pitch':
    case 'beat':
      return [
        ...(mode === 'beat' ? [] : [{ kind: 'beatFx', value: BEAT_MODE_KIND[mode]! }]),
        {
          kind: 'beatFxBeats',
          value:
            BEAT_DIVISIONS[
              Math.min(BEAT_DIVISIONS.length - 1, Math.floor(px * BEAT_DIVISIONS.length))
            ],
        },
        { kind: 'beatFxDepth', value: py },
        { kind: 'beatFxOn', value: 1 },
      ];
    case 'eq':
      return [
        { kind: 'low', value: Math.min(1, px * 2) },
        { kind: 'high', value: Math.min(1, (1 - px) * 2) },
        { kind: 'mid', value: py * 2 },
      ];
  }
}
/** Letting go returns to neutral, so a gesture never leaves a channel changed. */
export function xyRelease(mode: XYMode): XYMove[] {
  switch (mode) {
    case 'color':
      return [{ kind: 'color', value: 0 }];
    case 'filter':
      return [
        { kind: 'filter', value: 1 },
        { kind: 'highPass', value: 0 },
        { kind: 'echo', value: 0 },
      ];
    case 'stutter':
    case 'recycler':
    case 'gate':
    case 'pitch':
    case 'beat':
      return [{ kind: 'beatFxOn', value: 0 }];
    case 'eq':
      return [
        { kind: 'low', value: 1 },
        { kind: 'mid', value: 1 },
        { kind: 'high', value: 1 },
      ];
  }
}
/** Where the engine's state puts the point, so MIDI and the knobs show up here too. */
export function xyPoint(mode: XYMode, slot: Slot, fx: BeatFxState): [number, number] {
  switch (mode) {
    case 'color':
      return [(slot.color + 1) / 2, slot.colorParam];
    case 'filter':
      return [slot.highPass > 0 ? 0.5 + slot.highPass / 2 : slot.filter / 2, slot.echo];
    case 'stutter':
    case 'recycler':
    case 'gate':
    case 'pitch':
    case 'beat': {
      const at = BEAT_DIVISIONS.indexOf(fx.beats);
      return [(Math.max(at, 0) + 0.5) / BEAT_DIVISIONS.length, fx.depth];
    }
    case 'eq':
      return [slot.low < 1 ? slot.low / 2 : slot.high < 1 ? 1 - slot.high / 2 : 0.5, slot.mid / 2];
  }
}
/** What a tracked hand's distance also moves. Mouse and touch have no third axis. */
export type ZTarget = 'filter' | 'echo' | 'off';
export const Z_TARGETS: { target: ZTarget; label: string }[] = [
  { target: 'filter', label: 'Z FILT' },
  { target: 'echo', label: 'Z ECHO' },
  { target: 'off', label: 'Z OFF' },
];
/** Around the distance where the pinch began: nearer one way, farther the other, still between. */
const Z_DEAD = 0.08;
export function zMoves(target: ZTarget, mode: XYMode, z: number): XYMove[] {
  // A surface already on these leaves them to its own axes.
  if (target === 'off' || mode === 'filter') return [];
  const from = clamp(z) - 0.5;
  const past = Math.max(0, Math.abs(from) - Z_DEAD) / (0.5 - Z_DEAD);
  if (target === 'echo') return [{ kind: 'echo', value: from > 0 ? past : 0 }];
  return [
    { kind: 'filter', value: from < 0 ? 1 - past : 1 },
    { kind: 'highPass', value: from > 0 ? past : 0 },
  ];
}
export function zRelease(target: ZTarget, mode: XYMode): XYMove[] {
  return zMoves(target, mode, 0.5);
}
/** The Beat FX unit of the master, which no channel's surface reaches. */
export const MASTER_FX_UNIT = 2;
/**
 * Where a face goes when hands are also at work: the one channel being worked, as a second effect
 * on it, or the master. It never takes a unit a hand is already playing.
 */
export function faceUnit(engaged: [boolean, boolean], modes: [XYMode, XYMode]): number {
  if (engaged[0] === engaged[1]) return MASTER_FX_UNIT;
  const side = engaged[0] ? 0 : 1;
  return usesBeatFx(modes[side]) ? MASTER_FX_UNIT : side;
}
/** A face has distance too: nearer is deeper, within what its height asks for. */
export function faceMoves(mode: XYMode, x: number, y: number, z: number): XYMove[] {
  return xyMoves(mode, x, y).map((move) =>
    move.kind === 'beatFxDepth' ? { ...move, value: move.value * (0.5 + 0.5 * clamp(z)) } : move,
  );
}
export const FACE_MODES = XY_MODES.filter(({ mode }) => usesBeatFx(mode));
