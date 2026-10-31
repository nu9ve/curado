import { tr } from '../lib/language';
import { Square, Plus, Play, Pause } from 'lucide-react';
import { control, parameter, useSnapshot } from '../lib/bridge';
import type { KeyBinding, Track } from '../lib/types';
import { EMPTY_SLOT } from '../lib/types';
import { bindingLabel } from '../lib/keyboard';
import { actionKey } from '../lib/feedback';
import { COLOR_FX } from '../lib/console-pads';
import { Key, Knob, LimiterLamp, Meter, Slider } from './Controls';

const eq = (v: number) => {
  const db = v > 0 ? 20 * Math.log10(v) : -Infinity;
  return Number.isFinite(db) ? `${db > 0 ? '+' : ''}${db.toFixed(0)}` : '−∞';
};

function Channel({ index, onError }: { index: number; onError: (e: unknown) => void }) {
  const { audio } = useSnapshot();
  const slot = audio?.slots[index] ?? EMPTY_SLOT;
  const tone = index ? 'b' : 'a';
  const param = (kind: string) => (v: number) => parameter(index, kind, v, onError);
  const act = (kind: string) => actionKey({ slot: index, kind, value: 0 });
  return (
    <div
      className={`channel tone-${tone}`}
      aria-label={tr(`Canal ${index ? 'B' : 'A'}`, `Channel ${index ? 'B' : 'A'}`)}
    >
      <span className="channel-tag">{index ? 'B' : 'A'}</span>
      <div className="channel-knobs">
        <Knob
          label="TRIM"
          action={act('trim')}
          tone={tone}
          max={2}
          origin={1}
          defaultValue={1}
          value={slot.trim}
          format={eq}
          onChange={param('trim')}
        />
        <Knob
          label={slot.colorFx ? COLOR_FX[slot.colorFx - 1] : 'COLOR'}
          action={act('color')}
          tone={tone}
          min={-1}
          max={1}
          origin={0}
          defaultValue={0}
          value={slot.color}
          format={(v) => (Math.abs(v) < 0.01 ? tr('centro', 'center') : v.toFixed(2))}
          onChange={param('color')}
        />
        <Knob
          label="HI"
          action={act('high')}
          tone={tone}
          max={2}
          origin={1}
          defaultValue={1}
          value={slot.high}
          format={eq}
          onChange={param('high')}
        />
        <Knob
          label="FILTER"
          action={act('filter')}
          tone={tone}
          value={slot.filter}
          defaultValue={1}
          origin={1}
          format={(v) => (v > 0.99 ? 'OFF' : `${Math.round(80 * 200 ** v)}`)}
          onChange={param('filter')}
        />
        <Knob
          label="MID"
          action={act('mid')}
          tone={tone}
          max={2}
          origin={1}
          defaultValue={1}
          value={slot.mid}
          format={eq}
          onChange={param('mid')}
        />
        <Knob
          label="ECHO"
          action={act('echo')}
          tone={tone}
          value={slot.echo}
          onChange={param('echo')}
        />
        <Knob
          label="LOW"
          action={act('low')}
          tone={tone}
          max={2}
          origin={1}
          defaultValue={1}
          value={slot.low}
          format={eq}
          onChange={param('low')}
        />
        <Knob
          label="HP"
          action={act('highPass')}
          tone={tone}
          value={slot.highPass}
          onChange={param('highPass')}
          format={(v) => (v < 0.001 ? 'OFF' : `${Math.round(20 * 1000 ** v)} Hz`)}
        />
      </div>
      <label className="echo-division">
        ECHO · BEATS
        <select
          aria-label={tr(
            `División echo ${index ? 'B' : 'A'}`,
            `Echo division ${index ? 'B' : 'A'}`,
          )}
          data-action={act('echoBeats')}
          value={slot.echoBeats}
          onChange={(e) => control(index, 'echoBeats', +e.target.value).catch(onError)}
        >
          {[0.25, 0.5, 0.75, 1, 2].map((n) => (
            <option key={n} value={n}>
              {n}
            </option>
          ))}
        </select>
      </label>
      <div className="channel-fader">
        <Meter
          vertical
          value={slot.peak}
          label={tr(`Nivel deck ${index ? 'B' : 'A'}`, `Deck ${index ? 'B' : 'A'} level`)}
        />
        <Slider
          label="VOL"
          action={act('gain')}
          tone={tone}
          max={1.5}
          defaultValue={0.8}
          value={slot.gain}
          format={(v) => `${Math.round((v / 1.5) * 100)}`}
          onChange={param('gain')}
        />
      </div>
    </div>
  );
}

export function Mixer({ keys, onError }: { keys: KeyBinding[]; onError: (e: unknown) => void }) {
  const { audio } = useSnapshot();
  const cross = keys.filter((k) => k.action.kind === 'crossfader');
  const panic = keys.find((k) => k.action.kind === 'panic');
  return (
    <section className="mixer" aria-label="Mixer">
      <Channel index={0} onError={onError} />
      <div className="mixer-master">
        <span className="mixer-brand">mx·2</span>
        <LimiterLamp reduction={audio?.limiterDb ?? 0} />
        <Meter vertical value={audio?.peak ?? 0} label={tr('Nivel master', 'Master level')} />
        <Slider
          label="MASTER"
          action={actionKey({ slot: 0, kind: 'master', value: 0 })}
          tone="master"
          defaultValue={0.75}
          value={audio?.master ?? 0.75}
          onChange={(v) => parameter(0, 'master', v, onError)}
        />
        <button
          className="panic"
          onClick={() => control(0, 'panic').catch(onError)}
          data-action={actionKey({ slot: 0, kind: 'panic', value: 0 })}
          title={tr(
            'Detiene todo y baja el master a cero',
            'Stops everything and turns master down to zero',
          )}
        >
          <Square size={9} fill="currentColor" />
          {panic && <Key code={panic.code}>{bindingLabel(panic)}</Key>}
        </button>
      </div>
      <Channel index={1} onError={onError} />
      <div className="xfader">
        <div
          className="linked-transport"
          role="group"
          aria-label={tr('Transporte simultáneo A y B', 'Linked transport A and B')}
        >
          <span>A + B</span>
          <button
            aria-label={tr('Reproducir ambos decks', 'Play both decks')}
            data-action="0:playBoth:0"
            onClick={() => control(0, 'playBoth').catch(onError)}
            disabled={!audio?.slots[0]?.id && !audio?.slots[1]?.id}
          >
            <Play size={12} /> {tr('ambos', 'both')}{' '}
          </button>
          <button
            aria-label={tr('Pausar ambos decks', 'Pause both decks')}
            data-action="0:pauseBoth:0"
            onClick={() => control(0, 'pauseBoth').catch(onError)}
          >
            <Pause size={12} />
          </button>
          <button
            aria-label={tr('Hot cue 1 en ambos decks', 'Hot cue 1 on both decks')}
            data-action="0:cueBoth:0"
            onClick={() => control(0, 'cueBoth').catch(onError)}
            disabled={!audio?.slots[0]?.id && !audio?.slots[1]?.id}
          >
            cue 1
          </button>
        </div>
        <Slider
          vertical={false}
          label="CROSSFADER"
          action={actionKey({ slot: 0, kind: 'crossfader', value: 0 })}
          tone="master"
          origin={0.5}
          defaultValue={0.5}
          value={audio?.crossfader ?? 0.5}
          format={(v) =>
            v < 0.48
              ? `A ${Math.round((1 - v) * 100)}`
              : v > 0.52
                ? `B ${Math.round(v * 100)}`
                : '· ·'
          }
          onChange={(v) => parameter(0, 'crossfader', v, onError)}
        />
        <div className="xfader-keys">
          {[0, 0.5, 1].map((value) => {
            const b = cross.find((k) => k.action.value === value);
            return (
              <button
                key={value}
                data-action={actionKey({ slot: 0, kind: 'crossfader', value })}
                onClick={() => control(0, 'crossfader', value).catch(onError)}
                aria-label={`Crossfader ${['A', tr('centro', 'center'), 'B'][value * 2]}`}
              >
                {b ? <Key code={b.code}>{bindingLabel(b)}</Key> : '—'}
              </button>
            );
          })}
        </div>
      </div>
    </section>
  );
}

export function SampleBank({
  tracks,
  keys,
  onError,
  onLoad,
  selected,
}: {
  tracks: Track[];
  keys: KeyBinding[];
  onError: (e: unknown) => void;
  onLoad: (slot: number, id: number) => void;
  selected: number | null;
}) {
  const { audio } = useSnapshot();
  return (
    <section className="sample-bank" aria-label="Samples">
      <div className="sample-bank-head">
        <span className="eyebrow">SAMPLES</span>
        <Knob
          label={tr('NIVEL', 'LEVEL')}
          tone="master"
          action={actionKey({ slot: 2, kind: 'sampler', value: 0 })}
          max={1.5}
          defaultValue={1}
          origin={1}
          value={audio?.sampler ?? 1}
          onChange={(v) => parameter(2, 'sampler', v, onError)}
        />
      </div>
      <div className="sample-pads">
        {[2, 3, 4, 5, 6, 7, 8, 9].map((slot, i) => {
          const state = audio?.slots[slot];
          const track = tracks.find((t) => t.id === state?.id);
          const b = keys.find((k) => k.action.slot === slot && k.action.kind === 'trigger');
          return (
            <div className={`sample-pad ${state?.playing ? 'active' : ''}`} key={slot}>
              <button
                disabled={!track}
                onClick={() => control(slot, 'trigger').catch(onError)}
                title={track?.title ?? tr('Carga una pista', 'Load a track')}
                data-action={actionKey({ slot, kind: 'trigger', value: 0 })}
              >
                <span>
                  S{i + 1}
                  {b && <Key code={b.code}>{bindingLabel(b)}</Key>}
                </span>
                <strong>{track?.title ?? tr('vacío', 'empty')}</strong>
              </button>
              <button
                className="pad-load"
                aria-label={tr(
                  `Cargar selección en sample ${i + 1}`,
                  `Load selection into sample ${i + 1}`,
                )}
                title={tr('Cargar la pista seleccionada', 'Load selected track')}
                disabled={!selected}
                onClick={() => selected && onLoad(slot, selected)}
              >
                <Plus size={12} />
              </button>
            </div>
          );
        })}
      </div>
    </section>
  );
}
