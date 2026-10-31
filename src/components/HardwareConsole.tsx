import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import type { CSSProperties } from 'react';
import { Play, Pause } from 'lucide-react';
import { control, parameter, useSnapshot } from '../lib/bridge';
import { actionKey } from '../lib/feedback';
import { tr } from '../lib/language';
import { consoleScale } from '../lib/console-fit';
import {
  angleDelta,
  BEAT_FX,
  beatLabel,
  COLOR_FX,
  JUMP_PADS,
  jogSeconds,
  LOOP_PADS,
  nextDivision,
  nextTempoRange,
  ROLL_PADS,
  SAMPLE_PADS,
  samplePadIntent,
  tapTempo,
} from '../lib/console-pads';
import type { PadMode } from '../lib/console-pads';
import { cueColor, useCueInfo } from '../lib/cues';
import { EMPTY_SLOT, remaining } from '../lib/types';
import type { BeatFxState, KeyBinding, Library, LibraryEdit, Slot, Track } from '../lib/types';
import { Knob, LimiterLamp, Meter, Slider } from './Controls';
import { keyFor } from './Deck';
import { Mixer } from './Mixer';
import { MusicalControls } from './MusicalControls';
import { KeyboardMap } from './KeyboardMap';
import { useHud } from '../lib/feedback';
import { ZoomWave } from './Waveform';
import { UnavailableControl } from './UnavailableControl';
import './hardware-console.css';

const act = (slot: number, kind: string, value = 0) => actionKey({ slot, kind, value });
const eqValue = (v: number) => (v > 0 ? `${(20 * Math.log10(v)).toFixed(0)} dB` : '−∞');
const padName = (index: number) => String.fromCharCode(65 + index);

/** A platter drag: scratch, or a pitch bend while playing with vinyl mode off. */
function useJog(
  index: number,
  playing: boolean,
  vinyl: boolean,
  sensitivity: number,
  onError: (e: unknown) => void,
) {
  const drag = useRef<{
    angle: number;
    moved: boolean;
    touching: boolean;
    pending: number;
    frame: number;
  } | null>(null);
  const moved = useRef(false);
  const angleOf = (element: Element, x: number, y: number) => {
    const bounds = element.getBoundingClientRect();
    return Math.atan2(y - bounds.top - bounds.height / 2, x - bounds.left - bounds.width / 2);
  };
  // One message per frame at most, whatever rate the pointer reports.
  const flush = () => {
    const state = drag.current;
    if (!state) return;
    state.frame = 0;
    if (state.touching) control(index, 'jogMove', state.pending).catch(onError);
    else control(index, 'jogBend', Math.max(-0.5, Math.min(0.5, state.pending * 2))).catch(onError);
    state.pending = 0;
  };
  useEffect(() => () => cancelAnimationFrame(drag.current?.frame ?? 0), []);
  return {
    wasDragged: () => moved.current,
    handlers: {
      onPointerDown: (e: React.PointerEvent<HTMLButtonElement>) => {
        e.currentTarget.setPointerCapture(e.pointerId);
        moved.current = false;
        drag.current = {
          angle: angleOf(e.currentTarget, e.clientX, e.clientY),
          moved: false,
          touching: false,
          pending: 0,
          frame: 0,
        };
      },
      onPointerMove: (e: React.PointerEvent<HTMLButtonElement>) => {
        const state = drag.current;
        if (!state) return;
        const angle = angleOf(e.currentTarget, e.clientX, e.clientY);
        const delta = angleDelta(state.angle, angle);
        if (!state.moved && Math.abs(delta) < 0.05) return;
        state.angle = angle;
        if (!state.moved) {
          state.moved = true;
          moved.current = true;
          state.touching = vinyl || !playing;
          if (state.touching) control(index, 'jogTouch', 1).catch(onError);
        }
        state.pending += jogSeconds(delta, sensitivity);
        state.frame ||= requestAnimationFrame(flush);
      },
      onPointerUp: () => {
        const state = drag.current;
        if (!state) return;
        cancelAnimationFrame(state.frame);
        drag.current = null;
        if (!state.moved) return;
        if (state.touching) {
          if (state.pending) control(index, 'jogMove', state.pending).catch(onError);
          control(index, 'jogTouch', 0).catch(onError);
        } else control(index, 'jogBend', 0).catch(onError);
      },
    },
  };
}

/** A visible, focusable reference control, never an audio action. */
function Soon({ children, dial = false }: { children: string; dial?: boolean }) {
  return <UnavailableControl label={children} dial={dial} />;
}

function RxDeck({
  index,
  slot,
  track,
  keys,
  loading,
  masterDeck,
  masterHasBpm,
  onError,
  samples,
  tracks,
  selected,
  onLoad,
  loadingSlots,
}: {
  index: number;
  slot: Slot;
  track?: Track;
  keys: KeyBinding[];
  loading: boolean;
  masterDeck: number;
  masterHasBpm: boolean;
  onError: (e: unknown) => void;
  samples: Slot[];
  tracks: Track[];
  selected: number | null;
  onLoad: (slot: number, id: number) => void;
  loadingSlots: number[];
}) {
  const name = index ? 'B' : 'A';
  const tone = index ? 'b' : 'a';
  const empty = !track || loading;
  const [padMode, setPadMode] = useState<PadMode>('cue');
  const [vinyl, setVinyl] = useState(true);
  const [sensitivity, setSensitivity] = useState(1);
  const [tempoRange, setTempoRange] = useState(0.5);
  const jog = useJog(index, slot.playing, vinyl, sensitivity, onError);
  const cueInfo = useCueInfo(slot.id || undefined);
  const send = (kind: string, value = 0) => control(index, kind, value).catch(onError);
  return (
    <section className={`rx-deck rx-deck-${index} tone-${tone}`} aria-label={`Deck ${name}`}>
      <div className="rx-deck-top">
        <div className="rx-deck-label">
          DECK {name}
          <small>{tr('REPRODUCTOR', 'PLAYER')}</small>
        </div>
        <div className="rx-mini-buttons">
          <button
            disabled={empty}
            aria-pressed={masterDeck === index}
            data-action={act(index, 'masterDeck')}
            onClick={() => send('masterDeck')}
          >
            MASTER
          </button>
          <button
            disabled={empty || masterDeck === index || !slot.music.bpm || !masterHasBpm}
            aria-pressed={slot.sync}
            data-action={act(index, 'sync', slot.sync ? 0 : 1)}
            onClick={() => send('sync', slot.sync ? 0 : 1)}
          >
            SYNC
          </button>
          <button
            disabled={empty}
            aria-pressed={slot.reverse}
            data-action={act(index, 'reverse', slot.reverse ? 0 : 1)}
            onClick={() => send('reverse', slot.reverse ? 0 : 1)}
          >
            REVERSE
          </button>
          <button
            disabled={empty}
            aria-pressed={slot.slip}
            data-action={act(index, 'slip', slot.slip ? 0 : 1)}
            onClick={() => send('slip', slot.slip ? 0 : 1)}
          >
            SLIP
          </button>
          <button
            type="button"
            aria-pressed={vinyl}
            title={tr(
              'Plato: scratch encendido; apagado dobla el tempo al arrastrar',
              'Platter: on scratches; off bends the tempo when dragged',
            )}
            onClick={() => setVinyl(!vinyl)}
          >
            VINYL
          </button>
        </div>
      </div>
      <div className="rx-loop-panel">
        <span>LOOP</span>
        <button disabled={empty} data-action={act(index, 'loopIn')} onClick={() => send('loopIn')}>
          IN
        </button>
        <button
          disabled={empty}
          data-action={act(index, 'loopOut')}
          onClick={() => send('loopOut')}
        >
          OUT
        </button>
        <button
          disabled={empty}
          aria-pressed={slot.looping}
          data-action={act(index, 'loop')}
          onClick={() => send('loop')}
        >
          LOOP
        </button>
        <button
          disabled={empty || slot.loopOut <= slot.loopIn}
          data-action={act(index, 'loopScale', 0.5)}
          onClick={() => send('loopScale', 0.5)}
        >
          1/2X
        </button>
        <button
          disabled={empty || slot.loopOut <= slot.loopIn}
          data-action={act(index, 'loopScale', 2)}
          onClick={() => send('loopScale', 2)}
        >
          2X
        </button>
      </div>
      <div className="rx-jog-area">
        <div className="rx-jog-side">
          <Knob
            label="BRAKE"
            tone={tone}
            action={act(index, 'brake')}
            max={3}
            value={slot.brake}
            format={(v) => (v < 0.01 ? 'OFF' : `${v.toFixed(1)} s`)}
            onChange={(v) => parameter(index, 'brake', v, onError)}
          />
          <Knob
            label="JOG"
            tone={tone}
            min={0.25}
            max={2}
            defaultValue={1}
            origin={1}
            value={sensitivity}
            format={(v) => `×${v.toFixed(2)}`}
            onChange={setSensitivity}
          />
        </div>
        <button
          type="button"
          className="rx-jog"
          disabled={empty}
          aria-label={tr(
            `Plato deck ${name}: clic reproduce o pausa; arrastrar mueve la pista`,
            `Deck ${name} platter: click plays or pauses; dragging moves the track`,
          )}
          title={tr(
            'Clic: reproducir o pausar · arrastrar en círculo: scratch o doblar el tempo',
            'Click: play or pause · drag around: scratch or bend the tempo',
          )}
          data-action={act(index, 'toggle')}
          {...jog.handlers}
          onPointerCancel={jog.handlers.onPointerUp}
          onClick={() => {
            if (!jog.wasDragged()) send('toggle');
          }}
        >
          <span
            className="rx-jog-rotor"
            style={{ '--turn': `${slot.position * 200}deg` } as CSSProperties}
          >
            <i />
          </span>
          <span className="rx-jog-screen">
            <b>{name}</b>
            <small>{slot.playing ? '▶' : 'Ⅱ'}</small>
            <em>{remaining(slot)}</em>
          </span>
        </button>
        <div className="rx-tempo">
          <button
            disabled={empty}
            aria-pressed={slot.tuning.keyLock}
            title={tr(
              'Key lock de curado: el tono se conserva al cambiar el tempo',
              'curado key lock: pitch holds when the tempo changes',
            )}
            data-action={act(index, 'keyLock', slot.tuning.keyLock ? 0 : 1)}
            onClick={() => send('keyLock', slot.tuning.keyLock ? 0 : 1)}
          >
            MASTER TEMPO
          </button>
          <button
            type="button"
            title={tr('Rango del fader de tempo', 'Tempo fader range')}
            onClick={() => setTempoRange(nextTempoRange(tempoRange))}
          >
            ±{Math.round(tempoRange * 100)} %
          </button>
          <Slider
            label="TEMPO"
            action={act(index, 'rate')}
            tone={tone}
            min={1 - tempoRange}
            max={1 + tempoRange}
            origin={1}
            defaultValue={1}
            value={slot.tuning.rate}
            format={(v) => `${((v - 1) * 100).toFixed(1)}%`}
            onChange={(v) => parameter(index, 'rate', v, onError)}
          />
        </div>
      </div>
      <div className="rx-lower">
        <div className="rx-transport">
          <div className="rx-search">
            <button
              disabled={empty}
              aria-label={tr('Nudge atrás', 'Nudge backward')}
              data-action={act(index, 'nudge', -0.05)}
              onClick={() => send('nudge', -0.05)}
            >
              ◀
            </button>
            <button
              disabled={empty}
              aria-label={tr('Nudge adelante', 'Nudge forward')}
              data-action={act(index, 'nudge', 0.05)}
              onClick={() => send('nudge', 0.05)}
            >
              ▶
            </button>
          </div>
          <button
            className="rx-cue"
            disabled={empty}
            title={tr('Salta al hot cue 1 y reproduce', 'Jump to hot cue 1 and play')}
            data-action={act(index, 'cue', 0)}
            onClick={() => send('cue', 0)}
          >
            CUE {keyFor(keys, index, 'cue', 0)}
          </button>
          <button
            className="rx-play"
            disabled={empty}
            aria-label={`${slot.playing ? tr('Pausar', 'Pause') : tr('Reproducir', 'Play')} deck ${name}`}
            data-action={act(index, 'toggle')}
            onClick={() => send('toggle')}
          >
            {slot.playing ? <Pause size={18} /> : <Play size={18} />}{' '}
            {keyFor(keys, index, 'toggle')}
          </button>
        </div>
        <div className="rx-pad-section">
          <div className="rx-pad-modes">
            {(
              [
                ['cue', 'HOT CUE'],
                ['loop', 'BEAT LOOP'],
                ['jump', 'BEAT JUMP'],
                ['roll', 'ROLL'],
                ['samples', 'SAMPLER'],
              ] as const
            ).map(([mode, label]) => (
              <button
                key={mode}
                type="button"
                aria-pressed={padMode === mode}
                onClick={() => setPadMode(mode)}
              >
                {label}
              </button>
            ))}
          </div>
          <div className="rx-pads">
            {padMode === 'samples' &&
              samples.map((sample, cue) => {
                const sampleSlot = cue + 2;
                const sampleTrack = tracks.find((t) => t.id === sample.id);
                const instruction = tr(
                  'Clic: reproducir · Alt + clic: detener · Shift + clic: cargar selección',
                  'Click: play · Alt-click: stop · Shift-click: load selection',
                );
                return (
                  <button
                    key={cue}
                    className={`rx-pad rx-sample-pad ${sample.playing ? 'is-set' : ''}`}
                    disabled={
                      loadingSlots.includes(sampleSlot) || (!sampleTrack && selected === null)
                    }
                    aria-label={`${tr('Sample', 'Sample')} ${cue + 1} · ${sampleTrack?.title ?? tr('vacío', 'empty')} · ${instruction}`}
                    title={`${sampleTrack?.title ?? tr('Vacío', 'Empty')} · ${instruction}`}
                    data-action={act(sampleSlot, 'trigger')}
                    onClick={(e) => {
                      if (e.altKey) {
                        control(sampleSlot, 'pause').catch(onError);
                        return;
                      }
                      const intent = samplePadIntent(
                        cue,
                        !!sampleTrack,
                        selected,
                        e.shiftKey,
                        loadingSlots.includes(sampleSlot),
                      );
                      if (intent?.kind === 'load') onLoad(intent.slot, intent.id);
                      else if (intent?.kind === 'trigger')
                        control(intent.slot, 'trigger').catch(onError);
                    }}
                  >
                    <span>S{cue + 1}</span>
                    {keyFor(keys, sampleSlot, 'trigger')}
                    <small>
                      {loadingSlots.includes(sampleSlot)
                        ? tr('cargando', 'loading')
                        : (sampleTrack?.title ?? tr('vacío', 'empty'))}
                    </small>
                  </button>
                );
              })}
            {padMode === 'cue' &&
              Array.from({ length: 8 }, (_, cue) => (
                <button
                  key={cue}
                  className={`rx-pad ${slot.cues[cue] > 0 ? 'is-set' : ''} ${slot.cueEnds[cue] > 0 ? 'is-loop' : ''}`}
                  style={{ '--cue-color': cueColor(cueInfo.hot[cue]) } as CSSProperties}
                  disabled={empty}
                  title={
                    cueInfo.hot[cue]?.label ??
                    tr(
                      'Clic: saltar y reproducir · Shift + clic: guardar',
                      'Click: jump and play · Shift-click: save',
                    )
                  }
                  aria-label={`Deck ${name} · Hot cue ${padName(cue)}${cueInfo.hot[cue]?.label ? ` · ${cueInfo.hot[cue]?.label}` : ''}`}
                  data-action={`${act(index, 'cue', cue)} ${act(index, 'setCue', cue)}`}
                  onClick={(e) => send(e.shiftKey ? 'setCue' : 'cue', cue)}
                >
                  <span>{padName(cue)}</span>
                  {keyFor(keys, index, 'cue', cue, true)}
                </button>
              ))}
            {padMode === 'loop' &&
              LOOP_PADS.map((beats) => {
                const active = slot.looping && slot.loopBeats === beats;
                return (
                  <button
                    key={beats}
                    className={`rx-pad ${active ? 'is-set' : ''}`}
                    disabled={empty}
                    aria-pressed={active}
                    aria-label={`Deck ${name} · ${tr('Loop de', 'Loop of')} ${beatLabel(beats)} ${tr('beats', 'beats')}`}
                    data-action={act(index, 'loopBeats', beats)}
                    onClick={() => (active ? send('loop') : send('loopBeats', beats))}
                  >
                    <span>{beatLabel(beats)}</span>
                  </button>
                );
              })}
            {padMode === 'jump' &&
              JUMP_PADS.map((beats) => (
                <button
                  key={beats}
                  className="rx-pad"
                  disabled={empty}
                  aria-label={`Deck ${name} · ${tr('Saltar', 'Jump')} ${beats} ${tr('beats', 'beats')}`}
                  data-action={act(index, 'beatJump', beats)}
                  onClick={() => send('beatJump', beats)}
                >
                  <span>{beats < 0 ? `◀ ${-beats}` : `${beats} ▶`}</span>
                </button>
              ))}
            {padMode === 'roll' &&
              ROLL_PADS.map((beats) => {
                const active = slot.rolling && slot.loopBeats === beats;
                const hold = () => send('roll', beats);
                const release = () => send('rollEnd');
                return (
                  <button
                    key={beats}
                    className={`rx-pad ${active ? 'is-set' : ''}`}
                    disabled={empty}
                    aria-label={`Deck ${name} · Roll ${beatLabel(beats)}`}
                    title={tr(
                      'Mantén: repite y al soltar vuelve donde iría la pista',
                      'Hold: repeats, and on release returns to where the track would be',
                    )}
                    data-action={act(index, 'roll', beats)}
                    onPointerDown={(e) => {
                      e.currentTarget.setPointerCapture(e.pointerId);
                      hold();
                    }}
                    onPointerUp={release}
                    onPointerCancel={release}
                    onKeyDown={(e) => {
                      if ((e.key === 'Enter' || e.key === ' ') && !e.repeat) {
                        e.preventDefault();
                        hold();
                      }
                    }}
                    onKeyUp={(e) => {
                      if (e.key === 'Enter' || e.key === ' ') release();
                    }}
                    onBlur={() => {
                      if (slot.rolling) release();
                    }}
                  >
                    <span>{beatLabel(beats)}</span>
                  </button>
                );
              })}
          </div>
          <div className="rx-pad-footer">
            PERFORMANCE PADS{' '}
            <small>
              {
                {
                  cue: tr('8 HOT CUES', '8 HOT CUES'),
                  loop: tr('LOOP EN BEATS', 'LOOP IN BEATS'),
                  jump: tr('SALTO EN BEATS', 'JUMP IN BEATS'),
                  roll: tr('MANTÉN PARA REPETIR', 'HOLD TO REPEAT'),
                  samples: tr('8 SAMPLES COMPARTIDOS', '8 SHARED SAMPLES'),
                }[padMode]
              }
            </small>
          </div>
          {padMode === 'samples' && (
            <div className="rx-sample-help">
              {tr(
                'Clic en vacío: cargar selección · ⇧ clic: reemplazar · ⌥ clic: detener',
                'Click empty: load selection · ⇧ click: replace · ⌥ click: stop',
              )}
            </div>
          )}
        </div>
      </div>
    </section>
  );
}

function RxChannel({
  index,
  slot,
  onError,
}: {
  index: number;
  slot: Slot;
  onError: (e: unknown) => void;
}) {
  const tone = index ? 'b' : 'a';
  const send = (kind: string) => (v: number) => parameter(index, kind, v, onError);
  return (
    <div
      className={`rx-channel tone-${tone}`}
      aria-label={`${tr('Canal', 'Channel')} ${index ? 'B' : 'A'}`}
    >
      <b>CH {index + 1}</b>
      <Soon>SOURCE</Soon>
      <Knob
        label="TRIM"
        tone={tone}
        action={act(index, 'trim')}
        max={2}
        origin={1}
        defaultValue={1}
        value={slot.trim}
        format={eqValue}
        onChange={send('trim')}
      />
      {(['high', 'mid', 'low'] as const).map((kind) => (
        <Knob
          key={kind}
          label={kind === 'high' ? 'HI' : kind.toUpperCase()}
          tone={tone}
          action={act(index, kind)}
          max={2}
          origin={1}
          defaultValue={1}
          value={slot[kind]}
          format={eqValue}
          onChange={send(kind)}
        />
      ))}
      <Knob
        label={slot.colorFx ? COLOR_FX[slot.colorFx - 1] : 'COLOR'}
        tone={tone}
        action={act(index, 'color')}
        min={-1}
        max={1}
        origin={0}
        defaultValue={0}
        value={slot.color}
        format={(v) => (Math.abs(v) < 0.01 ? tr('centro', 'center') : v.toFixed(2))}
        onChange={send('color')}
      />
      <Soon>CUE</Soon>
      <div className="rx-channel-level">
        <Meter vertical value={slot.peak} label={`${tr('Nivel', 'Level')} ${index ? 'B' : 'A'}`} />
        <Slider
          label="LEVEL"
          tone={tone}
          action={act(index, 'gain')}
          max={1.5}
          defaultValue={0.8}
          value={slot.gain}
          onChange={send('gain')}
        />
      </div>
    </div>
  );
}

/** Each channel picks its own color effect, so the two can differ. */
function ColorFxPanel({ slots, onError }: { slots: Slot[]; onError: (e: unknown) => void }) {
  return (
    <>
      <strong>SOUND COLOR FX</strong>
      {COLOR_FX.map((label, i) => (
        <div className="rx-fx-row" key={label}>
          <span>{label}</span>
          {slots.map((slot, index) => {
            const active = slot.colorFx === i + 1;
            return (
              <button
                key={index}
                className={`tone-${index ? 'b' : 'a'}`}
                aria-pressed={active}
                aria-label={`${label} · ${tr('canal', 'channel')} ${index + 1}`}
                data-action={act(index, 'colorFx', i + 1)}
                onClick={() => control(index, 'colorFx', active ? 0 : i + 1).catch(onError)}
              >
                {index + 1}
              </button>
            );
          })}
        </div>
      ))}
      <div className="rx-fx-params">
        {slots.map((slot, index) => (
          <Knob
            key={index}
            label={`PRM ${index + 1}`}
            tone={index ? 'b' : 'a'}
            action={act(index, 'colorParam')}
            defaultValue={0.5}
            origin={0.5}
            value={slot.colorParam}
            onChange={(v) => parameter(index, 'colorParam', v, onError)}
          />
        ))}
      </div>
    </>
  );
}

const UNIT_LABELS = ['CH 1', 'CH 2', 'MST'];
/** One panel over three independent units: switching what it shows changes nothing on the others. */
function BeatFxPanel({
  units,
  onError,
}: {
  units: BeatFxState[];
  onError: (e: unknown) => void;
}) {
  const [unit, setUnit] = useState(0);
  const taps = useRef<number[]>([]);
  const state = units[unit] ?? { kind: 1, on: false, beats: 1, depth: 0.5, bpm: 0 };
  const send = (kind: string, value: number) => control(unit, kind, value).catch(onError);
  return (
    <>
      <strong>BEAT FX</strong>
      <div className="rx-fx-units" role="group" aria-label={tr('Destino', 'Target')}>
        {UNIT_LABELS.map((label, i) => (
          <button
            key={label}
            aria-pressed={unit === i}
            className={units[i]?.on ? 'is-on' : ''}
            onClick={() => {
              setUnit(i);
              taps.current = [];
            }}
          >
            {label}
          </button>
        ))}
      </div>
      <select
        aria-label={tr(`Efecto de ${UNIT_LABELS[unit]}`, `Effect on ${UNIT_LABELS[unit]}`)}
        value={state.kind}
        data-action={act(unit, 'beatFx', state.kind)}
        onChange={(e) => send('beatFx', +e.target.value)}
      >
        {BEAT_FX.map((label, i) => (
          <option key={label} value={i}>
            {label}
          </option>
        ))}
      </select>
      <div className="rx-fx-beat">
        <button
          aria-label={tr('División más corta', 'Shorter division')}
          data-action={act(unit, 'beatFxBeats', nextDivision(state.beats, -1))}
          onClick={() => send('beatFxBeats', nextDivision(state.beats, -1))}
        >
          ◀
        </button>
        <b>{beatLabel(state.beats)}</b>
        <button
          aria-label={tr('División más larga', 'Longer division')}
          data-action={act(unit, 'beatFxBeats', nextDivision(state.beats, 1))}
          onClick={() => send('beatFxBeats', nextDivision(state.beats, 1))}
        >
          ▶
        </button>
      </div>
      <button
        className="rx-fx-tap"
        title={tr(
          'Marca el tempo del efecto · Shift + clic: volver al de la pista',
          "Tap the effect's tempo · Shift-click: back to the track's",
        )}
        onClick={(e) => {
          if (e.shiftKey) {
            taps.current = [];
            send('beatFxBpm', 0);
            return;
          }
          taps.current = [...taps.current.slice(-15), performance.now()];
          const bpm = tapTempo(taps.current);
          if (bpm !== null) send('beatFxBpm', bpm);
        }}
      >
        {state.bpm ? `TAP ${state.bpm.toFixed(1)}` : 'TAP'}
      </button>
      <Knob
        label="LEVEL / DEPTH"
        tone="master"
        action={act(unit, 'beatFxDepth')}
        defaultValue={0.5}
        value={state.depth}
        onChange={(v) => parameter(unit, 'beatFxDepth', v, onError)}
      />
      <button
        className="rx-fx-on"
        aria-pressed={state.on}
        data-action={act(unit, 'beatFxOn', state.on ? 0 : 1)}
        onClick={() => send('beatFxOn', state.on ? 0 : 1)}
      >
        ON / OFF
      </button>
    </>
  );
}

/** Existing curado effects, kept distinct from the unavailable hardware FX. */
function DeckExtensions({
  index,
  slot,
  onError,
}: {
  index: number;
  slot: Slot;
  onError: (e: unknown) => void;
}) {
  const tone = index ? 'b' : 'a';
  const send = (kind: string) => (value: number) => parameter(index, kind, value, onError);
  return (
    <div className={`rx-extension tone-${tone}`}>
      <span className="rx-rail-label">
        {tr('EFECTOS DE CURADO', 'CURADO EFFECTS')} · {index ? 'B' : 'A'}
      </span>
      <div className="rx-extension-knobs">
        <Knob
          label="LP"
          tone={tone}
          action={act(index, 'filter')}
          value={slot.filter}
          defaultValue={1}
          origin={1}
          format={(v) => (v > 0.99 ? 'OFF' : `${Math.round(80 * 200 ** v)} Hz`)}
          onChange={send('filter')}
        />
        <Knob
          label="HP"
          tone={tone}
          action={act(index, 'highPass')}
          value={slot.highPass}
          format={(v) => (v < 0.001 ? 'OFF' : `${Math.round(20 * 1000 ** v)} Hz`)}
          onChange={send('highPass')}
        />
        <Knob
          label="ECHO"
          tone={tone}
          action={act(index, 'echo')}
          value={slot.echo}
          onChange={send('echo')}
        />
      </div>
      <label className="echo-division">
        ECHO · BEATS{' '}
        <select
          aria-label={tr(
            `División echo ${index ? 'B' : 'A'}`,
            `Echo division ${index ? 'B' : 'A'}`,
          )}
          value={slot.echoBeats}
          data-action={act(index, 'echoBeats')}
          onChange={(e) => control(index, 'echoBeats', +e.target.value).catch(onError)}
        >
          {[0.25, 0.5, 0.75, 1, 2].map((n) => (
            <option key={n} value={n}>
              {n}
            </option>
          ))}
        </select>
      </label>
    </div>
  );
}

export function KeyboardGuide({
  keys,
  onTrigger,
}: {
  keys: KeyBinding[];
  onTrigger: (binding: KeyBinding) => void;
}) {
  const entries = useHud();
  const [pointed, setPointed] = useState('');
  const inspect = (target: EventTarget) => {
    const key =
      target instanceof Element ? target.closest<HTMLButtonElement>('button[data-key]') : null;
    setPointed(key?.title.replaceAll('\n', ' · ') ?? '');
  };
  return (
    <div
      className="rx-keyboard"
      onPointerOver={(e) => inspect(e.target)}
      onPointerLeave={() => setPointed('')}
      onFocus={(e) => inspect(e.target)}
      onBlur={() => setPointed('')}
    >
      <KeyboardMap keys={keys} mode="mix" onTrigger={onTrigger} />
      <div className="rx-keyboard-description" aria-live="polite">
        {pointed ||
          (entries.length
            ? `${entries[entries.length - 1].key} · ${entries[entries.length - 1].label}`
            : tr(
                'Toca una tecla o señala el mapa para ver su función',
                'Press a key or point at the map to see its action',
              ))}
      </div>
    </div>
  );
}

/** Lives in the side panel, so that opening it never shrinks the console. */
export function DeckTools({
  library,
  loading,
  onError,
  onLibrary,
}: {
  library: Library;
  loading: number[];
  onError: (e: unknown) => void;
  onLibrary: (data: Library) => void;
}) {
  const { audio } = useSnapshot();
  const slots = [0, 1].map((i) => audio?.slots[i] ?? EMPTY_SLOT);
  const hasTrack = (index: number) => library.tracks.some((t) => t.id === slots[index].id);
  return (
    <>
      {[0, 1].map((index) => (
        <div className="rx-deck-extras" key={index}>
          <DeckExtensions index={index} slot={slots[index]} onError={onError} />
          <MusicalControls
            index={index}
            slot={slots[index]}
            disabled={!hasTrack(index) || loading.includes(index)}
            onError={onError}
            onLibrary={onLibrary}
          />
        </div>
      ))}
      <div
        className="rx-linked"
        role="group"
        aria-label={tr('Transporte simultáneo A y B', 'Linked transport A and B')}
      >
        <span className="rx-rail-label">{tr('TRANSPORTE A + B', 'TRANSPORT A + B')}</span>
        <div>
          <button
            disabled={!slots[0].id && !slots[1].id}
            data-action={act(0, 'playBoth')}
            onClick={() => control(0, 'playBoth').catch(onError)}
          >
            {tr('Reproducir ambos', 'Play both')}
          </button>
          <button
            data-action={act(0, 'pauseBoth')}
            onClick={() => control(0, 'pauseBoth').catch(onError)}
          >
            {tr('Pausar ambos', 'Pause both')}
          </button>
          <button
            disabled={!slots[0].id && !slots[1].id}
            data-action={act(0, 'cueBoth')}
            onClick={() => control(0, 'cueBoth').catch(onError)}
          >
            CUE 1
          </button>
        </div>
      </div>
    </>
  );
}

export function HardwareConsole({
  library,
  keys,
  loading,
  onError,
  selected,
  onLoad,
  platters,
  onEdit,
  autoGain,
  onAutoGain,
}: {
  library: Library;
  keys: KeyBinding[];
  loading: number[];
  onError: (e: unknown) => void;
  selected: number | null;
  onLoad: (slot: number, id: number) => void;
  platters: boolean;
  onEdit: (edit: LibraryEdit) => unknown;
  autoGain: boolean;
  onAutoGain: (on: boolean) => void;
}) {
  const { audio } = useSnapshot();
  const scrollRef = useRef<HTMLDivElement>(null);
  const bodyRef = useRef<HTMLDivElement>(null);
  const scaleRef = useRef(1);
  const [scale, setScale] = useState(1);
  useLayoutEffect(() => {
    const scroll = scrollRef.current;
    const body = bodyRef.current;
    if (!scroll || !body) return;
    const measure = () => {
      const bounds = body.getBoundingClientRect();
      // Read the applied zoom from the rendered box, rather than a pending React state update.
      const appliedScale = bounds.width / 1170;
      const next = consoleScale(
        scroll.clientWidth - 8,
        scroll.clientHeight - 8,
        1170,
        bounds.height / appliedScale,
      );
      if (Math.abs(next - scaleRef.current) >= 0.002) {
        scaleRef.current = next;
        setScale(next);
      }
    };
    const observer = new ResizeObserver(measure);
    observer.observe(scroll);
    observer.observe(body);
    window.addEventListener('resize', measure);
    measure();
    return () => {
      observer.disconnect();
      window.removeEventListener('resize', measure);
    };
  }, []);
  const slots = [0, 1].map((i) => audio?.slots[i] ?? EMPTY_SLOT);
  const samples = Array.from(
    { length: SAMPLE_PADS },
    (_, i) => audio?.slots[i + 2] ?? EMPTY_SLOT,
  );
  const selection = library.tracks.find((track) => track.id === selected);
  const tracks = slots.map((slot) => library.tracks.find((track) => track.id === slot.id));
  return (
    <div
      ref={scrollRef}
      className="rx-scroll"
      role="region"
      aria-label={tr('Cabina de mezcla', 'Mixing console')}
      tabIndex={0}
    >
      <div ref={bodyRef} className={`rx-body ${platters ? '' : 'no-platters'}`} style={{ zoom: scale }}>
            <div className="rx-layout">
              <RxDeck
                index={0}
                slot={slots[0]}
                track={tracks[0]}
                keys={keys}
                loading={loading.includes(0)}
                masterDeck={audio?.masterDeck ?? 0}
                masterHasBpm={!!audio?.slots[audio.masterDeck]?.music.bpm}
                onError={onError}
                samples={samples}
                tracks={library.tracks}
                selected={selected}
                onLoad={onLoad}
                loadingSlots={loading}
              />
              <div className="rx-center">
                <div className="rx-screen-frame">
                  <div className="rx-screen-head">
                    <span>curado</span>
                    <span>WAVEFORM</span>
                    <span>DECK A / B</span>
                  </div>
                  {slots.map((slot, i) => (
                    <div className="rx-screen-track" key={i}>
                      <div>
                        <b>{i ? 'B' : 'A'}</b>
                        <span>{tracks[i]?.title ?? tr('Sin pista', 'No track')}</span>
                        <strong>{remaining(slot)}</strong>
                      </div>
                      <ZoomWave
                        name={i ? 'B' : 'A'}
                        tone={i ? 'b' : 'a'}
                        trackId={tracks[i]?.id}
                        slot={slot}
                        onSeek={tracks[i] ? (s) => control(i, 'seek', s).catch(onError) : undefined}
                      />
                    </div>
                  ))}
                  <div className="rx-screen-bottom">
                    <span>{tr('FORMA DE ONDA', 'WAVEFORM')}</span>
                    <span>{tr('BPM ESTIMADO', 'ESTIMATED BPM')}</span>
                  </div>
                </div>
                <div className="rx-browser">
                  <button
                    disabled={!selected || loading.includes(0)}
                    title={tr('Cargar la pista seleccionada en A', 'Load selected track into A')}
                    onClick={() => selected && onLoad(0, selected)}
                  >
                    LOAD A
                  </button>
                  <button
                    disabled={!selection}
                    aria-pressed={!!selection?.favorite}
                    title={tr(
                      'Marca la pista seleccionada como favorita',
                      'Marks the selected track as a favorite',
                    )}
                    onClick={() =>
                      selection &&
                      onEdit({
                        kind: 'favorite',
                        track: selection.id,
                        value: !selection.favorite,
                      })
                    }
                  >
                    TAG TRACK
                  </button>
                  <button
                    disabled={!selected || loading.includes(1)}
                    title={tr('Cargar la pista seleccionada en B', 'Load selected track into B')}
                    onClick={() => selected && onLoad(1, selected)}
                  >
                    LOAD B
                  </button>
                </div>
                <div className="rx-mixer">
                  <div className="rx-color-fx">
                    <ColorFxPanel slots={slots} onError={onError} />
                    <strong>HEADPHONES</strong>
                    <Soon>MASTER CUE</Soon>
                    <Soon>LINK CUE</Soon>
                    <Soon dial>MIXING</Soon>
                    <Soon dial>LEVEL</Soon>
                    <small>{tr('REQUIERE SALIDA DE CUE', 'NEEDS A CUE OUTPUT')}</small>
                  </div>
                  <RxChannel index={0} slot={slots[0]} onError={onError} />
                  <RxChannel index={1} slot={slots[1]} onError={onError} />
                  <div className="rx-beat-fx">
                    <strong>MASTER</strong>
                    <Knob
                      label="MASTER"
                      tone="master"
                      action={act(0, 'master')}
                      defaultValue={0.75}
                      value={audio?.master ?? 0.75}
                      onChange={(v) => parameter(0, 'master', v, onError)}
                    />
                    <Meter
                      vertical
                      value={audio?.peak ?? 0}
                      label={tr('Nivel master', 'Master level')}
                    />
                    <LimiterLamp reduction={audio?.limiterDb ?? 0} />
                    <button
                      className="rx-fx-on"
                      aria-pressed={autoGain}
                      title={tr(
                        'Iguala el nivel de las pistas a −14 LUFS, hasta ±12 dB',
                        'Matches track levels to −14 LUFS, up to ±12 dB',
                      )}
                      onClick={() => {
                        onAutoGain(!autoGain);
                        control(0, 'autoGain', autoGain ? 0 : 1).catch(onError);
                      }}
                    >
                      AUTO GAIN
                    </button>
                    <BeatFxPanel units={audio?.beatFx ?? []} onError={onError} />
                  </div>
                  <div className="rx-cross">
                    <Slider
                      vertical={false}
                      label="CROSSFADER"
                      tone="master"
                      action={act(0, 'crossfader')}
                      origin={0.5}
                      defaultValue={0.5}
                      value={audio?.crossfader ?? 0.5}
                      onChange={(v) => parameter(0, 'crossfader', v, onError)}
                    />
                  </div>
                </div>
              </div>
              <RxDeck
                index={1}
                slot={slots[1]}
                track={tracks[1]}
                keys={keys}
                loading={loading.includes(1)}
                masterDeck={audio?.masterDeck ?? 0}
                masterHasBpm={!!audio?.slots[audio.masterDeck]?.music.bpm}
                onError={onError}
                samples={samples}
                tracks={library.tracks}
                selected={selected}
                onLoad={onLoad}
                loadingSlots={loading}
              />
            </div>
      </div>
    </div>
  );
}
