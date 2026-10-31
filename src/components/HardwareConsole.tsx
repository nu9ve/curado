import { createContext, useContext, useEffect, useLayoutEffect, useRef, useState } from 'react';
import type { CSSProperties, ReactNode } from 'react';
import { Play, Pause } from 'lucide-react';
import { control, parameter, useDevices, useSnapshot } from '../lib/bridge';
import { actionKey, useHud } from '../lib/feedback';
import { tr } from '../lib/language';
import { consoleScale } from '../lib/console-fit';
import {
  LOOP_PADS,
  BEAT_DIVISIONS,
  BEAT_FX,
  BEAT_FX_ORDER,
  beatLabel,
  COLOR_FX,
  COLOR_FX_LABELS,
  neighborCue,
  nextDivision,
  nextTempoRange,
  PITCH_FX,
  pitchLabel,
  SAMPLE_PADS,
  samplePadIntent,
  tapTempo,
} from '../lib/console-pads';
import type { PadMode } from '../lib/console-pads';
import { cueColor, useCueInfo } from '../lib/cues';
import { useCover } from '../lib/covers';
import { haptic } from '../lib/haptics';
import { keyLabel, transposeKey } from '../lib/music';
import { useLivePosition } from '../lib/position';
import { EMPTY_SLOT, remaining } from '../lib/types';
import type { BeatFxState, KeyBinding, Library, LibraryEdit, Slot, Track } from '../lib/types';
import { Knob, LimiterLamp, Meter, Slider, tabbed } from './Controls';
import { PerformancePads, useJog } from './Performance';
import { MusicalControls } from './MusicalControls';
import { KeyboardMap } from './KeyboardMap';
import { WaveOverview, ZoomWave } from './Waveform';
import { bindingLabel, TOGGLE_KINDS } from '../lib/keyboard';
import {
  overlayPlacement,
  setTrackingOverlay,
  useTrackedPoint,
  useTrackingControl,
  useTrackingOverlay,
  useTrackingPreview,
  useTrackingStatus,
} from '../lib/tracking';
import type { TrackMode } from '../lib/tracking';
import { XY_MODES, xyMoves, xyPoint, xyRelease } from '../lib/xy';
import type { XYMode } from '../lib/xy';
import { UnavailableControl } from './UnavailableControl';
import './hardware-console.css';

/** The console's own size, taken from the hardware's proportions. It is scaled to fit as a whole. */
const BODY_WIDTH = 1430;
const act = (slot: number, kind: string, value = 0) => actionKey({ slot, kind, value });
const eqValue = (v: number) => (v > 0 ? `${(20 * Math.log10(v)).toFixed(0)} dB` : '−∞');
const padName = (index: number) => String.fromCharCode(65 + index);
/** Controls sit where they do on the hardware, by the center of each. */
const at = (x: number, y: number): CSSProperties => ({ left: x, top: y });

const KeysContext = createContext<KeyBinding[]>([]);
/**
 * The key for a control, printed on it. Any control saying what it does gets one; digits win for
 * a hot cue, since those are the pad's own keys.
 */
function KeyBadge({ action }: { action?: string }) {
  const keys = useContext(KeysContext);
  if (!action) return null;
  for (const token of action.split(' ')) {
    const [slot, kind, value] = token.split(':');
    const matches = keys.filter(
      (key) =>
        key.action.slot === +slot &&
        key.action.kind === kind &&
        (TOGGLE_KINDS.has(kind) || key.action.value === +value),
    );
    const binding = matches.find((key) => key.code.startsWith('Digit')) ?? matches[0];
    if (binding) return <kbd className="rx-badge">{bindingLabel(binding)}</kbd>;
  }
  return null;
}

/** A visible, focusable reference control, never an audio action. */
function Soon({
  children,
  dial = false,
  x,
  y,
}: {
  children: string;
  dial?: boolean;
  x: number;
  y: number;
}) {
  return (
    <div className="rx-at" style={at(x, y)}>
      <UnavailableControl label={children} dial={dial} />
    </div>
  );
}

function Key({
  label,
  x,
  y,
  on,
  disabled,
  action,
  title,
  wide,
  onClick,
}: {
  label: string;
  x: number;
  y: number;
  on?: boolean;
  disabled?: boolean;
  action?: string;
  title?: string;
  wide?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      className={`rx-at rx-key ${wide ? 'wide' : ''}`}
      style={at(x, y)}
      aria-pressed={on}
      disabled={disabled}
      data-action={action}
      title={title}
      onClick={onClick}
    >
      {label}
      <KeyBadge action={action} />
    </button>
  );
}

/** The small round lit buttons around the decks, with their name printed above. */
function Lamp({
  label,
  x,
  y,
  on,
  disabled,
  action,
  title,
  size = 20,
  onClick,
}: {
  label: string;
  x: number;
  y: number;
  on?: boolean;
  disabled?: boolean;
  action?: string;
  title?: string;
  size?: number;
  onClick: () => void;
}) {
  return (
    <div className="rx-at rx-lamp-wrap" style={at(x, y)}>
      <span>{label}</span>
      <button
        className={`rx-lamp ${on ? 'is-on' : ''}`}
        style={{ width: size, height: size }}
        aria-label={label}
        aria-pressed={on}
        disabled={disabled}
        data-action={action}
        title={title}
        onClick={onClick}
      />
      <KeyBadge action={action} />
    </div>
  );
}

/**
 * A dial that moves through a list: drag it, scroll on it, use the arrows, or press a name
 * printed around it.
 */
function StepDial({
  label,
  x,
  y,
  index,
  names,
  size = 36,
  reach = 0,
  action,
  onStep,
}: {
  label: string;
  x: number;
  y: number;
  index: number;
  names: string[];
  size?: number;
  reach?: number;
  action?: string;
  onStep: (next: number) => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const latest = useRef({ index, onStep, count: names.length });
  latest.current = { index, onStep, count: names.length };
  const drag = useRef<number | null>(null);
  const move = (delta: number) => {
    const { index, onStep, count } = latest.current;
    const next = Math.max(0, Math.min(count - 1, index + delta));
    if (next !== index) haptic('level');
    onStep(next);
  };
  useEffect(() => {
    const element = ref.current;
    if (!element) return;
    const wheel = (e: WheelEvent) => {
      e.preventDefault();
      move(e.deltaY > 0 ? 1 : -1);
    };
    element.addEventListener('wheel', wheel, { passive: false });
    return () => element.removeEventListener('wheel', wheel);
  }, []);
  const angle = (i: number) => (names.length > 1 ? -135 + (i * 270) / (names.length - 1) : 0);
  return (
    <div className="rx-at rx-step" style={at(x, y)}>
      {label && (
        <span className="rx-step-title" style={{ bottom: size / 2 + 3 }}>
          {label}
        </span>
      )}
      {reach > 0 &&
        names.map((name, i) => {
          // Two columns, bottom left around to bottom right, so long names never collide.
          const half = Math.ceil(names.length / 2);
          const left = i < half;
          const row = left ? half - 1 - i : i - half;
          const spacing = names.length > 6 ? 9.5 : 18;
          const across = (Math.max(half, names.length - half) - 1) * spacing;
          return (
            <button
              key={name}
              className={`rx-step-name ${i === index ? 'is-on' : ''} ${left ? 'to-left' : ''}`}
              style={at(left ? -reach : reach, row * spacing - across / 2)}
              data-action={action}
              onClick={() => onStep(i)}
            >
              {name}
            </button>
          );
        })}
      <div
        ref={ref}
        className="rx-step-dial"
        role="spinbutton"
        tabIndex={0}
        aria-label={label}
        aria-valuenow={index}
        aria-valuetext={names[index]}
        aria-valuemin={0}
        aria-valuemax={names.length - 1}
        title={names[index]}
        data-action={action}
        style={
          {
            width: size,
            height: size,
            '--angle': `${angle(index)}deg`,
          } as CSSProperties
        }
        onPointerDown={(e) => {
          e.currentTarget.setPointerCapture(e.pointerId);
          drag.current = e.clientY;
        }}
        onPointerMove={(e) => {
          if (drag.current === null || Math.abs(e.clientY - drag.current) < 12) return;
          move(e.clientY > drag.current ? 1 : -1);
          drag.current = e.clientY;
        }}
        onPointerUp={() => {
          drag.current = null;
        }}
        onKeyDown={(e) => {
          if (!tabbed()) return;
          const delta = { ArrowUp: -1, ArrowLeft: -1, ArrowDown: 1, ArrowRight: 1 }[e.key];
          if (!delta) return;
          e.preventDefault();
          e.stopPropagation();
          move(delta);
        }}
      />
    </div>
  );
}

/**
 * A loop with one press. On the hardware the four-beat button is on the deck; here the lengths
 * sit together in the empty stretch above it, each deck's in its own color.
 */
function AutoLoop({
  index,
  slot,
  x,
  onError,
}: {
  index: number;
  slot: Slot;
  x: number;
  onError: (e: unknown) => void;
}) {
  const send = (kind: string, value = 0) => control(index, kind, value).catch(onError);
  const disabled = !slot.id || !slot.music.bpm;
  const beats = slot.loopBeats;
  return (
    <div
      className={`rx-at rx-autoloop ${index ? 'tone-b' : ''}`}
      role="group"
      aria-label={`Auto loop deck ${index + 1}`}
      style={at(x, 28)}
    >
      <button
        className="rx-autoloop-main"
        aria-pressed={slot.looping}
        disabled={disabled}
        data-action={act(index, 'autoLoop')}
        title={tr(
          'Empieza o termina un loop desde aquí, sin marcar IN y OUT',
          'Starts or ends a loop from here, without marking IN and OUT',
        )}
        onClick={() => send('autoLoop')}
      >
        <span>AUTO LOOP</span>
        <KeyBadge action={act(index, 'autoLoop')} />
        <b>{beatLabel(beats)}</b>
      </button>
      <div>
        {LOOP_PADS.map((value) => (
          <button
            key={value}
            aria-pressed={slot.looping && beats === value}
            data-next={beats === value}
            disabled={disabled}
            aria-label={`Auto loop ${beatLabel(value)} ${tr('beats', 'beats')}`}
            data-action={act(index, 'loopBeats', value)}
            onClick={() => send('loopBeats', value)}
          >
            {beatLabel(value)}
          </button>
        ))}
      </div>
    </div>
  );
}

/** Its own component, so the fast position stream redraws this and not the whole deck. */
function JogDisc({ index, slot, name }: { index: number; slot: Slot; name: string }) {
  const live = useLivePosition(index, slot.position);
  const progress = slot.duration ? Math.min(1, live / slot.duration) : 0;
  const { element, image } = useCover(slot.id ? { kind: 'track', id: slot.id } : null);
  return (
    <>
      <span className="rx-jog-disc" style={{ '--turn': `${live * 200}deg` } as CSSProperties}>
        <i />
      </span>
      <span ref={element} className="rx-jog-lcd" style={{ '--p': progress } as CSSProperties}>
        {image && <img alt="" src={image} />}
        <b>{name}</b>
        <em>{remaining({ position: live, duration: slot.duration })}</em>
        <small>{slot.sync ? 'SYNC' : slot.playing ? '▶' : 'Ⅱ'}</small>
      </span>
    </>
  );
}

/** The CUE button's own key, not the hot cue A key it shares an action with. */
function CueKey({ index }: { index: number }) {
  const keys = useContext(KeysContext);
  const binding =
    keys.find(
      (key) =>
        key.action.slot === index &&
        key.action.kind === 'cue' &&
        key.action.value === 0 &&
        !key.code.startsWith('Digit'),
    ) ?? null;
  return binding ? <kbd className="rx-badge">{bindingLabel(binding)}</kbd> : null;
}

function RxDeck({
  index,
  slot,
  track,
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
  const name = index ? '2' : '1';
  const tone = index ? 'b' : 'a';
  const empty = !track || loading;
  const [padMode, setPadMode] = useState<PadMode>('cue');
  const [vinyl, setVinyl] = useState(true);
  const [sensitivity, setSensitivity] = useState(1);
  const [tempoRange, setTempoRange] = useState(0.1);
  const [shift, setShift] = useState(false);
  const jog = useJog(index, slot.playing, vinyl, sensitivity, onError);
  const cueInfo = useCueInfo(slot.id || undefined);
  const send = (kind: string, value = 0) => control(index, kind, value).catch(onError);
  const loopSet = slot.loopOut > slot.loopIn;
  const neighbor = (direction: 1 | -1) => {
    const at = tracks.findIndex((t) => t.id === track?.id);
    return tracks[at + direction];
  };
  const callCue = (direction: 1 | -1) => {
    if (slot.looping) {
      send('loopScale', direction > 0 ? 2 : 0.5);
      return;
    }
    const target = neighborCue(slot.cues, slot.position, direction);
    if (target !== null) send('seek', target);
  };
  const modes: { mode: PadMode; label: string; second: PadMode; secondLabel: string }[] = [
    { mode: 'cue', label: 'HOT CUE', second: 'keyboard', secondLabel: 'KEYBOARD' },
    { mode: 'loop', label: 'BEAT LOOP', second: 'keys', secondLabel: 'KEY SHIFT' },
    {
      mode: 'roll',
      label: 'SLIP LOOP',
      second: padMode === 'release' ? 'padfx' : 'release',
      secondLabel: padMode === 'padfx' ? 'PAD FX' : 'RELEASE FX',
    },
    { mode: 'jump', label: 'BEAT JUMP', second: 'samples', secondLabel: 'SAMPLER' },
  ];
  return (
    <section className={`rx-deck rx-deck-${index} tone-${tone}`} aria-label={`Deck ${name}`}>
      <Lamp
        label="IN"
        x={96}
        y={33}
        size={30}
        on={slot.looping}
        disabled={empty}
        action={act(index, 'loopIn')}
        onClick={() => send('loopIn')}
      />
      <Lamp
        label="OUT"
        x={146}
        y={33}
        size={30}
        on={slot.looping}
        disabled={empty}
        action={act(index, 'loopOut')}
        onClick={() => send('loopOut')}
      />
      <Lamp
        label="RELOOP/EXIT"
        x={205}
        y={33}
        on={slot.looping}
        disabled={empty || !loopSet}
        action={act(index, 'loop')}
        onClick={() => send('loop')}
      />
      <Lamp
        label="1/2X"
        x={291}
        y={33}
        size={18}
        disabled={empty}
        title={tr(
          'En loop: reduce a la mitad · sin loop: cue anterior',
          'In a loop: halves it · otherwise: previous cue',
        )}
        action={act(index, 'loopScale', 0.5)}
        onClick={() => callCue(-1)}
      />
      <Lamp
        label="2X"
        x={334}
        y={33}
        size={18}
        disabled={empty}
        title={tr(
          'En loop: duplica · sin loop: cue siguiente',
          'In a loop: doubles it · otherwise: next cue',
        )}
        action={act(index, 'loopScale', 2)}
        onClick={() => callCue(1)}
      />
      <Lamp
        label="DELETE"
        x={380}
        y={33}
        disabled={empty}
        title={tr('Borra el último hot cue guardado', 'Deletes the last saved hot cue')}
        onClick={() => {
          const last = slot.cues.map((cue) => cue > 0).lastIndexOf(true);
          if (last >= 0) send('clearCue', last);
        }}
      />
      <Lamp
        label="MEMORY"
        x={425}
        y={33}
        disabled={empty}
        title={tr(
          'Guarda la posición en el primer hot cue libre',
          'Saves the position to the first free hot cue',
        )}
        onClick={() => {
          const free = slot.cues.findIndex((cue) => cue <= 0);
          if (free >= 0) send('setCue', free);
        }}
      />

      <Lamp
        label="SLIP"
        x={53}
        y={92}
        on={slot.slip}
        disabled={empty}
        action={act(index, 'slip', slot.slip ? 0 : 1)}
        onClick={() => send('slip', slot.slip ? 0 : 1)}
      />
      <Lamp
        label="QUANTIZE"
        x={105}
        y={92}
        on={slot.quantize}
        disabled={empty}
        action={act(index, 'quantize', slot.quantize ? 0 : 1)}
        onClick={() => send('quantize', slot.quantize ? 0 : 1)}
      />
      <span className="rx-at rx-print" style={at(53, 124)}>
        DIRECTION
      </span>
      <Key
        label="REV"
        x={53}
        y={143}
        on={slot.reverse}
        disabled={empty}
        action={act(index, 'reverse', slot.reverse ? 0 : 1)}
        onClick={() => send('reverse', slot.reverse ? 0 : 1)}
      />
      <span className="rx-at rx-print" style={at(53, 190)}>
        TRACK SEARCH
      </span>
      <div className="rx-at rx-pair" style={at(53, 212)}>
        <button
          aria-label={tr('Pista anterior', 'Previous track')}
          disabled={!track || !neighbor(-1)}
          onClick={() => {
            const target = neighbor(-1);
            if (target) onLoad(index, target.id);
          }}
        >
          ⏮
        </button>
        <button
          aria-label={tr('Pista siguiente', 'Next track')}
          disabled={!track || !neighbor(1)}
          onClick={() => {
            const target = neighbor(1);
            if (target) onLoad(index, target.id);
          }}
        >
          ⏭
        </button>
      </div>
      <span className="rx-at rx-print" style={at(53, 246)}>
        SEARCH
      </span>
      <div className="rx-at rx-pair" style={at(53, 268)}>
        <button
          aria-label={tr('Un beat atrás', 'One beat back')}
          disabled={empty}
          data-action={act(index, 'beatJump', -1)}
          onClick={() => send('beatJump', -1)}
        >
          ◀◀
          <KeyBadge action={act(index, 'beatJump', -1)} />
        </button>
        <button
          aria-label={tr('Un beat adelante', 'One beat forward')}
          disabled={empty}
          data-action={act(index, 'beatJump', 1)}
          onClick={() => send('beatJump', 1)}
        >
          ▶▶
          <KeyBadge action={act(index, 'beatJump', 1)} />
        </button>
      </div>
      <button
        className={`rx-at rx-key ${shift ? 'is-held' : ''}`}
        style={at(53, 335)}
        aria-pressed={shift}
        title={tr(
          'Mantén para las funciones secundarias de los pads',
          "Hold for the pads' second functions",
        )}
        onPointerDown={() => setShift(true)}
        onPointerUp={() => setShift(false)}
        onPointerLeave={() => setShift(false)}
      >
        SHIFT
      </button>

      <button
        type="button"
        ref={jog.ref}
        className={`rx-at rx-jog ${vinyl ? 'is-vinyl' : ''}`}
        style={at(254, 227)}
        disabled={empty}
        aria-label={tr(
          `Plato deck ${name}: arrastra en círculo para hacer scratch; el borde dobla el tempo`,
          `Deck ${name} platter: drag around to scratch; the edge bends the tempo`,
        )}
        title={tr(
          'Arriba: scratch (VINYL) · borde: doblar el tempo · rueda: doblar el tempo',
          'Top: scratch (VINYL) · edge: bend the tempo · scroll: bend the tempo',
        )}
        data-help="jog"
        {...jog.handlers}
        onPointerCancel={jog.handlers.onPointerUp}
      >
        <JogDisc index={index} slot={slot} name={name} />
      </button>
      <span className="rx-at rx-print rx-jog-side-label" style={at(128, 384)}>
        ← REV
      </span>
      <span className="rx-at rx-print rx-jog-side-label" style={at(380, 384)}>
        FWD +
      </span>

      <div className="rx-at rx-mini-knob" style={at(381, 92)}>
        <Knob
          label="JOG ADJUST"
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
      <div className="rx-at rx-mini-knob" style={at(456, 92)}>
        <Knob
          label="VINYL SPEED"
          tone={tone}
          action={act(index, 'brake')}
          max={3}
          value={slot.brake}
          format={(v) => (v < 0.01 ? 'OFF' : `${v.toFixed(1)} s`)}
          onChange={(v) => parameter(index, 'brake', v, onError)}
        />
      </div>
      <span className="rx-at rx-print" style={at(456, 132)}>
        JOG MODE
      </span>
      <Key
        label="VINYL"
        x={456}
        y={150}
        on={vinyl}
        title={tr(
          'Encendido: el plato hace scratch · apagado: dobla el tempo',
          'On: the platter scratches · off: it bends the tempo',
        )}
        onClick={() => setVinyl(!vinyl)}
      />
      <span className="rx-at rx-print" style={at(456, 180)}>
        BEAT SYNC
      </span>
      <Key
        label="SYNC"
        x={432}
        y={198}
        on={slot.sync}
        disabled={empty || masterDeck === index || !slot.music.bpm || !masterHasBpm}
        action={act(index, 'sync', slot.sync ? 0 : 1)}
        onClick={() => send('sync', slot.sync ? 0 : 1)}
      />
      <Key
        label="MASTER"
        x={474}
        y={198}
        on={masterDeck === index}
        disabled={empty}
        action={act(index, 'masterDeck')}
        onClick={() => send('masterDeck')}
      />
      <Lamp
        label={`TEMPO ±${Math.round(tempoRange * 100)}`}
        x={456}
        y={250}
        size={16}
        title={tr('Rango del fader de tempo', 'Tempo fader range')}
        onClick={() => setTempoRange(nextTempoRange(tempoRange))}
      />
      <Lamp
        label="MASTER TEMPO"
        x={456}
        y={296}
        size={16}
        on={slot.tuning.keyLock}
        disabled={empty}
        title={tr(
          'Key lock: el tono se conserva al cambiar el tempo',
          'Key lock: pitch holds when the tempo changes',
        )}
        action={act(index, 'keyLock', slot.tuning.keyLock ? 0 : 1)}
        onClick={() => send('keyLock', slot.tuning.keyLock ? 0 : 1)}
      />
      <div className="rx-at rx-tempo-fader" style={at(456, 440)}>
        <Slider
          label="TEMPO"
          action={act(index, 'rate')}
          tone={tone}
          min={1 - tempoRange}
          max={1 + tempoRange}
          origin={1}
          defaultValue={1}
          value={slot.tuning.rate}
          format={(v) => `${v >= 1 ? '+' : ''}${((v - 1) * 100).toFixed(1)}%`}
          onChange={(v) => parameter(index, 'rate', v, onError)}
        />
      </div>

      <button
        className="rx-at rx-cue"
        style={at(66, 431)}
        disabled={empty}
        title={tr('Salta al hot cue A y reproduce', 'Jumps to hot cue A and plays')}
        data-action={act(index, 'cue', 0)}
        onClick={() => send('cue', 0)}
      >
        CUE
        <CueKey index={index} />
      </button>
      <span className="rx-at rx-print" style={at(66, 474)}>
        PLAY/PAUSE
      </span>
      <button
        className={`rx-at rx-play ${slot.playing ? 'is-on' : ''}`}
        style={at(66, 520)}
        disabled={empty}
        aria-label={`${slot.playing ? tr('Pausar', 'Pause') : tr('Reproducir', 'Play')} deck ${name}`}
        data-action={act(index, 'toggle')}
        onClick={() => send('toggle')}
      >
        {slot.playing ? <Pause size={20} /> : <Play size={20} />}
        <KeyBadge action={act(index, 'toggle')} />
      </button>

      <div className="rx-pad-section">
        <div className="rx-pad-title">PAD MODE</div>
        <div className="rx-pad-modes">
          {modes.map(({ mode, label, second, secondLabel }) => (
            <div key={mode}>
              <button
                type="button"
                aria-pressed={padMode === mode}
                onClick={() => setPadMode(shift ? second : mode)}
              >
                {label}
              </button>
              <button
                type="button"
                className="rx-pad-second"
                aria-pressed={
                  padMode === second || (mode === 'roll' && ['release', 'padfx'].includes(padMode))
                }
                title={
                  mode === 'roll'
                    ? tr('Vuelve a pulsar para PAD FX', 'Press again for PAD FX')
                    : undefined
                }
                onClick={() => setPadMode(second)}
              >
                {secondLabel}
              </button>
            </div>
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
                      e.shiftKey || shift,
                      loadingSlots.includes(sampleSlot),
                    );
                    if (intent?.kind === 'load') onLoad(intent.slot, intent.id);
                    else if (intent?.kind === 'trigger')
                      control(intent.slot, 'trigger').catch(onError);
                  }}
                >
                  <span>S{cue + 1}</span>
                  <KeyBadge action={act(sampleSlot, 'trigger')} />
                  <small>
                    {loadingSlots.includes(sampleSlot)
                      ? tr('cargando', 'loading')
                      : (sampleTrack?.title ?? '')}
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
                    'Vacío: guarda aquí · guardado: salta y reproduce · SHIFT: borra',
                    'Empty: saves here · saved: jumps and plays · SHIFT: deletes',
                  )
                }
                aria-label={`Deck ${name} · Hot cue ${padName(cue)}${cueInfo.hot[cue]?.label ? ` · ${cueInfo.hot[cue]?.label}` : ''}`}
                data-action={`${act(index, 'cue', cue)} ${act(index, 'setCue', cue)}`}
                onClick={(e) => {
                  if (e.shiftKey || shift) send('clearCue', cue);
                  else send(slot.cues[cue] > 0 ? 'cue' : 'setCue', cue);
                }}
              >
                <span>{padName(cue)}</span>
                <KeyBadge action={act(index, 'cue', cue)} />
              </button>
            ))}
          {padMode !== 'cue' && padMode !== 'samples' && (
            <PerformancePads
              mode={padMode}
              index={index}
              slot={slot}
              disabled={empty}
              padClass="rx-pad"
              activeClass="is-set"
              onError={onError}
            />
          )}
        </div>
      </div>
    </section>
  );
}

function RxChannel({
  index,
  x,
  slot,
  hasCue,
  onError,
}: {
  index: number;
  x: number;
  slot: Slot;
  hasCue: boolean;
  onError: (e: unknown) => void;
}) {
  const tone = index ? 'b' : 'a';
  const send = (kind: string) => (v: number) => parameter(index, kind, v, onError);
  const place = (y: number) => ({ className: 'rx-at rx-channel-knob', style: at(x, y) });
  return (
    <>
      <div {...place(32)}>
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
      </div>
      {(['high', 'mid', 'low'] as const).map((kind, i) => (
        <div key={kind} {...place(86 + i * 54)}>
          <Knob
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
        </div>
      ))}
      <div className="rx-at rx-color-knob" style={at(x, 250)}>
        <Knob
          label={slot.colorFx ? COLOR_FX_LABELS[slot.colorFx - 1] : 'COLOR'}
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
      </div>
      <Key
        label="CUE"
        x={x}
        y={298}
        on={slot.cue}
        disabled={!hasCue}
        title={
          hasCue
            ? tr('Escucha este canal en los audífonos', 'Hears this channel in the headphones')
            : NO_CUE
        }
        action={act(index, 'channelCue', slot.cue ? 0 : 1)}
        onClick={() => control(index, 'channelCue', slot.cue ? 0 : 1).catch(onError)}
      />
      <b className="rx-at rx-channel-number" style={at(x, 322)}>
        {index + 1}
      </b>
      <div className="rx-at rx-channel-fader" style={at(x, 405)}>
        <Slider
          label=""
          tone={tone}
          action={act(index, 'gain')}
          max={1.5}
          defaultValue={0.8}
          value={slot.gain}
          onChange={send('gain')}
        />
      </div>
    </>
  );
}

function ColorFxBlock({ slots, onError }: { slots: Slot[]; onError: (e: unknown) => void }) {
  return (
    <>
      <div className="rx-color-box">
        {COLOR_FX.map((label, i) => {
          const both = slots.every((slot) => slot.colorFx === i + 1);
          const row = i < 3 ? 0 : 1;
          return (
            <div
              key={label}
              className="rx-color-button"
              style={{ gridColumn: (i % 3) + 1, gridRow: row === 0 ? 1 : 3 }}
            >
              <button
                aria-pressed={both}
                data-action={act(0, 'colorFx', i + 1)}
                title={tr(
                  'Aplica a los dos canales · usa 1 o 2 para uno solo',
                  'Applies to both channels · use 1 or 2 for just one',
                )}
                onClick={() => {
                  for (const index of [0, 1])
                    control(index, 'colorFx', both ? 0 : i + 1).catch(onError);
                }}
              >
                {label}
              </button>
              <div>
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
            </div>
          );
        })}
        <span>SOUND COLOR FX</span>
      </div>
      <div className="rx-at rx-color-params" style={at(55, 300)}>
        <span>PARAMETER</span>
        <div>
          {slots.map((slot, index) => (
            <Knob
              key={index}
              label={String(index + 1)}
              tone={index ? 'b' : 'a'}
              action={act(index, 'colorParam')}
              defaultValue={0.5}
              origin={0.5}
              value={slot.colorParam}
              onChange={(v) => parameter(index, 'colorParam', v, onError)}
            />
          ))}
        </div>
      </div>
    </>
  );
}

const NO_CUE = 'Requiere una salida de 4 canales: la 3 y la 4 van a los audífonos';
const CH_SELECT = ['1', '2', 'MASTER'];
/** One set of controls over three independent units, chosen with CH SELECT. */
function BeatFxBlock({
  x,
  unit,
  onUnit,
  state,
  onError,
}: {
  x: number;
  unit: number;
  onUnit: (unit: number) => void;
  state: BeatFxState;
  onError: (e: unknown) => void;
}) {
  const taps = useRef<number[]>([]);
  const send = (kind: string, value: number) => control(unit, kind, value).catch(onError);
  const order = BEAT_FX_ORDER.indexOf(state.kind);
  const division = BEAT_DIVISIONS.indexOf(state.beats);
  return (
    <>
      <span className="rx-at rx-section-title" style={at(x, 130)}>
        BEAT FX
      </span>
      <span className="rx-at rx-print" style={at(x, 146)}>
        BEAT
      </span>
      <div className="rx-at rx-pair" style={at(x, 164)}>
        <button
          aria-label={tr('División más corta', 'Shorter division')}
          data-action={act(unit, 'beatFxBeats', nextDivision(state.beats, -1))}
          onClick={() => send('beatFxBeats', nextDivision(state.beats, -1))}
        >
          ◀
        </button>
        <button
          aria-label={tr('División más larga', 'Longer division')}
          data-action={act(unit, 'beatFxBeats', nextDivision(state.beats, 1))}
          onClick={() => send('beatFxBeats', nextDivision(state.beats, 1))}
        >
          ▶
        </button>
      </div>
      <button
        className="rx-at rx-tap"
        style={at(x, 197)}
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
        TAP
      </button>
      <span className="rx-at rx-print" style={at(x, 220)}>
        {state.bpm ? state.bpm.toFixed(1) : 'AUTO'}
      </span>
      <StepDial
        label=""
        x={x}
        y={276}
        size={30}
        reach={19}
        index={order < 0 ? 1 : order}
        names={BEAT_FX_ORDER.map((kind) => BEAT_FX[kind])}
        action={act(unit, 'beatFx', state.kind)}
        onStep={(next) => send('beatFx', BEAT_FX_ORDER[next])}
      />
      <StepDial
        label="CH SELECT"
        x={x}
        y={350}
        size={26}
        reach={17}
        index={unit}
        names={CH_SELECT}
        onStep={onUnit}
      />
      <StepDial
        label="TIME"
        x={x}
        y={412}
        size={30}
        index={division < 0 ? 5 : division}
        names={BEAT_DIVISIONS.map(state.kind === PITCH_FX ? pitchLabel : beatLabel)}
        action={act(unit, 'beatFxBeats', state.beats)}
        onStep={(next) => send('beatFxBeats', BEAT_DIVISIONS[next])}
      />
      <div className="rx-at rx-mini-knob" style={at(x, 458)}>
        <Knob
          label="LEVEL / DEPTH"
          tone="master"
          action={act(unit, 'beatFxDepth')}
          defaultValue={0.5}
          value={state.depth}
          onChange={(v) => parameter(unit, 'beatFxDepth', v, onError)}
        />
      </div>
      <span className="rx-at rx-print" style={at(x, 492)}>
        ON/OFF
      </span>
      <button
        className={`rx-at rx-fx-power ${state.on ? 'is-on' : ''}`}
        style={at(x, 518)}
        aria-pressed={state.on}
        aria-label={tr('Encender Beat FX', 'Turn Beat FX on')}
        data-action={act(unit, 'beatFxOn', state.on ? 0 : 1)}
        onClick={() => send('beatFxOn', state.on ? 0 : 1)}
      >
        <KeyBadge action={act(unit, 'beatFxOn', 1)} />
      </button>
    </>
  );
}

/** Existing curado effects: the hardware has no equivalent, so they live in the side panel. */
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

/** What the hardware has no control for: analysis, tempo typed in, linked transport. */
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

export type ScreenKey = 'source' | 'browse' | 'tag' | 'playlist' | 'search' | 'menu';
const SCREEN_KEYS: [ScreenKey, string][] = [
  ['source', 'SOURCE'],
  ['browse', 'BROWSE'],
  ['tag', 'TAG LIST'],
  ['playlist', 'PLAYLIST'],
  ['search', 'SEARCH'],
  ['menu', 'MENU'],
];
const BANK_DIVISIONS = [0.0625, 0.125, 0.25, 0.5, 0.75, 1, 2, 4];
const FX_BANK = [7, 10, 4];

/** Slide along it to choose the division. The effect runs for as long as a finger is down. */
function XPad({
  unit,
  fx,
  onError,
}: {
  unit: number;
  fx: BeatFxState;
  onError: (e: unknown) => void;
}) {
  const wasOn = useRef(false);
  const at = (element: Element, x: number) => {
    const bounds = element.getBoundingClientRect();
    const place = Math.floor(((x - bounds.left) / bounds.width) * BANK_DIVISIONS.length);
    return BANK_DIVISIONS[Math.max(0, Math.min(BANK_DIVISIONS.length - 1, place))];
  };
  const send = (kind: string, value: number) => control(unit, kind, value).catch(onError);
  return (
    <div
      className="rx-xpad"
      role="group"
      aria-label="X-PAD"
      title={tr(
        'X-PAD: desliza para la división; el efecto suena mientras lo tocas',
        'X-PAD: slide for the division; the effect runs while you touch it',
      )}
      onPointerDown={(e) => {
        e.currentTarget.setPointerCapture(e.pointerId);
        wasOn.current = fx.on;
        send('beatFxBeats', at(e.currentTarget, e.clientX));
        send('beatFxOn', 1);
      }}
      onPointerMove={(e) => {
        if (!e.currentTarget.hasPointerCapture(e.pointerId)) return;
        const beats = at(e.currentTarget, e.clientX);
        if (beats !== fx.beats) {
          haptic('level');
          send('beatFxBeats', beats);
        }
      }}
      onPointerUp={() => {
        if (!wasOn.current) send('beatFxOn', 0);
      }}
      onPointerCancel={() => {
        if (!wasOn.current) send('beatFxOn', 0);
      }}
    >
      {BANK_DIVISIONS.map((beats) => (
        <i key={beats} className={fx.beats === beats ? 'is-on' : ''}>
          {fx.kind === PITCH_FX ? pitchLabel(beats) : beatLabel(beats)}
        </i>
      ))}
    </div>
  );
}

/** A surface for two hands at once, one per channel, showing what the engine has set. */
function XYPad({
  index,
  slot,
  fx,
  onError,
}: {
  index: number;
  slot: Slot;
  fx: BeatFxState;
  onError: (e: unknown) => void;
}) {
  const [mode, setMode] = useState<XYMode>('color');
  const [touching, setTouching] = useState(false);
  const send = (moves: { kind: string; value: number }[]) => {
    for (const { kind, value } of moves) parameter(index, kind, value, onError);
  };
  const point = (element: Element, e: React.PointerEvent) => {
    const bounds = element.getBoundingClientRect();
    return xyMoves(
      mode,
      (e.clientX - bounds.left) / bounds.width,
      1 - (e.clientY - bounds.top) / bounds.height,
    );
  };
  const [x, y] = xyPoint(mode, slot, fx);
  // A tracked hand is one more way of pointing, through the same moves as a finger.
  const tracked = useTrackedPoint(index);
  const overlayMode = useTrackingOverlay();
  const image = useTrackingPreview();
  const place = overlayPlacement(index, overlayMode);
  const engaged = tracked?.engaged ?? false;
  const held = useRef(false);
  const modeNow = useRef(mode);
  modeNow.current = mode;
  useEffect(() => {
    if (engaged && tracked) {
      held.current = true;
      for (const { kind, value } of xyMoves(modeNow.current, tracked.x, tracked.y))
        parameter(index, kind, value, onError);
    } else if (held.current) {
      held.current = false;
      for (const { kind, value } of xyRelease(modeNow.current))
        parameter(index, kind, value, onError);
    }
  }, [engaged, tracked, index, onError]);
  const finish = () => {
    setTouching(false);
    send(xyRelease(mode));
  };
  const lastCell = useRef('');
  /** A notch crossing the grid drawn on the surface, a snap at its center. */
  const feel = (element: Element, e: React.PointerEvent) => {
    const bounds = element.getBoundingClientRect();
    const cell = (value: number) => Math.floor(Math.max(0, Math.min(0.999, value)) * 4);
    const cx = cell((e.clientX - bounds.left) / bounds.width);
    const cy = cell((e.clientY - bounds.top) / bounds.height);
    const here = `${cx}:${cy}`;
    if (lastCell.current && here !== lastCell.current)
      haptic(cx >= 1 && cx <= 2 && cy >= 1 && cy <= 2 ? 'alignment' : 'level');
    lastCell.current = here;
  };
  return (
    <div className={`rx-xy tone-${index ? 'b' : 'a'}`}>
      <div className="rx-xy-modes" role="group" aria-label={`XY ${index + 1}`}>
        <b>XY {index + 1}</b>
        {XY_MODES.map((entry) => (
          <button
            key={entry.mode}
            aria-pressed={mode === entry.mode}
            onClick={() => setMode(entry.mode)}
          >
            {entry.label}
          </button>
        ))}
      </div>
      <div
        className={`rx-xy-surface ${touching ? 'is-touched' : ''}`}
        role="application"
        aria-label={tr(
          `Superficie XY del canal ${index + 1}`,
          `XY surface of channel ${index + 1}`,
        )}
        title={tr(
          'Arrastra para mover dos parámetros a la vez; al soltar vuelve a neutro',
          'Drag to move two parameters at once; letting go returns to neutral',
        )}
        onPointerDown={(e) => {
          e.currentTarget.setPointerCapture(e.pointerId);
          setTouching(true);
          send(point(e.currentTarget, e));
        }}
        onPointerMove={(e) => {
          if (!e.currentTarget.hasPointerCapture(e.pointerId)) return;
          feel(e.currentTarget, e);
          send(point(e.currentTarget, e));
        }}
        onPointerUp={finish}
        onPointerCancel={finish}
      >
        {overlayMode !== 'off' && image && (overlayMode === 'hands' || index === 0) && (
          <img
            className="rx-xy-camera"
            alt=""
            src={`data:image/jpeg;base64,${image}`}
            style={{
              left: `${place.left}%`,
              width: `${place.width}%`,
              top: `${place.top}%`,
              height: `${place.height}%`,
            }}
          />
        )}
        <i style={{ left: `${x * 100}%`, bottom: `${y * 100}%` }} />
        {tracked && (
          <i
            className={`rx-xy-hand ${engaged ? 'is-pressed' : ''}`}
            style={{ left: `${tracked.x * 100}%`, bottom: `${tracked.y * 100}%` }}
          />
        )}
      </div>
    </div>
  );
}

/** What the hardware keeps on its screen, always in view: samples at a glance. */
function SamplesPanel({
  samples,
  tracks,
  selected,
  sampler,
  loading,
  onLoad,
  onError,
}: {
  samples: Slot[];
  tracks: Track[];
  selected: number | null;
  sampler: number;
  loading: number[];
  onLoad: (slot: number, id: number) => void;
  onError: (e: unknown) => void;
}) {
  return (
    <div className="rx-module rx-samples">
      <div className="rx-module-head">
        <b>SAMPLES</b>
        <div className="rx-mini-knob">
          <Knob
            label="LEVEL"
            tone="master"
            action={act(2, 'sampler')}
            max={1.5}
            defaultValue={1}
            origin={1}
            value={sampler}
            onChange={(v) => parameter(2, 'sampler', v, onError)}
          />
        </div>
      </div>
      <div className="rx-sample-grid">
        {samples.map((sample, i) => {
          const slot = i + 2;
          const track = tracks.find((t) => t.id === sample.id);
          return (
            <button
              key={i}
              className={sample.playing ? 'is-on' : ''}
              disabled={loading.includes(slot) || (!track && selected === null)}
              title={`${track?.title ?? tr('Vacío', 'Empty')} · ${tr(
                'Clic: reproducir · Shift: cargar selección · Alt: detener',
                'Click: play · Shift: load selection · Alt: stop',
              )}`}
              data-action={act(slot, 'trigger')}
              onClick={(e) => {
                if (e.altKey) {
                  control(slot, 'pause').catch(onError);
                  return;
                }
                const intent = samplePadIntent(
                  i,
                  !!track,
                  selected,
                  e.shiftKey,
                  loading.includes(slot),
                );
                if (intent?.kind === 'load') onLoad(intent.slot, intent.id);
                else if (intent?.kind === 'trigger') control(intent.slot, 'trigger').catch(onError);
              }}
            >
              <span>S{i + 1}</span>
              <small>{track?.title ?? ''}</small>
              <KeyBadge action={act(slot, 'trigger')} />
            </button>
          );
        })}
      </div>
    </div>
  );
}

type CameraViewMode = 'none' | 'panel' | 'overlay';
const CAMERA_STATUS: Record<string, [string, string]> = {
  off: ['apagada', 'off'],
  paused: ['en pausa · libre', 'paused · released'],
  starting: ['abriendo…', 'opening…'],
  asking: ['pide permiso…', 'asking permission…'],
  running: ['encendida', 'on'],
  lost: ['reconectando…', 'reconnecting…'],
  busy: ['en uso por otra app', 'in use by another app'],
  denied: ['sin permiso: Ajustes del Sistema', 'no permission: System Settings'],
  nocamera: ['no hay cámara', 'no camera'],
};
/** Whether the camera is on is always in view here, and it runs only while this is on screen. */
function CameraControls({
  mode,
  onMode,
  view,
  onView,
}: {
  mode: TrackMode;
  onMode: (mode: TrackMode) => void;
  view: CameraViewMode;
  onView: (view: CameraViewMode) => void;
}) {
  const status = useTrackingStatus();
  const words = CAMERA_STATUS[status] ?? CAMERA_STATUS.off;
  return (
    <div
      className="rx-camera"
      role="group"
      aria-label={tr('Seguimiento con cámara', 'Camera tracking')}
    >
      <i className={`rx-led ${status === 'running' ? 'is-on' : ''}`} />
      {(
        [
          ['off', 'OFF'],
          ['hands', tr('MANOS', 'HANDS')],
          ['face', tr('CARA', 'FACE')],
        ] as const
      ).map(([value, label]) => (
        <button
          key={value}
          aria-pressed={mode === value}
          title={
            value === 'hands'
              ? tr(
                  'Pellizca para accionar XY: cada mano mueve su lado',
                  'Pinch to work XY: each hand moves its side',
                )
              : value === 'face'
                ? tr('Acércate para accionar XY 1', 'Lean in to work XY 1')
                : tr('Apaga la cámara', 'Turns the camera off')
          }
          onClick={() => onMode(value)}
        >
          {label}
        </button>
      ))}
      {mode !== 'off' && (
        <>
          <button
            aria-pressed={view === 'panel'}
            title={tr('Muestra la cámara aquí', 'Shows the camera here')}
            onClick={() => onView(view === 'panel' ? 'none' : 'panel')}
          >
            {tr('VER', 'VIEW')}
          </button>
          <button
            aria-pressed={view === 'overlay'}
            title={tr(
              'Muestra cada lado de la cámara tenue dentro de su superficie XY',
              'Shows each side of the camera faintly inside its XY surface',
            )}
            onClick={() => onView(view === 'overlay' ? 'none' : 'overlay')}
          >
            XY
          </button>
        </>
      )}
      <small>{tr(...words)}</small>
    </div>
  );
}
function CameraView({ mode }: { mode: TrackMode }) {
  const image = useTrackingPreview();
  const points = [useTrackedPoint(0), useTrackedPoint(1)];
  return (
    <div className="rx-camera-view">
      {image && <img alt="" src={`data:image/jpeg;base64,${image}`} />}
      {points.map(
        (point, i) =>
          point && (
            <i
              key={i}
              className={point.engaged ? 'is-pressed' : ''}
              style={{ left: `${point.x * 100}%`, bottom: `${point.y * 100}%` }}
            />
          ),
      )}
      <span>
        {mode === 'hands'
          ? tr('pellizca para accionar', 'pinch to work')
          : tr('acércate para accionar', 'lean in to work')}
      </span>
    </div>
  );
}

/** What is connected, and what it is doing right now. */
function ConnectionsPanel({ onSettings }: { onSettings: () => void }) {
  const { midi, output } = useSnapshot();
  const devices = useDevices();
  const lastMidi = useRef('');
  const [active, setActive] = useState(false);
  const [view, setView] = useState<CameraViewMode>('none');
  const { mode, setMode } = useTrackingControl(view !== 'none');
  useEffect(() => {
    setTrackingOverlay(view === 'overlay' ? mode : 'off');
    return () => setTrackingOverlay('off');
  }, [view, mode]);
  useEffect(() => {
    if (!midi.lastMessage || midi.lastMessage === lastMidi.current) return;
    lastMidi.current = midi.lastMessage;
    setActive(true);
    const timer = setTimeout(() => setActive(false), 180);
    return () => clearTimeout(timer);
  }, [midi.lastMessage]);
  const cue = (output?.channels ?? 2) >= 4;
  return (
    <div className="rx-module rx-connections">
      <div className="rx-module-head">
        <b>{tr('CONEXIONES', 'CONNECTIONS')}</b>
        <button onClick={onSettings}>MIDI LEARN</button>
      </div>
      <CameraControls mode={mode} onMode={setMode} view={view} onView={setView} />
      {mode !== 'off' && view === 'panel' ? (
        <CameraView mode={mode} />
      ) : (
        <dl>
          <dt>
            <i className={`rx-led ${midi.connected ? 'is-on' : ''} ${active ? 'is-hit' : ''}`} />
            MIDI
          </dt>
          <dd>{midi.connected ?? tr('sin controlador', 'no controller')}</dd>
          <dt />
          <dd>{midi.lastMessage ?? `${midi.bindings.length} ${tr('asignaciones', 'mappings')}`}</dd>
          <dt>
            <i className={`rx-led ${output ? 'is-on' : ''}`} />
            AUDIO
          </dt>
          <dd>
            {output
              ? `${output.channels} ch · ${output.sampleRate / 1000} kHz`
              : tr('sin salida', 'no output')}
          </dd>
          <dt>
            <i className={`rx-led ${cue ? 'is-on' : ''}`} />
            CUE
          </dt>
          <dd>
            {cue
              ? tr('salidas 3 y 4', 'outputs 3 and 4')
              : tr('sin salida propia', 'no own output')}
          </dd>
          <dt>
            <i className={`rx-led ${devices.length ? 'is-on' : ''}`} />
            USB
          </dt>
          <dd>{devices.map((device) => device.name).join(' · ') || '—'}</dd>
        </dl>
      )}
    </div>
  );
}

function Screen({
  slots,
  tracks,
  unit,
  fx,
  onKey,
  onError,
}: {
  slots: Slot[];
  tracks: (Track | undefined)[];
  unit: number;
  fx: BeatFxState;
  onKey: (key: ScreenKey) => void;
  onError: (e: unknown) => void;
}) {
  const shownBpm = (slot: Slot) => (slot.music.bpm ? slot.music.bpm * slot.tuning.rate : null);
  const beatMs = (slot: Slot) => {
    const bpm = fx.bpm || shownBpm(slot) || 120;
    return Math.round((60000 / bpm) * fx.beats);
  };
  return (
    <div className="rx-screen">
      <div className="rx-screen-keys">
        {SCREEN_KEYS.map(([key, label]) => (
          <button key={key} onClick={() => onKey(key)}>
            {label}
          </button>
        ))}
      </div>
      <div className="rx-display">
        <div className="rx-display-waves">
          {slots.map((slot, i) => (
            <div className="rx-display-row" key={i}>
              <div className="rx-display-tag">
                <small>DECK {i + 1}</small>
                <b>{keyLabel(transposeKey(slot.music.key, slot.tuning.semitones), 'camelot')}</b>
                <small>{shownBpm(slot)?.toFixed(1) ?? '—'}</small>
              </div>
              <ZoomWave
                name={i ? '2' : '1'}
                tone={i ? 'b' : 'a'}
                trackId={tracks[i]?.id}
                slot={slot}
                onSeek={tracks[i] ? (s) => control(i, 'seek', s).catch(onError) : undefined}
              />
            </div>
          ))}
          <div className="rx-display-fx">
            <small>BEAT FX</small>
            <b>{BEAT_FX[fx.kind]}</b>
            <small>CH {CH_SELECT[unit]}</small>
            <b>{beatMs(slots[Math.min(unit, 1)])} ms</b>
            <small>{fx.on ? 'ON' : 'OFF'}</small>
          </div>
        </div>
        <div className="rx-display-bank">
          {FX_BANK.map((kind) => (
            <button
              key={kind}
              aria-pressed={fx.kind === kind}
              data-action={act(unit, 'beatFx', kind)}
              onClick={() => control(unit, 'beatFx', kind).catch(onError)}
            >
              {BEAT_FX[kind]}
            </button>
          ))}
          <XPad unit={unit} fx={fx} onError={onError} />
        </div>
        <div className="rx-display-decks">
          {slots.map((slot, i) => (
            <div className={`rx-display-deck tone-${i ? 'b' : 'a'}`} key={i}>
              <div className="rx-display-title">
                <b>{i + 1}</b>
                <span>{tracks[i]?.title ?? tr('Sin pista', 'No track')}</span>
              </div>
              <div className="rx-display-time">
                <strong>{remaining(slot)}</strong>
                <em>{`${slot.tuning.rate >= 1 ? '+' : ''}${((slot.tuning.rate - 1) * 100).toFixed(2)}%`}</em>
                <b>{shownBpm(slot)?.toFixed(1) ?? '—'}</b>
              </div>
              <WaveOverview
                compact
                trackId={tracks[i]?.id}
                slot={slot}
                tone={i ? 'b' : 'a'}
                label={`NEEDLE SEARCH ${i + 1}`}
                onSeek={tracks[i] ? (s) => control(i, 'seek', s).catch(onError) : undefined}
              />
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

const CURVES = ['THRU', '⌒', '⊓'];
const IDLE_FX: BeatFxState = { kind: 1, on: false, beats: 1, depth: 0.5, bpm: 0 };

export function HardwareConsole({
  library,
  keys,
  loading,
  onError,
  selected,
  onLoad,
  onEdit,
  badges,
  autoGain,
  onAutoGain,
  onScreenKey,
  onBrowse,
}: {
  library: Library;
  keys: KeyBinding[];
  loading: number[];
  onError: (e: unknown) => void;
  selected: number | null;
  onLoad: (slot: number, id: number) => void;
  onEdit: (edit: LibraryEdit) => unknown;
  badges: boolean;
  autoGain: boolean;
  onAutoGain: (on: boolean) => void;
  onScreenKey: (key: ScreenKey) => void;
  onBrowse: (delta: number) => void;
}) {
  const { audio, output } = useSnapshot();
  const devices = useDevices();
  const hasCue = (output?.channels ?? 2) >= 4;
  const scrollRef = useRef<HTMLDivElement>(null);
  const bodyRef = useRef<HTMLDivElement>(null);
  const scaleRef = useRef(1);
  const [scale, setScale] = useState(1);
  const [unit, setUnit] = useState(0);
  useLayoutEffect(() => {
    const scroll = scrollRef.current;
    const body = bodyRef.current;
    if (!scroll || !body) return;
    const measure = () => {
      const bounds = body.getBoundingClientRect();
      // Read the applied zoom from the rendered box, rather than a pending React state update.
      const appliedScale = bounds.width / BODY_WIDTH;
      const next = consoleScale(
        scroll.clientWidth - 8,
        scroll.clientHeight - 8,
        BODY_WIDTH,
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
  const samples = Array.from({ length: SAMPLE_PADS }, (_, i) => audio?.slots[i + 2] ?? EMPTY_SLOT);
  const selection = library.tracks.find((track) => track.id === selected);
  const tracks = slots.map((slot) => library.tracks.find((track) => track.id === slot.id));
  const fx = audio?.beatFx?.[unit] ?? IDLE_FX;
  const drives = devices.slice(0, 2);
  const deck = (index: number): ReactNode => (
    <RxDeck
      index={index}
      slot={slots[index]}
      track={tracks[index]}
      loading={loading.includes(index)}
      masterDeck={audio?.masterDeck ?? 0}
      masterHasBpm={!!audio?.slots[audio.masterDeck]?.music.bpm}
      onError={onError}
      samples={samples}
      tracks={library.tracks}
      selected={selected}
      onLoad={onLoad}
      loadingSlots={loading}
    />
  );
  return (
    <div
      ref={scrollRef}
      className="rx-scroll"
      role="region"
      aria-label={tr('Cabina de mezcla', 'Mixing console')}
      tabIndex={0}
    >
      <KeysContext.Provider value={keys}>
        <div
          ref={bodyRef}
          className={`rx-body ${badges ? '' : 'no-badges'}`}
          style={{ zoom: scale }}
        >
          <div className="rx-top rx-top-left">
            <SamplesPanel
              samples={samples}
              tracks={library.tracks}
              selected={selected}
              sampler={audio?.sampler ?? 1}
              loading={loading}
              onLoad={onLoad}
              onError={onError}
            />
            <XYPad index={0} slot={slots[0]} fx={audio?.beatFx?.[0] ?? IDLE_FX} onError={onError} />
          </div>
          <div className="rx-top rx-top-right">
            <XYPad index={1} slot={slots[1]} fx={audio?.beatFx?.[1] ?? IDLE_FX} onError={onError} />
            <ConnectionsPanel onSettings={() => onScreenKey('menu')} />
          </div>
          <div className="rx-chassis" />
          <div className="rx-band rx-band-left" aria-label="MIC">
            <AutoLoop index={0} slot={slots[0]} x={98} onError={onError} />
            <span className="rx-at rx-section-title" style={at(236, 24)}>
              MIC
            </span>
            <Soon x={95} y={72}>
              OFF / ON / TALK
            </Soon>
            <Soon dial x={190} y={72}>
              LOW
            </Soon>
            <Soon dial x={245} y={72}>
              HI
            </Soon>
            <Soon dial x={310} y={72}>
              MIC 1
            </Soon>
            <Soon dial x={370} y={72}>
              MIC 2
            </Soon>
          </div>
          <div className="rx-band rx-band-right">
            <AutoLoop index={1} slot={slots[1]} x={210} onError={onError} />
            <b className="rx-at rx-brand" style={at(375, 22)}>
              curado
            </b>
            {[0, 1].map((i) => (
              <div
                key={i}
                className={`rx-at rx-usb ${drives[i] ? 'is-on' : ''}`}
                style={at(i ? 415 : 335, 78)}
                title={drives[i]?.name}
              >
                <span>USB {i + 1}</span>
                <i />
                <small>{drives[i]?.name ?? '—'}</small>
              </div>
            ))}
          </div>
          <Screen
            slots={slots}
            tracks={tracks}
            unit={unit}
            fx={fx}
            onKey={onScreenKey}
            onError={onError}
          />
          <div className="rx-browser">
            <Key label="BACK" x={22} y={24} onClick={() => onScreenKey('browse')} />
            <Key
              label="TAG TRACK"
              x={82}
              y={24}
              wide
              disabled={!selection}
              on={!!selection?.favorite}
              title={tr(
                'Marca la pista seleccionada como favorita',
                'Marks the selected track as a favorite',
              )}
              onClick={() =>
                selection &&
                onEdit({ kind: 'favorite', track: selection.id, value: !selection.favorite })
              }
            />
            <div className="rx-at rx-selector" style={at(52, 82)}>
              <StepDial
                label=""
                x={0}
                y={0}
                size={62}
                index={50}
                names={Array.from({ length: 101 }, (_, i) => String(i))}
                onStep={(next) => onBrowse(next - 50)}
              />
            </div>
            <span className="rx-at rx-print" style={at(52, 128)}>
              LOAD
            </span>
            <kbd className="rx-at rx-badge rx-badge-free" style={at(92, 82)}>
              ↑↓
            </kbd>
            <Key
              label="1"
              x={32}
              y={148}
              action={act(0, 'listLoad')}
              disabled={!selected || loading.includes(0)}
              title={tr('Cargar la selección en el deck 1', 'Load the selection into deck 1')}
              onClick={() => selected && onLoad(0, selected)}
            />
            <Key
              label="2"
              x={72}
              y={148}
              action={act(1, 'listLoad')}
              disabled={!selected || loading.includes(1)}
              title={tr('Cargar la selección en el deck 2', 'Load the selection into deck 2')}
              onClick={() => selected && onLoad(1, selected)}
            />
          </div>
          {deck(0)}
          <div className="rx-mixer">
            <Soon x={55} y={32}>
              DECK 1 / LINE / PHONO
            </Soon>
            <span className="rx-at rx-section-title" style={at(55, 62)}>
              AUX
            </span>
            <Soon x={55} y={86}>
              OFF / LINE / PORTABLE
            </Soon>
            <Soon dial x={55} y={130}>
              TRIM
            </Soon>
            <ColorFxBlock slots={slots} onError={onError} />
            <span className="rx-at rx-section-title" style={at(55, 368)}>
              HEADPHONES
            </span>
            <Key
              label="MASTER"
              x={28}
              y={392}
              on={audio?.cueMaster}
              disabled={!hasCue}
              title={
                hasCue
                  ? tr('Oye la mezcla en los audífonos', 'Hears the mix in the headphones')
                  : NO_CUE
              }
              action={act(0, 'masterCue', audio?.cueMaster ? 0 : 1)}
              onClick={() => control(0, 'masterCue', audio?.cueMaster ? 0 : 1).catch(onError)}
            />
            <div className="rx-at rx-mini-knob" style={at(74, 402)}>
              <Knob
                label="MIXING"
                tone="master"
                action={act(0, 'cueMix')}
                value={audio?.cueMix ?? 0}
                onChange={(v) => parameter(0, 'cueMix', v, onError)}
              />
            </div>
            <div className="rx-at rx-mini-knob" style={at(55, 462)}>
              <Knob
                label="LEVEL"
                tone="master"
                action={act(0, 'cueLevel')}
                defaultValue={0.7}
                value={audio?.cueLevel ?? 0.7}
                onChange={(v) => parameter(0, 'cueLevel', v, onError)}
              />
            </div>

            <RxChannel index={0} x={150} slot={slots[0]} hasCue={hasCue} onError={onError} />
            <RxChannel index={1} x={282} slot={slots[1]} hasCue={hasCue} onError={onError} />

            <div className="rx-at rx-mini-knob" style={at(216, 32)}>
              <Knob
                label="MASTER LEVEL"
                tone="master"
                action={act(0, 'master')}
                defaultValue={0.75}
                value={audio?.master ?? 0.75}
                onChange={(v) => parameter(0, 'master', v, onError)}
              />
            </div>
            <div className="rx-at rx-meters" style={at(216, 165)}>
              <span>CH1 · MASTER · CH2</span>
              <div>
                <Meter vertical value={slots[0].peak} label="CH 1" />
                <Meter
                  vertical
                  value={audio?.peak ?? 0}
                  label={tr('Nivel master', 'Master level')}
                />
                <Meter vertical value={slots[1].peak} label="CH 2" />
              </div>
            </div>
            <div className="rx-at" style={at(216, 266)}>
              <LimiterLamp reduction={audio?.limiterDb ?? 0} />
            </div>
            <Key
              label="AUTO GAIN"
              x={216}
              y={298}
              wide
              on={autoGain}
              title={tr(
                'Iguala el nivel de las pistas a −14 LUFS, hasta ±12 dB',
                'Matches track levels to −14 LUFS, up to ±12 dB',
              )}
              onClick={() => {
                onAutoGain(!autoGain);
                control(0, 'autoGain', autoGain ? 0 : 1).catch(onError);
              }}
            />
            <span className="rx-at rx-print" style={at(216, 340)}>
              CROSSFADER CURVE
            </span>
            <div className="rx-at rx-curve" role="group" style={at(216, 362)}>
              {CURVES.map((label, curve) => (
                <button
                  key={label}
                  aria-pressed={(audio?.crossfaderCurve ?? 1) === curve}
                  data-action={act(0, 'crossfaderCurve', curve)}
                  onClick={() => control(0, 'crossfaderCurve', curve).catch(onError)}
                >
                  {label}
                </button>
              ))}
            </div>
            <b className="rx-at rx-brand" style={at(216, 440)}>
              curado
            </b>

            <Soon x={372} y={32}>
              DECK 2 / LINE / PHONO
            </Soon>
            <Soon dial x={372} y={92}>
              BOOTH MONITOR
            </Soon>
            <BeatFxBlock x={372} unit={unit} onUnit={setUnit} state={fx} onError={onError} />

            <div className="rx-at rx-cross" style={at(216, 505)}>
              <b>A</b>
              <Slider
                vertical={false}
                label=""
                tone="master"
                action={act(0, 'crossfader')}
                origin={0.5}
                defaultValue={0.5}
                value={audio?.crossfader ?? 0.5}
                onChange={(v) => parameter(0, 'crossfader', v, onError)}
              />
              <b>B</b>
            </div>
          </div>
          {deck(1)}
        </div>
      </KeysContext.Provider>
    </div>
  );
}
