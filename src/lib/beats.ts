import { useEffect, useState } from 'react';
import { call, native } from './bridge';

/** Every beat of a track whose tempo moves. Most tracks have none: one tempo describes them. */
export function useBeatMap(id: number | undefined, ticket: number): number[] | null {
  const [beats, setBeats] = useState<number[] | null>(null);
  useEffect(() => {
    if (!id || !native) {
      setBeats(null);
      return;
    }
    let current = true;
    call<number[] | null>('track_beats', { id })
      .then((loaded) => {
        if (current) setBeats(loaded);
      })
      .catch(() => {
        if (current) setBeats(null);
      });
    return () => {
      current = false;
    };
  }, [id, ticket]);
  return beats;
}
