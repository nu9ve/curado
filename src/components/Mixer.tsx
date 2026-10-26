import { Volume2, Square } from 'lucide-react';
import { control, parameter, useSnapshot } from '../lib/bridge';
import type { KeyBinding, Track } from '../lib/types';
import { bindingLabel } from '../lib/keyboard';
import { Fader, Key, Meter } from './Controls';
export function Mixer({
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
    <section className="mix-bus">
      <div className="crossfader">
        <div className="section-line">
          <span className="eyebrow">CROSSFADER</span>
          <small>Equal power</small>
        </div>
        <Fader
          label="A ↔ B"
          value={audio?.crossfader ?? 0.5}
          onChange={(v) => parameter(0, 'crossfader', v, onError)}
          format={(v) =>
            v < 0.48
              ? `A ${Math.round((1 - v) * 100)}%`
              : v > 0.52
                ? `B ${Math.round(v * 100)}%`
                : 'CENTRO'
          }
        />
        <div className="cross-keys">
          {[0, 0.5, 1].map((value, index) => {
            const binding = keys.find(
              (k) => k.action.kind === 'crossfader' && k.action.value === value,
            );
            return (
              <Key key={value}>
                {binding ? bindingLabel(binding) : '—'} {['A', 'centro', 'B'][index]}
              </Key>
            );
          })}
        </div>
      </div>
      <div className="sample-bank">
        <div className="section-line">
          <span className="eyebrow">SAMPLE BANK</span>
          <small>Selecciona una pista · + para cargar</small>
        </div>
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
                >
                  <span>
                    S0{i + 1}
                    {b && <Key>{bindingLabel(b)}</Key>}
                  </span>
                  <strong>{track?.title ?? 'Vacío'}</strong>
                </button>
                <button
                  className="pad-load"
                  aria-label={`Cargar selección en sample ${i + 1}`}
                  disabled={!selected}
                  onClick={() => selected && onLoad(slot, selected)}
                >
                  +
                </button>
              </div>
            );
          })}
        </div>
      </div>
      <div className="master">
        <span className="eyebrow">
          <Volume2 size={13} /> MASTER
        </span>
        <Fader
          label="Salida"
          value={audio?.master ?? 0.75}
          onChange={(v) => parameter(0, 'master', v, onError)}
        />
        <Meter value={audio?.peak ?? 0} />
        <button className="panic" onClick={() => control(0, 'panic').catch(onError)}>
          <Square size={10} fill="currentColor" />
          Silenciar
          <Key>
            {keys.find((k) => k.action.kind === 'panic')
              ? bindingLabel(keys.find((k) => k.action.kind === 'panic')!)
              : '—'}
          </Key>
        </button>
      </div>
    </section>
  );
}
