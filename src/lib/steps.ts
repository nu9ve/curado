/**
 * The suite's step modifiers for knobs, sliders and arrow-driven values (nu9ve-interaction-v1 §2,
 * nu9ve-controls-v1): Option (Alt on Windows and Linux) is fine, a tenth of a step; Shift is a
 * large step, four of them. Option wins when both are held.
 */
export const FINE_STEP = 0.1;
export const LARGE_STEPS = 4;
export function stepScale(e: { altKey: boolean; shiftKey: boolean }): number {
  return e.altKey ? FINE_STEP : e.shiftKey ? LARGE_STEPS : 1;
}
