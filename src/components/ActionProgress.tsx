// The common view of a long action (nu9ve-interaction-v1 §7), in its three sizes: a compact floating
// panel with one row per action, the most recent on top, the screen view a row opens, and the
// indicator chip in the title bar. Both read the
// same `ActionView`, so the bar, the percentage and the numbers are always the same.
import { useEffect } from 'react';
import {
  Check,
  ChevronDown,
  LoaderCircle,
  Minimize2,
  RotateCcw,
  TriangleAlert,
  X,
} from 'lucide-react';
import { tr, useLanguage } from '../lib/language';
import {
  cancelAction,
  dismissAction,
  openAction,
  useActionRuns,
  useOpenedAction,
  type ActionRun,
} from '../lib/action-runs';
import { systemProgress } from '../lib/action-system';
import type { ActionRecord, ActionView } from '../lib/action';

/** What the view says for a stage; an app stage it does not know reads «Working». */
export function stageText(stage: string): string {
  const shared: Record<string, [string, string]> = {
    preparing: ['Preparando', 'Preparing'],
    listing: ['Contando qué hay que hacer', 'Counting what to do'],
    scanning: ['Escaneando', 'Scanning'],
    reviewing: ['Revisando lo que ya está', 'Reviewing what is already there'],
    copying: ['Copiando', 'Copying'],
    verifying: ['Verificando', 'Verifying'],
    rendering: ['Renderizando', 'Rendering'],
    analyzing: ['Analizando', 'Analyzing'],
    releasing: ['Liberando originales', 'Releasing originals'],
    writing: ['Escribiendo el resultado', 'Writing the result'],
    finishing: ['Cerrando', 'Finishing'],
    'curado.loading_model': ['Cargando el modelo', 'Loading the model'],
    'curado.separating': ['Separando', 'Separating'],
  };
  const words = shared[stage];
  return words ? tr(...words) : tr('Trabajando', 'Working');
}
function actionText(action: string): string {
  const words: Record<string, [string, string]> = {
    import: ['Importando y analizando música', 'Importing and analyzing music'],
    analyze: ['Analizando BPM, grilla y clave', 'Analyzing BPM, grid and key'],
    separate: ['Separando en stems', 'Separating into stems'],
  };
  return words[action] ? tr(...words[action]) : action;
}
/** Minutes and seconds, or hours and minutes past an hour. */
export function duration(seconds: number): string {
  const s = Math.max(0, Math.round(seconds));
  if (s >= 3600) return `${Math.floor(s / 3600)} h ${Math.floor((s % 3600) / 60)} min`;
  if (s >= 60) return `${Math.floor(s / 60)} min ${s % 60} s`;
  return `${s} s`;
}
function countOf(record: ActionRecord): string | null {
  const { files_done, files_total, unit, overall_done, overall_total } = record;
  if (files_total !== undefined)
    return tr(
      `${files_done ?? 0} de ${files_total} archivos`,
      `${files_done ?? 0} of ${files_total} files`,
    );
  if (unit === 'items' && overall_total)
    return tr(`${overall_done} de ${overall_total}`, `${overall_done} of ${overall_total}`);
  return null;
}
function timing(view: ActionView): string | null {
  if (view.display === 'no_signal' && view.silentFor !== null)
    return tr(
      `Sin señal de la acción desde hace ${duration(view.silentFor)}`,
      `No signal from the action for ${duration(view.silentFor)}`,
    );
  const elapsed = view.elapsed !== null ? duration(view.elapsed) : null;
  if (view.eta === 'shown' && view.etaSeconds !== null)
    return tr(
      `faltan ~${duration(view.etaSeconds)} · ${elapsed}`,
      `~${duration(view.etaSeconds)} left · ${elapsed}`,
    );
  if (view.eta === 'pending') return tr(`ETA pendiente · ${elapsed}`, `ETA pending · ${elapsed}`);
  return elapsed;
}
const running = (view: ActionView) =>
  view.display === 'running' || view.display === 'no_signal' || view.display === 'preparing';

function Bar({ view }: { view: ActionView }) {
  const kind =
    view.display === 'failed'
      ? 'failed'
      : view.display === 'complete_with_warnings' || view.display === 'cancelled'
        ? 'warning'
        : view.display === 'complete'
          ? 'done'
          : 'running';
  const indeterminate = view.percent === null && running(view);
  return (
    <div
      className={`action-bar ${kind}${indeterminate ? ' indeterminate' : ''}`}
      role="progressbar"
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={view.percent ?? undefined}
      aria-valuetext={
        view.percent === null ? stageText(view.record?.stage ?? 'preparing') : undefined
      }
    >
      <span style={{ width: indeterminate ? undefined : `${view.percent ?? 0}%` }} />
    </div>
  );
}

function Closing({ run }: { run: ActionRun }) {
  const { view } = run;
  const record = view.record;
  if (view.display === 'gone')
    return (
      <p className="action-closing">
        {tr(
          'La acción terminó; su resultado ya no está disponible',
          'The action ended; its result is no longer available',
        )}
      </p>
    );
  if (!record) return null;
  const summary = record.outcome?.summary;
  return (
    <div className="action-closing">
      {view.display === 'failed' && record.error && (
        <p className="action-error">{record.error.message}</p>
      )}
      {view.display === 'cancelled' && <p>{tr('Cancelado', 'Cancelled')}</p>}
      {summary && <p>{summary}</p>}
      {record.outcome?.warnings?.map((warning) => (
        <p className="action-warning" key={warning.code}>
          {warning.message}
        </p>
      ))}
      {view.display === 'failed' && (
        <button
          type="button"
          onClick={() => {
            dismissAction(run.id);
            run.retry();
          }}
        >
          <RotateCcw size={12} /> {tr('Reintentar', 'Retry')}
        </button>
      )}
    </div>
  );
}

function Row({ run, onOpen }: { run: ActionRun; onOpen: () => void }) {
  const { view } = run;
  const record = view.record;
  const label = record?.label ?? actionText(run.action);
  const ended = !running(view);
  return (
    <li className={`action-row ${view.display}`}>
      <div className="action-row-head">
        {view.display === 'complete' ? (
          <Check size={13} className="action-done" />
        ) : view.display === 'failed' || view.display === 'complete_with_warnings' ? (
          <TriangleAlert size={13} className="action-alert" />
        ) : null}
        <button
          type="button"
          className="action-title"
          onClick={onOpen}
          title={tr('Ver en grande', 'Show large')}
        >
          {label}
        </button>
        <span className="action-percent">{view.percent === null ? '' : `${view.percent} %`}</span>
        {ended ? (
          <button
            type="button"
            className="action-icon"
            aria-label={tr('Cerrar', 'Close')}
            onClick={() => dismissAction(run.id)}
          >
            <X size={12} />
          </button>
        ) : (
          record?.cancellable && (
            <button
              type="button"
              className="action-cancel"
              onClick={() => void cancelAction(run.id)}
            >
              {tr('Cancelar', 'Cancel')}
            </button>
          )
        )}
      </div>
      <Bar view={view} />
      {!ended && record && (
        <p className="action-meta">
          {[
            stageText(record.stage),
            record.phase && `${record.phase.index}/${record.phase.count}`,
            countOf(record),
            timing(view),
          ]
            .filter(Boolean)
            .join(' · ')}
        </p>
      )}
      {ended && <Closing run={run} />}
    </li>
  );
}

/** The screen size: what is done, the file in hand, what cancelling does, and the folded log. */
function Screen({ run, onClose }: { run: ActionRun; onClose: () => void }) {
  const { view } = run;
  const record = view.record;
  const label = record?.label ?? actionText(run.action);
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', key);
    return () => window.removeEventListener('keydown', key);
  }, [onClose]);
  return (
    <div className="modal-backdrop" onClick={onClose}>
      <section
        className="modal action-screen"
        role="dialog"
        aria-label={label}
        onClick={(e) => e.stopPropagation()}
      >
        <header>
          <h2>{label}</h2>
          <button
            type="button"
            className="action-icon"
            aria-label={tr('Reducir', 'Minimize')}
            onClick={onClose}
          >
            <Minimize2 size={14} />
          </button>
        </header>
        <Bar view={view} />
        <p className="action-numbers">
          {[
            view.percent === null ? null : `${view.percent} %`,
            record && countOf(record),
            timing(view),
          ]
            .filter(Boolean)
            .join(' · ')}
        </p>
        {record && running(view) && (
          <>
            <p className="action-stage">
              {stageText(record.stage)}
              {record.phase &&
                ` · ${record.phase.label} (${record.phase.index}/${record.phase.count})`}
            </p>
            {record.current && (
              <p className="action-current">
                {record.current.name}
                {record.current.from &&
                  record.current.to &&
                  ` · ${record.current.from} → ${record.current.to}`}
              </p>
            )}
            <p className="action-hint">
              {record.cancellable
                ? tr(
                    'Cancelar detiene la acción; lo que ya quedó hecho se conserva.',
                    'Cancel stops the action; what is already done stays.',
                  )
                : tr('Esta acción no se puede cancelar.', 'This action cannot be cancelled.')}
            </p>
          </>
        )}
        {!running(view) && <Closing run={run} />}
        {record?.log && record.log.length > 0 && (
          <details className="action-log">
            <summary>
              <ChevronDown size={12} /> {tr('Detalle', 'Detail')}
            </summary>
            <ol>
              {record.log.map((entry, i) => (
                <li key={`${entry.at}-${i}`} className={entry.kind}>
                  {entry.kind === 'failed'
                    ? '✕'
                    : entry.kind === 'duplicate' || entry.kind === 'skipped'
                      ? '−'
                      : '✓'}{' '}
                  {entry.name}
                  {entry.text && <span> · {entry.text}</span>}
                </li>
              ))}
            </ol>
          </details>
        )}
        {running(view) && record?.cancellable && (
          <footer>
            <button type="button" onClick={() => void cancelAction(run.id)}>
              {tr('Cancelar', 'Cancel')}
            </button>
          </footer>
        )}
      </section>
    </div>
  );
}

/** The compact panel, and the screen view of the row it opens. */
export function ActionPanel() {
  useLanguage();
  const runs = useActionRuns();
  const open = useOpenedAction();
  const opened = runs.find((run) => run.id === open);
  if (!runs.length) return null;
  return (
    <>
      <ol className="action-panel" aria-label={tr('Acciones en curso', 'Running actions')}>
        {runs.map((run) => (
          <Row key={run.id} run={run} onOpen={() => openAction(run.id)} />
        ))}
      </ol>
      {opened && <Screen run={opened} onClose={() => openAction(null)} />}
    </>
  );
}

/** The indicator: the percentage of what runs, in the title bar. A click opens the newest one large. */
export function ActionChip() {
  useLanguage();
  const runs = useActionRuns();
  const active = runs.filter((run) => running(run.view));
  if (!active.length) return null;
  const shown = systemProgress(active);
  const text = 'progress' in shown ? `${shown.progress} %` : '…';
  const label =
    active.length === 1
      ? (active[0].view.record?.label ?? actionText(active[0].action))
      : tr(`${active.length} acciones en curso`, `${active.length} actions running`);
  return (
    <button
      type="button"
      className={`action-chip${shown.status === 'error' ? ' stalled' : ''}`}
      title={label}
      aria-label={`${label} · ${text}`}
      onClick={() => openAction(active[0].id)}
    >
      <LoaderCircle size={13} className="spinning" aria-hidden="true" />
      {text}
      {active.length > 1 && <b>{active.length}</b>}
    </button>
  );
}
