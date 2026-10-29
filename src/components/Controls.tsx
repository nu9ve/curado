import { tr } from '../lib/language';
import { useEffect, useRef, useState } from 'react';
import type { CSSProperties, KeyboardEvent as ReactKeyboardEvent, ReactNode } from 'react';

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
function keyStep(
  e: ReactKeyboardEvent,
  value: number,
  min: number,
  max: number,
  reset: number,
): number | null {
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
  const set = (v: number) => {
    const next = clampTo(v, min, max);
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
        // No text selection or native drag while turning; keep keyboard focus on the knob.
        e.preventDefault();
        e.currentTarget.focus({ preventScroll: true });
        e.currentTarget.setPointerCapture(e.pointerId);
        drag.current = { y: e.clientY, value: shown };
        setActive(true);
      }}
      onPointerMove={(e) => {
        if (!drag.current) return;
        const scale = e.shiftKey ? 600 : 150;
        set(drag.current.value + ((drag.current.y - e.clientY) / scale) * (max - min));
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
  const set = (v: number) => {
    const next = clampTo(v, min, max);
    hold(next);
    onChange(next);
  };
  const fromPointer = (x: number, y: number) => {
    const r = track.current?.getBoundingClientRect();
    if (!r) return;
    const t = vertical ? 1 - (y - r.top) / r.height : (x - r.left) / r.width;
    set(min + clampTo(t, 0, 1) * (max - min));
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
          e.currentTarget.focus({ preventScroll: true });
          e.currentTarget.setPointerCapture(e.pointerId);
          active.current = true;
          fromPointer(e.clientX, e.clientY);
        }}
        onPointerMove={(e) => active.current && fromPointer(e.clientX, e.clientY)}
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
