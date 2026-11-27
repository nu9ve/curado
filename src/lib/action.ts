// Reading long actions, nu9ve-action-v1 (vendor/nu9ve-core/contracts/nu9ve-action-v1.md). A port of the
// reference in core's scripts/action-examples.py: `recordProblem` is its §3 check, `ActionWatch` its §4
// client view, and both pass data/action-examples-v1.json (action.test.ts).

export const ACTION_CONTRACT = 'nu9ve-action/1';
const PROGRESS_ID = /^[a-z0-9][a-z0-9._:-]{0,127}$/;
const WORD = /^[a-z][a-z0-9_]{0,63}$/;
const APP_STAGE = /^[a-z][a-z0-9-]{0,31}\.[a-z][a-z0-9_]{0,63}$/;
const INSTANT = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(\.\d{1,9})?Z$/;
const WINDOWS_ABSOLUTE = /^[A-Za-z]:[\\/]/;
export const STAGES = [
  'preparing',
  'listing',
  'scanning',
  'reviewing',
  'copying',
  'verifying',
  'rendering',
  'analyzing',
  'releasing',
  'writing',
  'finishing',
] as const;
const STATES = ['running', 'complete', 'failed', 'cancelled'];
const UNITS = ['bytes', 'files', 'items', 'seconds', 'work'];
const MAX_SAFE = 2 ** 53 - 1;
const MAX_RECORD = 256 * 1024;
const MAX_LOG = 60;
const MAX_WARNINGS = 20;
export const ETA_AFTER_SECONDS = 5;
export const ETA_AFTER_FRACTION = 0.02;
export const ETA_PENDING_SECONDS = 15;
export const NO_SIGNAL_SECONDS = 60;
/// §4: how often a view on screen or compact reads, and how often only the indicator does.
export const POLL_VISIBLE_MS = 750;
export const POLL_INDICATOR_MS = 5000;

export type ActionState = 'running' | 'complete' | 'failed' | 'cancelled';
export interface ActionWarning {
  code: string;
  message: string;
}
export interface ActionRecord {
  contract: string;
  progress_id: string;
  action: string;
  label?: string;
  state: ActionState;
  stage: string;
  phase?: { index: number; count: number; label: string };
  step?: { index: number; count: number; label: string; fraction?: number | null };
  overall_done: number;
  overall_total: number | null;
  unit: 'bytes' | 'files' | 'items' | 'seconds' | 'work';
  files_done?: number;
  files_total?: number;
  current?: {
    name: string;
    from?: string;
    to?: string;
    action?: string;
    file_done?: number;
    file_total?: number;
  };
  log?: { at: string; kind: string; name: string; target?: string; text?: string }[];
  cancellable: boolean;
  started_at: string;
  progressed_at: string;
  updated_at: string;
  outcome?: { summary: string; warnings?: ActionWarning[]; receipt?: string | null };
  error?: { code: string; message: string };
  resumable?: boolean;
}
export interface UnknownRecord {
  contract: string;
  progress_id: string;
  state: 'unknown';
}
export type ActionReply = ActionRecord | UnknownRecord;

type Json = Record<string, unknown>;
const isObject = (v: unknown): v is Json =>
  typeof v === 'object' && v !== null && !Array.isArray(v);
const isInt = (v: unknown): v is number => typeof v === 'number' && Number.isInteger(v);
const countOk = (v: unknown): v is number => isInt(v) && v >= 0 && v <= MAX_SAFE;
const length = (s: string) => [...s].length;
const shortText = (v: unknown, limit: number): v is string =>
  typeof v === 'string' && length(v) > 0 && length(v) <= limit && !v.includes('\0');
const keys = (v: Json) => Object.keys(v);
const has = (v: Json, key: string) => Object.prototype.hasOwnProperty.call(v, key);

/** Seconds since the epoch of an RFC 3339 UTC instant with Z, or null. */
export function instant(value: unknown): number | null {
  if (typeof value !== 'string') return null;
  const m = INSTANT.exec(value);
  if (!m) return null;
  const [y, mo, d, h, mi, s] = m.slice(1, 7).map(Number);
  const ms = Date.UTC(y, mo - 1, d, h, mi, s);
  const back = new Date(ms);
  // Reject what strptime rejects: a day or a time that does not exist.
  if (
    back.getUTCFullYear() !== y ||
    back.getUTCMonth() !== mo - 1 ||
    back.getUTCDate() !== d ||
    h > 23 ||
    mi > 59 ||
    s > 59
  )
    return null;
  return ms / 1000 + (m[7] ? Number('0' + m[7]) : 0);
}

/** §2: null when `action_progress` accepts the params, otherwise the error code. */
export function paramsProblem(params: unknown): string | null {
  if (!isObject(params) || keys(params).length !== 1 || !has(params, 'progress_id'))
    return 'invalid_params';
  const id = params.progress_id;
  return typeof id === 'string' && PROGRESS_ID.test(id) ? null : 'invalid_params';
}
export const validProgressId = (id: string) => PROGRESS_ID.test(id);

function counterProblem(value: unknown, name: 'phase' | 'step'): string | null {
  const bad = `bad_${name}`;
  if (!isObject(value)) return bad;
  const allowed = new Set(['index', 'count', 'label', ...(name === 'step' ? ['fraction'] : [])]);
  if (keys(value).some((k) => !allowed.has(k))) return bad;
  if (!['index', 'count', 'label'].every((k) => has(value, k))) return bad;
  const { index, count } = value;
  if (!isInt(index) || !isInt(count) || !(1 <= index && index <= count && count <= MAX_SAFE))
    return bad;
  if (!shortText(value.label, 200)) return bad;
  const fraction = value.fraction;
  if (
    fraction !== undefined &&
    fraction !== null &&
    (typeof fraction !== 'number' || fraction < 0 || fraction > 1)
  )
    return bad;
  return null;
}
function currentProblem(value: unknown): string | null {
  if (!isObject(value) || !shortText(value.name, 255) || value.name.includes('/'))
    return 'bad_current';
  for (const key of ['from', 'to', 'action'])
    if (has(value, key) && typeof value[key] !== 'string') return 'bad_current';
  for (const key of ['file_done', 'file_total'])
    if (has(value, key) && !countOk(value[key])) return 'bad_current';
  if (
    has(value, 'file_done') &&
    has(value, 'file_total') &&
    (value.file_done as number) > (value.file_total as number)
  )
    return 'bad_current';
  return null;
}
function logProblem(value: unknown): string | null {
  if (!Array.isArray(value) || value.length > MAX_LOG) return 'bad_log';
  for (const entry of value) {
    if (!isObject(entry) || instant(entry.at) === null) return 'bad_log';
    if (typeof entry.kind !== 'string' || !WORD.test(entry.kind)) return 'bad_log';
    if (typeof entry.name !== 'string') return 'bad_log';
    for (const key of ['target', 'text'])
      if (has(entry, key) && typeof entry[key] !== 'string') return 'bad_log';
  }
  return null;
}
const absolute = (path: unknown) =>
  typeof path === 'string' && (path.startsWith('/') || WINDOWS_ABSOLUTE.test(path));
function outcomeProblem(value: unknown): string | null {
  if (!isObject(value) || !shortText(value.summary, 500)) return 'bad_outcome';
  const warnings = has(value, 'warnings') ? value.warnings : [];
  if (!Array.isArray(warnings) || warnings.length > MAX_WARNINGS) return 'bad_outcome';
  for (const w of warnings)
    if (
      !isObject(w) ||
      keys(w).length !== 2 ||
      typeof w.code !== 'string' ||
      !WORD.test(w.code) ||
      !shortText(w.message, 500)
    )
      return 'bad_outcome';
  const receipt = value.receipt;
  if (receipt !== undefined && receipt !== null && !absolute(receipt)) return 'bad_outcome';
  return null;
}

/** §3: null when `record` (the data of an action_progress reply) is valid for `progressId`, otherwise
 * why not. An `unknown` reply is valid with only its three fields. */
export function recordProblem(record: unknown, progressId: string): string | null {
  if (!isObject(record)) return 'not_an_object';
  if (new TextEncoder().encode(JSON.stringify(record)).length > MAX_RECORD) return 'too_large';
  if (record.contract !== ACTION_CONTRACT) return 'bad_contract';
  if (record.progress_id !== progressId) return 'other_progress_id';
  const state = record.state;
  if (state === 'unknown') return keys(record).length === 3 ? null : 'unknown_with_fields';
  if (typeof state !== 'string' || !STATES.includes(state)) return 'bad_state';
  if (typeof record.action !== 'string' || !WORD.test(record.action)) return 'bad_action';
  if (has(record, 'label') && !shortText(record.label, 200)) return 'bad_label';
  const stage = record.stage;
  if (
    typeof stage !== 'string' ||
    (!(STAGES as readonly string[]).includes(stage) && !APP_STAGE.test(stage))
  )
    return 'bad_stage';
  for (const name of ['phase', 'step'] as const)
    if (has(record, name)) {
      const problem = counterProblem(record[name], name);
      if (problem) return problem;
    }
  const done = record.overall_done;
  if (!countOk(done)) return 'bad_overall';
  if (!has(record, 'overall_total')) return 'bad_overall';
  const total = record.overall_total;
  if (total !== null && !countOk(total)) return 'bad_overall';
  if (total && done > total) return 'done_over_total';
  if (typeof record.unit !== 'string' || !UNITS.includes(record.unit)) return 'bad_unit';
  for (const key of ['files_done', 'files_total'])
    if (has(record, key) && !countOk(record[key])) return 'bad_files';
  if (
    has(record, 'files_done') &&
    has(record, 'files_total') &&
    (record.files_done as number) > (record.files_total as number)
  )
    return 'bad_files';
  if (has(record, 'current')) {
    const problem = currentProblem(record.current);
    if (problem) return problem;
  }
  if (has(record, 'log')) {
    const problem = logProblem(record.log);
    if (problem) return problem;
  }
  if (typeof record.cancellable !== 'boolean') return 'bad_cancellable';
  const [started, progressed, updated] = ['started_at', 'progressed_at', 'updated_at'].map((k) =>
    instant(record[k]),
  );
  if (started === null || progressed === null || updated === null) return 'bad_instant';
  if (!(started <= progressed && progressed <= updated)) return 'instants_out_of_order';
  if (state === 'running') {
    if (total && done === total) return 'full_while_running';
    if (['outcome', 'error', 'resumable'].some((k) => has(record, k)))
      return 'closing_while_running';
    return null;
  }
  // §3.4: a finished record
  if (record.cancellable || record.current) return 'still_working_after_end';
  if (has(record, 'outcome')) {
    const problem = outcomeProblem(record.outcome);
    if (problem) return problem;
  }
  if (state === 'complete') {
    if (!has(record, 'outcome')) return 'complete_without_outcome';
    if (has(record, 'error') || has(record, 'resumable')) return 'error_on_complete';
    if (total && done !== total) return 'complete_not_full';
    return null;
  }
  if (typeof record.resumable !== 'boolean') return 'bad_resumable';
  if (state === 'failed') {
    const error = record.error;
    if (
      !isObject(error) ||
      keys(error).length !== 2 ||
      typeof error.code !== 'string' ||
      !WORD.test(error.code) ||
      !shortText(error.message, 500)
    )
      return 'bad_error';
  } else if (has(record, 'error')) return 'error_on_cancelled';
  return null;
}

export function fraction(record: ActionRecord): number | null {
  return record.overall_total ? record.overall_done / record.overall_total : null;
}

/** §3.3 and §3.4 between two valid records of the same action, read in this order. */
export function stepProblem(before: ActionReply, after: ActionReply): string | null {
  if (before.state === 'unknown' || after.state === 'unknown') return null;
  if (after.started_at !== before.started_at) return null; // another action with the same id (§1)
  if (before.state !== 'running' && after.state === 'running') return 'running_after_end';
  if ((instant(after.updated_at) ?? 0) < (instant(before.updated_at) ?? 0))
    return 'updated_went_back';
  const a = fraction(before);
  const b = fraction(after);
  if (a !== null && b !== null && b < a) return 'fraction_went_back';
  return null;
}

export type ActionDisplay =
  | 'preparing'
  | 'running'
  | 'no_signal'
  | 'gone'
  | 'complete'
  | 'complete_with_warnings'
  | 'failed'
  | 'cancelled';
export interface ActionShows {
  display: ActionDisplay;
  /** Truncated, never rounded; null without a total. The bar uses the same number. */
  percent: number | null;
  eta: 'none' | 'pending' | 'shown';
  resumable?: boolean;
  ignored?: string;
}
export interface ActionView extends ActionShows {
  /** The last valid record, to show its label, files, file in hand, log and closing. */
  record: ActionRecord | null;
  /** Seconds left, smoothed, while `eta` is `shown`. */
  etaSeconds: number | null;
  /** Seconds since the action started, by the client's clock. */
  elapsed: number | null;
  /** Seconds without a heartbeat while `display` is `no_signal`. */
  silentFor: number | null;
}

/** The common view's memory of one action (§4): it keeps the highest fraction, ignores invalid reads,
 * and works out the ETA. */
export class ActionWatch {
  private shown: ActionView | null = null;
  private best: number | null = null;
  private seen = false;
  private eta: number | null = null;
  constructor(readonly progressId: string) {}
  /** Whether a record of this action has been read. */
  get started() {
    return this.seen;
  }
  get last(): ActionView | null {
    return this.shown;
  }
  /** Feeds one reply (parsed JSON) read at `now` seconds since the epoch. */
  read(reply: unknown, now: number): ActionView {
    const problem = recordProblem(reply, this.progressId);
    if (problem) {
      if (typeof console !== 'undefined')
        console.warn(`action ${this.progressId}: invalid record (${problem})`);
      const last: ActionView = this.shown
        ? { ...this.shown }
        : { ...EMPTY_VIEW, display: 'preparing' };
      last.ignored = problem;
      this.shown = last;
      return last;
    }
    const r = reply as ActionReply;
    if (r.state === 'unknown') {
      const gone = this.seen;
      return this.keep({
        ...EMPTY_VIEW,
        display: gone ? 'gone' : 'preparing',
        record: gone ? (this.shown?.record ?? null) : null,
      });
    }
    this.seen = true;
    const f = fraction(r);
    if (f !== null) this.best = this.best === null ? f : Math.max(this.best, f);
    const percent = f === null || this.best === null ? null : Math.floor(this.best * 100);
    const elapsed = Math.max(0, now - (instant(r.started_at) ?? now));
    const base = { record: r, elapsed, etaSeconds: null, silentFor: null };
    if (r.state === 'complete')
      return this.keep({
        ...base,
        display: r.outcome?.warnings?.length ? 'complete_with_warnings' : 'complete',
        percent: f !== null ? 100 : null,
        eta: 'none',
      });
    if (r.state === 'failed' || r.state === 'cancelled')
      return this.keep({
        ...base,
        display: r.state,
        percent,
        eta: 'none',
        resumable: r.resumable,
      });
    const silent = now - (instant(r.updated_at) ?? now);
    const display = silent > NO_SIGNAL_SECONDS ? 'no_signal' : 'running';
    let eta: ActionShows['eta'];
    if (this.best === null || elapsed < ETA_AFTER_SECONDS || this.best < ETA_AFTER_FRACTION)
      eta = 'none';
    else if (now - (instant(r.progressed_at) ?? now) > ETA_PENDING_SECONDS) eta = 'pending';
    else eta = 'shown';
    if (eta === 'shown' && this.best !== null) {
      const raw = (elapsed * (1 - this.best)) / this.best;
      this.eta = this.eta === null ? raw : this.eta + 0.3 * (raw - this.eta);
    }
    return this.keep({
      ...base,
      display,
      percent,
      eta,
      etaSeconds: eta === 'shown' ? this.eta : null,
      silentFor: display === 'no_signal' ? silent : null,
    });
  }
  private keep(view: ActionView) {
    this.shown = view;
    return view;
  }
}
const EMPTY_VIEW: ActionView = {
  display: 'preparing',
  percent: null,
  eta: 'none',
  record: null,
  etaSeconds: null,
  elapsed: null,
  silentFor: null,
};
export const finishedDisplay = (display: ActionDisplay) =>
  display === 'complete' ||
  display === 'complete_with_warnings' ||
  display === 'failed' ||
  display === 'cancelled' ||
  display === 'gone';

/** The reference's `client_view`: what the view shows after each read, as the examples write it. */
export function clientView(reads: { at: string; reply: string }[], progressId: string) {
  const watch = new ActionWatch(progressId);
  return reads.map(({ at, reply }) => {
    const { display, percent, eta, resumable, ignored } = watch.read(
      JSON.parse(reply),
      instant(at) ?? 0,
    );
    const shows: ActionShows = { display, percent, eta };
    if (resumable !== undefined) shows.resumable = resumable;
    if (ignored !== undefined) shows.ignored = ignored;
    return shows;
  });
}

let launched = 0;
/** A `progress_id` unique in time, `<action>-<date>-<n>` as §1 suggests. */
export function newProgressId(action: string, now = new Date()) {
  launched += 1;
  const stamp = now
    .toISOString()
    .replace(/[^0-9]/g, '')
    .slice(0, 14);
  return `${action}-${stamp}-${launched}`;
}
