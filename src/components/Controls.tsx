import { tr } from '../lib/language';
import { limiterLevel } from '../lib/limiter';
import { useEffect, useRef, useState } from 'react';
import type { CSSProperties, KeyboardEvent as ReactKeyboardEvent, ReactNode } from 'react';
import { currentPressure, detent, fineFactor, haptic } from '../lib/haptics';

export function Key({ children, code }: { children: ReactNode; code?: string }) {
  return <kbd data-key={code}>{children}</kbd>;
}

/** Classic labelled range input; used in forms (settings, HSV). */
export function Fader({
  label,
  value,
  onChange,
  min = 0,
  max = 1,
  step = 0.01,
  format,
}: {
  label: string;
  value: number;
  onChange: (n: number) => void;
  min?: number;
  max?: number;
  step?: number;
  format?: (n: number) => string;
}) {
  return (
    <label className="fader">
      <span>
        {label}
        <b>{format ? format(value) : `${Math.round(value * 100)}%`}</b>
      </span>
      <input
        aria-label={label}
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        style={{ '--t': (value - min) / (max - min) } as CSSProperties}
        onChange={(e) => onChange(+e.target.value)}
      />
    </label>
  );
}

/** Peak (0–1 linear) shown on a −48…0 dB scale with LED segments. */
export function Meter({
  value,
  vertical = false,
  label = tr('Nivel de audio', 'Audio level'),
}: {
  value: number;
  vertical?: boolean;
  label?: string;
}) {
  const db = value > 0 ? 20 * Math.log10(value) : -96;
  const t = Math.max(0, Math.min(1, (db + 48) / 48));
  return (
    <div
      className={`meter ${vertical ? 'vertical' : ''}`}
      role="meter"
      aria-label={label}
      aria-valuemin={-48}
      aria-valuemax={0}
      aria-valuenow={Math.round(Math.max(-48, db))}
      style={{ '--t': t } as CSSProperties}
    >
      <i />
    </div>
  );
}

/** Lights while the master limiter turns the mix down, with the reduction in dB. */
export function LimiterLamp({ reduction }: { reduction: number }) {
  const level = limiterLevel(reduction);
  const amount = level === 'idle' ? 0 : reduction;
  return (
    <div
      className="limiter-lamp"
      data-level={level}
      data-help="limiter"
      role="meter"
      aria-label={tr('Limitador del master', 'Master limiter')}
      aria-valuemin={0}
      aria-valuemax={24}
      aria-valuenow={Math.min(24, Math.round(amount * 10) / 10)}
      aria-valuetext={
        level === 'idle' ? tr('Sin reducción', 'No reduction') : `−${amount.toFixed(1)} dB`
      }
    >
      <i aria-hidden="true" />
      LIM
      {level !== 'idle' && <span>−{amount.toFixed(1)}</span>}
    </div>
  );
}

/**
 * Engine values arrive at 20 Hz. While the user drags, the local value wins so the
 * control does not jump back to a stale snapshot; it hands over shortly after release.
 */
function useLiveValue(value: number) {
  const [live, setLive] = useState<number | null>(null);
  const release = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  useEffect(() => () => clearTimeout(release.current), []);
  return {
    shown: live ?? value,
    hold: (v: number) => {
      clearTimeout(release.current);
      setLive(v);
    },
    letGo: () => {
      clearTimeout(release.current);
      release.current = setTimeout(() => setLive(null), 300);
    },
  };
}

interface RangeProps {
  label: string;
  value: number;
  onChange: (n: number) => void;
  min?: number;
  max?: number;
  /** Double click and Home/End reset here. */
  defaultValue?: number;
  /** Fill grows from this value (bipolar controls such as pitch or crossfader). */
  origin?: number;
  format?: (n: number) => string;
  action?: string;
  tone?: 'a' | 'b' | 'master';
  className?: string;
}
const clampTo = (v: number, min: number, max: number) => Math.min(max, Math.max(min, v));
/** Whether a control got its focus from the keyboard, rather than from a click. */
export const tabbed = (e: { currentTarget: Element }) => e.currentTarget.matches(':focus-visible');
function keyStep(
  e: ReactKeyboardEvent,
  value: number,
  min: number,
  max: number,
  reset: number,
): number | null {
  // Only when reached with Tab: otherwise these keys are the mix's, even right after a drag.
  if (!tabbed(e)) return null;
  const step = (max - min) / (e.shiftKey ? 100 : 25);
  const next =
    e.key === 'ArrowUp' || e.key === 'ArrowRight'
      ? value + step
      : e.key === 'ArrowDown' || e.key === 'ArrowLeft'
        ? value - step
        : e.key === 'PageUp'
          ? value + step * 4
          : e.key === 'PageDown'
            ? value - step * 4
            : e.key === 'Home'
              ? reset
              : e.key === 'End'
                ? max
                : null;
  return next === null ? null : clampTo(next, min, max);
}

const SEGMENTS = 25;
const SWEEP = 270;
/** Notches a full turn or travel has, felt on a Force Touch trackpad. */
const KNOB_STEPS = 24;
const SLIDER_STEPS = 16;
/** Rotary control: vertical drag (Shift = fine), wheel, arrows, double click to reset. */
export function Knob({
  label,
  value,
  onChange,
  min = 0,
  max = 1,
  defaultValue = min,
  origin = min,
  format,
  action,
  tone,
  className = '',
}: RangeProps) {
  const { shown, hold, letGo } = useLiveValue(value);
  const ref = useRef<HTMLDivElement>(null);
  const drag = useRef<{ y: number; value: number } | null>(null);
  const [active, setActive] = useState(false);
  const shownNow = useRef(shown);
  shownNow.current = shown;
  const set = (v: number) => {
    const next = clampTo(v, min, max);
    const feel = detent(shownNow.current, next, min, max, origin, KNOB_STEPS);
    if (feel) haptic(feel);
    shownNow.current = next;
    hold(next);
    onChange(next);
  };
  const latest = useRef({ shown, set, letGo, min, max });
  latest.current = { shown, set, letGo, min, max };
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    // Non-passive so the page does not scroll while turning a knob.
    const wheel = (e: WheelEvent) => {
      e.preventDefault();
      const { shown, set, letGo, min, max } = latest.current;
      set(shown - e.deltaY * 0.0012 * (max - min));
      letGo();
    };
    el.addEventListener('wheel', wheel, { passive: false });
    return () => el.removeEventListener('wheel', wheel);
  }, []);
  const t = (shown - min) / (max - min);
  const o = (origin - min) / (max - min);
  const angle = -SWEEP / 2 + t * SWEEP;
  return (
    <div
      ref={ref}
      className={`knob ${tone ? `tone-${tone}` : ''} ${active ? 'is-active' : ''} ${className}`}
      style={{ '--angle': `${angle}deg` } as CSSProperties}
      role="slider"
      tabIndex={0}
      aria-label={label}
      aria-valuemin={min}
      aria-valuemax={max}
      aria-valuenow={Math.round(shown * 100) / 100}
      aria-valuetext={format?.(shown)}
      data-action={action}
      onPointerDown={(e) => {
        // No text selection or native drag while turning.
        e.preventDefault();
        e.currentTarget.setPointerCapture(e.pointerId);
        drag.current = { y: e.clientY, value: shown };
        setActive(true);
      }}
      onPointerMove={(e) => {
        if (!drag.current) return;
        // Each step counts by how fine it is now, so pressing harder never jumps the value.
        const moved = drag.current.y - e.clientY;
        drag.current.y = e.clientY;
        const scale = 150 * fineFactor(currentPressure(), e.shiftKey);
        drag.current.value = clampTo(drag.current.value + (moved / scale) * (max - min), min, max);
        set(drag.current.value);
      }}
      onPointerUp={() => {
        drag.current = null;
        setActive(false);
        letGo();
      }}
      onPointerCancel={() => {
        drag.current = null;
        setActive(false);
        letGo();
      }}
      onDoubleClick={() => {
        set(defaultValue);
        letGo();
      }}
      onKeyDown={(e) => {
        const next = keyStep(e, shown, min, max, defaultValue);
        if (next === null) return;
        e.preventDefault();
        e.stopPropagation();
        set(next);
        letGo();
      }}
    >
      <span className="knob-dial" aria-hidden="true">
        <svg viewBox="0 0 48 48">
          {Array.from({ length: SEGMENTS }, (_, i) => {
            const p = i / (SEGMENTS - 1);
            const lit = p >= Math.min(o, t) - 0.001 && p <= Math.max(o, t) + 0.001;
            return (
              <rect
                key={i}
                className={lit ? 'lit' : undefined}
                x="23"
                y="1.5"
                width="2"
                height="5"
                transform={`rotate(${-SWEEP / 2 + p * SWEEP} 24 24)`}
              />
            );
          })}
        </svg>
        <i className="knob-cap" />
      </span>
      <span className="knob-label">{label}</span>
      <b className="knob-value">{format ? format(shown) : Math.round(t * 100)}</b>
    </div>
  );
}

/** Linear fader. Click/drag positions the cap; arrows step; double click resets. */
export function Slider({
  label,
  value,
  onChange,
  min = 0,
  max = 1,
  defaultValue = min,
  origin = min,
  format,
  action,
  tone,
  className = '',
  vertical = true,
}: RangeProps & { vertical?: boolean }) {
  const { shown, hold, letGo } = useLiveValue(value);
  const track = useRef<HTMLDivElement>(null);
  const active = useRef(false);
  const shownNow = useRef(shown);
  shownNow.current = shown;
  const fine = useRef<{ at: number; value: number } | null>(null);
  const set = (v: number) => {
    const next = clampTo(v, min, max);
    const feel = detent(shownNow.current, next, min, max, origin, SLIDER_STEPS);
    if (feel) haptic(feel);
    shownNow.current = next;
    hold(next);
    onChange(next);
  };
  const fromPointer = (x: number, y: number, shift: boolean) => {
    const r = track.current?.getBoundingClientRect();
    if (!r) return;
    const along = vertical ? 1 - (y - r.top) / r.height : (x - r.left) / r.width;
    const factor = fineFactor(currentPressure(), shift);
    // Normally the cap goes to the pointer. Once fine, it moves by less than the pointer does,
    // and stays that way until let go, since going back to the pointer would jump.
    if (factor > 1.2 || fine.current) {
      if (!fine.current) {
        fine.current = { at: along, value: shownNow.current };
        haptic('alignment');
      }
      const change = ((along - fine.current.at) / factor) * (max - min);
      fine.current = {
        at: along,
        value: clampTo(fine.current.value + change, min, max),
      };
      set(fine.current.value);
      return;
    }
    set(min + clampTo(along, 0, 1) * (max - min));
  };
  const t = (shown - min) / (max - min);
  const o = (origin - min) / (max - min);
  return (
    <div
      className={`slider ${vertical ? 'vertical' : 'horizontal'} ${tone ? `tone-${tone}` : ''} ${className}`}
      style={{ '--t': t, '--o': o } as CSSProperties}
    >
      <div
        ref={track}
        className="slider-track"
        role="slider"
        tabIndex={0}
        aria-label={label}
        aria-orientation={vertical ? 'vertical' : 'horizontal'}
        aria-valuemin={min}
        aria-valuemax={max}
        aria-valuenow={Math.round(shown * 100) / 100}
        aria-valuetext={format?.(shown)}
        data-action={action}
        onPointerDown={(e) => {
          e.preventDefault();
          e.currentTarget.setPointerCapture(e.pointerId);
          active.current = true;
          fine.current = null;
          fromPointer(e.clientX, e.clientY, false);
        }}
        onPointerMove={(e) => active.current && fromPointer(e.clientX, e.clientY, e.shiftKey)}
        onPointerUp={() => {
          active.current = false;
          letGo();
        }}
        onPointerCancel={() => {
          active.current = false;
          letGo();
        }}
        onDoubleClick={() => {
          set(defaultValue);
          letGo();
        }}
        onKeyDown={(e) => {
          const next = keyStep(e, shown, min, max, defaultValue);
          if (next === null) return;
          e.preventDefault();
          e.stopPropagation();
          set(next);
          letGo();
        }}
      >
        <i className="slider-fill" />
        <i className="slider-cap" />
      </div>
      <span className="slider-label">
        {label}
        <b>{format ? format(shown) : `${Math.round(t * 100)}`}</b>
      </span>
    </div>
  );
}
