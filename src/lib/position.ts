import { useSyncExternalStore } from 'react';

/**
 * Deck positions from the engine, more often than the full state. Nothing here predicts: a value
 * is what the engine last reported, and when reports stop, the full state speaks again.
 */
export const STALE_MS = 120;
export interface LivePositions {
  values: number[];
  at: number;
}
export function livePosition(
  live: LivePositions | null,
  deck: number,
  fallback: number,
  now: number,
): number {
  if (!live || now - live.at > STALE_MS) return fallback;
  return live.values[deck] ?? fallback;
}

let live: LivePositions | null = null;
let frame = 0;
const listeners = new Set<() => void>();
const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
};
/** Arrives at its own pace, and anything drawn from it is drawn once per screen refresh. */
export function receivePositions(values: number[]) {
  live = { values, at: performance.now() };
  if (frame) return;
  frame = requestAnimationFrame(() => {
    frame = 0;
    listeners.forEach((listener) => listener());
  });
}
export function useLivePosition(deck: number, fallback: number): number {
  return useSyncExternalStore(subscribe, () =>
    livePosition(live, deck, fallback, performance.now()),
  );
}
