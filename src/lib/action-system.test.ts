import { describe, expect, test } from 'bun:test';
import { newlyEnded, systemProgress } from './action-system';
import type { ActionRun } from './action-runs';
import type { ActionDisplay } from './action';

const run = (id: string, display: ActionDisplay, percent: number | null): ActionRun => ({
  id,
  action: 'import',
  retry: () => {},
  view: {
    display,
    percent,
    eta: 'none',
    record: null,
    etaSeconds: null,
    elapsed: null,
    silentFor: null,
  },
});

describe('long actions on the Dock', () => {
  test('nothing running clears the bar', () => {
    expect(systemProgress([])).toEqual({ status: 'none' });
    expect(systemProgress([run('a', 'complete', 100), run('b', 'failed', 40)])).toEqual({
      status: 'none',
    });
  });
  test('the least advanced measured action leads, and one without a measure is indeterminate', () => {
    expect(systemProgress([run('a', 'running', 70), run('b', 'running', 20)])).toEqual({
      status: 'normal',
      progress: 20,
    });
    expect(systemProgress([run('a', 'preparing', null)])).toEqual({ status: 'indeterminate' });
    expect(systemProgress([run('a', 'running', null), run('b', 'running', 30)])).toEqual({
      status: 'normal',
      progress: 30,
    });
  });
  test('an action without a heartbeat marks the bar', () => {
    expect(systemProgress([run('a', 'no_signal', 50)])).toEqual({ status: 'error', progress: 50 });
  });
  test('each action asks for attention once, when it ends', () => {
    const before = new Map([
      ['a', true],
      ['b', true],
      ['c', false],
    ]);
    const now = [run('a', 'complete', 100), run('b', 'running', 50), run('c', 'failed', 10)];
    expect(newlyEnded(before, now)).toEqual(['a']);
    expect(newlyEnded(new Map(), [run('d', 'complete', 100)])).toEqual([]);
  });
});
