// The long actions this window launched, read with `action_progress` (nu9ve-action-v1 §4). Each run gets
// its own `progress_id`, is polled while it is on screen and stays after it ends until its closing has
// been seen: a success for a few seconds, a warning, failure or cancellation until dismissed.
import { useSyncExternalStore } from 'react';
import { call, native } from './bridge';
import {
  ActionWatch,
  POLL_VISIBLE_MS,
  finishedDisplay,
  newProgressId,
  type ActionView,
} from './action';

export interface ActionRun {
  id: string;
  action: string;
  view: ActionView;
  /** Launches the same work again under a new id, for «Reintentar». */
  retry: () => void;
}
/** How long a plain success stays before it leaves by itself. */
export const SUCCESS_STAYS_MS = 6000;

let runs: ActionRun[] = [];
const listeners = new Set<() => void>();
const subscribe = (cb: () => void) => {
  listeners.add(cb);
  return () => {
    listeners.delete(cb);
  };
};
function publish(next: ActionRun[]) {
  runs = next;
  listeners.forEach((f) => f());
}
/** The runs a view shows, most recent first, once a record of each has been read. */
export function useActionRuns() {
  return useSyncExternalStore(subscribe, () => runs);
}
export function dismissAction(id: string) {
  publish(runs.filter((run) => run.id !== id));
}
export async function cancelAction(id: string) {
  if (native) await call<boolean>('action_cancel', { progressId: id });
}

/** Runs `work` under a new progress id and shows its progress until it ends. Resolves with what the
 * command returns, as calling it directly would. */
export async function runAction<T>(
  action: string,
  work: (progressId: string) => Promise<T>,
): Promise<T> {
  const id = newProgressId(action);
  const watch = new ActionWatch(id);
  let settled = false;
  let timer: ReturnType<typeof setTimeout> | undefined;
  const retry = () => void runAction(action, work).catch(() => {});
  const show = (view: ActionView) => {
    // Nothing shows before the first record: a closed file picker leaves no trace.
    if (!watch.started) return;
    const run: ActionRun = { id, action, view, retry };
    const others = runs.filter((r) => r.id !== id);
    publish(
      runs.some((r) => r.id === id) ? runs.map((r) => (r.id === id ? run : r)) : [run, ...others],
    );
    if (view.display === 'complete')
      setTimeout(() => {
        if (runs.find((r) => r.id === id)?.view.display === 'complete') dismissAction(id);
      }, SUCCESS_STAYS_MS);
  };
  const poll = async () => {
    timer = undefined;
    let view: ActionView;
    try {
      view = watch.read(
        await call<unknown>('action_progress', { progressId: id }),
        Date.now() / 1000,
      );
    } catch {
      view = watch.last ?? watch.read(null, Date.now() / 1000);
    }
    show(view);
    const ended = finishedDisplay(view.display);
    // Once the command returned, a run that never wrote a record had nothing to do.
    if (ended || (settled && !watch.started)) return;
    timer = setTimeout(() => void poll(), POLL_VISIBLE_MS);
  };
  if (native) timer = setTimeout(() => void poll(), POLL_VISIBLE_MS);
  try {
    return await work(id);
  } finally {
    settled = true;
    if (native && timer !== undefined) {
      // Read the closing at once instead of waiting for the next tick.
      clearTimeout(timer);
      void poll();
    }
  }
}
