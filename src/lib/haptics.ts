import { call, native } from './bridge';

export type HapticPattern = 'generic' | 'alignment' | 'level';
/** The hardware cannot make out faster taps, and a fast drag crosses many steps at once. */
const MIN_GAP_MS = 28;
let enabled = true;
let last = 0;
export function setHapticsEnabled(on: boolean) {
  enabled = on;
}
export function haptic(pattern: HapticPattern) {
  if (!enabled || !native) return;
  const now = performance.now();
  // A stop or a snap matters more than a passing notch, so only notches give way.
  if (pattern === 'level' && now - last < MIN_GAP_MS) return;
  last = now;
  call('haptic', { pattern }).catch(() => {});
}

/**
 * What moving between two values should feel like. An end of the range is a stop, its origin a
 * snap and each step a notch; passing several at once is one tap, of the strongest.
 */
export function detent(
  previous: number,
  next: number,
  min: number,
  max: number,
  origin: number,
  steps: number,
): HapticPattern | null {
  if (previous === next) return null;
  if ((next <= min && previous > min) || (next >= max && previous < max)) return 'generic';
  const crossed = (a: number, b: number, at: number) => (a < at && b >= at) || (a > at && b <= at);
  if (origin > min && origin < max && crossed(previous, next, origin)) return 'alignment';
  const place = (value: number) => Math.floor(((value - min) / (max - min)) * steps);
  return place(previous) !== place(next) ? 'level' : null;
}

/** Pressing harder on a Force Touch trackpad makes a drag move less, for fine work. */
let pressure = 0;
export const currentPressure = () => pressure;
/** WebKit reports 1 at a normal click and 2 at a force click, up to 3. */
export function pressureFromForce(force: number): number {
  return Math.max(0, Math.min(1, force - 1));
}
export const fineFactor = (level: number, shift: boolean) =>
  (shift ? 4 : 1) * (1 + 3 * Math.max(0, Math.min(1, level)));
if (typeof window !== 'undefined') {
  window.addEventListener('webkitmouseforcechanged', (event) => {
    pressure = pressureFromForce((event as unknown as { webkitForce: number }).webkitForce);
  });
  // Without this, a deep press also asks the system to look up what is under the pointer.
  window.addEventListener('webkitmouseforcewillbegin', (event) => event.preventDefault());
  window.addEventListener('pointerup', () => {
    pressure = 0;
  });
}
