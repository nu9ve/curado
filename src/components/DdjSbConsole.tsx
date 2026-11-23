import { useState } from 'react';
import type { CSSProperties } from 'react';
import { ChevronLeft, ChevronRight, Pause, Play } from 'lucide-react';
import { control, parameter, useSnapshot } from '../lib/bridge';
import { BEAT_FX, COLOR_FX_LABELS, samplePadIntent } from '../lib/console-pads';
import { cueColor, useCueInfo } from '../lib/cues';
import { DDJ_AUTO_LOOP_BEATS, DDJ_MANUAL_PADS, DDJ_PAD_MODES } from '../lib/ddj-sb-pads';
import type { DdjPadMode } from '../lib/ddj-sb-pads';
import { actionKey } from '../lib/feedback';
import { tr } from '../lib/language';
import { EMPTY_SLOT } from '../lib/types';
import type { BeatFxState, Slot, Track } from '../lib/types';
import { Knob, Meter, Slider } from './Controls';
import { Platter } from './Deck';
import './ddj-sb-console.css';

const act = (slot: number, kind: string, value = 0) => actionKey({ slot, kind, value });
const FILTER = COLOR_FX_LABELS.indexOf('FILTER') + 1;

type Load = (slot: number, id: number) => void;
type ReportError = (error: unknown) => void;

function DdjPads({
  index,
  slot,
  empty,
  slots,
  loading,
  selected,
  shift,
  mode,
  onLoad,
  onError,
}: {
  index: number;
  slot: Slot;
  empty: boolean;
  slots: Slot[];
  loading: number[];
  selected: number | null;
  shift: boolean;
  mode: DdjPadMode;
  onLoad: Load;
  onError: ReportError;
}) {
  const cueInfo = useCueInfo(slot.id || undefined);
  const send = (kind: string, value = 0, target = index) =>
    control(target, kind, value).catch(onError);
  return (
    <div className="sb-pads">
      {DDJ_AUTO_LOOP_BEATS.map((beats, pad) => {
        if (mode === 'cue') {
          const saved = slot.cues[pad] > 0;
          const kind = saved ? 'cue' : 'setCue';
          const name = String.fromCharCode(65 + pad);
          return (
            <button
              key={pad}
              type="button"
              disabled={empty || (shift && !saved)}
              data-action={
                shift
                  ? act(index, 'clearCue', pad)
                  : `${act(index, 'cue', pad)} ${act(index, 'setCue', pad)}`
              }
              data-set={saved}
              aria-label={`${shift ? tr('Borrar hot cue', 'Clear hot cue') : 'Hot Cue'} ${name}`}
              title={cueInfo.hot[pad]?.label ?? undefined}
              style={{ '--cue-color': cueColor(cueInfo.hot[pad]) } as CSSProperties}
              onClick={(event) => send(shift || event.shiftKey ? 'clearCue' : kind, pad)}
            >
              {shift ? `× ${name}` : name}
            </button>
          );
        }
        if (mode === 'auto') {
          const kind = shift ? 'autoLoop' : 'toggleLoopBeats';
          const value = shift ? 0 : beats;
          const active = slot.looping && (shift || slot.loopBeats === beats);
          return (
            <button
              key={pad}
              type="button"
              disabled={empty || (!slot.music.bpm && !active)}
              aria-pressed={active}
              aria-label={shift ? 'Auto Loop' : `Auto Loop · ${beats} beats`}
              data-action={act(index, kind, value)}
              onClick={(event) => {
                if (event.shiftKey) send('autoLoop');
                else send(kind, value);
              }}
            >
              {shift ? (slot.looping ? 'EXIT' : 'LOOP') : beats}
            </button>
          );
        }
        if (mode === 'manual') {
          const padAction = DDJ_MANUAL_PADS[shift ? 'shifted' : 'primary'][pad];
          return (
            <button
              key={pad}
              type="button"
              disabled={empty || !padAction}
              aria-label={padAction ? undefined : tr('Pad sin acción', 'Unassigned pad')}
              aria-pressed={padAction?.kind === 'loop' ? slot.looping : undefined}
              data-action={padAction ? act(index, padAction.kind, padAction.value) : undefined}
              onClick={(event) => {
                const action = event.shiftKey ? DDJ_MANUAL_PADS.shifted[pad] : padAction;
                if (action) send(action.kind, action.value);
              }}
            >
              {padAction?.label ?? '—'}
            </button>
          );
        }
        const sampleSlot = 2 + index * 4 + pad;
        const sample = slots[sampleSlot];
        const busy = loading.includes(sampleSlot);
        const intent = samplePadIntent(sampleSlot - 2, !!sample?.id, selected, shift, busy);
        return (
          <button
            key={pad}
            type="button"
            disabled={!intent}
            data-action={act(sampleSlot, 'trigger')}
            data-set={!!sample?.id}
            aria-pressed={!!sample?.playing}
            title={
              shift || !sample?.id
                ? tr('Cargar la selección en el pad', 'Load selection into pad')
                : tr('Disparar muestra', 'Trigger sample')
            }
            onClick={(event) => {
              const next = event.shiftKey
                ? samplePadIntent(sampleSlot - 2, !!sample?.id, selected, true, busy)
                : intent;
              if (next?.kind === 'load') onLoad(next.slot, next.id);
              if (next?.kind === 'trigger') send('trigger', 0, next.slot);
            }}
          >
            S{sampleSlot - 1}
            {busy ? '…' : ''}
          </button>
        );
      })}
    </div>
  );
}

function DdjDeck({
  index,
  track,
  masterDeck,
  masterHasBpm,
  tempoRange,
  tempoPickup,
  slots,
  fx,
  loading,
  selected,
  onLoad,
  onError,
}: {
  index: number;
  track?: Track;
  masterDeck: number;
  masterHasBpm: boolean;
  tempoRange: number;
  tempoPickup: boolean;
  slots: Slot[];
  fx?: BeatFxState;
  loading: number[];
  selected: number | null;
  onLoad: Load;
  onError: ReportError;
}) {
  const slot = slots[index] ?? EMPTY_SLOT;
  const name = index ? 'B' : 'A';
  const tone = index ? 'b' : 'a';
  const busy = loading.includes(index);
  const empty = !slot.id || busy;
  const [mode, setMode] = useState<DdjPadMode>('cue');
  const [shift, setShift] = useState(false);
  const send = (kind: string, value = 0) => control(index, kind, value).catch(onError);
  const outside = Math.abs(slot.tuning.rate - 1) > tempoRange + 0.000001;
  const rangeLabel = `±${Math.round(tempoRange * 100)}%`;
  const fxToggle = (kind: string, label: string, on: boolean, disabled = false) => (
    <button
      type="button"
      disabled={disabled}
      aria-pressed={on}
      data-action={act(index, kind, 1)}
      onClick={() => send(kind, on ? 0 : 1)}
    >
      {label}
    </button>
  );
  return (
    <section className={`sb-deck tone-${tone}`} aria-label={`Deck ${name}`}>
      <header className="sb-track">
        <b className="deck-tag">{name}</b>
        <strong title={track?.title}>
          {busy ? tr('Cargando…', 'Loading…') : (track?.title ?? tr('Sin pista', 'No track'))}
        </strong>
        <button
          type="button"
          disabled={empty}
          aria-pressed={masterDeck === index}
          data-action={act(index, 'masterDeck')}
          title={tr('Referencia de sincronización', 'Sync reference')}
          onClick={() => send('masterDeck')}
        >
          MASTER
        </button>
      </header>
      <div className="sb-fx" role="group" aria-label={`FX · Deck ${name}`}>
        {fxToggle('beatFxOn', fx ? BEAT_FX[fx.kind] : 'FX', !!fx?.on, !fx)}
        {fxToggle('slip', 'SLIP', slot.slip, empty)}
        {fxToggle('reverse', 'REV', slot.reverse, empty)}
        <Knob
          label={tr('Nivel FX', 'FX level')}
          value={fx?.depth ?? 0.5}
          action={act(index, 'beatFxDepth')}
          tone={tone}
          onChange={(value) => parameter(index, 'beatFxDepth', value, onError)}
        />
      </div>
      <div className="sb-platter-row">
        <Platter
          slot={slot}
          index={index}
          disabled={empty}
          onToggle={() => send('toggle')}
          onError={onError}
        />
        <div className="sb-tempo">
          <button
            type="button"
            disabled={empty && !shift}
            aria-pressed={shift ? undefined : slot.tuning.keyLock}
            data-action={act(index, shift ? 'tempoRange' : 'keyLock', shift ? 0 : 1)}
            onClick={(event) => {
              if (shift || event.shiftKey) send('tempoRange');
              else send('keyLock', slot.tuning.keyLock ? 0 : 1);
            }}
          >
            {shift ? 'RANGE' : 'KEY LOCK'}
          </button>
          <button
            type="button"
            data-action={act(index, 'tempoRange')}
            title={tr('Cambiar rango de tempo', 'Cycle tempo range')}
            aria-label={`${tr('Rango de tempo', 'Tempo range')} · ${rangeLabel}`}
            onClick={() => send('tempoRange')}
          >
            {rangeLabel}
          </button>
          <Slider
            anchored
            label="TEMPO"
            action={act(index, 'rate')}
            tone={tone}
            min={1 - tempoRange}
            max={1 + tempoRange}
            origin={1}
            defaultValue={1}
            value={slot.tuning.rate}
            format={(value) => `${value >= 1 ? '+' : ''}${((value - 1) * 100).toFixed(1)}%`}
            onChange={(value) => parameter(index, 'rate', value, onError)}
          />
          <small className="sb-tempo-status">
            {outside
              ? tr('Fuera de rango', 'Out of range')
              : tempoPickup
                ? tr('Cruza tempo', 'Match tempo')
                : null}
          </small>
        </div>
      </div>
      <div className="sb-pad-modes" role="group" aria-label={tr('Modo de pads', 'Pad mode')}>
        {DDJ_PAD_MODES.map(([value, label]) => (
          <button
            key={value}
            type="button"
            aria-pressed={mode === value}
            onClick={() => setMode(value)}
          >
            {label}
          </button>
        ))}
      </div>
      <DdjPads
        {...{ index, slot, empty, slots, loading, selected, shift, mode, onLoad, onError }}
      />
      <div
        className="sb-transport"
        role="group"
        aria-label={`Deck ${name} · ${tr('Transporte', 'Transport')}`}
      >
        <button
          type="button"
          disabled={empty}
          aria-pressed={slot.playing}
          aria-label={slot.playing ? tr('Pausar', 'Pause') : tr('Reproducir', 'Play')}
          data-action={act(index, 'toggle')}
          onClick={() => send('toggle')}
        >
          {slot.playing ? <Pause size={15} /> : <Play size={15} />}
        </button>
        <button
          type="button"
          disabled={empty}
          title={tr('Salta al hot cue A y reproduce', 'Jumps to hot cue A and plays')}
          data-action={act(index, 'cue')}
          onClick={() => send('cue')}
        >
          CUE
        </button>
        <button
          type="button"
          disabled={
            empty || (!slot.sync && (masterDeck === index || !slot.music.bpm || !masterHasBpm))
          }
          aria-pressed={slot.sync}
          data-action={act(index, 'sync', 1)}
          onClick={() => send('sync', slot.sync ? 0 : 1)}
        >
          SYNC
        </button>
        <button
          type="button"
          aria-pressed={shift}
          title={tr(
            'Activa las funciones secundarias en pantalla',
            'Toggles the on-screen secondary functions',
          )}
          onClick={() => setShift(!shift)}
        >
          SHIFT
        </button>
      </div>
    </section>
  );
}

function DdjChannel({
  index,
  slot,
  hasCue,
  onError,
}: {
  index: number;
  slot: Slot;
  hasCue: boolean;
  onError: ReportError;
}) {
  const tone = index ? 'b' : 'a';
  const filterOn = slot.colorFx === FILTER;
  const send = (kind: string) => (value: number) => parameter(index, kind, value, onError);
  return (
    <div className={`sb-channel tone-${tone}`} role="group" aria-label={`CH ${index + 1}`}>
      {(['high', 'mid', 'low'] as const).map((kind) => (
        <Knob
          key={kind}
          label={kind === 'high' ? 'HI' : kind.toUpperCase()}
          action={act(index, kind)}
          tone={tone}
          max={2}
          origin={1}
          defaultValue={1}
          value={slot[kind]}
          format={(value) => (value > 0 ? `${(20 * Math.log10(value)).toFixed(0)} dB` : '−∞')}
          onChange={send(kind)}
        />
      ))}
      <div className="sb-filter">
        <Knob
          label={filterOn ? 'LP ↔ HP' : (COLOR_FX_LABELS[slot.colorFx - 1] ?? 'COLOR')}
          action={act(index, 'color')}
          tone={tone}
          min={-1}
          max={1}
          origin={0}
          defaultValue={0}
          value={slot.color}
          format={(value) => (value === 0 ? tr('centro', 'center') : value.toFixed(2))}
          onChange={send('color')}
        />
        <button
          type="button"
          aria-pressed={filterOn}
          data-action={act(index, 'colorFx', FILTER)}
          onClick={() => control(index, 'colorFx', filterOn ? 0 : FILTER).catch(onError)}
        >
          FILTER
        </button>
      </div>
      <button
        type="button"
        disabled={!hasCue}
        aria-pressed={slot.cue}
        aria-label={`CH ${index + 1} · ${tr('Cue de audífonos', 'Headphone cue')}`}
        title={
          hasCue ? undefined : tr('Requiere una salida de 4 canales', 'Requires a 4-channel output')
        }
        data-action={act(index, 'channelCue', 1)}
        onClick={() => control(index, 'channelCue', slot.cue ? 0 : 1).catch(onError)}
      >
        CUE
      </button>
      <div className="sb-level">
        <Slider
          label={`CH ${index + 1}`}
          action={act(index, 'gain')}
          tone={tone}
          max={1.5}
          defaultValue={0.8}
          value={slot.gain}
          onChange={send('gain')}
        />
        <Meter vertical value={slot.peak} label={`CH ${index + 1} · ${tr('Nivel', 'Level')}`} />
      </div>
    </div>
  );
}

export function DdjSbConsole({
  tracks,
  loading,
  sampleSelection,
  browseSelection,
  onLoad,
  onLoadChosen,
  onBrowse,
  onOpenLibrary,
  onError,
}: {
  tracks: Track[];
  loading: number[];
  sampleSelection: number | null;
  browseSelection?: string;
  onLoad: Load;
  onLoadChosen: (slot: number) => void;
  onBrowse: (delta: number) => void;
  onOpenLibrary: () => void;
  onError: ReportError;
}) {
  const { audio, output, midi, tempoRanges } = useSnapshot();
  const slots = audio?.slots ?? [];
  const hasCue = (output?.channels ?? 2) >= 4;
  const masterDeck = audio?.masterDeck ?? 0;
  const deck = (index: number) => (
    <DdjDeck
      index={index}
      track={tracks.find((track) => track.id === slots[index]?.id)}
      masterDeck={masterDeck}
      masterHasBpm={!!slots[masterDeck]?.music.bpm}
      tempoRange={tempoRanges?.[index] ?? 0.1}
      tempoPickup={!!midi.tempoPickup?.[index]}
      fx={audio?.beatFx?.[index]}
      selected={sampleSelection}
      {...{ slots, loading, onLoad, onError }}
    />
  );
  return (
    <section className="sb-console" aria-label={tr('Cabina DDJ-SB', 'DDJ-SB console')}>
      <div
        className="sb-browse"
        role="group"
        aria-label={tr('Explorar y cargar', 'Browse and load')}
      >
        <button
          type="button"
          disabled={browseSelection === undefined || loading.includes(0)}
          data-action={act(0, 'listLoad')}
          onClick={() => onLoadChosen(0)}
        >
          LOAD A
        </button>
        <button
          type="button"
          aria-label={tr('Pista anterior en la lista', 'Previous track in list')}
          data-action={act(0, 'listMove', -1)}
          onClick={() => onBrowse(-1)}
        >
          <ChevronLeft size={16} />
        </button>
        <button
          type="button"
          className="sb-selection"
          title={browseSelection}
          onClick={onOpenLibrary}
        >
          {browseSelection ?? tr('Explorar biblioteca', 'Browse library')}
        </button>
        <button
          type="button"
          aria-label={tr('Pista siguiente en la lista', 'Next track in list')}
          data-action={act(0, 'listMove', 1)}
          onClick={() => onBrowse(1)}
        >
          <ChevronRight size={16} />
        </button>
        <button
          type="button"
          disabled={browseSelection === undefined || loading.includes(1)}
          data-action={act(1, 'listLoad')}
          onClick={() => onLoadChosen(1)}
        >
          LOAD B
        </button>
      </div>
      <div className="sb-surface">
        {deck(0)}
        <section className="sb-mixer" aria-label={tr('Mezclador', 'Mixer')}>
          <div className="sb-channels">
            {[0, 1].map((index) => (
              <DdjChannel
                key={index}
                index={index}
                slot={slots[index] ?? EMPTY_SLOT}
                hasCue={hasCue}
                onError={onError}
              />
            ))}
          </div>
          <Slider
            vertical={false}
            label="CROSSFADER"
            action={[0, 0.5, 1].map((value) => act(0, 'crossfader', value)).join(' ')}
            defaultValue={0.5}
            origin={0.5}
            value={audio?.crossfader ?? 0.5}
            onChange={(value) => parameter(0, 'crossfader', value, onError)}
          />
          <Meter value={audio?.peak ?? 0} label={tr('Nivel master', 'Master level')} />
          <div className="sb-output">
            <Knob
              label="MASTER"
              action={act(0, 'master')}
              value={audio?.master ?? 0.8}
              onChange={(value) => parameter(0, 'master', value, onError)}
            />
            {hasCue && (
              <Knob
                label="CUE MIX"
                action={act(0, 'cueMix')}
                value={audio?.cueMix ?? 0}
                onChange={(value) => parameter(0, 'cueMix', value, onError)}
              />
            )}
          </div>
          {hasCue && (
            <button
              type="button"
              aria-pressed={!!audio?.cueMaster}
              data-action={act(0, 'masterCue', 1)}
              title={tr(
                'Escuchar solo el master en audífonos',
                'Hear only the master in headphones',
              )}
              onClick={() => control(0, 'masterCue', audio?.cueMaster ? 0 : 1).catch(onError)}
            >
              MASTER CUE
            </button>
          )}
        </section>
        {deck(1)}
      </div>
    </section>
  );
}
