import { ArrowUpRight, ChevronDown, ChevronRight } from 'lucide-react';
import { useRef } from 'react';
import { call, control, useSnapshot } from '../lib/bridge';
import { PREVIEW_SLOT } from '../lib/console-pads';
import { WaveOverview } from './Waveform';
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

/**
 * Hold on the waveform to hear that part without loading the track. It goes to the cue output
 * when there is one; on a plain stereo output it can only sound through the speakers.
 */
function TouchPreview({ track }: { track: Track }) {
  const { audio, output } = useSnapshot();
  const held = useRef(false);
  const slot = audio?.slots[PREVIEW_SLOT] ?? EMPTY_SLOT;
  const cue = (output?.channels ?? 2) >= 4;
  const start = async (seconds: number) => {
    held.current = true;
    if (slot.id !== track.id) await call('load_track', { slot: PREVIEW_SLOT, id: track.id });
    if (!held.current) return;
    await control(PREVIEW_SLOT, 'seek', seconds);
    await control(PREVIEW_SLOT, 'play');
  };
  const stop = () => {
    held.current = false;
    void control(PREVIEW_SLOT, 'pause').catch(() => {});
  };
  return (
    <div
      className="touch-preview"
      title={
        cue
          ? tr('Mantén para oír en audífonos', 'Hold to hear in the headphones')
          : tr(
              'Mantén para oír: sin salida de cue se oye en la salida',
              'Hold to hear: with no cue output it plays through the speakers',
            )
      }
      onPointerDown={(e) => {
        e.currentTarget.setPointerCapture(e.pointerId);
        const bounds = e.currentTarget.getBoundingClientRect();
        const place = Math.max(0, Math.min(1, (e.clientX - bounds.left) / bounds.width));
        void start(place * track.duration).catch(stop);
      }}
      onPointerUp={stop}
      onPointerCancel={stop}
    >
      <WaveOverview
        compact
        trackId={track.id}
        slot={{
          ...EMPTY_SLOT,
          duration: track.duration,
          position: slot.id === track.id ? slot.position : 0,
        }}
        label="TOUCH PREVIEW"
      />
      <small>
        TOUCH PREVIEW{cue ? '' : tr(' · se oye en la salida', ' · plays through the speakers')}
      </small>
    </div>
  );
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
          {choice && <TouchPreview track={choice} />}
          {choice && (
            <div className="track-info-row track-info-choice">
              <b>▸</b>
              <span>
                <strong>{choice.title}</strong>
                <small>{matchText(choice, slots[audio?.masterDeck ?? 0]) || choice.artist}</small>
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
