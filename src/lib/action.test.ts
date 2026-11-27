import { describe, expect, test } from 'bun:test';
import examples from '../../vendor/nu9ve-core/data/action-examples-v1.json';
import {
  ActionWatch,
  clientView,
  instant,
  newProgressId,
  paramsProblem,
  recordProblem,
  stepProblem,
  validProgressId,
} from './action';

const PID = 'ingreso-2026-11-26-1';

describe('long actions (nu9ve-action-v1)', () => {
  test('requests: action_progress params as §2 accepts them', () => {
    expect(examples.requests.length).toBe(7);
    for (const { name, request, ok, code } of examples.requests as {
      name: string;
      request: string;
      ok: boolean;
      code?: string;
    }[]) {
      const problem = paramsProblem(JSON.parse(request).params);
      expect(problem, name).toBe(ok ? null : (code ?? null));
    }
  });
  test('every valid record is valid', () => {
    expect(examples.valid.length).toBe(11);
    for (const { name, record } of examples.valid)
      expect(recordProblem(JSON.parse(record), PID), name).toBeNull();
  });
  test('every invalid record is refused for its own reason', () => {
    expect(examples.invalid.length).toBe(20);
    for (const { name, record, problem } of examples.invalid)
      expect(recordProblem(JSON.parse(record), PID), name).toBe(problem);
  });
  test('sequences: what the view shows after each read, and each step', () => {
    expect(examples.sequences.length).toBe(6);
    for (const sequence of examples.sequences as {
      name: string;
      reads: { at: string; reply: string; shows: Record<string, unknown> }[];
      steps: (string | null)[];
    }[]) {
      const shown = clientView(sequence.reads, PID);
      shown.forEach((shows, i) =>
        expect(shows, `${sequence.name} #${i}`).toEqual(sequence.reads[i].shows as never),
      );
      const replies = sequence.reads.map((read) => JSON.parse(read.reply));
      // Pairs of consecutive reads; a pair with an invalid read has no step, as the reference writes it.
      const steps = replies
        .slice(1)
        .map((after, i) =>
          recordProblem(replies[i], PID) === null && recordProblem(after, PID) === null
            ? stepProblem(replies[i], after)
            : null,
        );
      expect(steps, sequence.name).toEqual(sequence.steps);
    }
  });
  test('instants: RFC 3339 in UTC with Z only', () => {
    expect(instant('2026-11-26T18:02:11Z')).toBe(1795716131);
    expect(instant('2026-11-26T18:02:11.250Z')).toBe(1795716131.25);
    expect(instant('2026-11-26T18:02:11+00:00')).toBeNull();
    expect(instant('2026-02-30T00:00:00Z')).toBeNull();
  });
  test('the ETA is smoothed and only while it is shown', () => {
    const watch = new ActionWatch(PID);
    const base = JSON.parse(examples.valid[0].record);
    const at = (s: number) => instant('2026-11-26T18:00:00Z')! + s;
    const record = (done: number, progressed: number, updated: number) => ({
      ...base,
      overall_done: done,
      progressed_at: new Date(at(progressed) * 1000).toISOString().replace('.000Z', 'Z'),
      updated_at: new Date(at(updated) * 1000).toISOString().replace('.000Z', 'Z'),
    });
    const first = watch.read(record(1_000_000_000, 20, 20), at(20));
    expect(first.eta).toBe('shown');
    expect(first.etaSeconds).toBeCloseTo(60);
    const second = watch.read(record(2_000_000_000, 30, 30), at(30));
    // Raw is 30 s; smoothing moves 30 % of the way from 60.
    expect(second.etaSeconds).toBeCloseTo(51);
    const stalled = watch.read(record(2_000_000_000, 30, 50), at(50));
    expect(stalled.eta).toBe('pending');
    expect(stalled.etaSeconds).toBeNull();
  });
  test('ids curado launches follow the pattern and do not repeat', () => {
    const now = new Date('2026-11-27T10:11:12Z');
    const a = newProgressId('import', now);
    const b = newProgressId('import', now);
    expect(validProgressId(a)).toBe(true);
    expect(a).not.toBe(b);
    expect(a.startsWith('import-20261127101112-')).toBe(true);
  });
});
