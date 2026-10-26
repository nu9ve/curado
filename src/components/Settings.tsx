import { useEffect, useState } from 'react';
import { X, Radio, Keyboard, SlidersHorizontal, Trash2, Palette } from 'lucide-react';
import { ThemePanel } from './ThemePanel';
import { call, useSnapshot } from '../lib/bridge';
import { DEFAULT_KEYS, actionLabel, bindingLabel } from '../lib/keyboard';
import type { Action, KeyBinding } from '../lib/types';
export function Settings({
  keys,
  onKeys,
  onClose,
  onError,
}: {
  keys: KeyBinding[];
  onKeys: (keys: KeyBinding[]) => Promise<void>;
  onClose: () => void;
  onError: (e: unknown) => void;
}) {
  const [tab, setTab] = useState('appearance');
  const [devices, setDevices] = useState<string[]>([]);
  const [ports, setPorts] = useState<string[]>([]);
  const [device, setDevice] = useState('');
  const [frames, setFrames] = useState('');
  const [busy, setBusy] = useState(false);
  const [capture, setCapture] = useState<number | null>(null);
  const snap = useSnapshot();
  const [target, setTarget] = useState('0:toggle');
  const [cue, setCue] = useState('0');
  const refresh = () => {
    call<string[]>('audio_devices').then(setDevices).catch(onError);
    call<string[]>('midi_ports').then(setPorts).catch(onError);
  };
  useEffect(() => {
    refresh();
  }, []);
  useEffect(() => {
    if (capture !== null) return;
    const close = (e: KeyboardEvent) => {
      if (e.code === 'Escape') onClose();
    };
    window.addEventListener('keydown', close);
    return () => window.removeEventListener('keydown', close);
  }, [capture, onClose]);
  useEffect(() => {
    if (capture === null) return;
    const handler = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.code === 'Escape') {
        setCapture(null);
        return;
      }
      if (e.ctrlKey || e.metaKey || e.altKey || e.code.startsWith('Shift')) return;
      const next = keys.map((k, i) =>
        i === capture ? { ...k, code: e.code, shift: e.shiftKey } : k,
      );
      if (next.some((k, i) => i !== capture && k.code === e.code && k.shift === e.shiftKey)) {
        onError(new Error('Esa tecla ya está asignada. Elige otra.'));
        return;
      }
      onKeys(next).catch(onError);
      setCapture(null);
    };
    window.addEventListener('keydown', handler, true);
    return () => window.removeEventListener('keydown', handler, true);
  }, [capture, keys, onKeys, onError]);
  const learn = () => {
    const [slot, kind] = target.split(':');
    const action: Action = {
      slot: +slot,
      kind,
      value: kind === 'cue' || kind === 'setCue' ? +cue : kind === 'rate' ? 1 : 0,
    };
    call('midi_learn', { action }).catch(onError);
  };
  return (
    <div className="modal-backdrop" onClick={onClose}>
      <section
        className="settings modal"
        role="dialog"
        aria-modal="true"
        aria-label="Configuración"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="modal-header">
          <div>
            <span className="eyebrow">a tu manera</span>
            <h2>configuración</h2>
          </div>
          <button aria-label="Cerrar configuración" onClick={onClose}>
            <X size={18} />
          </button>
        </div>
        <div className="settings-tabs">
          {[
            ['appearance', 'Apariencia', Palette],
            ['audio', 'Audio', SlidersHorizontal],
            ['keyboard', 'Teclado', Keyboard],
            ['midi', 'MIDI', Radio],
          ].map(([id, label, Icon]) => (
            <button
              key={id as string}
              className={tab === id ? 'active' : ''}
              onClick={() => setTab(id as string)}
            >
              {typeof Icon !== 'string' && <Icon size={14} />} {label as string}
            </button>
          ))}
        </div>
        {tab === 'appearance' && <ThemePanel onError={onError} />}
        {tab === 'audio' && (
          <div className="settings-body">
            <h3>el audio tiene prioridad.</h3>
            <p>
              El motor Rust reproduce y procesa audio fuera de la interfaz. Los visuales y la
              biblioteca trabajan por separado.
            </p>
            <div className="info-grid">
              <span>
                Backend<b>{snap.output?.backend ?? 'Sin salida'}</b>
              </span>
              <span>
                Frecuencia<b>{snap.output?.sampleRate ?? '—'} Hz</b>
              </span>
              <span>
                Callbacks<b>{snap.audio?.callbacks ?? 0}</b>
              </span>
              <span>
                Muestras limitadas<b>{snap.audio?.clipped ?? 0}</b>
              </span>
            </div>
            {(snap.error || snap.audio?.streamError) && (
              <p className="error-text">
                {snap.error ??
                  'El dispositivo ha reportado un error. Selecciona una salida y reinicia el motor.'}
              </p>
            )}
            <label>
              Dispositivo de salida
              <select value={device} onChange={(e) => setDevice(e.target.value)}>
                <option value="">Predeterminado del sistema</option>
                {devices.map((d) => (
                  <option key={d}>{d}</option>
                ))}
              </select>
            </label>
            <label>
              Buffer de audio
              <select value={frames} onChange={(e) => setFrames(e.target.value)}>
                <option value="">Automático · recomendado</option>
                {[128, 256, 512, 1024].map((n) => (
                  <option key={n} value={n}>
                    {n} frames
                    {snap.output
                      ? ` · ${((n / snap.output.sampleRate) * 1000).toFixed(1)} ms por buffer`
                      : ''}
                  </option>
                ))}
              </select>
            </label>
            <p className="notice">
              Al aplicar se descargan los decks y samples. Vuelve a cargar las pistas. Un buffer
              menor exige más al sistema; la cifra no representa la latencia total.
            </p>
            <div className="button-row">
              <button onClick={refresh}>Actualizar dispositivos</button>
              <button
                className="primary"
                disabled={busy}
                onClick={async () => {
                  setBusy(true);
                  try {
                    await call('configure_audio', {
                      device: device || null,
                      frames: frames ? +frames : null,
                    });
                  } catch (e) {
                    onError(e);
                  } finally {
                    setBusy(false);
                  }
                }}
              >
                {busy ? 'Aplicando…' : 'Aplicar / recuperar audio'}
              </button>
            </div>
            <p className="small-print">
              Velocidad y tono cambian juntos. El EQ es de tres bandas básicas; echo de 375 ms. Sin
              sincronización automática de beats ni preescucha independiente todavía.
            </p>
          </div>
        )}
        {tab === 'keyboard' && (
          <div className="settings-body">
            <h3>un teclado. toda la sesión.</h3>
            <p>
              Atajos físicos con la ventana enfocada. Al escribir o usar sliders no se disparan
              acciones. Shift + cue guarda la posición. Haz clic en una tecla para cambiarla.
            </p>
            <div className="keymap">
              {keys.map((k, i) => (
                <div key={i}>
                  <span>
                    {k.action.slot < 2
                      ? `Deck ${k.action.slot ? 'B' : 'A'}`
                      : `Sample ${k.action.slot - 1}`}{' '}
                    · {actionLabel(k.action.kind)}
                    {['cue', 'setCue'].includes(k.action.kind) ? ` ${k.action.value + 1}` : ''}
                  </span>
                  <button className={capture === i ? 'capture' : ''} onClick={() => setCapture(i)}>
                    {capture === i ? 'Pulsa una tecla…' : bindingLabel(k)}
                  </button>
                </div>
              ))}
            </div>
            <button onClick={() => onKeys(DEFAULT_KEYS).catch(onError)}>
              Restaurar distribución original
            </button>
            <p className="small-print">
              ESC silencia todos los decks y baja el master a cero. Sube el master para reanudar.
              Los atajos no son globales del sistema.
            </p>
          </div>
        )}
        {tab === 'midi' && (
          <div className="settings-body">
            <h3>conecta. mueve. aprende.</h3>
            <p>
              Note On y CC absolutos por dispositivo, canal y número. Los mapeos se guardan
              automáticamente.
            </p>
            <label>
              Entrada MIDI
              <select
                value={snap.midi.connected ?? ''}
                onChange={(e) =>
                  call('midi_connect', { name: e.target.value || null }).catch(onError)
                }
              >
                <option value="">Desconectado</option>
                {ports.map((p) => (
                  <option key={p}>{p}</option>
                ))}
              </select>
            </label>
            <button onClick={refresh}>Buscar dispositivos</button>
            <div className="learn-box">
              <label>
                Acción
                <select value={target} onChange={(e) => setTarget(e.target.value)}>
                  {[0, 1].flatMap((slot) =>
                    [
                      'toggle',
                      'cue',
                      'setCue',
                      'loop',
                      'gain',
                      'rate',
                      'low',
                      'mid',
                      'high',
                      'filter',
                      'highPass',
                      'echo',
                    ].map((kind) => (
                      <option key={`${slot}:${kind}`} value={`${slot}:${kind}`}>
                        Deck {slot ? 'B' : 'A'} · {actionLabel(kind)}
                      </option>
                    )),
                  )}
                  {[2, 3, 4, 5].map((slot) => (
                    <option key={slot} value={`${slot}:trigger`}>
                      Sample {slot - 1} · Disparar
                    </option>
                  ))}
                  <option value="0:crossfader">Mixer · Crossfader</option>
                  <option value="0:master">Mixer · Master</option>
                  <option value="0:panic">Mixer · Silenciar todo</option>
                </select>
              </label>
              {(target.endsWith(':cue') || target.endsWith(':setCue')) && (
                <select
                  aria-label="Número de cue"
                  value={cue}
                  onChange={(e) => setCue(e.target.value)}
                >
                  {[0, 1, 2].map((i) => (
                    <option key={i} value={i}>
                      Cue {i + 1}
                    </option>
                  ))}
                </select>
              )}
              <button className="primary" disabled={!snap.midi.connected} onClick={learn}>
                {snap.midi.learning ? 'Esperando control…' : 'Aprender siguiente control'}
              </button>
              {snap.midi.learning && (
                <button onClick={() => call('midi_learn', { action: null }).catch(onError)}>
                  Cancelar aprendizaje
                </button>
              )}
              <small>{snap.midi.lastMessage ?? 'Mueve un control para ver su mensaje aquí.'}</small>
            </div>
            <div className="midi-mappings">
              {snap.midi.bindings.map((b, i) => (
                <div key={`${b.device}:${b.channel}:${b.number}:${i}`}>
                  <span>
                    <b>{b.device}</b>
                    <small>
                      CH {b.channel + 1} · {b.status === 176 ? 'CC' : 'NOTE'} {b.number} →{' '}
                      {actionLabel(b.action.kind)}
                    </small>
                  </span>
                  <button
                    aria-label="Eliminar mapeo MIDI"
                    onClick={() => call('midi_remove', { index: i }).catch(onError)}
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              ))}
            </div>
            {snap.midi.error && <p className="error-text">{snap.midi.error}</p>}
            <p className="small-print">
              Controladores relativos, MIDI Clock, feedback LED, SysEx y reconexión automática
              quedan pendientes. Tras desconectar físicamente un equipo, vuelve a buscarlo y
              conectarlo.
            </p>
          </div>
        )}
      </section>
    </div>
  );
}
