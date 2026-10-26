import { useSyncExternalStore } from 'react';
import type { Action, KeyBinding } from './types';
import { actionLabel, bindingLabel } from './keyboard';

/**
 * Visual confirmation for keyboard actions. It animates the DOM directly through the
 * Web Animations API: no React render, no layout read, nothing on the audio path.
 * Controls opt in with `data-action="slot:kind:value"` (space separated) and keycaps
 * with `data-key="KeyQ"`.
 */
export const actionKey = (a: Pick<Action, 'slot' | 'kind' | 'value'>) =>
  `${a.slot}:${a.kind}:${a.value}`;

const reduced = () =>
  typeof matchMedia !== 'undefined' && matchMedia('(prefers-reduced-motion: reduce)').matches;

function flash(selector: string) {
  if (typeof document === 'undefined') return;
  const motion = !reduced();
  document.querySelectorAll<HTMLElement>(selector).forEach((el) => {
    el.animate(
      motion
        ? [
            { transform: 'scale(0.9)', filter: 'brightness(1.7) saturate(1.3)' },
            { transform: 'scale(1)', filter: 'none' },
          ]
        : [{ filter: 'brightness(1.7)' }, { filter: 'none' }],
      { duration: 280, easing: 'cubic-bezier(.2,.8,.2,1)' },
    );
  });
}

export interface HudEntry {
  id: number;
  key: string;
  label: string;
  slot: number;
}
let entries: HudEntry[] = [];
let counter = 0;
const listeners = new Set<() => void>();
const subscribe = (fn: () => void) => {
  listeners.add(fn);
  return () => {
    listeners.delete(fn);
  };
};
const emit = () => listeners.forEach((fn) => fn());
export const useHud = () => useSyncExternalStore(subscribe, () => entries);

function describe(a: Action) {
  const target = a.slot < 2 ? (a.slot ? 'B' : 'A') : `S${a.slot - 1}`;
  if (a.kind === 'crossfader') return `Crossfader ${['A', 'centro', 'B'][Math.round(a.value * 2)]}`;
  if (a.kind === 'panic' || a.kind === 'master') return actionLabel(a.kind);
  const cue = ['cue', 'setCue'].includes(a.kind) ? ` ${a.value + 1}` : '';
  const nudge = a.kind === 'nudge' ? (a.value < 0 ? ' −' : ' +') : '';
  return `${target} · ${actionLabel(a.kind)}${cue}${nudge}`;
}

/** Called by the keyboard handler after the action has been sent to Rust. */
export function feedback(binding: KeyBinding, ok = true) {
  flash(`[data-action~="${CSS.escape(actionKey(binding.action))}"]`);
  flash(`[data-key="${CSS.escape(binding.code)}"]`);
  const entry = {
    id: ++counter,
    key: bindingLabel(binding),
    label: ok ? describe(binding.action) : `${describe(binding.action)} · no disponible`,
    slot: binding.action.slot,
  };
  entries = [...entries.slice(-3), entry];
  emit();
  setTimeout(() => {
    entries = entries.filter((e) => e.id !== entry.id);
    emit();
  }, 1100);
}
