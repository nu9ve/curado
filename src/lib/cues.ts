import { useEffect, useState } from 'react';
import { call, native } from './bridge';
import type { CueInfo, CueStyle } from './types';

const EMPTY: CueInfo = { hot: [], memory: [] };
const known = new Map<number, CueInfo>();

export function invalidateCueInfo(id: number) {
  known.delete(id);
}
/** Colors, names and memory cues of a track, which only exports have. Positions stay native. */
export function useCueInfo(id: number | undefined): CueInfo {
  const [info, setInfo] = useState<CueInfo>(EMPTY);
  useEffect(() => {
    if (!id || !native) {
      setInfo(EMPTY);
      return;
    }
    const cached = known.get(id);
    if (cached) {
      setInfo(cached);
      return;
    }
    let current = true;
    call<CueInfo>('track_cue_info', { id })
      .then((loaded) => {
        known.set(id, loaded);
        if (current) setInfo(loaded);
      })
      .catch(() => {
        if (current) setInfo(EMPTY);
      });
    return () => {
      current = false;
    };
  }, [id]);
  return info;
}
export function cueColor(style: CueStyle | null | undefined): string | undefined {
  if (!style?.color) return undefined;
  const [r, g, b] = style.color;
  return `rgb(${r} ${g} ${b})`;
}
