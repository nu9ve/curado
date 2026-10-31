export type LimiterLevel = 'idle' | 'active' | 'hot';

/**
 * The engine reports recent gain reduction in dB. Below 0.1 dB nothing is audible; from 6 dB
 * the limiter is working hard enough to flatten the mix, so the lamp asks for less level.
 */
export function limiterLevel(reductionDb: number): LimiterLevel {
  if (!(reductionDb >= 0.1)) return 'idle';
  return reductionDb < 6 ? 'active' : 'hot';
}
