import { createContext, useContext } from 'react';
import { tr } from '../lib/language';
import { keyLabel, outputBpm, outputKey } from '../lib/music';
import { time } from '../lib/types';
import type { Slot } from '../lib/types';
import type { Workspace } from '../lib/workspace';

export const DeckKeyNotation = createContext<Workspace['deckKeyNotation']>('camelot');

/** Stationary center of the platter. Position comes from the deck's native stream. */
export function DeckReadout({ id, slot, position }: { id?: string; slot: Slot; position: number }) {
  const notation = useContext(DeckKeyNotation);
  const seconds = Math.max(0, Math.min(position, slot.duration));
  const clock = slot.id ? time(seconds) : '—';
  const bpm = slot.id ? outputBpm(slot)?.toFixed(1) : undefined;
  const key = slot.id ? outputKey(slot) : null;
  const alternate = notation === 'camelot' ? 'american' : 'camelot';
  const primaryKey = keyLabel(key, notation);
  const secondaryKey = keyLabel(key, alternate);
  return (
    <span className="deck-readout" id={id}>
      <strong
        className="deck-readout-time"
        aria-label={tr(`Tiempo actual ${clock}`, `Current time ${clock}`)}
      >
        {clock}
      </strong>
      <span className="deck-readout-music">
        <span className="deck-readout-value">
          <span>{bpm ?? '—'}</span>
          <small>BPM</small>
        </span>
        <span
          className="deck-readout-value"
          aria-label={tr(
            `Tonalidad actual ${primaryKey}${key ? `, ${secondaryKey}` : ''}`,
            `Current key ${primaryKey}${key ? `, ${secondaryKey}` : ''}`,
          )}
        >
          <span data-notation={notation}>{primaryKey}</span>
          <small aria-hidden="true">{key ? secondaryKey : '\u00a0'}</small>
        </span>
      </span>
    </span>
  );
}
