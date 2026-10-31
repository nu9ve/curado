import { cueColor, useCueInfo } from '../lib/cues';
import { tr } from '../lib/language';
import { useEffect, useMemo, useRef, useState } from 'react';
import type { CSSProperties } from 'react';
import { useTheme, currentPalette } from '../lib/theme-store';
import { isDownbeat, isMapDownbeat } from '../lib/music';
import { useBeatMap } from '../lib/beats';
import { findPhrases, PHRASE_LABELS } from '../lib/phrases';
import { useLivePosition } from '../lib/position';
import { drawOverview, drawWindow, renderTiles, useWaveform } from '../lib/waveform';
import type { WaveColors } from '../lib/waveform';
import type { Slot } from '../lib/types';

function useColors(tone: 'a' | 'b'): {
  colors: WaveColors;
  style: ReturnType<typeof useTheme>['waveform'];
  crisp: boolean;
  key: string;
} {
  const theme = useTheme();
  return useMemo(() => {
    const p = currentPalette();
    const colors = { ...p.wave, accent: tone === 'b' ? p.secondary : p.accent };
    return {
      colors,
      style: theme.waveform,
      crisp: theme.style === 'pixel',
      key: `${JSON.stringify(colors)}|${theme.waveform}|${p.dark}`,
    };
  }, [theme, tone]);
}

function useSize<T extends HTMLElement>() {
  const ref = useRef<T>(null);
  const [size, setSize] = useState({ w: 0, h: 0 });
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const ro = new ResizeObserver(([entry]) => {
      const { width, height } = entry.contentRect;
      setSize((s) =>
        Math.round(s.w) === Math.round(width) && Math.round(s.h) === Math.round(height)
          ? s
          : { w: width, h: height },
      );
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  return [ref, size] as const;
}

interface OverviewProps {
  trackId?: number;
  slot: Pick<Slot, 'position' | 'duration' | 'cues' | 'looping' | 'loopIn' | 'loopOut'> &
    Partial<Pick<Slot, 'music'>>;
  tone?: 'a' | 'b';
  onSeek?: (seconds: number) => void;
  compact?: boolean;
  label?: string;
}
/**
 * Whole-track view. The canvas is drawn only when the data, size or theme change;
 * position updates move DOM overlays with transforms (no 20 Hz canvas repaint).
 */
export function WaveOverview({
  trackId,
  slot,
  tone = 'a',
  onSeek,
  compact = false,
  label = tr('Posición de reproducción', 'Playback position'),
}: OverviewProps) {
  const wave = useWaveform(trackId);
  const cueInfo = useCueInfo(trackId);
  const { colors, style, key } = useColors(tone);
  const canvas = useRef<HTMLCanvasElement>(null);
  const [box, size] = useSize<HTMLDivElement>();
  useEffect(() => {
    if (canvas.current) drawOverview(canvas.current, wave, colors, style);
    // `key` captures colors/style; size triggers redraw after layout changes.
  }, [wave, key, size]);
  const d = slot.duration;
  const progress = d ? Math.min(1, slot.position / d) : 0;
  const phrases = useMemo(
    () => (wave && slot.music ? findPhrases(wave, slot.music.bpm, slot.music.downbeat, d) : []),
    [wave, slot.music?.bpm, slot.music?.downbeat, d],
  );
  const seekAt = (clientX: number, el: HTMLElement) => {
    const r = el.getBoundingClientRect();
    onSeek?.(Math.max(0, Math.min(1, (clientX - r.left) / r.width)) * d);
  };
  return (
    <div
      ref={box}
      className={`wave-overview tone-${tone} ${compact ? 'compact' : ''}`}
      style={{ '--p': progress } as CSSProperties}
      role={onSeek ? 'slider' : 'img'}
      data-action={onSeek ? `${tone === 'b' ? 1 : 0}:seek:0` : undefined}
      aria-label={label}
      aria-valuemin={onSeek ? 0 : undefined}
      aria-valuemax={onSeek ? Math.round(d) : undefined}
      aria-valuenow={onSeek ? Math.round(slot.position) : undefined}
      tabIndex={onSeek ? 0 : undefined}
      onPointerDown={(e) => {
        if (!onSeek || !d) return;
        e.preventDefault();
        e.currentTarget.focus({ preventScroll: true });
        e.currentTarget.setPointerCapture(e.pointerId);
        seekAt(e.clientX, e.currentTarget);
      }}
      onPointerMove={(e) => {
        if (e.buttons === 1 && onSeek && d) seekAt(e.clientX, e.currentTarget);
      }}
      onKeyDown={(e) => {
        if (!onSeek || !d || (e.key !== 'ArrowRight' && e.key !== 'ArrowLeft')) return;
        e.preventDefault();
        e.stopPropagation();
        const delta = (e.shiftKey ? 1 : 5) * (e.key === 'ArrowRight' ? 1 : -1);
        onSeek(Math.max(0, Math.min(d, slot.position + delta)));
      }}
    >
      <canvas ref={canvas} aria-hidden="true" />
      {phrases.length > 0 && (
        <div className="wave-phrases" aria-hidden="true">
          {phrases.map((phrase) => (
            <i
              key={phrase.start}
              className={`phrase-${phrase.kind}`}
              style={{
                left: `${(phrase.start / d) * 100}%`,
                width: `${((phrase.end - phrase.start) / d) * 100}%`,
              }}
              title={PHRASE_LABELS[phrase.kind]}
            >
              <span>{PHRASE_LABELS[phrase.kind]}</span>
            </i>
          ))}
        </div>
      )}
      <i className="wave-played" />
      {d > 0 && slot.loopOut > slot.loopIn && (
        <i
          className={`wave-loop ${slot.looping ? 'active' : ''}`}
          style={{
            left: `${(slot.loopIn / d) * 100}%`,
            width: `${((slot.loopOut - slot.loopIn) / d) * 100}%`,
          }}
        />
      )}
      {d > 0 &&
        slot.cues.map((c, i) =>
          c > 0 ? (
            <i
              key={i}
              className="wave-cue"
              style={
                {
                  left: `${(c / d) * 100}%`,
                  '--cue-color': cueColor(cueInfo.hot[i]),
                } as CSSProperties
              }
            >
              <b>{i + 1}</b>
            </i>
          ) : null,
        )}
      {d > 0 &&
        cueInfo.memory.map((cue) => (
          <i
            key={cue.start}
            className="wave-memory"
            title={cue.label ?? undefined}
            style={{ left: `${(cue.start / d) * 100}%` }}
          />
        ))}
      <i className="wave-head" />
    </div>
  );
}

const WINDOW_SECONDS = 8;
/** Scrolling detail view with the playhead fixed in the centre, like a club deck. */
export function ZoomWave({
  trackId,
  slot,
  tone,
  onSeek,
  name,
}: {
  trackId?: number;
  slot: Slot;
  tone: 'a' | 'b';
  onSeek?: (seconds: number) => void;
  name: string;
}) {
  const wave = useWaveform(trackId);
  const cueInfo = useCueInfo(trackId);
  const position = useLivePosition(tone === 'b' ? 1 : 0, slot.position);
  const beatMap = useBeatMap(trackId, slot.loadTicket);
  const { colors, style, crisp, key } = useColors(tone);
  const canvas = useRef<HTMLCanvasElement>(null);
  const [box, size] = useSize<HTMLDivElement>();
  const tiles = useMemo(() => (wave ? renderTiles(wave, colors, style, 96) : []), [wave, key]);
  useEffect(() => {
    const el = canvas.current;
    if (!el || !size.w) return;
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    if (el.width !== Math.round(size.w * dpr)) el.width = Math.round(size.w * dpr);
    if (el.height !== Math.round(size.h * dpr)) el.height = Math.round(size.h * dpr);
  }, [size]);
  useEffect(() => {
    const el = canvas.current;
    const ctx = el?.getContext('2d');
    if (!el || !ctx || !size.w) return;
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const { w, h } = size;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, w, h);
    const p = currentPalette();
    const rate = wave?.rate ?? 50;
    const pxPerBin = w / (WINDOW_SECONDS * rate);
    const centre = w / 2;
    const toX = (seconds: number) => centre + (seconds - position) * rate * pxPerBin;
    if (slot.loopOut > slot.loopIn) {
      ctx.fillStyle = slot.looping ? `${p.accent}33` : `${p.text}14`;
      ctx.fillRect(toX(slot.loopIn), 0, (slot.loopOut - slot.loopIn) * rate * pxPerBin, h);
    }
    if (tiles.length) {
      ctx.imageSmoothingEnabled = !crisp;
      drawWindow(ctx, tiles, position * rate - centre / pxPerBin, pxPerBin, w, h);
    } else {
      ctx.fillStyle = `${p.text}22`;
      ctx.fillRect(0, h / 2 - 0.5, w, 1);
    }
    const drawBeat = (seconds: number, bar: boolean) => {
      const x = Math.round(toX(seconds));
      if (x < 0 || x > w) return;
      ctx.globalAlpha = bar ? 0.85 : 0.4;
      ctx.fillRect(x, 0, bar ? 2 : 1, h);
      ctx.globalAlpha = 1;
      ctx.fillRect(x, h - (bar ? 10 : 6), bar ? 2 : 1, bar ? 10 : 6);
    };
    if (beatMap && !slot.music.manual) {
      ctx.fillStyle = p.tokens['--text-muted'];
      const left = position - WINDOW_SECONDS / 2;
      const first = beatMap.findIndex((seconds) => seconds >= left);
      for (let n = Math.max(first, 0); n < beatMap.length; n++) {
        if (beatMap[n] > left + WINDOW_SECONDS) break;
        drawBeat(beatMap[n], isMapDownbeat(beatMap, n, slot.music.downbeat));
      }
    } else if (slot.music.bpm) {
      const beat = 60 / slot.music.bpm;
      const first = Math.max(
        0,
        Math.ceil((position - WINDOW_SECONDS / 2 - slot.music.beatOffset) / beat),
      );
      ctx.fillStyle = p.tokens['--text-muted'];
      for (let n = first; n < first + WINDOW_SECONDS / beat + 2; n++) {
        const time = slot.music.beatOffset + n * beat;
        drawBeat(time, isDownbeat(time, beat, slot.music.downbeat));
      }
    }
    // Played side slightly dimmed so the direction of travel reads at a glance.
    ctx.fillStyle = p.tokens['--wave-dim'];
    ctx.globalAlpha = 0.45;
    ctx.fillRect(0, 0, centre, h);
    ctx.globalAlpha = 1;
    slot.cues.forEach((c, i) => {
      if (c <= 0) return;
      const x = Math.round(toX(c));
      if (x < -10 || x > w + 10) return;
      ctx.fillStyle = cueColor(cueInfo.hot[i]) ?? (tone === 'b' ? p.secondary : p.accent);
      ctx.fillRect(x, 0, 2, h);
      ctx.fillRect(x, 0, 12, 12);
      ctx.fillStyle = tone === 'b' ? p.tokens['--deck-b-ink'] : p.tokens['--accent-ink'];
      ctx.font = '600 9px "Geist Mono", ui-monospace, monospace';
      ctx.fillText(String(i + 1), x + 3.5, 9.5);
    });
    ctx.fillStyle = p.text;
    ctx.fillRect(Math.round(centre) - 1, 0, 2, h);
    ctx.fillRect(Math.round(centre) - 5, 0, 10, 3);
    ctx.fillRect(Math.round(centre) - 5, h - 3, 10, 3);
  }, [
    tiles,
    size,
    position,
    slot.cues,
    cueInfo,
    beatMap,
    slot.loopIn,
    slot.loopOut,
    slot.looping,
    slot.music,
    crisp,
    tone,
    wave,
    key,
  ]);
  return (
    <div
      ref={box}
      className={`zoom-wave tone-${tone}`}
      role={onSeek ? 'slider' : 'img'}
      data-action={onSeek ? `${tone === 'b' ? 1 : 0}:seek:0` : undefined}
      tabIndex={onSeek ? 0 : undefined}
      aria-valuemin={onSeek ? 0 : undefined}
      aria-valuemax={onSeek ? Math.round(slot.duration) : undefined}
      aria-valuenow={onSeek ? Math.round(slot.position) : undefined}
      aria-label={tr(`Waveform detallada deck ${name}`, `Detailed waveform deck ${name}`)}
      onKeyDown={(e) => {
        if (!onSeek || !slot.duration || !['ArrowRight', 'ArrowLeft'].includes(e.key)) return;
        e.preventDefault();
        e.stopPropagation();
        const delta = (e.shiftKey ? 1 : 5) * (e.key === 'ArrowRight' ? 1 : -1);
        onSeek(Math.max(0, Math.min(slot.duration, slot.position + delta)));
      }}
      onPointerDown={(e) => {
        if (!onSeek || !slot.duration) return;
        const r = e.currentTarget.getBoundingClientRect();
        const seconds =
          slot.position + ((e.clientX - r.left - r.width / 2) / r.width) * WINDOW_SECONDS;
        onSeek(Math.max(0, Math.min(slot.duration, seconds)));
      }}
    >
      <canvas ref={canvas} aria-hidden="true" />
      <span className="zoom-tag">{name}</span>
    </div>
  );
}
