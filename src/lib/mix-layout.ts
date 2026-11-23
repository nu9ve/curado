export const MIX_LAYOUTS = ['console', 'ddj-sb', 'classic', 'compact', 'monitor'] as const;
export type MixLayout = (typeof MIX_LAYOUTS)[number];

export function suggestedMixLayout(connected: string | null, presetId?: string): MixLayout | null {
  return connected && presetId === 'pioneer-ddj-sb' ? 'ddj-sb' : null;
}
const key = 'curado.mix-layout.v1';

export function readMixLayout(storage: Pick<Storage, 'getItem'> | undefined): MixLayout {
  try {
    const saved = storage?.getItem(key);
    return MIX_LAYOUTS.find((layout) => layout === saved) ?? 'console';
  } catch {
    return 'console';
  }
}

export function saveMixLayout(storage: Pick<Storage, 'setItem'> | undefined, layout: MixLayout) {
  try {
    storage?.setItem(key, layout);
  } catch {
    // A blocked or full store must not prevent using the view in this session.
  }
}
