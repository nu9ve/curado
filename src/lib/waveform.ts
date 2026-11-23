import { useEffect, useState, useSyncExternalStore } from 'react';
import { call, native } from './bridge';
import { mix } from './theme';
import type { WaveStyle, WaveTime } from './theme';

/** Mirrors `analysis::encode` in the audio crate: `[version, binsPerSecond, 0, 0]` + 4 bytes/bin. */
export interface WaveData {
  rate: number;
  /** `[peak, low, mid, high]` per bin, linear amplitude 0–255. */
  bins: Uint8Array;
  count: number;
}
export function parseWaveform(buffer: ArrayBuffer): WaveData | null {
  const bytes = new Uint8Array(buffer);
  if (bytes.length < 8 || bytes[0] !== 1 || !bytes[1]) return null;
  const bins = bytes.subarray(4, 4 + Math.floor((bytes.length - 4) / 4) * 4);
  return { rate: bytes[1], bins, count: bins.length / 4 };
}

const MAX_CACHED = 24;
const cache = new Map<number, Promise<WaveData | null>>();
let revision = 0;
const listeners = new Set<() => void>();
const subscribe = (fn: () => void) => {
  listeners.add(fn);
  return () => {
    listeners.delete(fn);
  };
};
function fetchWaveform(id: number) {
  let entry = cache.get(id);
  if (!entry) {
    entry = call<ArrayBuffer>('track_waveform', { id })
      .then(parseWaveform)
      .catch(() => null);
    cache.set(id, entry);
    if (cache.size > MAX_CACHED) cache.delete(cache.keys().next().value!);
  }
  return entry;
}
/** Re-reads a waveform that may have been backfilled by a deck load (pre-v3 libraries). */
export async function refreshWaveform(id: number) {
  const known = await cache.get(id);
  if (cache.has(id) && known) return;
  cache.delete(id);
  revision++;
  listeners.forEach((fn) => fn());
}
export function useWaveform(id: number | undefined): WaveData | null {
  const rev = useSyncExternalStore(subscribe, () => revision);
  const [data, setData] = useState<{ id: number; wave: WaveData | null } | null>(null);
  useEffect(() => {
    if (!id || !native) return;
    let live = true;
    void fetchWaveform(id).then((wave) => {
      if (live) setData({ id, wave });
    });
    return () => {
      live = false;
    };
  }, [id, rev]);
  return data && data.id === id ? data.wave : null;
}

export interface WaveColors {
  low: string;
  mid: string;
  high: string;
  accent: string;
}
const GAIN = [1, 0.95, 1.12, 1.55];
const level = (v: number, band: number) => Math.min(1, ((v / 255) * GAIN[band]) ** 0.72);

/**
 * Draws one column. `bands` layers low → mid → high like a three-band club display;
 * `rgb` maps low/mid/high to red/green/blue; `mono` uses the accent.
 */
function column(
  ctx: CanvasRenderingContext2D,
  x: number,
  w: number,
  mid: number,
  half: number,
  v: ArrayLike<number>,
  colors: WaveColors,
  style: WaveStyle,
) {
  if (style === 'heatmap') {
    const row = (half * 2) / 3;
    for (const [index, color] of [colors.high, colors.mid, colors.low].entries()) {
      ctx.fillStyle = color;
      ctx.globalAlpha = level(v[3 - index], 3 - index);
      ctx.fillRect(x, mid - half + index * row, w, row);
    }
    ctx.globalAlpha = 1;
    return;
  }
  if (style === 'bands') {
    for (const [band, color] of [
      [1, colors.low],
      [2, colors.mid],
      [3, colors.high],
    ] as const) {
      const h = Math.max(band === 1 ? 0.5 : 0, level(v[band], band) * half);
      if (h <= 0) continue;
      ctx.fillStyle = color;
      ctx.fillRect(x, mid - h, w, h * 2);
    }
    return;
  }
  const h = Math.max(0.5, level(v[0], 0) * half);
  if (style === 'mono') {
    ctx.fillStyle = colors.accent;
  } else {
    const max = Math.max(1, v[1], v[2], v[3]);
    const c = (n: number) => Math.round(70 + (n / max) * 185);
    ctx.fillStyle = `rgb(${c(v[1])},${c(v[2])},${c(v[3])})`;
  }
  ctx.fillRect(x, mid - h, w, h * 2);
}

/** Whole-track overview: one column per device pixel group, max-reduced across bins. */
export function drawOverview(
  canvas: HTMLCanvasElement,
  wave: WaveData | null,
  colors: WaveColors,
  style: WaveStyle,
) {
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  const width = canvas.clientWidth,
    height = canvas.clientHeight;
  if (!width || !height) return;
  if (canvas.width !== Math.round(width * dpr)) canvas.width = Math.round(width * dpr);
  if (canvas.height !== Math.round(height * dpr)) canvas.height = Math.round(height * dpr);
  const ctx = canvas.getContext('2d');
  if (!ctx) return;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, width, height);
  if (!wave || !wave.count) {
    ctx.fillStyle = colors.accent;
    ctx.globalAlpha = 0.25;
    ctx.fillRect(0, height / 2 - 0.5, width, 1);
    ctx.globalAlpha = 1;
    return;
  }
  if (style === 'heatmap') {
    colors = heatmapColors(colors);
  }
  const step = 2;
  const columns = Math.floor(width / step);
  const v = [0, 0, 0, 0];
  for (let i = 0; i < columns; i++) {
    const from = Math.floor((i / columns) * wave.count);
    const to = Math.max(from + 1, Math.floor(((i + 1) / columns) * wave.count));
    v.fill(0);
    for (let b = from; b < to; b++)
      for (let k = 0; k < 4; k++) v[k] = Math.max(v[k], wave.bins[b * 4 + k]);
    column(
      ctx,
      i * step,
      step - (style === 'heatmap' ? 0 : style === 'bands' ? 0.5 : 1),
      height / 2,
      height / 2 - 1,
      v,
      colors,
      style,
    );
  }
}

function heatmapColors(colors: WaveColors): WaveColors {
  return {
    ...colors,
    low: mix(colors.low, colors.accent, 0.35),
    mid: mix(colors.mid, colors.accent, 0.55),
    high: mix(colors.high, colors.accent, 0.3),
  };
}

export function waveWindowSeconds(mode: WaveTime, rate: number): number {
  return 8 * (mode === 'playback' ? Math.max(0.5, Math.min(1.5, rate || 1)) : 1);
}

const TILE = 1024;
const CACHED_TILES = 4;

/** Renders visible waveform tiles on demand, with bounded work when a new track loads. */
export function waveTiles(wave: WaveData, colors: WaveColors, style: WaveStyle) {
  const tiles = new Map<number, HTMLCanvasElement>();
  const palette = style === 'heatmap' ? heatmapColors(colors) : colors;
  return (index: number) => {
    const known = tiles.get(index);
    if (known) return known;
    const start = index * TILE;
    const canvas = document.createElement('canvas');
    canvas.width = Math.min(TILE, wave.count - start);
    canvas.height = 96;
    const ctx = canvas.getContext('2d');
    if (!ctx) return canvas;
    for (let i = 0; i < canvas.width; i++)
      column(
        ctx,
        i,
        1,
        48,
        47,
        wave.bins.subarray((start + i) * 4, (start + i) * 4 + 4),
        palette,
        style,
      );
    tiles.set(index, canvas);
    if (tiles.size > CACHED_TILES) tiles.delete(tiles.keys().next().value!);
    return canvas;
  };
}

/** Copies the visible window of tiles. Cost: one drawImage per intersecting tile. */
export function drawWindow(
  ctx: CanvasRenderingContext2D,
  tileAt: ReturnType<typeof waveTiles>,
  count: number,
  firstBin: number,
  pxPerBin: number,
  width: number,
  height: number,
) {
  const lastBin = Math.min(count, firstBin + width / pxPerBin);
  for (let index = Math.max(0, Math.floor(firstBin / TILE)); index * TILE < lastBin; index++) {
    const tile = tileAt(index);
    const tileStart = index * TILE;
    const from = Math.max(firstBin, tileStart);
    const to = Math.min(lastBin, tileStart + tile.width);
    if (to <= from) continue;
    ctx.drawImage(
      tile,
      from - tileStart,
      0,
      to - from,
      tile.height,
      (from - firstBin) * pxPerBin,
      0,
      (to - from) * pxPerBin,
      height,
    );
  }
}
