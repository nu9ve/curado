import { cueColor, useCueInfo } from '../lib/cues';
import { tr } from '../lib/language';
import { useState } from 'react';
import { useLivePosition } from '../lib/position';
import type { CSSProperties } from 'react';
import type { PadMode } from '../lib/console-pads';
import { PerformancePads, useJog } from './Performance';
import { Play, Pause, Repeat2, ChevronsLeft, ChevronsRight } from 'lucide-react';
import { useSnapshot, control, parameter } from '../lib/bridge';
import type { KeyBinding, Track, Library } from '../lib/types';
import { MusicalControls } from './MusicalControls';
import { EMPTY_SLOT, remaining, time } from '../lib/types';
import { bindingLabel } from '../lib/keyboard';
import { actionKey } from '../lib/feedback';
import { deckStatus } from '../lib/hints';
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
  onError,
}: {
  slot: typeof EMPTY_SLOT;
  index: number;
  disabled: boolean;
  onToggle: () => void;
  onError: (e: unknown) => void;
}) {
  const turn = useLivePosition(index, slot.position) * 200;
  const jog = useJog(index, slot.playing, true, 1, onError);
  return (
    <button
      className={`platter ${slot.playing ? 'spinning' : ''}`}
      disabled={disabled}
      ref={jog.ref}
      {...jog.handlers}
      onPointerCancel={jog.handlers.onPointerUp}
      onClick={() => {
        if (!jog.wasDragged()) onToggle();
      }}
      aria-label={`${slot.playing ? tr('Pausar', 'Pause') : tr('Reproducir', 'Play')} deck ${index ? 'B' : 'A'}`}
      title={tr(
        'Clic: reproducir o pausar · arrastrar en círculo: scratch',
        'Click: play or pause · drag around: scratch',
      )}
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
  onLibrary,
}: {
  index: number;
  tracks: Track[];
  keys: KeyBinding[];
  onError: (e: unknown) => void;
  loading?: boolean;
  onLibrary: (data: Library) => void;
}) {
  const snap = useSnapshot();
  const slot = snap.audio?.slots[index] ?? EMPTY_SLOT;
  const cueInfo = useCueInfo(slot.id || undefined);
  const track = tracks.find((t) => t.id === slot.id);
  const tone = index ? 'b' : 'a';
  const send = (kind: string, value = 0) => control(index, kind, value).catch(onError);
  const act = (kind: string, value = 0) => actionKey({ slot: index, kind, value });
  const tag = (kind: string, value = 0, digit = false) => keyFor(keys, index, kind, value, digit);
  const empty = !track || loading;
  const [padMode, setPadMode] = useState<'cue' | 'loop' | 'jump' | 'roll'>('cue');
  return (
    <section
      className={`deck deck-${index} tone-${tone} ${slot.playing ? 'is-playing' : ''}`}
      aria-label={`Deck ${index ? 'B' : 'A'}`}
    >
      <header className="deck-head">
        <span className="deck-tag">{index ? 'B' : 'A'}</span>
        <div className="deck-title">
          <strong>
            {loading ? tr('cargando…', 'loading…') : (track?.title ?? tr('sin pista', 'no track'))}
          </strong>
          <small>
            {track?.artist ??
              tr(
                `Selecciona una pista y cárgala en ${index ? 'B' : 'A'}`,
                `Select a track and load it into ${index ? 'B' : 'A'}`,
              )}
          </small>
          <small className="deck-status">
            {loading ? tr('Cargando…', 'Loading…') : deckStatus(snap.audio, index)}
          </small>
        </div>
        <div className="deck-time">
          <b>{remaining(slot)}</b>
          <small>
            {time(slot.position)} · {time(slot.duration)}
          </small>
        </div>
      </header>
      <MusicalControls
        index={index}
        slot={slot}
        disabled={!!empty}
        onError={onError}
        onLibrary={onLibrary}
      />
      <WaveOverview
        trackId={track?.id}
        slot={slot}
        tone={tone}
        label={`Waveform deck ${index ? 'B' : 'A'}`}
        onSeek={!empty ? (s) => send('seek', s) : undefined}
      />
      <div className="deck-body">
        <Platter
          slot={slot}
          index={index}
          disabled={!!empty}
          onToggle={() => send('toggle')}
          onError={onError}
        />
        <div className="deck-pads">
          <div className="pad-tabs" role="group" aria-label={tr('Modo de pads', 'Pad mode')}>
            {(
              [
                ['cue', 'CUE'],
                ['loop', 'LOOP'],
                ['jump', 'JUMP'],
                ['roll', 'ROLL'],
              ] as const
            ).map(([mode, label]) => (
              <button
                key={mode}
                type="button"
                aria-pressed={padMode === mode}
                onClick={() => setPadMode(mode)}
              >
                {label}
              </button>
            ))}
          </div>
          <div className="cue-pads">
            {padMode !== 'cue' && (
              <PerformancePads
                mode={padMode}
                index={index}
                slot={slot}
                disabled={!!empty}
                padClass="cue-pad"
                activeClass="set"
                onError={onError}
              />
            )}
            {padMode === 'cue' &&
              slot.cues.map((position, i) => (
                <button
                  key={i}
                  className={`cue-pad ${position > 0 ? 'set' : ''} ${slot.cueEnds[i] > 0 ? 'loop' : ''}`}
                  style={{ '--cue-color': cueColor(cueInfo.hot[i]) } as CSSProperties}
                  title={
                    cueInfo.hot[i]?.label ??
                    tr(
                      'Clic para saltar y reproducir · Shift + clic para guardar',
                      'Click to jump and play · Shift-click to save',
                    )
                  }
                  aria-label={`Deck ${index ? 'B' : 'A'} · Hot cue ${i + 1}${cueInfo.hot[i]?.label ? ` · ${cueInfo.hot[i]?.label}` : ''}`}
                  disabled={!!empty}
                  data-action={`${act('cue', i)} ${act('setCue', i)}`}
                  onClick={(e) => send(e.shiftKey ? 'setCue' : 'cue', i)}
                >
                  <span>CUE {i + 1}</span>
                  <b>{time(position)}</b>
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
              aria-label={`${slot.playing ? tr('Pausar', 'Pause') : tr('Reproducir', 'Play')} deck ${index ? 'B' : 'A'}`}
            >
              {slot.playing ? (
                <Pause size={16} fill="currentColor" />
              ) : (
                <Play size={16} fill="currentColor" />
              )}
              {tag('toggle')}
            </button>
            <button disabled={!!empty} onClick={() => send('cue', 0)} data-action={act('cue', 0)}>
              CUE{tag('cue', 0)}
            </button>
            <button
              disabled={!track}
              aria-label={tr('Nudge atrás', 'Nudge backward')}
              onClick={() => send('nudge', -0.05)}
              data-action={act('nudge', -0.05)}
            >
              <ChevronsLeft size={14} />
              {tag('nudge', -0.05)}
            </button>
            <button
              disabled={!track}
              aria-label={tr('Nudge adelante', 'Nudge forward')}
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
            <button
              disabled={!track || slot.loopOut <= slot.loopIn}
              onClick={() => send('loopScale', 0.5)}
              data-action={act('loopScale', 0.5)}
            >
              1/2X
            </button>
            <button
              disabled={!track || slot.loopOut <= slot.loopIn}
              onClick={() => send('loopScale', 2)}
              data-action={act('loopScale', 2)}
            >
              2X
            </button>
          </div>
          <div className="loop-row">
            <button
              className={slot.slip ? 'selected' : ''}
              aria-pressed={slot.slip}
              disabled={!track}
              onClick={() => send('slip', slot.slip ? 0 : 1)}
              data-action={act('slip', slot.slip ? 0 : 1)}
            >
              SLIP
            </button>
            <button
              className={slot.reverse ? 'selected' : ''}
              aria-pressed={slot.reverse}
              disabled={!track}
              onClick={() => send('reverse', slot.reverse ? 0 : 1)}
              data-action={act('reverse', slot.reverse ? 0 : 1)}
            >
              REV
            </button>
          </div>
        </div>
        <Slider
          className="pitch"
          label="PITCH"
          action={act('rate')}
          tone={tone}
          min={0.5}
          max={1.5}
          origin={1}
          defaultValue={1}
          value={slot.tuning.rate}
          format={(v) => `${v >= 1 ? '+' : '−'}${Math.abs((v - 1) * 100).toFixed(1)}%`}
          onChange={(v) => parameter(index, 'rate', v, onError)}
        />
      </div>
    </section>
  );
}
