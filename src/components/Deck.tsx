import { useTheme, currentPalette } from '../lib/theme-store';
import { Play, Pause, Repeat2, Flag, Disc3 } from 'lucide-react';
import { useSnapshot, control, parameter } from '../lib/bridge';
import type { KeyBinding, Track } from '../lib/types';
import { EMPTY_SLOT, time } from '../lib/types';
import { bindingLabel } from '../lib/keyboard';
import { Waveform } from './Waveform';
import { Fader, Key, Meter } from './Controls';
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
  useTheme();
  const colors = currentPalette();
  const snap = useSnapshot();
  const slot = snap.audio?.slots[index] ?? EMPTY_SLOT;
  const track = tracks.find((t) => t.id === slot.id);
  const send = (kind: string, value = 0) => control(index, kind, value).catch(onError);
  const param = (kind: string, value: number) => parameter(index, kind, value, onError);
  const tag = (kind: string, value = 0, numeric = false) => {
    const candidates = keys.filter(
      (k) => k.action.slot === index && k.action.kind === kind && k.action.value === value,
    );
    const b =
      (numeric ? candidates.find((k) => k.code.startsWith('Digit')) : undefined) ?? candidates[0];
    return b ? <Key>{bindingLabel(b)}</Key> : null;
  };
  return (
    <section className={`deck deck-${index}`} aria-label={`Deck ${index ? 'B' : 'A'}`}>
      <div className="deck-top">
        <span className="deck-letter">{index ? 'B' : 'A'}</span>
        <span className="eyebrow">DECK {index ? 'B' : 'A'}</span>
        <span className={`status-dot ${slot.playing ? 'active' : ''}`} />
        <small>
          {loading ? 'Cargando…' : slot.playing ? 'EN REPRODUCCIÓN' : track ? 'LISTO' : 'SIN PISTA'}
        </small>
        <span className="deck-clock">
          {time(slot.position)} <em>/ {time(slot.duration)}</em>
        </span>
      </div>
      <div className="track-identity">
        <div className={`record ${slot.playing ? 'spinning' : ''}`}>
          <Disc3 size={38} strokeWidth={1} />
        </div>
        <div>
          <h2>{track?.title ?? 'Tu próxima selección'}</h2>
          <p>{track?.artist ?? `Selecciona una pista y cárgala en ${index ? 'B' : 'A'}`}</p>
        </div>
        <div className="speed-value">
          {slot.rate.toFixed(2)}
          <small>VELOCIDAD</small>
        </div>
      </div>
      <Waveform
        peaks={track?.peaks ?? []}
        progress={slot.duration ? slot.position / slot.duration : 0}
        color={index ? colors.secondary : colors.accent}
        onSeek={track ? (f) => send('seek', f * slot.duration) : undefined}
      />
      <div className="wave-caption">
        <span>
          {slot.looping
            ? `LOOP ${time(slot.loopIn)} → ${time(slot.loopOut)}`
            : 'Haz clic para buscar en la pista'}
        </span>
        <span>
          {track ? `${(track.sampleRate / 1000).toFixed(1)} kHz · estéreo` : 'AUDIO NATIVO'}
        </span>
      </div>
      <div className="transport">
        <button
          className={`play-button ${slot.playing ? 'playing' : ''}`}
          disabled={!track || loading}
          onClick={() => send('toggle')}
          aria-label={`${slot.playing ? 'Pausar' : 'Reproducir'} deck ${index ? 'B' : 'A'}`}
        >
          {slot.playing ? (
            <Pause size={17} fill="currentColor" />
          ) : (
            <Play size={17} fill="currentColor" />
          )}
          {slot.playing ? 'Pausa' : 'Play'}
          {tag('toggle')}
        </button>
        <button disabled={!track} onClick={() => send('cue', 0)}>
          <Flag size={14} />
          Cue{tag('cue', 0)}
        </button>
        <div className="transport-spacer" />
        <button
          className={slot.looping ? 'selected' : ''}
          disabled={!track}
          onClick={() => send('loop')}
        >
          <Repeat2 size={15} />
          Loop{tag('loop')}
        </button>
        <button disabled={!track} onClick={() => send('loopIn')}>
          IN
        </button>
        <button disabled={!track} onClick={() => send('loopOut')}>
          OUT
        </button>
      </div>
      <div className="cue-row">
        <span>HOT CUES</span>
        {[0, 1, 2].map((i) => (
          <button
            key={i}
            title="Clic para saltar. Shift + clic para guardar posición."
            disabled={!track}
            onClick={(e) => send(e.shiftKey ? 'setCue' : 'cue', i)}
          >
            {tag('cue', i, true)}
            <b>{time(slot.cues[i] ?? 0)}</b>
          </button>
        ))}
        <small>⇧ guardar</small>
      </div>
      <div className="effects">
        <Fader label="Volumen" value={slot.gain} max={1.5} onChange={(v) => param('gain', v)} />
        <Fader
          label="Velocidad"
          min={0.8}
          max={1.2}
          value={slot.rate}
          format={(v) => `${((v - 1) * 100).toFixed(0)}%`}
          onChange={(v) => param('rate', v)}
        />
        <Fader label="Graves" max={2} value={slot.low} onChange={(v) => param('low', v)} />
        <Fader label="Medios" max={2} value={slot.mid} onChange={(v) => param('mid', v)} />
        <Fader label="Agudos" max={2} value={slot.high} onChange={(v) => param('high', v)} />
        <Fader label="Filtro LP" value={slot.filter} onChange={(v) => param('filter', v)} />
        <Fader label="Echo" value={slot.echo} onChange={(v) => param('echo', v)} />
        <div className="deck-meter">
          <span>NIVEL</span>
          <Meter value={slot.peak} />
        </div>
      </div>
    </section>
  );
}
