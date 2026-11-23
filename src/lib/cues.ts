import { useEffect, useState } from 'react';
import { call, native } from './bridge';
import type { CueInfo, CueStyle } from './types';

const EMPTY: CueInfo = { hot: [], memory: [] };
const known = new Map<number, CueInfo>();

export function invalidateCueInfo(id: number) {
  known.delete(id);
}
/** Reads colors, memory cues and saved loop markers from the native library. */
export function useCueInfo(id: number | undefined, refresh = false): CueInfo {
  const [data, setData] = useState<{ id: number; info: CueInfo } | null>(null);
  useEffect(() => {
    if (!id || !native) {
      return;
    }
    const cached = refresh ? undefined : known.get(id);
    if (cached) {
      setData({ id, info: cached });
      return;
    }
    let current = true;
    call<CueInfo>('track_cue_info', { id })
      .then((loaded) => {
        known.set(id, loaded);
        if (current) setData({ id, info: loaded });
      })
      .catch(() => {
        if (current) setData(null);
      });
    return () => {
      current = false;
    };
  }, [id, refresh]);
  return data && data.id === id ? data.info : EMPTY;
}
export function cueColor(style: CueStyle | null | undefined): string | undefined {
  if (!style?.color) return undefined;
  const [r, g, b] = style.color;
  return `rgb(${r} ${g} ${b})`;
}
