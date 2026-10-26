import { useId } from 'react';

/**
 * The curado mark: a pixel ring opened to the right (a lowercase "c" and a record)
 * with the spindle in the middle. Same geometry as `scripts/icon.py`.
 */
const GRID = 16;
export const LOGO_CELLS: [number, number][] = (() => {
  const cells: [number, number][] = [];
  for (let y = 0; y < GRID; y++)
    for (let x = 0; x < GRID; x++) {
      const dx = x + 0.5 - GRID / 2;
      const dy = y + 0.5 - GRID / 2;
      const r = Math.hypot(dx, dy);
      // Flat-cut opening on the right: terminals read as a lowercase c, not a pie slice.
      const gap = dx > 1 && Math.abs(dy) < 3;
      const ring = r >= 4.3 && r <= 7.7 && !gap;
      const spindle = Math.abs(dx) < 1 && Math.abs(dy) < 1;
      if (ring || spindle) cells.push([x, y]);
    }
  return cells;
})();

export function Logo({ size = 28, title = 'curado' }: { size?: number; title?: string }) {
  const id = useId().replace(/:/g, '');
  return (
    <svg
      className="logo-mark"
      width={size}
      height={size}
      viewBox={`0 0 ${GRID} ${GRID}`}
      shapeRendering="crispEdges"
      role="img"
      aria-label={title}
    >
      <defs>
        <linearGradient id={`g${id}`} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" style={{ stopColor: 'var(--accent)' }} />
          <stop offset="1" style={{ stopColor: 'var(--gradient-end)' }} />
        </linearGradient>
      </defs>
      <g fill={`url(#g${id})`}>
        {LOGO_CELLS.map(([x, y]) => (
          <rect key={`${x}-${y}`} x={x} y={y} width="1.02" height="1.02" />
        ))}
      </g>
    </svg>
  );
}

export function Wordmark() {
  return (
    <span className="wordmark">
      curado
      <i aria-hidden="true" />
    </span>
  );
}
