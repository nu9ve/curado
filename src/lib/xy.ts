import { BEAT_DIVISIONS } from './console-pads';
import type { BeatFxState, Slot } from './types';

export type XYMode = 'color' | 'filter' | 'beat' | 'eq';
export const XY_MODES: { mode: XYMode; label: string }[] = [
  { mode: 'color', label: 'COLOR' },
  { mode: 'filter', label: 'FILTER' },
  { mode: 'beat', label: 'BEAT FX' },
  { mode: 'eq', label: 'EQ' },
];
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
    case 'beat':
      return [
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
    case 'beat': {
      const at = BEAT_DIVISIONS.indexOf(fx.beats);
      return [(Math.max(at, 0) + 0.5) / BEAT_DIVISIONS.length, fx.depth];
    }
    case 'eq':
      return [slot.low < 1 ? slot.low / 2 : slot.high < 1 ? 1 - slot.high / 2 : 0.5, slot.mid / 2];
  }
}
