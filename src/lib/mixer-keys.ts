import { BEAT_DIVISIONS, BEAT_FX_ORDER, COLOR_FX, nextDivision } from './console-pads';
import { tr } from './language';
import type { Action, Audio } from './types';

/**
 * Hold Option, tap what to change, then arrows: ↑ top, ↓ silent, ← → center. Each hand reaches
 * its own deck, and the outer column runs down the EQ as on the channel. Option because Control
 * with these letters already deletes cues and jumps, which a chord must never do by accident.
 */
interface ValueTarget {
  type: 'value';
  code: string;
  slot: number;
  kind: string;
  min: number;
  center: number;
  /** Where ↑ goes. The channel fader stops at unity, since its top is a jump of over 5 dB. */
  top: number;
  /** The control's real end, which Shift with arrows can reach. */
  max: number;
  label: () => string;
}
interface ChoiceTarget {
  type: 'colorFx' | 'beatFx' | 'beatDivision';
  code: string;
  slot: number;
  label: () => string;
}
export type MixerTarget = ValueTarget | ChoiceTarget;
const eq = (code: string, slot: number, kind: string, label: () => string): ValueTarget => ({
  type: 'value',
  code,
  slot,
  kind,
  min: 0,
  center: 1,
  top: 2,
  max: 2,
  label,
});
const HANDS = [
  {
    slot: 0,
    keys: ['KeyQ', 'KeyA', 'KeyZ', 'KeyW', 'KeyS', 'KeyX', 'KeyE', 'KeyD', 'KeyC', 'KeyR', 'KeyF'],
  },
  {
    slot: 1,
    keys: [
      'KeyP',
      'Semicolon',
      'Slash',
      'KeyO',
      'KeyL',
      'Period',
      'KeyI',
      'KeyK',
      'Comma',
      'KeyU',
      'KeyJ',
    ],
  },
];
export const MIXER_TARGETS: MixerTarget[] = [
  ...HANDS.flatMap(({ slot, keys }): MixerTarget[] => [
    eq(keys[0], slot, 'high', () => 'HI'),
    eq(keys[1], slot, 'mid', () => 'MID'),
    eq(keys[2], slot, 'low', () => 'LOW'),
    eq(keys[3], slot, 'trim', () => 'TRIM'),
    {
      type: 'value',
      code: keys[4],
      slot,
      kind: 'color',
      min: -1,
      center: 0,
      top: 1,
      max: 1,
      label: () => 'COLOR',
    },
    {
      type: 'value',
      code: keys[5],
      slot,
      kind: 'gain',
      min: 0,
      center: 0.8,
      top: 1,
      max: 1.5,
      label: () => 'FADER',
    },
    { type: 'colorFx', code: keys[6], slot, label: () => tr('Efecto de color', 'Color effect') },
    { type: 'beatFx', code: keys[7], slot, label: () => 'Beat FX' },
    {
      type: 'beatDivision',
      code: keys[8],
      slot,
      label: () => tr('División de Beat FX', 'Beat FX division'),
    },
    {
      type: 'value',
      code: keys[9],
      slot,
      kind: 'colorParam',
      min: 0,
      center: 0.5,
      top: 1,
      max: 1,
      label: () => 'PARAMETER',
    },
    {
      type: 'value',
      code: keys[10],
      slot,
      kind: 'beatFxDepth',
      min: 0,
      center: 0.5,
      top: 1,
      max: 1,
      label: () => 'LEVEL / DEPTH',
    },
  ]),
  {
    type: 'value',
    code: 'KeyG',
    slot: 0,
    kind: 'master',
    min: 0,
    center: 0.75,
    top: 1,
    max: 1,
    label: () => 'MASTER',
  },
  {
    type: 'value',
    code: 'KeyH',
    slot: 0,
    kind: 'sampler',
    min: 0,
    center: 1,
    top: 1.5,
    max: 1.5,
    label: () => 'SAMPLER',
  },
];
const ARROWS = new Set(['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight']);
export const isArrow = (code: string) => ARROWS.has(code);
export const mixerTarget = (code: string) => MIXER_TARGETS.find((target) => target.code === code);
export const mixerTargetFor = (slot: number, kind: string) =>
  MIXER_TARGETS.find((t) => t.type === 'value' && t.slot === slot && t.kind === kind);

function current(target: ValueTarget, audio: Audio): number {
  if (target.kind === 'master') return audio.master;
  if (target.kind === 'sampler') return audio.sampler;
  if (target.kind === 'beatFxDepth') return audio.beatFx[target.slot]?.depth ?? target.center;
  const value = audio.slots[target.slot]?.[target.kind as 'high'];
  return typeof value === 'number' ? value : target.center;
}
const around = (at: number, count: number, direction: number) =>
  ((at < 0 ? 0 : at + direction) + count) % count;
/** What an arrow means for a target, from what the engine reports now. Null when nothing changes. */
export function mixerMove(
  target: MixerTarget,
  code: string,
  shift: boolean,
  audio: Audio,
): Pick<Action, 'slot' | 'kind' | 'value'> | null {
  const { slot } = target;
  if (target.type === 'value') {
    let value = target.center;
    if (code === 'ArrowUp') value = target.top;
    if (code === 'ArrowDown') value = target.min;
    if (shift && (code === 'ArrowUp' || code === 'ArrowDown')) {
      const step = ((target.max - target.min) / 25) * (code === 'ArrowUp' ? 1 : -1);
      value = Math.max(target.min, Math.min(target.max, current(target, audio) + step));
    }
    return { slot, kind: target.kind, value: Math.round(value * 1000) / 1000 };
  }
  if (target.type === 'beatDivision') {
    const up = code === 'ArrowUp' || code === 'ArrowRight';
    const value = nextDivision(audio.beatFx[slot]?.beats ?? 1, up ? 1 : -1);
    return BEAT_DIVISIONS.includes(value) ? { slot, kind: 'beatFxBeats', value } : null;
  }
  if (target.type === 'beatFx') {
    if (code === 'ArrowUp') return { slot, kind: 'beatFxOn', value: 1 };
    if (code === 'ArrowDown') return { slot, kind: 'beatFxOn', value: 0 };
    const at = BEAT_FX_ORDER.indexOf(audio.beatFx[slot]?.kind ?? -1);
    const next = around(at, BEAT_FX_ORDER.length, code === 'ArrowRight' ? 1 : -1);
    return { slot, kind: 'beatFx', value: BEAT_FX_ORDER[next] };
  }
  // 0 is no effect, so a choice walks the six and ↓ takes it away.
  const active = audio.slots[slot]?.colorFx ?? 0;
  if (code === 'ArrowDown') return { slot, kind: 'colorFx', value: 0 };
  if (code === 'ArrowUp' && active) return null;
  const next = around(active - 1, COLOR_FX.length, code === 'ArrowLeft' ? -1 : active ? 1 : 0);
  return { slot, kind: 'colorFx', value: next + 1 };
}
/** Shows the badges while Option is down and marks which control the arrows would change. */
export function showMixerKeys(on: boolean, target: MixerTarget | null = null) {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  if (on) root.dataset.mixKeys = 'on';
  else delete root.dataset.mixKeys;
  document.querySelectorAll<HTMLElement>('.rx-mix-key').forEach((badge) => {
    badge.classList.toggle('is-target', !!target && badge.dataset.code === target.code);
  });
}
