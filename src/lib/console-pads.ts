export type SamplePadIntent =
  { kind: 'trigger'; slot: number } | { kind: 'load'; slot: number; id: number } | null;

/** Sample pads on either deck address the same four native one-shot slots. */
export function samplePadIntent(
  index: number,
  loaded: boolean,
  selected: number | null,
  shift: boolean,
  loading: boolean,
): SamplePadIntent {
  if (!Number.isInteger(index) || index < 0 || index >= 4 || loading) return null;
  const slot = index + 2;
  if (shift || !loaded) return selected === null ? null : { kind: 'load', slot, id: selected };
  return { kind: 'trigger', slot };
}
