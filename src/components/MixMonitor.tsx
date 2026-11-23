import { Pause, Play } from 'lucide-react';
import { control, parameter } from '../lib/bridge';
import { actionKey } from '../lib/feedback';
import { tr } from '../lib/language';
import { useLivePosition } from '../lib/position';
import type { Slot } from '../lib/types';
import { Meter, Slider } from './Controls';
import { DeckReadout } from './DeckReadout';

function MonitorDeck({
  index,
  slot,
  title,
  onError,
}: {
  index: number;
  slot: Slot;
  title?: string;
  onError: (e: unknown) => void;
}) {
  const position = useLivePosition(index, slot.position);
  const name = index === 0 ? 'A' : 'B';
  const tone = index === 0 ? 'a' : 'b';
  return (
    <section className={`mix-monitor-deck tone-${tone}`} aria-label={`Deck ${name}`}>
      <div className="mix-monitor-track">
        <b>{name}</b>
        <span>{title ?? tr('Sin pista', 'No track')}</span>
        <button
          type="button"
          disabled={!slot.id}
          aria-label={
            slot.playing
              ? tr(`Pausar deck ${name}`, `Pause deck ${name}`)
              : tr(`Reproducir deck ${name}`, `Play deck ${name}`)
          }
          data-action={actionKey({ slot: index, kind: 'toggle', value: 0 })}
          onClick={() => control(index, 'toggle').catch(onError)}
        >
          {slot.playing ? <Pause size={15} /> : <Play size={15} />}
        </button>
      </div>
      <DeckReadout slot={slot} position={position} />
      <Meter value={slot.peak} label={tr(`Nivel del deck ${name}`, `Deck ${name} level`)} />
      <div className="mix-monitor-state">
        {slot.sync && <span>SYNC</span>}
        {slot.looping && <span>LOOP</span>}
        {slot.cue && <span>CUE</span>}
      </div>
    </section>
  );
}

export function MixMonitor({
  slots,
  titles,
  crossfader,
  masterPeak,
  onError,
}: {
  slots: Slot[];
  titles: (string | undefined)[];
  crossfader: number;
  masterPeak: number;
  onError: (e: unknown) => void;
}) {
  return (
    <div className="mix-monitor">
      <div className="mix-monitor-decks">
        {slots.map((slot, index) => (
          <MonitorDeck
            key={index}
            slot={slot}
            title={titles[index]}
            index={index}
            onError={onError}
          />
        ))}
      </div>
      <div className="mix-monitor-output">
        <Meter value={masterPeak} label={tr('Nivel master', 'Master level')} />
        <Slider
          label="CROSSFADER"
          value={crossfader}
          origin={0.5}
          defaultValue={0.5}
          action={actionKey({ slot: 0, kind: 'crossfader', value: 0 })}
          vertical={false}
          onChange={(value) => parameter(0, 'crossfader', value, onError)}
        />
      </div>
    </div>
  );
}
