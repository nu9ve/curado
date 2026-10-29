import { tr } from '../lib/language';
import type { CSSProperties } from 'react';
import { Play, Pause, SkipBack, SkipForward, Maximize2, Volume2 } from 'lucide-react';
import { control, parameter, useSnapshot } from '../lib/bridge';
import type { KeyBinding, Track } from '../lib/types';
import { EMPTY_SLOT, remaining, time } from '../lib/types';
import { actionKey } from '../lib/feedback';
import { WaveOverview } from './Waveform';
import { Meter, Slider } from './Controls';
import { keyFor } from './Deck';
import { Logo } from './Logo';
import { ContextHelp } from './ContextHelp';

/** Generative cover from the track id: theme colors, a smooth gradient and dithered grain. */
export function TrackArt({ id, size = 'md' }: { id?: number; size?: 'sm' | 'md' | 'lg' }) {
  const seed = id ? Math.imul(id, 2654435761) >>> 0 : 0;
  const style = {
    '--art-mix': `${20 + (seed % 60)}%`,
    '--art-angle': `${seed % 360}deg`,
    '--art-x': `${(seed >> 8) % 100}%`,
    '--art-y': `${(seed >> 16) % 100}%`,
  } as CSSProperties;
  return (
    <div className={`track-art ${size} ${id ? '' : 'empty'}`} style={style} aria-hidden="true">
      {!id && <Logo size={size === 'lg' ? 64 : 22} />}
    </div>
  );
}

export function NowPlaying({
  tracks,
  keys,
  onError,
}: {
  tracks: Track[];
  keys: KeyBinding[];
  onError: (e: unknown) => void;
}) {
  const s = useSnapshot();
  const slot = s.audio?.slots[0] ?? EMPTY_SLOT;
  const track = tracks.find((t) => t.id === slot.id);
  return (
    <section className={`now-playing ${slot.playing ? 'is-playing' : ''}`}>
      <TrackArt id={track?.id} size="lg" />
      <div className="now-info">
        <span className="eyebrow">
          {s.queue.active
            ? `${tr('COLA', 'QUEUE')} ${s.queue.index + 1}/${s.queue.count}`
            : tr('AHORA SUENA', 'NOW PLAYING')}
          {s.queue.active &&
            s.queue.repeat !== 'off' &&
            ` · ${tr('REPETIR', 'REPEAT')} ${s.queue.repeat === 'one' ? tr('PISTA', 'TRACK') : tr('TODO', 'ALL')}`}
        </span>
        <h2>{track?.title ?? tr('dale espacio a tu música', 'give your music room to play')}</h2>
        <p>
          {track?.artist ??
            tr(
              'Importa una pista o prueba la sesión de ejemplo.',
              'Import a track or try the demo session.',
            )}
        </p>
        {s.queue.error && (
          <p className="error-text">
            {s.queue.active
              ? tr('Aviso de cola', 'Queue notice')
              : tr('Cola detenida', 'Queue stopped')}
            : {s.queue.error}
          </p>
        )}
      </div>
      <WaveOverview
        trackId={track?.id}
        slot={slot}
        onSeek={track ? (sec) => control(0, 'seek', sec).catch(onError) : undefined}
      />
      <div className="now-transport">
        <span className="now-time">{time(slot.position)}</span>
        <button
          className="round-play"
          disabled={!track}
          aria-label={slot.playing ? tr('Pausar', 'Pause') : tr('Reproducir', 'Play')}
          data-action={actionKey({ slot: 0, kind: 'toggle', value: 0 })}
          onClick={() => control(0, 'toggle').catch(onError)}
        >
          {slot.playing ? (
            <Pause size={22} fill="currentColor" />
          ) : (
            <Play size={22} fill="currentColor" />
          )}
        </button>
        {keyFor(keys, 0, 'toggle')}
        <span className="now-time end">{remaining(slot)}</span>
      </div>
    </section>
  );
}

export function BottomBar({
  tracks,
  onError,
  onNext,
  keys,
  helpExpanded,
  onHelpExpand,
}: {
  tracks: Track[];
  onError: (e: unknown) => void;
  onNext: (direction: number) => void;
  keys: KeyBinding[];
  helpExpanded: boolean;
  onHelpExpand: (expanded: boolean) => void;
}) {
  const s = useSnapshot();
  const a = s.audio?.slots[0] ?? EMPTY_SLOT;
  const track = tracks.find((t) => t.id === a.id);
  const progress = a.duration ? a.position / a.duration : 0;
  return (
    <footer className="bottom-bar">
      <div className="bottom-playback">
        <div className="bottom-track">
          <TrackArt id={track?.id} size="sm" />
          <span>
            <strong>{track?.title ?? tr('tu música, a tu ritmo', 'your music, your pace')}</strong>
            <small>
              {track?.artist ?? tr('Selecciona una pista para comenzar', 'Select a track to begin')}
            </small>
          </span>
        </div>
        <div className="bottom-transport">
          <button
            aria-label={tr('Pista anterior', 'Previous track')}
            disabled={!tracks.length}
            onClick={() => onNext(-1)}
          >
            <SkipBack size={14} />
          </button>
          <button
            className="mini-play"
            disabled={!track}
            aria-label={tr('Play o pausa deck A', 'Play or pause deck A')}
            data-action={actionKey({ slot: 0, kind: 'toggle', value: 0 })}
            onClick={() => control(0, 'toggle').catch(onError)}
          >
            {a.playing ? (
              <Pause size={15} fill="currentColor" />
            ) : (
              <Play size={15} fill="currentColor" />
            )}
          </button>
          <button
            aria-label={tr('Pista siguiente', 'Next track')}
            disabled={!tracks.length}
            onClick={() => onNext(1)}
          >
            <SkipForward size={14} />
          </button>
          <span className="mono">{time(a.position)}</span>
          <div className="bottom-progress" style={{ '--p': progress } as CSSProperties}>
            <i />
          </div>
          <span className="mono">{time(a.duration)}</span>
        </div>
        <div className="bottom-master">
          <Volume2 size={14} />
          <Slider
            vertical={false}
            label="MASTER"
            tone="master"
            action={actionKey({ slot: 0, kind: 'master', value: 0 })}
            defaultValue={0.75}
            value={s.audio?.master ?? 0.75}
            onChange={(v) => parameter(0, 'master', v, onError)}
          />
          <Meter value={s.audio?.peak ?? 0} label={tr('Nivel master', 'Master level')} />
        </div>
      </div>
      <ContextHelp keys={keys} expanded={helpExpanded} onExpand={onHelpExpand} />
    </footer>
  );
}

/** Always-on-top compact view: one line of identity, waveform and transport. */
export function MiniPlayer({
  tracks,
  onError,
  onExit,
  onNext,
}: {
  tracks: Track[];
  onError: (e: unknown) => void;
  onExit: () => void;
  onNext: (direction: number) => void;
}) {
  const s = useSnapshot();
  const a = s.audio?.slots[0] ?? EMPTY_SLOT;
  const track = tracks.find((t) => t.id === a.id);
  return (
    <div className="mini-player" data-tauri-drag-region>
      <TrackArt id={track?.id} size="sm" />
      <div className="mini-info" data-tauri-drag-region>
        <strong>{track?.title ?? 'curado'}</strong>
        <small>
          {track?.artist ?? tr('sin pista', 'no track')} · {remaining(a)}
        </small>
        <WaveOverview
          compact
          trackId={track?.id}
          slot={a}
          onSeek={track ? (sec) => control(0, 'seek', sec).catch(onError) : undefined}
        />
      </div>
      <div className="mini-controls">
        <button
          aria-label={tr('Pista anterior', 'Previous track')}
          onClick={() => onNext(-1)}
          disabled={!tracks.length}
        >
          <SkipBack size={13} />
        </button>
        <button
          className="mini-play"
          disabled={!track}
          aria-label={tr('Play o pausa', 'Play or pause')}
          data-action={actionKey({ slot: 0, kind: 'toggle', value: 0 })}
          onClick={() => control(0, 'toggle').catch(onError)}
        >
          {a.playing ? (
            <Pause size={15} fill="currentColor" />
          ) : (
            <Play size={15} fill="currentColor" />
          )}
        </button>
        <button
          aria-label={tr('Pista siguiente', 'Next track')}
          onClick={() => onNext(1)}
          disabled={!tracks.length}
        >
          <SkipForward size={13} />
        </button>
        <button
          aria-label={tr('Salir de la vista mini', 'Exit mini view')}
          title={tr('Vista completa', 'Full view')}
          onClick={onExit}
        >
          <Maximize2 size={13} />
        </button>
      </div>
    </div>
  );
}
