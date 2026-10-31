import { useLayoutEffect, useRef, useState } from 'react';
import type { CSSProperties, ReactNode } from 'react';
import { Play, Pause } from 'lucide-react';
import { control, parameter, useSnapshot } from '../lib/bridge';
import { actionKey } from '../lib/feedback';
import { tr } from '../lib/language';
import { consoleScale } from '../lib/console-fit';
import { samplePadIntent } from '../lib/console-pads';
import { EMPTY_SLOT, remaining } from '../lib/types';
import type { KeyBinding, Library, Slot, Track } from '../lib/types';
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
  children,
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
  children: ReactNode;
  samples: Slot[];
  tracks: Track[];
  selected: number | null;
  onLoad: (slot: number, id: number) => void;
  loadingSlots: number[];
}) {
  const name = index ? 'B' : 'A';
  const tone = index ? 'b' : 'a';
  const empty = !track || loading;
  const [padMode, setPadMode] = useState<'cue' | 'samples'>('cue');
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
          <Soon>REVERSE</Soon>
          <Soon>SLIP</Soon>
          <Soon>JOG MODE</Soon>
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
        <Soon>1/2X</Soon>
        <Soon>2X</Soon>
      </div>
      <div className="rx-jog-area">
        <div className="rx-jog-side">
          <Soon>VINYL SPEED</Soon>
          <Soon dial>JOG ADJUST</Soon>
        </div>
        <button
          type="button"
          className="rx-jog"
          disabled={empty}
          aria-label={tr(
            `Plato deck ${name}: reproducir o pausar; scratch pendiente`,
            `Deck ${name} platter: play or pause; scratch unavailable`,
          )}
          title={tr(
            'Clic: reproducir o pausar · scratch próximamente',
            'Click: play or pause · scratch coming soon',
          )}
          data-action={act(index, 'toggle')}
          onClick={() => send('toggle')}
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
          <Soon>TEMPO RANGE</Soon>
          <Slider
            label="TEMPO"
            action={act(index, 'rate')}
            tone={tone}
            min={0.5}
            max={1.5}
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
            <button
              type="button"
              aria-pressed={padMode === 'cue'}
              onClick={() => setPadMode('cue')}
            >
              HOT CUE
            </button>
            <button
              type="button"
              aria-pressed={padMode === 'samples'}
              onClick={() => setPadMode('samples')}
            >
              SAMPLER
            </button>
            <Soon>BEAT LOOP</Soon>
            <Soon>BEAT JUMP</Soon>
          </div>
          <div className="rx-pads">
            {Array.from({ length: 8 }, (_, cue) =>
              padMode === 'samples' ? (
                cue < 4 ? (
                  (() => {
                    const sampleSlot = cue + 2;
                    const sample = samples[cue];
                    const sampleTrack = tracks.find((t) => t.id === sample?.id);
                    const instruction = tr(
                      'Clic: reproducir · Shift + clic: cargar selección',
                      'Click: play · Shift-click: load selection',
                    );
                    return (
                      <button
                        key={cue}
                        className={`rx-pad rx-sample-pad ${sample?.playing ? 'is-set' : ''}`}
                        disabled={
                          loadingSlots.includes(sampleSlot) || (!sampleTrack && selected === null)
                        }
                        aria-label={`${tr('Sample', 'Sample')} ${cue + 1} · ${sampleTrack?.title ?? tr('vacío', 'empty')} · ${instruction}`}
                        title={`${sampleTrack?.title ?? tr('Vacío', 'Empty')} · ${instruction}`}
                        data-action={act(sampleSlot, 'trigger')}
                        onClick={(e) => {
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
                  })()
                ) : (
                  <Soon key={cue}>{`SAMPLE ${cue + 1}`}</Soon>
                )
              ) : cue < 3 ? (
                <button
                  key={cue}
                  className={`rx-pad ${slot.cues[cue] > 0 ? 'is-set' : ''}`}
                  disabled={empty}
                  title={tr(
                    'Clic: saltar y reproducir · Shift + clic: guardar',
                    'Click: jump and play · Shift-click: save',
                  )}
                  aria-label={`Deck ${name} · Hot cue ${cue + 1}`}
                  data-action={`${act(index, 'cue', cue)} ${act(index, 'setCue', cue)}`}
                  onClick={(e) => send(e.shiftKey ? 'setCue' : 'cue', cue)}
                >
                  <span>{String.fromCharCode(65 + cue)}</span>
                  {keyFor(keys, index, 'cue', cue, true)}
                </button>
              ) : (
                <Soon key={cue}>{`HOT CUE ${String.fromCharCode(65 + cue)}`}</Soon>
              ),
            )}
          </div>
          <div className="rx-pad-footer">
            PERFORMANCE PADS{' '}
            <small>
              {padMode === 'cue'
                ? tr('3 CUES DISPONIBLES', '3 CUES AVAILABLE')
                : tr('4 SAMPLES COMPARTIDOS', '4 SHARED SAMPLES')}
            </small>
          </div>
          {padMode === 'samples' && (
            <div className="rx-sample-help">
              {tr(
                'Clic en vacío: cargar selección · ⇧ clic: reemplazar',
                'Click empty: load selection · ⇧ click: replace',
              )}
            </div>
          )}
        </div>
      </div>
      <div className="rx-deck-extras">{children}</div>
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
      <Soon dial>TRIM</Soon>
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
      <Soon dial>COLOR</Soon>
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

function KeyboardGuide({
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

export function HardwareConsole({
  library,
  keys,
  loading,
  onError,
  onLibrary,
  selected,
  onLoad,
  onTrigger,
}: {
  library: Library;
  keys: KeyBinding[];
  loading: number[];
  onError: (e: unknown) => void;
  onLibrary: (data: Library) => void;
  selected: number | null;
  onLoad: (slot: number, id: number) => void;
  onTrigger: (binding: KeyBinding) => void;
}) {
  const { audio } = useSnapshot();
  const scrollRef = useRef<HTMLDivElement>(null);
  const bodyRef = useRef<HTMLDivElement>(null);
  const scaleRef = useRef(1);
  const [scale, setScale] = useState(1);
  useLayoutEffect(() => {
    const scroll = scrollRef.current;
    const body = bodyRef.current;
    const sheet = scroll?.closest<HTMLElement>('.sheet-scroll');
    if (!scroll || !body || !sheet) return;
    const measure = () => {
      const bounds = body.getBoundingClientRect();
      const sheetBounds = sheet.getBoundingClientRect();
      const scrollBounds = scroll.getBoundingClientRect();
      const topInSheet = scrollBounds.top - sheetBounds.top + sheet.scrollTop;
      const availableHeight = Math.max(1, sheet.clientHeight - topInSheet - 18);
      // Read the applied zoom from the rendered box, rather than a pending React state update.
      const appliedScale = bounds.width / 1170;
      const next = consoleScale(
        scroll.clientWidth - 8,
        availableHeight,
        1170,
        bounds.height / appliedScale,
      );
      if (Math.abs(next - scaleRef.current) >= 0.002) {
        scaleRef.current = next;
        setScale(next);
      }
    };
    const observer = new ResizeObserver(measure);
    observer.observe(sheet);
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
  const samples = [2, 3, 4, 5].map((i) => audio?.slots[i] ?? EMPTY_SLOT);
  const tracks = slots.map((slot) => library.tracks.find((track) => track.id === slot.id));
  const deckExtras = (index: number) => (
    <>
      <DeckExtensions index={index} slot={slots[index]} onError={onError} />
      <MusicalControls
        index={index}
        slot={slots[index]}
        disabled={!tracks[index] || loading.includes(index)}
        onError={onError}
        onLibrary={onLibrary}
      />
      {index === 0 ? (
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
      ) : (
        <KeyboardGuide keys={keys} onTrigger={onTrigger} />
      )}
    </>
  );
  return (
    <>
      <p className="rx-explanation">
        {tr(
          'Cabina inspirada en RX3 · controles atenuados: próximamente · el plato aún no hace scratch',
          'RX3-inspired console · dimmed controls: coming soon · platter scratch is not available yet',
        )}
      </p>
      <div className="rx-stage">
        <div
          ref={scrollRef}
          className="rx-scroll"
          role="region"
          aria-label={tr('Cabina de mezcla desplazable', 'Scrollable mixing console')}
          tabIndex={0}
        >
          <div ref={bodyRef} className="rx-body" style={{ zoom: scale }}>
            <div className="rx-head">
              <b>curado</b>
              <span>ALL-IN-ONE DJ SYSTEM</span>
              <span>2 CHANNEL PERFORMANCE</span>
            </div>
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
              >
                {deckExtras(0)}
              </RxDeck>
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
                  <Soon>BACK</Soon>
                  <Soon dial>BROWSE</Soon>
                  <Soon>TAG TRACK</Soon>
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
                    <strong>SOUND COLOR FX</strong>
                    {['SPACE', 'DUB ECHO', 'SWEEP', 'NOISE', 'CRUSH', 'FILTER'].map((fx) => (
                      <Soon key={fx}>{fx}</Soon>
                    ))}
                    <Soon dial>PARAMETER</Soon>
                    <strong>HEADPHONES</strong>
                    <Soon>MASTER CUE</Soon>
                    <Soon>LINK CUE</Soon>
                    <Soon dial>MIXING</Soon>
                    <Soon dial>LEVEL</Soon>
                    <small>{tr('PRÓXIMAMENTE', 'COMING SOON')}</small>
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
                    <strong>BEAT FX</strong>
                    <Soon>BEAT ◀ ▶</Soon>
                    <Soon>TAP</Soon>
                    <Soon dial>SELECT</Soon>
                    <Soon dial>CHANNEL</Soon>
                    <Soon dial>TIME</Soon>
                    <Soon dial>LEVEL / DEPTH</Soon>
                    <Soon>ON / OFF</Soon>
                    <small>{tr('PRÓXIMAMENTE', 'COMING SOON')}</small>
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
              >
                {deckExtras(1)}
              </RxDeck>
            </div>
            <div className="rx-bottom-edge">
              <span>curado</span>
              <span>2 DECKS / 2 CHANNELS</span>
              <span>{tr('SUPERFICIE DE PRÁCTICA', 'PRACTICE SURFACE')}</span>
            </div>
          </div>
        </div>
      </div>
      <details className="rx-extra">
        <summary>{tr('Mixer completo de curado', 'Full curado mixer')}</summary>
        <div className="rx-extra-content">
          <Mixer keys={keys} onError={onError} />
        </div>
      </details>
    </>
  );
}
