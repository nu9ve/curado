export type MixLayout = 'classic' | 'console';
const key = 'curado.mix-layout.v1';

export function readMixLayout(storage: Pick<Storage, 'getItem'> | undefined): MixLayout {
  try {
    return storage?.getItem(key) === 'classic' ? 'classic' : 'console';
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
