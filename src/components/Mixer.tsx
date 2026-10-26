import { Square, Plus } from 'lucide-react';
import { control, parameter, useSnapshot } from '../lib/bridge';
import type { KeyBinding, Track } from '../lib/types';
import { EMPTY_SLOT } from '../lib/types';
import { bindingLabel } from '../lib/keyboard';
import { actionKey } from '../lib/feedback';
import { Key, Knob, Meter, Slider } from './Controls';

const eq = (v: number) => {
  const db = v > 0 ? 20 * Math.log10(v) : -Infinity;
  return Number.isFinite(db) ? `${db > 0 ? '+' : ''}${db.toFixed(0)}` : '−∞';
};

function Channel({ index, onError }: { index: number; onError: (e: unknown) => void }) {
  const { audio } = useSnapshot();
  const slot = audio?.slots[index] ?? EMPTY_SLOT;
  const tone = index ? 'b' : 'a';
  const param = (kind: string) => (v: number) => parameter(index, kind, v, onError);
  return (
    <div className={`channel tone-${tone}`} aria-label={`Canal ${index ? 'B' : 'A'}`}>
      <span className="channel-tag">{index ? 'B' : 'A'}</span>
      <div className="channel-knobs">
        <Knob
          label="HI"
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
          tone={tone}
          value={slot.filter}
          defaultValue={1}
          origin={1}
          format={(v) => (v > 0.99 ? 'OFF' : `${Math.round(80 * 200 ** v)}`)}
          onChange={param('filter')}
        />
        <Knob
          label="MID"
          tone={tone}
          max={2}
          origin={1}
          defaultValue={1}
          value={slot.mid}
          format={eq}
          onChange={param('mid')}
        />
        <Knob label="ECHO" tone={tone} value={slot.echo} onChange={param('echo')} />
        <Knob
          label="LOW"
          tone={tone}
          max={2}
          origin={1}
          defaultValue={1}
          value={slot.low}
          format={eq}
          onChange={param('low')}
        />
        <div className="channel-rate" title="Velocidad del deck">
          <span>RATE</span>
          <b>{`${slot.rate >= 1 ? '+' : '−'}${Math.abs((slot.rate - 1) * 100).toFixed(1)}`}</b>
        </div>
      </div>
      <div className="channel-fader">
        <Meter vertical value={slot.peak} label={`Nivel deck ${index ? 'B' : 'A'}`} />
        <Slider
          label="VOL"
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
        <Meter vertical value={audio?.peak ?? 0} label="Nivel master" />
        <Slider
          label="MASTER"
          tone="master"
          defaultValue={0.75}
          value={audio?.master ?? 0.75}
          onChange={(v) => parameter(0, 'master', v, onError)}
        />
        <button
          className="panic"
          onClick={() => control(0, 'panic').catch(onError)}
          data-action={actionKey({ slot: 0, kind: 'panic', value: 0 })}
          title="Detiene todo y baja el master a cero"
        >
          <Square size={9} fill="currentColor" />
          {panic && <Key code={panic.code}>{bindingLabel(panic)}</Key>}
        </button>
      </div>
      <Channel index={1} onError={onError} />
      <div className="xfader">
        <Slider
          vertical={false}
          label="CROSSFADER"
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
                aria-label={`Crossfader ${['A', 'centro', 'B'][value * 2]}`}
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
      <span className="eyebrow">SAMPLES</span>
      <div className="sample-pads">
        {[2, 3, 4, 5].map((slot, i) => {
          const state = audio?.slots[slot];
          const track = tracks.find((t) => t.id === state?.id);
          const b = keys.find((k) => k.action.slot === slot && k.action.kind === 'trigger');
          return (
            <div className={`sample-pad ${state?.playing ? 'active' : ''}`} key={slot}>
              <button
                disabled={!track}
                onClick={() => control(slot, 'trigger').catch(onError)}
                title={track?.title ?? 'Carga una pista'}
                data-action={actionKey({ slot, kind: 'trigger', value: 0 })}
              >
                <span>
                  S{i + 1}
                  {b && <Key code={b.code}>{bindingLabel(b)}</Key>}
                </span>
                <strong>{track?.title ?? 'vacío'}</strong>
              </button>
              <button
                className="pad-load"
                aria-label={`Cargar selección en sample ${i + 1}`}
                title="Cargar la pista seleccionada"
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
