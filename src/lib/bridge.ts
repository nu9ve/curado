import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useSyncExternalStore } from 'react';
import type { Action, AppSnapshot } from './types';
import { tr } from './language';
export const native = isTauri();
let snapshot: AppSnapshot = {
  mode: 'player',
  persistenceError: null,
  queue: { count: 0, index: 0, active: false, repeat: 'off', error: null },
  audio: null,
  output: null,
  error: native ? null : 'Vista previa web. Abre la app de escritorio para reproducir audio.',
  midi: {
    connected: null,
    bindings: [],
    learning: null,
    lastMessage: null,
    error: null,
    dropped: 0,
  },
};
const listeners = new Set<() => void>();
const subscribe = (cb: () => void) => {
  listeners.add(cb);
  return () => {
    listeners.delete(cb);
  };
};
export function useSnapshot() {
  return useSyncExternalStore(subscribe, () => snapshot);
}
export function useMode() {
  return useSyncExternalStore(subscribe, () => snapshot.mode);
}
export async function setSessionMode(mode: 'player' | 'mix') {
  if (native) await call('set_mode', { mode });
  update({ ...snapshot, mode });
}
export const getSnapshot = () => snapshot;
function update(s: AppSnapshot) {
  snapshot = s;
  listeners.forEach((f) => f());
}
export async function connect() {
  if (!native) return () => {};
  const unlisten = await listen<AppSnapshot>('engine:state', (e) => update(e.payload));
  try {
    update(await invoke<AppSnapshot>('status'));
  } catch (e) {
    unlisten();
    throw e;
  }
  return unlisten;
}
export async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!native)
    throw new Error(
      tr(
        'Esta función requiere curado de escritorio. Ejecuta bun run desktop.',
        'This feature requires the curado desktop app. Run bun run desktop.',
      ),
    );
  return invoke<T>(command, args);
}
export const control = (slot: number, kind: string, value = 0) =>
  call<void>('audio_action', { action: { slot, kind, value } satisfies Action });
// Sliders coalesce on the UI side. Transport events bypass this path.
const pending = new Map<string, Action>();
let scheduled = false;
export function parameter(
  slot: number,
  kind: string,
  value: number,
  onError: (e: unknown) => void,
) {
  pending.set(`${slot}:${kind}`, { slot, kind, value });
  if (scheduled) return;
  scheduled = true;
  window.setTimeout(() => {
    scheduled = false;
    for (const a of pending.values()) call<void>('audio_action', { action: a }).catch(onError);
    pending.clear();
  }, 16);
}
