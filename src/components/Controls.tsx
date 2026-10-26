import type { ReactNode } from 'react';
export function Key({ children }: { children: ReactNode }) {
  return <kbd>{children}</kbd>;
}
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
        onChange={(e) => onChange(+e.target.value)}
      />
    </label>
  );
}
export function Meter({ value, vertical = false }: { value: number; vertical?: boolean }) {
  return (
    <div
      className={`meter ${vertical ? 'vertical' : ''}`}
      role="meter"
      aria-label="Nivel de audio"
      aria-valuemin={0}
      aria-valuemax={1}
      aria-valuenow={value}
    >
      <i
        style={
          vertical
            ? { height: `${Math.min(1, value) * 100}%` }
            : { width: `${Math.min(1, value) * 100}%` }
        }
      />
    </div>
  );
}
