import { useEffect, useState } from 'react';
import { X, Radio, Keyboard, SlidersHorizontal, Trash2, Palette } from 'lucide-react';
import { MidiSettings } from './MidiSettings';
import { StemsSettings } from './StemsSettings';
import { ThemePanel } from './ThemePanel';
import { call, native, useSnapshot } from '../lib/bridge';
import { DEFAULT_KEYS, actionLabel, bindingLabel } from '../lib/keyboard';
import type { Action, KeyBinding } from '../lib/types';
import { setLanguage, tr, useLanguage } from '../lib/language';
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
  const language = useLanguage();
  const [tab, setTab] = useState('appearance');
  const [devices, setDevices] = useState<string[]>([]);
  const [device, setDevice] = useState('');
  const [frames, setFrames] = useState('');
  const [busy, setBusy] = useState(false);
  const [capture, setCapture] = useState<number | null>(null);
  const snap = useSnapshot();
  const refresh = () => {
    if (!native) return;
    call<string[]>('audio_devices').then(setDevices).catch(onError);
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
      // Command belongs to the system; a modifier alone is not a shortcut yet.
      if (e.metaKey || /^(Shift|Control|Alt|Meta)/.test(e.code)) return;
      const combination = { code: e.code, shift: e.shiftKey, ctrl: e.ctrlKey, alt: e.altKey };
      const next = keys.map((k, i) => (i === capture ? { ...k, ...combination } : k));
      if (
        next.some(
          (k, i) =>
            i !== capture &&
            k.code === e.code &&
            k.shift === e.shiftKey &&
            !!k.ctrl === e.ctrlKey &&
            !!k.alt === e.altKey,
        )
      ) {
        onError(
          new Error(
            tr(
              'Esa tecla ya está asignada. Elige otra.',
              'That key is already assigned. Choose another.',
            ),
          ),
        );
        return;
      }
      onKeys(next).catch(onError);
      setCapture(null);
    };
    window.addEventListener('keydown', handler, true);
    return () => window.removeEventListener('keydown', handler, true);
  }, [capture, keys, onKeys, onError]);
  return (
    <div className="modal-backdrop" onClick={onClose}>
      <section
        className="settings modal"
        role="dialog"
        aria-modal="true"
        aria-label={tr('Configuración', 'Settings')}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="modal-header">
          <div>
            <span className="eyebrow">{tr('a tu manera', 'your way')}</span>
            <h2>{tr('configuración', 'settings')}</h2>
          </div>
          <button aria-label={tr('Cerrar configuración', 'Close settings')} onClick={onClose}>
            <X size={18} />
          </button>
        </div>
        <div className="settings-tabs">
          {[
            ['appearance', tr('Apariencia', 'Appearance'), Palette],
            ['audio', 'Audio', SlidersHorizontal],
            ['keyboard', tr('Teclado', 'Keyboard'), Keyboard],
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
        {tab === 'appearance' && (
          <>
            <div className="settings-body">
              <label>
                {tr('Idioma', 'Language')}
                <select
                  value={language}
                  onChange={(e) => setLanguage(e.target.value === 'en' ? 'en' : 'es')}
                >
                  <option value="es">Español</option>
                  <option value="en">English (in progress)</option>
                </select>
              </label>
              {language === 'en' && (
                <p>
                  English translation is in progress; some controls and messages remain in Spanish.
                </p>
              )}
            </div>
            <ThemePanel onError={onError} />
          </>
        )}
        {tab === 'audio' && (
          <div className="settings-body">
            <h3>{tr('el audio tiene prioridad.', 'audio comes first.')}</h3>
            <p>
              {tr(
                'El motor Rust reproduce y procesa audio fuera de la interfaz. Los visuales y la biblioteca trabajan por separado.',
                'The Rust engine plays and processes audio independently of the interface. Visuals and the library run separately.',
              )}{' '}
            </p>
            <div className="info-grid">
              <span>
                Backend<b>{snap.output?.backend ?? tr('Sin salida', 'No output')}</b>
              </span>
              <span>
                {tr('Salida activa', 'Active output')}
                <b>{snap.output?.device ?? '—'}</b>
              </span>
              <span>
                {tr('Canales', 'Channels')}
                <b>{snap.output?.channels ?? '—'}</b>
              </span>
              <span>
                {tr('Frecuencia', 'Sample rate')}
                <b>{snap.output?.sampleRate ?? '—'} Hz</b>
              </span>
              <span>
                Callbacks<b>{snap.audio?.callbacks ?? 0}</b>
              </span>
              <span>
                {tr('Frames limitados', 'Limited frames')}
                <b>{snap.audio?.limited ?? 0}</b>
              </span>
              <span>
                {tr('Recorte duro', 'Hard clipping')}
                <b>{snap.audio?.clipped ?? 0}</b>
              </span>
            </div>
            {(snap.error || snap.audio?.streamError) && (
              <p className="error-text">
                {snap.error ??
                  tr(
                    'El dispositivo ha reportado un error. Selecciona una salida y reinicia el motor.',
                    'The device reported an error. Select an output and restart the engine.',
                  )}
              </p>
            )}
            <label>
              {tr('Dispositivo de salida', 'Output device')}{' '}
              <select value={device} onChange={(e) => setDevice(e.target.value)}>
                <option value="">{tr('Predeterminado del sistema', 'System default')}</option>
                {devices.map((d) => (
                  <option key={d}>{d}</option>
                ))}
              </select>
            </label>
            <label>
              {tr('Buffer de audio', 'Audio buffer')}{' '}
              <select value={frames} onChange={(e) => setFrames(e.target.value)}>
                <option value="">
                  {tr('Automático · recomendado', 'Automatic · recommended')}
                </option>
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
              {tr(
                'Al aplicar se descargan los decks y samples. Vuelve a cargar las pistas. Un buffer menor exige más al sistema; la cifra no representa la latencia total.',
                'Applying unloads decks and samples. Reload your tracks. A smaller buffer requires more from the system; the figure is not total latency.',
              )}{' '}
            </p>
            <div className="button-row">
              <button onClick={refresh}>{tr('Actualizar dispositivos', 'Refresh devices')}</button>
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
                {busy
                  ? tr('Aplicando…', 'Applying…')
                  : tr('Aplicar / recuperar audio', 'Apply / recover audio')}
              </button>
            </div>
            <p className="small-print">
              {tr(
                'Para preescucha independiente, elige una salida de cuatro canales o más. El master usa los canales 1–2 y el cue de audífonos usa 3–4.',
                'For independent headphone cue, choose an output with at least four channels. Master uses channels 1–2 and headphone cue uses 3–4.',
              )}{' '}
            </p>
            <StemsSettings onError={onError} />
          </div>
        )}
        {tab === 'keyboard' && (
          <div className="settings-body">
            <h3>{tr('un teclado. toda la sesión.', 'one keyboard. the whole session.')}</h3>
            <p>
              {tr(
                'Atajos físicos con la ventana enfocada. Al escribir o usar sliders no se disparan acciones. Shift + cue guarda la posición. Haz clic en una tecla para cambiarla.',
                'Physical shortcuts work while the window is focused. Typing or using sliders does not trigger actions. Shift + cue saves the position. Click a key to change it.',
              )}{' '}
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
                    {capture === i ? tr('Pulsa una tecla…', 'Press a key…') : bindingLabel(k)}
                  </button>
                </div>
              ))}
            </div>
            <button onClick={() => onKeys(DEFAULT_KEYS).catch(onError)}>
              {tr('Restaurar distribución original', 'Restore default layout')}{' '}
            </button>
            <p className="small-print">
              {tr(
                'ESC silencia todos los decks y baja el master a cero. Sube el master para reanudar. Los atajos no son globales del sistema.',
                'ESC stops all decks and turns master down to zero. Turn master up to resume. Shortcuts are not system-wide.',
              )}{' '}
            </p>
          </div>
        )}
        {tab === 'midi' && <MidiSettings onError={onError} />}
      </section>
    </div>
  );
}
