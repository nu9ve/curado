import { useEffect, useMemo, useState } from 'react';
import { Trash2 } from 'lucide-react';
import { call, native, useSnapshot } from '../lib/bridge';
import { tr } from '../lib/language';
import {
  CUE_KINDS,
  bindingKey,
  detectedLabel,
  djWizard,
  learnTargets,
  midiKeyLabel,
  modeLabel,
  padWizard,
  sameKey,
  targetId,
  targetLabel,
  type WizardStep,
} from '../lib/midi';
import type { Action, MidiBinding, MidiPort, MidiSeen } from '../lib/types';

/**
 * Connect, recognise, explore and learn a MIDI controller. The flow is in docs/MIDI.md: a known
 * controller maps itself; an unknown one is explored (noise first, then one control at a time)
 * and learned, by hand or with a guided list.
 */
export function MidiSettings({ onError }: { onError: (e: unknown) => void }) {
  const { midi } = useSnapshot();
  const [ports, setPorts] = useState<MidiPort[]>([]);
  const [bindings, setBindings] = useState<MidiBinding[]>([]);
  const [seen, setSeen] = useState<MidiSeen[]>([]);
  const [exploring, setExploring] = useState(false);
  const targets = useMemo(learnTargets, []);
  const [target, setTarget] = useState(targetId(targets[0]));
  const [cue, setCue] = useState(0);
  const [wizard, setWizard] = useState<{ steps: WizardStep[]; index: number } | null>(null);
  const [confirming, setConfirming] = useState<'preset' | 'clear' | null>(null);

  const refresh = () => {
    if (native) call<MidiPort[]>('midi_ports').then(setPorts).catch(onError);
  };
  useEffect(refresh, []);
  // While learning, the controller drives nothing: leaving these settings ends a learn.
  useEffect(
    () => () => {
      if (native) call('midi_learn', { action: null }).catch(() => {});
    },
    [],
  );
  useEffect(() => {
    if (native) call<MidiBinding[]>('midi_bindings').then(setBindings).catch(onError);
  }, [midi.bindingsVersion, onError]);
  useEffect(() => {
    if (!exploring || !native) return;
    const read = () => call<MidiSeen[]>('midi_seen').then(setSeen).catch(onError);
    read();
    const timer = window.setInterval(read, 400);
    return () => window.clearInterval(timer);
  }, [exploring, onError]);

  // The guided list: each step learns one control, and a learned one moves to the next.
  const step = wizard ? wizard.steps[wizard.index] : null;
  useEffect(() => {
    if (!wizard) return;
    if (!step) {
      // Skipping the last step leaves nothing to learn.
      setWizard(null);
      call('midi_learn', { action: null }).catch(onError);
      return;
    }
    call('midi_learn', { action: step.action }).catch(onError);
    // Each step starts its own learn; the index alone says which.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [wizard?.index, wizard?.steps]);
  useEffect(() => {
    if (wizard && !midi.learning && midi.learned)
      setWizard((w) => (w ? { ...w, index: w.index + 1 } : w));
    // Only a new binding moves the list forward.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [midi.bindingsVersion]);
  const stopWizard = () => {
    setWizard(null);
    call('midi_learn', { action: null }).catch(onError);
  };

  const chosen = targets.find((t) => targetId(t) === target) ?? targets[0];
  const learn = () => {
    const action: Action = {
      ...chosen,
      value: CUE_KINDS.includes(chosen.kind) ? cue : chosen.value,
    };
    call('midi_learn', { action }).catch(onError);
  };
  const connected = midi.connected;
  const own = bindings
    .map((binding, index) => ({ binding, index }))
    .filter(({ binding }) => binding.device === connected);
  const others = bindings
    .map((binding, index) => ({ binding, index }))
    .filter(({ binding }) => binding.device !== connected);
  const boundTo = (s: MidiSeen) =>
    own.find(({ binding }) => sameKey(bindingKey(binding), s.key))?.binding;
  const port = ports.find((p) => p.name === connected);
  const baseline = midi.baselineLeftMs;

  return (
    <div className="settings-body">
      <h3>{tr('conecta. explora. aprende.', 'connect. explore. learn.')}</h3>
      <p>
        {tr(
          'curado reconoce los controladores que ya conoce y los configura solo. Con uno nuevo, explóralo: mide el ruido, mueve cada control y aprende lo que hace. Faders de 14 bits, jogs y encoders se detectan por lo que envían.',
          'curado recognises controllers it already knows and sets them up by itself. With a new one, explore it: measure noise, move each control and learn what it does. 14-bit faders, jogs and encoders are detected from what they send.',
        )}
      </p>
      <label>
        {tr('Entrada MIDI', 'MIDI input')}{' '}
        <select
          value={connected ?? ''}
          onChange={(e) => call('midi_connect', { name: e.target.value || null }).catch(onError)}
        >
          <option value="">{tr('Desconectado', 'Disconnected')}</option>
          {ports.map((p) => (
            <option key={p.name} value={p.name}>
              {p.name}
              {p.preset ? ` · ${p.preset.vendor} ${p.preset.name}` : ''}
            </option>
          ))}
        </select>
      </label>
      <button onClick={refresh}>{tr('Buscar dispositivos', 'Find devices')}</button>
      <small className="small-print">
        {connected
          ? tr(
              `Conectado. ${midi.mapped} controles con función. Se conectará solo cada vez que lo enchufes.`,
              `Connected. ${midi.mapped} controls mapped. It connects by itself whenever it is plugged in.`,
            )
          : midi.remembered
            ? tr(
                `Esperando a ${midi.remembered}: se conecta solo al enchufarlo.`,
                `Waiting for ${midi.remembered}: it connects by itself when plugged in.`,
              )
            : tr('Ningún controlador elegido.', 'No controller chosen.')}
      </small>

      {connected && midi.preset && (
        <div className="learn-box">
          <b>
            {tr('Reconocido', 'Recognised')}: {midi.preset.vendor} {midi.preset.name}
          </b>
          <small>
            {tr(
              `El preset da función a ${midi.preset.mapped} de sus ${midi.preset.controls} controles. Cualquier control se puede aprender de nuevo encima.`,
              `The preset maps ${midi.preset.mapped} of its ${midi.preset.controls} controls. Any control can be learned again on top.`,
            )}
          </small>
          <details>
            <summary>{tr('Notas del equipo', 'Notes on this unit')}</summary>
            <ul>
              {midi.preset.notes.map((note) => (
                <li key={note}>{note}</li>
              ))}
            </ul>
          </details>
          {confirming === 'preset' ? (
            <span>
              <button
                className="primary"
                onClick={() => {
                  setConfirming(null);
                  call('midi_apply_preset').catch(onError);
                }}
              >
                {tr(
                  'Sí, reemplazar mis mapeos de este equipo',
                  'Yes, replace my mappings for this unit',
                )}
              </button>{' '}
              <button onClick={() => setConfirming(null)}>{tr('Cancelar', 'Cancel')}</button>
            </span>
          ) : (
            <button onClick={() => setConfirming('preset')}>
              {tr('Volver al preset', 'Restore the preset')}
            </button>
          )}
        </div>
      )}
      {connected && !midi.preset && !port?.preset && (
        <p className="small-print">
          {tr(
            'curado no conoce este equipo todavía. Usa el asistente o explora control por control; si es un teclado o un controlador de pads, el asistente de pads asigna samples y hot cues.',
            'curado does not know this unit yet. Use the guided setup or explore control by control; for a keyboard or a pad controller, the pad setup maps samples and hot cues.',
          )}
        </p>
      )}

      <h4>{tr('Explorar', 'Explore')}</h4>
      <div className="learn-box">
        <button
          disabled={!connected || baseline !== null}
          onClick={() => {
            setExploring(true);
            call('midi_baseline').catch(onError);
          }}
        >
          {baseline !== null
            ? tr(
                `No toques nada… ${Math.ceil(baseline / 1000)} s`,
                `Touch nothing… ${Math.ceil(baseline / 1000)} s`,
              )
            : tr('Medir ruido (3 s sin tocar nada)', 'Measure noise (3 s, touch nothing)')}
        </button>
        {midi.noisy.length > 0 && (
          <small>
            {tr('Se mueven solos', 'Moving by themselves')}:{' '}
            {midi.noisy.map(midiKeyLabel).join(', ')}
          </small>
        )}
        <small>
          {midi.detected
            ? `${tr('Último control', 'Last control')}: ${detectedLabel(midi.detected)}`
            : (midi.lastMessage ??
              tr('Mueve un control para ver qué envía.', 'Move a control to see what it sends.'))}
        </small>
        <button disabled={!connected} onClick={() => setExploring(!exploring)}>
          {exploring
            ? tr('Ocultar controles vistos', 'Hide controls seen')
            : tr('Ver controles vistos', 'Show controls seen')}
        </button>
        {exploring && (
          <div className="midi-mappings">
            {seen.map((s) => {
              const bound = boundTo(s);
              return (
                <div key={midiKeyLabel(s.key)}>
                  <span>
                    <b>{midiKeyLabel(s.key)}</b>
                    <small>
                      {modeLabel(s.mode, s.lsb, s.key.kind)} · {s.count}{' '}
                      {tr('mensajes', 'messages')} · {s.min}–{s.max}
                      {s.noisy ? ` · ${tr('se mueve solo', 'moves by itself')}` : ''}
                      {bound
                        ? ` → ${targetLabel(bound.action)}`
                        : ` · ${tr('sin función', 'unmapped')}`}
                    </small>
                  </span>
                </div>
              );
            })}
          </div>
        )}
      </div>

      <h4>{tr('Aprender', 'Learn')}</h4>
      <div className="learn-box">
        {wizard && step ? (
          <>
            <b>
              {tr('Paso', 'Step')} {wizard.index + 1} / {wizard.steps.length}: {step.ask}
            </b>
            <small>
              {tr(
                'Muévelo y suéltalo. Si tu equipo no lo tiene, salta este paso.',
                'Move it and let go. If your unit has none, skip this step.',
              )}
            </small>
            <span>
              <button onClick={() => setWizard({ ...wizard, index: wizard.index + 1 })}>
                {tr('Saltar', 'Skip')}
              </button>{' '}
              <button onClick={stopWizard}>{tr('Terminar', 'Finish')}</button>
            </span>
          </>
        ) : (
          <>
            <span>
              <button
                disabled={!connected}
                onClick={() => setWizard({ steps: djWizard(), index: 0 })}
              >
                {tr('Asistente: controlador DJ', 'Guided: DJ controller')}
              </button>{' '}
              <button
                disabled={!connected}
                onClick={() => setWizard({ steps: padWizard(), index: 0 })}
              >
                {tr('Asistente: teclado o pads', 'Guided: keyboard or pads')}
              </button>
            </span>
            <label>
              {tr('Acción', 'Action')}{' '}
              <select value={target} onChange={(e) => setTarget(e.target.value)}>
                {targets.map((t) => (
                  <option key={targetId(t)} value={targetId(t)}>
                    {targetLabel(t, false)}
                  </option>
                ))}
              </select>
            </label>
            {CUE_KINDS.includes(chosen.kind) && (
              <select
                aria-label={tr('Número de cue', 'Cue number')}
                value={cue}
                onChange={(e) => setCue(+e.target.value)}
              >
                {[0, 1, 2, 3, 4, 5, 6, 7].map((i) => (
                  <option key={i} value={i}>
                    Cue {i + 1}
                  </option>
                ))}
              </select>
            )}
            <button className="primary" disabled={!connected || !!midi.learning} onClick={learn}>
              {tr('Aprender siguiente control', 'Learn next control')}
            </button>
          </>
        )}
        {midi.learning && (
          <>
            <small>
              {tr('Esperando', 'Waiting for')} · {targetLabel(midi.learning)}.{' '}
              {tr(
                'Mueve el control y suéltalo: curado detecta si es botón, fader, perilla o jog.',
                'Move the control and let go: curado tells a button from a fader, knob or jog.',
              )}
            </small>
            {!wizard && (
              <button onClick={() => call('midi_learn', { action: null }).catch(onError)}>
                {tr('Cancelar aprendizaje', 'Cancel learn')}
              </button>
            )}
          </>
        )}
        {midi.learned && !midi.learning && (
          <small>
            {tr('Aprendido', 'Learned')}: {detectedLabel(midi.learned)}
          </small>
        )}
      </div>

      <h4>
        {tr('Mapeos', 'Mappings')} {connected ? `· ${connected}` : ''}
      </h4>
      {connected && own.length > 0 && (
        <span>
          {confirming === 'clear' ? (
            <>
              <button
                className="primary"
                onClick={() => {
                  setConfirming(null);
                  call('midi_clear_device').catch(onError);
                }}
              >
                {tr(`Sí, borrar los ${own.length}`, `Yes, remove all ${own.length}`)}
              </button>{' '}
              <button onClick={() => setConfirming(null)}>{tr('Cancelar', 'Cancel')}</button>
            </>
          ) : (
            <button onClick={() => setConfirming('clear')}>
              {tr('Borrar los mapeos de este equipo', 'Remove this unit’s mappings')}
            </button>
          )}
        </span>
      )}
      <div className="midi-mappings">
        {[...own, ...others].map(({ binding: b, index }) => (
          <div key={`${b.device}:${b.status}:${b.channel}:${b.number}:${index}`}>
            <span>
              <b>{b.label ?? (b.device === connected ? midiKeyLabel(bindingKey(b)) : b.device)}</b>
              <small>
                {b.label ? `${midiKeyLabel(bindingKey(b))} · ` : ''}
                {modeLabel(b.mode, b.lsb, bindingKey(b).kind)} → {targetLabel(b.action)}
                {b.device !== connected ? ` · ${b.device}` : ''}
              </small>
            </span>
            <button
              aria-label={tr('Eliminar mapeo MIDI', 'Remove MIDI mapping')}
              onClick={() => call('midi_remove', { index }).catch(onError)}
            >
              <Trash2 size={14} />
            </button>
          </div>
        ))}
      </div>
      {midi.error && <p className="error-text">{midi.error}</p>}
      <p className="small-print">
        {tr(
          'Pendiente: luces del controlador (LED), MIDI Clock y SysEx. Un fader que se mueve solo se filtra; si aun así molesta, borra su mapeo.',
          'Not yet: controller lights (LEDs), MIDI Clock and SysEx. A fader that moves by itself is filtered; if it still gets in the way, remove its mapping.',
        )}
      </p>
    </div>
  );
}
