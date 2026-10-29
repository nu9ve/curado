import { tr } from '../lib/language';
import { useEffect, useState } from 'react';
import { call, control, useSnapshot } from '../lib/bridge';
import { keyLabel, transposeKey, NOTES } from '../lib/music';
import type { Library, MusicAnalysis, Slot } from '../lib/types';

export function MusicalControls({
  index,
  slot,
  disabled,
  onError,
  onLibrary,
}: {
  index: number;
  slot: Slot;
  disabled: boolean;
  onError: (e: unknown) => void;
  onLibrary: (data: Library) => void;
}) {
  const { audio, tuningError } = useSnapshot();
  const master = audio?.masterDeck ?? 0;
  const pending = slot.tuneRevision !== slot.tunedRevision;
  const bpm = slot.music.bpm;
  const [draft, setDraft] = useState('');
  const [editing, setEditing] = useState(false);
  useEffect(() => {
    if (!editing) setDraft(bpm ? (bpm * slot.tuning.rate).toFixed(1) : '');
  }, [bpm, slot.tuning.rate, editing]);
  const shift =
    slot.appliedTuning.semitones + (slot.appliedTuning.keyLock ? 0 : 12 * Math.log2(slot.rate));
  const key = transposeKey(slot.music.key, shift);
  const cents = Math.round((shift - Math.round(shift)) * 100);
  const send = (kind: string, value = 0) => control(index, kind, value).catch(onError);
  return (
    <div className="musical-controls">
      <div className="musical-row">
        <label className="tempo-control" data-action={`${index}:tempo:0`}>
          BPM
          <input
            type="number"
            data-action={`${index}:tempo:0`}
            aria-label={`Tempo deck ${index ? 'B' : 'A'}`}
            value={draft}
            min={bpm ? Math.max(40, bpm * 0.5) : 40}
            max={bpm ? Math.min(300, bpm * 1.5) : 300}
            step="0.1"
            disabled={disabled || !bpm || slot.sync}
            onFocus={() => setEditing(true)}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') e.currentTarget.blur();
            }}
            onBlur={() => {
              setEditing(false);
              if (
                draft &&
                Number.isFinite(+draft) &&
                bpm &&
                Math.abs(+draft - bpm * slot.tuning.rate) > 0.04
              )
                send('tempo', +draft);
            }}
          />
        </label>
        <button
          disabled={disabled}
          aria-pressed={master === index}
          data-action={`${index}:masterDeck:0`}
          onClick={() => send('masterDeck')}
        >
          {master === index ? 'MASTER' : tr('hacer master', 'make master')}
        </button>
        <button
          disabled={disabled || master === index || !bpm || !audio?.slots[master]?.music.bpm}
          aria-pressed={slot.sync}
          data-action={`${index}:sync:${slot.sync ? 0 : 1}`}
          onClick={() => send('sync', slot.sync ? 0 : 1)}
        >
          SYNC
        </button>
        <button
          disabled={disabled}
          aria-pressed={slot.quantize}
          data-action={`${index}:quantize:${slot.quantize ? 0 : 1}`}
          onClick={() => send('quantize', slot.quantize ? 0 : 1)}
        >
          Q
        </button>
        <select
          aria-label={tr(
            `Loop en beats deck ${index ? 'B' : 'A'}`,
            `Beat loop deck ${index ? 'B' : 'A'}`,
          )}
          disabled={disabled || !bpm}
          value={slot.looping ? slot.loopBeats : ''}
          data-action={`${index}:loopBeats:0`}
          onChange={(e) => send('loopBeats', +e.target.value)}
        >
          <option value="" disabled>
            loop · beats
          </option>
          {[1, 2, 4, 8, 16].map((n) => (
            <option key={n} value={n}>
              {n} beats
            </option>
          ))}
        </select>
      </div>
      <div className="musical-row key-controls">
        <span
          className="musical-key"
          title={tr('Tonalidad de salida estimada', 'Estimated output key')}
        >
          {keyLabel(key)} <b>{keyLabel(key, 'camelot')}</b>
          {cents !== 0 && (
            <small>
              {cents > 0 ? '+' : ''}
              {cents}¢
            </small>
          )}
        </span>
        <button
          disabled={disabled}
          aria-pressed={slot.tuning.keyLock}
          data-action={`${index}:keyLock:${slot.tuning.keyLock ? 0 : 1}`}
          onClick={() => send('keyLock', slot.tuning.keyLock ? 0 : 1)}
        >
          key lock
        </button>
        <button
          disabled={disabled || slot.tuning.semitones <= -12}
          aria-label={tr('Bajar un semitono', 'Down one semitone')}
          data-action={`${index}:transposeStep:-1`}
          onClick={() => send('transposeStep', -1)}
        >
          −
        </button>
        <button
          disabled={disabled}
          aria-label={tr('Restaurar transposición', 'Reset transposition')}
          data-action={`${index}:transpose:0`}
          onClick={() => send('transpose', 0)}
        >
          {slot.tuning.semitones > 0 ? '+' : ''}
          {slot.tuning.semitones} st
        </button>
        <button
          disabled={disabled || slot.tuning.semitones >= 12}
          aria-label={tr('Subir un semitono', 'Up one semitone')}
          data-action={`${index}:transposeStep:1`}
          onClick={() => send('transposeStep', 1)}
        >
          +
        </button>
        <button
          disabled={
            disabled || master === index || !slot.music.key || !audio?.slots[master]?.music.key
          }
          data-action={`${index}:keySync:0`}
          onClick={() => send('keySync')}
        >
          key sync
        </button>
      </div>
      {pending && (
        <small className="tuning-pending" role="status">
          {tr(
            'Preparando ajuste de audio… el tempo anterior continúa hasta estar listo.',
            'Preparing audio adjustment… the previous tempo continues until ready.',
          )}{' '}
        </small>
      )}
      {slot.queued && (
        <small role="status">
          {tr(
            'Cue preparado · espera al siguiente beat del master.',
            "Cue queued · waiting for the master's next beat.",
          )}
        </small>
      )}
      {tuningError && pending && (
        <small className="error-text">
          {tuningError}{' '}
          <button onClick={() => send('retryTuning')}>{tr('reintentar', 'retry')}</button>
        </small>
      )}
      {!bpm && !disabled && (
        <small>
          {tr(
            'Sin beatgrid: analiza o corrige BPM para cuantizar.',
            'No beatgrid: analyze or correct BPM to quantize.',
          )}
        </small>
      )}
      {!disabled && (
        <AnalysisEditor
          id={slot.id}
          music={slot.music}
          position={slot.position}
          onLibrary={onLibrary}
          onError={onError}
        />
      )}
    </div>
  );
}

function AnalysisEditor({
  id,
  music,
  position,
  onLibrary,
  onError,
}: {
  id: number;
  music: MusicAnalysis;
  position: number;
  onLibrary: (l: Library) => void;
  onError: (e: unknown) => void;
}) {
  const [busy, setBusy] = useState(false);
  const [bpm, setBpm] = useState('');
  const [offset, setOffset] = useState('');
  const [key, setKey] = useState('');
  const reset = () => {
    setBpm(music.bpm?.toFixed(3) ?? '');
    setOffset(music.beatOffset.toFixed(4));
    setKey(music.key ? `${music.key.tonic}:${music.key.minor ? 1 : 0}` : '');
  };
  const run = async (command: string, args: Record<string, unknown>) => {
    setBusy(true);
    try {
      onLibrary(await call<Library>(command, { id, ...args }));
    } catch (e) {
      onError(e);
    } finally {
      setBusy(false);
    }
  };
  return (
    <details
      className="analysis-editor"
      key={id}
      onToggle={(e) => {
        if (e.currentTarget.open) reset();
      }}
    >
      <summary>
        {tr('beatgrid / análisis ·', 'beatgrid / analysis ·')}{' '}
        {music.manual ? 'manual' : tr('estimado', 'estimated')}
        {!music.manual && music.bpm
          ? ` · ${tr('periodicidad', 'periodicity')} ${Math.round(music.bpmConfidence * 100)}%`
          : ''}
      </summary>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          const [tonic, minor] = key.split(':').map(Number);
          void run('save_track_music', {
            music: {
              ...music,
              bpm: bpm ? +bpm : null,
              beatOffset: +offset,
              key: key ? { tonic, minor: !!minor } : null,
              manual: true,
            },
          });
        }}
      >
        <label>
          {tr('BPM original', 'Original BPM')}{' '}
          <input
            type="number"
            min="40"
            max="300"
            step="0.001"
            value={bpm}
            onChange={(e) => setBpm(e.target.value)}
          />
        </label>
        <button type="button" disabled={!bpm} onClick={() => setBpm((+bpm / 2).toFixed(3))}>
          ÷2
        </button>
        <button type="button" disabled={!bpm} onClick={() => setBpm((+bpm * 2).toFixed(3))}>
          ×2
        </button>
        <label>
          {tr('Offset beat (s)', 'Beat offset (s)')}{' '}
          <input
            type="number"
            min="0"
            step="0.0001"
            required
            value={offset}
            onChange={(e) => setOffset(e.target.value)}
          />
        </label>
        <button type="button" onClick={() => setOffset(position.toFixed(4))}>
          {tr('beat aquí', 'beat here')}{' '}
        </button>
        <label>
          {tr('Tonalidad', 'Key')}{' '}
          <select value={key} onChange={(e) => setKey(e.target.value)}>
            <option value="">{tr('desconocida', 'unknown')}</option>
            {[false, true].flatMap((minor) =>
              NOTES.map((_, tonic) => (
                <option key={`${tonic}:${minor}`} value={`${tonic}:${minor ? 1 : 0}`}>
                  {keyLabel({ tonic, minor })} · {keyLabel({ tonic, minor }, 'camelot')}
                </option>
              )),
            )}
          </select>
        </label>
        <button type="submit" disabled={busy}>
          {tr('guardar', 'save')}{' '}
        </button>
        <button
          type="button"
          disabled={busy}
          title={tr(
            'Recalcula y reemplaza las correcciones manuales',
            'Recalculate and replace manual corrections',
          )}
          onClick={() => void run('analyze_track', {})}
        >
          {busy ? tr('analizando…', 'analyzing…') : tr('reanalizar', 'reanalyze')}
        </button>
      </form>
    </details>
  );
}
