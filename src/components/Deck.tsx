import type { CSSProperties } from 'react';
import { Play, Pause, Repeat2, ChevronsLeft, ChevronsRight } from 'lucide-react';
import { useSnapshot, control, parameter } from '../lib/bridge';
import type { KeyBinding, Track } from '../lib/types';
import { EMPTY_SLOT, remaining, time } from '../lib/types';
import { bindingLabel } from '../lib/keyboard';
import { actionKey } from '../lib/feedback';
import { WaveOverview } from './Waveform';
import { Key, Slider } from './Controls';

/** Key label for an action, preferring digit keys for numbered cues. */
export function keyFor(keys: KeyBinding[], slot: number, kind: string, value = 0, digit = false) {
  const candidates = keys.filter(
    (k) => k.action.slot === slot && k.action.kind === kind && k.action.value === value,
  );
  const b =
    (digit ? candidates.find((k) => k.code.startsWith('Digit')) : undefined) ?? candidates[0];
  return b ? <Key code={b.code}>{bindingLabel(b)}</Key> : null;
}

/** Rotation is derived from the engine position (33⅓ rpm), never from a UI timer. */
function Platter({
  slot,
  index,
  disabled,
  onToggle,
}: {
  slot: typeof EMPTY_SLOT;
  index: number;
  disabled: boolean;
  onToggle: () => void;
}) {
  const turn = slot.position * 200;
  return (
    <button
      className={`platter ${slot.playing ? 'spinning' : ''}`}
      disabled={disabled}
      onClick={onToggle}
      aria-label={`${slot.playing ? 'Pausar' : 'Reproducir'} deck ${index ? 'B' : 'A'}`}
      data-action={actionKey({ slot: index, kind: 'toggle', value: 0 })}
    >
      <span className="platter-disc" style={{ '--turn': `${turn}deg` } as CSSProperties}>
        <i className="platter-label" />
        <i className="platter-mark" />
      </span>
      <span className="platter-letter">{index ? 'B' : 'A'}</span>
    </button>
  );
}

export function Deck({
  index,
  tracks,
  keys,
  onError,
  loading,
}: {
  index: number;
  tracks: Track[];
  keys: KeyBinding[];
  onError: (e: unknown) => void;
  loading?: boolean;
}) {
  const snap = useSnapshot();
  const slot = snap.audio?.slots[index] ?? EMPTY_SLOT;
  const track = tracks.find((t) => t.id === slot.id);
  const tone = index ? 'b' : 'a';
  const send = (kind: string, value = 0) => control(index, kind, value).catch(onError);
  const act = (kind: string, value = 0) => actionKey({ slot: index, kind, value });
  const tag = (kind: string, value = 0, digit = false) => keyFor(keys, index, kind, value, digit);
  const empty = !track || loading;
  return (
    <section
      className={`deck deck-${index} tone-${tone} ${slot.playing ? 'is-playing' : ''}`}
      aria-label={`Deck ${index ? 'B' : 'A'}`}
    >
      <header className="deck-head">
        <span className="deck-tag">{index ? 'B' : 'A'}</span>
        <div className="deck-title">
          <strong>{loading ? 'cargando…' : (track?.title ?? 'sin pista')}</strong>
          <small>{track?.artist ?? `Selecciona una pista y cárgala en ${index ? 'B' : 'A'}`}</small>
        </div>
        <div className="deck-time">
          <b>{remaining(slot)}</b>
          <small>
            {time(slot.position)} · {time(slot.duration)}
          </small>
        </div>
      </header>
      <WaveOverview
        trackId={track?.id}
        slot={slot}
        tone={tone}
        label={`Waveform deck ${index ? 'B' : 'A'}`}
        onSeek={track ? (s) => send('seek', s) : undefined}
      />
      <div className="deck-body">
        <Platter slot={slot} index={index} disabled={!!empty} onToggle={() => send('toggle')} />
        <div className="deck-pads">
          <div className="cue-pads">
            {[0, 1, 2].map((i) => (
              <button
                key={i}
                className={`cue-pad ${slot.cues[i] > 0 ? 'set' : ''}`}
                title="Clic para saltar · Shift + clic para guardar"
                disabled={!track}
                data-action={`${act('cue', i)} ${act('setCue', i)}`}
                onClick={(e) => send(e.shiftKey ? 'setCue' : 'cue', i)}
              >
                <span>CUE {i + 1}</span>
                <b>{slot.cues[i] > 0 ? time(slot.cues[i]) : '—'}</b>
                {tag('cue', i, true)}
              </button>
            ))}
          </div>
          <div className="transport">
            <button
              className={`play-button ${slot.playing ? 'playing' : ''}`}
              disabled={!!empty}
              onClick={() => send('toggle')}
              data-action={act('toggle')}
              aria-label={`${slot.playing ? 'Pausar' : 'Reproducir'} deck ${index ? 'B' : 'A'}`}
            >
              {slot.playing ? (
                <Pause size={16} fill="currentColor" />
              ) : (
                <Play size={16} fill="currentColor" />
              )}
              {tag('toggle')}
            </button>
            <button disabled={!track} onClick={() => send('cue', 0)} data-action={act('cue', 0)}>
              CUE{tag('cue', 0)}
            </button>
            <button
              disabled={!track}
              aria-label="Nudge atrás"
              onClick={() => send('nudge', -0.05)}
              data-action={act('nudge', -0.05)}
            >
              <ChevronsLeft size={14} />
              {tag('nudge', -0.05)}
            </button>
            <button
              disabled={!track}
              aria-label="Nudge adelante"
              onClick={() => send('nudge', 0.05)}
              data-action={act('nudge', 0.05)}
            >
              <ChevronsRight size={14} />
              {tag('nudge', 0.05)}
            </button>
          </div>
          <div className="loop-row">
            <button
              className={slot.looping ? 'selected' : ''}
              disabled={!track}
              onClick={() => send('loop')}
              data-action={act('loop')}
            >
              <Repeat2 size={14} />
              LOOP{tag('loop')}
            </button>
            <button disabled={!track} onClick={() => send('loopIn')} data-action={act('loopIn')}>
              IN{tag('loopIn')}
            </button>
            <button disabled={!track} onClick={() => send('loopOut')} data-action={act('loopOut')}>
              OUT{tag('loopOut')}
            </button>
          </div>
        </div>
        <Slider
          className="pitch"
          label="PITCH"
          tone={tone}
          min={0.8}
          max={1.2}
          origin={1}
          defaultValue={1}
          value={slot.rate}
          format={(v) => `${v >= 1 ? '+' : '−'}${Math.abs((v - 1) * 100).toFixed(1)}%`}
          onChange={(v) => parameter(index, 'rate', v, onError)}
        />
      </div>
    </section>
  );
}
