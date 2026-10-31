import { ArrowUpRight, ChevronDown, ChevronRight } from 'lucide-react';
import { useSnapshot } from '../lib/bridge';
import { tr } from '../lib/language';
import { harmony, keyLabel, transposeKey } from '../lib/music';
import { EMPTY_SLOT, remaining, time } from '../lib/types';
import type { Library, Slot, Track } from '../lib/types';

const shownBpm = (slot: Slot) => (slot.music.bpm ? slot.music.bpm * slot.tuning.rate : null);
const shownKey = (slot: Slot) => transposeKey(slot.music.key, slot.tuning.semitones);

/** How a candidate sits against the deck that leads, which is what a choice depends on. */
function matchText(track: Track, master: Slot) {
  const parts: string[] = [];
  const target = shownBpm(master);
  if (target && track.music?.bpm) {
    const change = (target / track.music.bpm - 1) * 100;
    parts.push(`${change >= 0 ? '+' : ''}${change.toFixed(1)} % ${tr('al master', 'to master')}`);
  }
  const match = harmony(track.music?.key ?? null, shownKey(master));
  if (match === 'same') parts.push(tr('misma tonalidad', 'same key'));
  if (match === 'compatible') parts.push(tr('tonalidad compatible', 'compatible key'));
  if (match === 'clash') parts.push(tr('tonalidad distante', 'distant key'));
  return parts.join(' · ');
}

export function TrackInfo({
  library,
  selected,
  open,
  onToggle,
  onLoad,
  loading,
}: {
  library: Library;
  selected: number | null;
  open: boolean;
  onToggle: () => void;
  onLoad: (slot: number, id: number) => void;
  loading: number[];
}) {
  const { audio } = useSnapshot();
  const slots = [0, 1].map((i) => audio?.slots[i] ?? EMPTY_SLOT);
  const choice = library.tracks.find((track) => track.id === selected);
  return (
    <section className="track-info" aria-label={tr('Información de pistas', 'Track information')}>
      <button className="track-info-toggle" aria-expanded={open} onClick={onToggle}>
        {open ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
        {tr('información', 'information')}
      </button>
      {open && (
        <div className="track-info-body">
          {slots.map((slot, i) => {
            const track = library.tracks.find((candidate) => candidate.id === slot.id);
            const bpm = shownBpm(slot);
            return (
              <div className={`track-info-row tone-${i ? 'b' : 'a'}`} key={i}>
                <b>{i ? 'B' : 'A'}</b>
                <span>
                  <strong>{track?.title ?? tr('Sin pista', 'No track')}</strong>
                  <small>{track?.artist ?? ''}</small>
                </span>
                <em>{bpm ? bpm.toFixed(1) : '—'}</em>
                <em>{keyLabel(shownKey(slot), 'camelot')}</em>
                <em>{remaining(slot)}</em>
              </div>
            );
          })}
          {choice && (
            <div className="track-info-row track-info-choice">
              <b>▸</b>
              <span>
                <strong>{choice.title}</strong>
                <small>
                  {matchText(choice, slots[audio?.masterDeck ?? 0]) || choice.artist}
                </small>
              </span>
              <em>{choice.music?.bpm?.toFixed(1) ?? '—'}</em>
              <em>{keyLabel(choice.music?.key ?? null, 'camelot')}</em>
              <em>{time(choice.duration)}</em>
              {[0, 1].map((slot) => (
                <button
                  key={slot}
                  disabled={loading.includes(slot)}
                  title={tr(
                    `Cargar la selección en ${slot ? 'B' : 'A'}`,
                    `Load selection into ${slot ? 'B' : 'A'}`,
                  )}
                  onClick={() => onLoad(slot, choice.id)}
                >
                  {slot ? 'B' : 'A'}
                  <ArrowUpRight size={11} />
                </button>
              ))}
            </div>
          )}
        </div>
      )}
    </section>
  );
}
