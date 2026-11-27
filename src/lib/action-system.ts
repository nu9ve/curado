// The indicator size of long actions outside the window (nu9ve-interaction-v1 §7): progress on the Dock
// icon or the taskbar button, and the Dock bouncing once when an action ends with the window in the
// background. A system notification (nu9ve-notifications-v1) is not here: it needs its own channel.
import { getCurrentWindow, ProgressBarStatus, UserAttentionType } from '@tauri-apps/api/window';
import { native } from './bridge';
import { finishedDisplay } from './action';
import { watchActionRuns, type ActionRun } from './action-runs';

const running = (run: ActionRun) => !finishedDisplay(run.view.display);

export type SystemProgress =
  | { status: 'none' }
  | { status: 'indeterminate' }
  | { status: 'normal' | 'error'; progress: number };

/** What the Dock shows for these runs: the least advanced running action with a measure, an
 * indeterminate bar while one runs without it, and nothing once all have ended. */
export function systemProgress(runs: readonly ActionRun[]): SystemProgress {
  const active = runs.filter(running);
  if (!active.length) return { status: 'none' };
  const measured = active
    .map((run) => run.view.percent)
    .filter((percent): percent is number => percent !== null);
  if (!measured.length) return { status: 'indeterminate' };
  const stalled = active.some((run) => run.view.display === 'no_signal');
  return { status: stalled ? 'error' : 'normal', progress: Math.min(...measured) };
}
/** The runs that ended since `before`: each gets one attention request, never one per file. */
export function newlyEnded(before: ReadonlyMap<string, boolean>, runs: readonly ActionRun[]) {
  return runs.filter((run) => before.get(run.id) === true && !running(run)).map((run) => run.id);
}

const STATUS = {
  none: ProgressBarStatus.None,
  indeterminate: ProgressBarStatus.Indeterminate,
  normal: ProgressBarStatus.Normal,
  error: ProgressBarStatus.Error,
} as const;

/** Mirrors the runs on the Dock until the returned function stops it. */
export function connectSystemProgress() {
  if (!native) return () => {};
  const win = getCurrentWindow();
  let shown = '';
  let wasRunning = new Map<string, boolean>();
  return watchActionRuns((runs) => {
    const next = systemProgress(runs);
    const key = JSON.stringify(next);
    if (key !== shown) {
      shown = key;
      void win
        .setProgressBar({
          status: STATUS[next.status],
          progress: 'progress' in next ? next.progress : undefined,
        })
        .catch(() => {});
    }
    const ended = newlyEnded(wasRunning, runs);
    wasRunning = new Map(runs.map((run) => [run.id, running(run)]));
    if (ended.length)
      void win
        .isFocused()
        .then((focused) => {
          if (!focused) return win.requestUserAttention(UserAttentionType.Informational);
        })
        .catch(() => {});
  });
}
