import { useEffect, useRef, useState } from 'react';
import { control, parameter } from '../lib/bridge';
import {
  angleDelta,
  BEAT_FX,
  beatLabel,
  COLOR_FX,
  JUMP_PADS,
  KEY_PADS,
  jogSeconds,
  LOOP_PADS,
  nextDivision,
  PAD_FX,
  PITCH_FX,
  RELEASE_PADS,
  pitchLabel,
  ROLL_PADS,
  tapTempo,
} from '../lib/console-pads';
import type { PadMode } from '../lib/console-pads';
import { actionKey } from '../lib/feedback';
import { haptic } from '../lib/haptics';
import { tr } from '../lib/language';
import type { BeatFxState, Slot } from '../lib/types';
import { Knob } from './Controls';

const act = (slot: number, kind: string, value = 0) => actionKey({ slot, kind, value });
const RIM_NOTCH = Math.PI / 15;

/**
 * A platter, as on the hardware: the top scratches with vinyl mode on and bends the tempo with it
 * off, and the outer edge always bends. A scroll gesture bends too, for a trackpad.
 */
export function useJog(
  index: number,
  playing: boolean,
  vinyl: boolean,
  sensitivity: number,
  onError: (e: unknown) => void,
) {
  const ref = useRef<HTMLButtonElement>(null);
  const drag = useRef<{
    angle: number;
    moved: boolean;
    touching: boolean;
    pending: number;
    frame: number;
    rim: number;
  } | null>(null);
  const moved = useRef(false);
  const measure = (element: Element, x: number, y: number) => {
    const bounds = element.getBoundingClientRect();
    const dx = x - bounds.left - bounds.width / 2;
    const dy = y - bounds.top - bounds.height / 2;
    return { angle: Math.atan2(dy, dx), reach: Math.hypot(dx, dy) / (bounds.width / 2) };
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
  const latest = useRef({ playing });
  latest.current = { playing };
  useEffect(() => {
    const element = ref.current;
    if (!element) return;
    const wheel = (e: WheelEvent) => {
      e.preventDefault();
      const amount = Math.max(-1, Math.min(1, -(e.deltaY + e.deltaX) / 200));
      if (latest.current.playing) control(index, 'jogBend', amount * 0.2).catch(onError);
      else control(index, 'nudge', amount * 0.05).catch(onError);
    };
    element.addEventListener('wheel', wheel, { passive: false });
    return () => {
      element.removeEventListener('wheel', wheel);
      cancelAnimationFrame(drag.current?.frame ?? 0);
    };
  }, [index, onError]);
  return {
    ref,
    wasDragged: () => moved.current,
    handlers: {
      onPointerDown: (e: React.PointerEvent<HTMLButtonElement>) => {
        e.currentTarget.setPointerCapture(e.pointerId);
        moved.current = false;
        const { angle, reach } = measure(e.currentTarget, e.clientX, e.clientY);
        // The edge of the wheel never scratches. A stopped record just moves under the hand.
        const touching = !playing || (vinyl && reach < 0.8);
        drag.current = { angle, moved: false, touching, pending: 0, frame: 0, rim: 0 };
        e.currentTarget.dataset.touch = String(touching);
      },
      onPointerMove: (e: React.PointerEvent<HTMLButtonElement>) => {
        const state = drag.current;
        if (!state) return;
        const { angle } = measure(e.currentTarget, e.clientX, e.clientY);
        const delta = angleDelta(state.angle, angle);
        if (!state.moved && Math.abs(delta) < 0.03) return;
        state.angle = angle;
        // One notch per dimple on the rim as it passes under the hand.
        state.rim += Math.abs(delta);
        if (state.rim >= RIM_NOTCH) {
          state.rim %= RIM_NOTCH;
          haptic('level');
        }
        if (!state.moved) {
          state.moved = true;
          moved.current = true;
          if (state.touching) control(index, 'jogTouch', 1).catch(onError);
        }
        state.pending += jogSeconds(delta, sensitivity);
        state.frame ||= requestAnimationFrame(flush);
      },
      onPointerUp: (e: React.PointerEvent<HTMLButtonElement>) => {
        const state = drag.current;
        delete e.currentTarget.dataset.touch;
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

/** Pad modes both mixing views share. Hot cues and samples stay with each view's own pads. */
export function PerformancePads({
  mode,
  index,
  slot,
  disabled,
  padClass,
  activeClass,
  onError,
}: {
  mode: Exclude<PadMode, 'cue' | 'samples'>;
  index: number;
  slot: Slot;
  disabled: boolean;
  padClass: string;
  activeClass: string;
  onError: (e: unknown) => void;
}) {
  const name = index ? 'B' : 'A';
  const send = (kind: string, value = 0) => control(index, kind, value).catch(onError);
  if (mode === 'loop')
    return LOOP_PADS.map((beats) => {
      const active = slot.looping && slot.loopBeats === beats;
      return (
        <button
          key={beats}
          className={`${padClass} ${active ? activeClass : ''}`}
          disabled={disabled}
          aria-pressed={active}
          aria-label={`Deck ${name} · ${tr('Loop de', 'Loop of')} ${beatLabel(beats)} ${tr('beats', 'beats')}`}
          data-action={act(index, 'loopBeats', beats)}
          onClick={() => (active ? send('loop') : send('loopBeats', beats))}
        >
          <span>{beatLabel(beats)}</span>
        </button>
      );
    });
  if (mode === 'keys' || mode === 'keyboard')
    return KEY_PADS.map((semitones) => {
      const active = slot.tuning.semitones === semitones;
      const label = semitones > 0 ? `+${semitones}` : String(semitones);
      return (
        <button
          key={semitones}
          className={`${padClass} ${active ? activeClass : ''}`}
          disabled={disabled}
          aria-pressed={active}
          aria-label={`Deck ${name} · ${label} ${tr('semitonos', 'semitones')}`}
          title={
            mode === 'keyboard'
              ? tr('Toca el hot cue A en este tono', 'Plays hot cue A at this pitch')
              : tr('Cambia el tono de la pista', 'Changes the pitch of the track')
          }
          data-action={act(index, 'transpose', semitones)}
          onClick={() => {
            send('transpose', semitones);
            if (mode === 'keyboard') send('cue', 0);
          }}
        >
          <span>{label}</span>
        </button>
      );
    });
  if (mode === 'release')
    return RELEASE_PADS.map((label, kind) => {
      const hold = () => send('releaseFx', kind);
      const release = () => send('releaseEnd');
      return (
        <button
          key={label}
          className={padClass}
          disabled={disabled}
          aria-label={`Deck ${name} · Release FX ${label}`}
          title={
            kind === 5 || kind === 7
              ? tr('Mantén para aplicar', 'Hold to apply')
              : tr('Termina la pista con este efecto', 'Ends the track with this effect')
          }
          data-action={act(index, 'releaseFx', kind)}
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
        >
          <span>{label}</span>
        </button>
      );
    });
  if (mode === 'padfx')
    return PAD_FX.map((kind) => {
      const hold = () => {
        send('beatFx', kind);
        send('beatFxOn', 1);
      };
      const release = () => send('beatFxOn', 0);
      return (
        <button
          key={kind}
          className={padClass}
          disabled={disabled}
          aria-label={`Deck ${name} · ${BEAT_FX[kind]}`}
          title={tr('Mantén para aplicar', 'Hold to apply')}
          data-action={act(index, 'beatFx', kind)}
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
        >
          <span>{BEAT_FX[kind]}</span>
        </button>
      );
    });
  if (mode === 'jump')
    return JUMP_PADS.map((beats) => (
      <button
        key={beats}
        className={padClass}
        disabled={disabled}
        aria-label={`Deck ${name} · ${tr('Saltar', 'Jump')} ${beats} ${tr('beats', 'beats')}`}
        data-action={act(index, 'beatJump', beats)}
        onClick={() => send('beatJump', beats)}
      >
        <span>{beats < 0 ? `◀ ${-beats}` : `${beats} ▶`}</span>
      </button>
    ));
  return ROLL_PADS.map((beats) => {
    const active = slot.rolling && slot.loopBeats === beats;
    const hold = () => send('roll', beats);
    const release = () => send('rollEnd');
    return (
      <button
        key={beats}
        className={`${padClass} ${active ? activeClass : ''}`}
        disabled={disabled}
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
  });
}

/** Each channel picks its own color effect, so the two can differ. */
export function ColorFxPanel({ slots, onError }: { slots: Slot[]; onError: (e: unknown) => void }) {
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
export function BeatFxPanel({
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
        <b>{state.kind === PITCH_FX ? pitchLabel(state.beats) : beatLabel(state.beats)}</b>
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
