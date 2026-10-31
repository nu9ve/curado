/**
 * Trigger FX: a Beat FX for as long as a pad is held, harder meaning more. It borrows the channel's
 * unit and gives it back, and unlike Release FX the track carries on. Shift keeps it on.
 */
export interface FxBefore {
  kind: number;
  depth: number;
  on: boolean;
}
type Send = (kind: string, value: number) => void;
/** A tail keeps ringing after the effect stops, so its settings return once it has faded. */
const RESTORE_MS = 2000;
const LOWEST = 0.35;
export const triggerDepth = (level: number) =>
  Math.round((LOWEST + (1 - LOWEST) * Math.max(0, Math.min(1, level))) * 1000) / 1000;

export function createTrigger(
  later: (run: () => void, ms: number) => unknown = setTimeout,
  cancel: (timer: unknown) => void = (timer) => clearTimeout(timer as number),
) {
  let active: number | null = null;
  let latched = false;
  let before: FxBefore | null = null;
  let timer: unknown = null;
  const finish = (send: Send) => {
    active = null;
    latched = false;
    const saved = before;
    if (!saved) return;
    if (saved.on) {
      send('beatFx', saved.kind);
      send('beatFxDepth', saved.depth);
      before = null;
      return;
    }
    send('beatFxOn', 0);
    timer = later(() => {
      timer = null;
      send('beatFx', saved.kind);
      send('beatFxDepth', saved.depth);
      before = null;
    }, RESTORE_MS);
  };
  return {
    active: () => active,
    latched: () => latched,
    press(kind: number, latch: boolean, now: FxBefore, send: Send) {
      if (active === kind && latched && now.on) {
        finish(send);
        return;
      }
      if (timer !== null) {
        cancel(timer);
        timer = null;
      }
      before ??= now;
      active = kind;
      latched = latch;
      send('beatFx', kind);
      send('beatFxDepth', triggerDepth(0));
      send('beatFxOn', 1);
    },
    lift(kind: number, send: Send) {
      if (active === kind && !latched) finish(send);
    },
  };
}
export const DECK_TRIGGERS = [createTrigger(), createTrigger()];
