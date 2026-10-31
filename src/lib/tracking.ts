import { useEffect, useState, useSyncExternalStore } from 'react';
import { listen } from '@tauri-apps/api/event';
import { call, native } from './bridge';

export type TrackMode = 'off' | 'hands' | 'face';
export interface TrackFrame {
  hands?: { x: number; y: number; pinch: number; size: number }[];
  face?: { x: number; y: number; size: number } | null;
  jpeg?: string | null;
  status?: string | null;
}
export interface TrackPoint {
  x: number;
  y: number;
  engaged: boolean;
}
const SMOOTHING = 0.45;
/** Past this with no sighting a hand is gone, and whatever it held has to let go. */
export const LOST_MS = 300;
/** A pinch starts and ends at different distances, so one held near the line does not flicker. */
const PINCH_ON = 0.45;
const PINCH_OFF = 0.7;
const LEAN_ON = 0.28;
const LEAN_OFF = 0.23;
/** Reaching the very edge of a camera's view is awkward, so the middle of it is the whole range. */
const MARGIN = 0.1;
const fit = (value: number) => Math.max(0, Math.min(1, (value - MARGIN) / (1 - MARGIN * 2)));

/**
 * The picture comes unmirrored, so the side a person sees their hand on is the opposite one:
 * flipping it makes the left hand reach the left surface, like a mirror. Each half of the view is
 * a whole surface, so a hand reaches all of its own without crossing into the other's. A face has
 * one surface and the whole view.
 */
export function assignSources(frame: TrackFrame): [Source | null, Source | null] {
  if (frame.face) {
    return [{ x: 1 - frame.face.x, y: frame.face.y, press: frame.face.size, kind: 'face' }, null];
  }
  const sides: [Source | null, Source | null] = [null, null];
  for (const hand of frame.hands ?? []) {
    const mirrored = 1 - hand.x;
    const side = mirrored < 0.5 ? 0 : 1;
    sides[side] ??= {
      x: (mirrored - side * 0.5) * 2,
      y: hand.y,
      press: hand.pinch,
      kind: 'hand',
    };
  }
  return sides;
}
/**
 * Where to put the camera picture inside a surface so that a hand lands under its point, in
 * percent of the surface. It stretches, because half of a 4:3 picture is being shown on a wider
 * box, and being true to the point matters more here.
 */
export function overlayPlacement(index: number, mode: TrackMode) {
  const visible = 1 - 2 * MARGIN;
  const across = mode === 'face' ? visible : visible / 2;
  const first = mode === 'face' ? MARGIN : (index + MARGIN) / 2;
  return {
    left: -(first / across) * 100,
    width: (1 / across) * 100,
    top: -(MARGIN / visible) * 100,
    height: (1 / visible) * 100,
  };
}
export interface Source {
  x: number;
  y: number;
  /** Pinch distance for a hand, where smaller presses; size for a face, where larger does. */
  press: number;
  kind: 'hand' | 'face';
}
/** Turns noisy sightings into a steady point, and into a clean press and release. */
export class Follower {
  private point: TrackPoint | null = null;
  private seen = 0;
  update(source: Source | null, now: number): TrackPoint | null {
    if (!source) {
      if (this.point && now - this.seen > LOST_MS) this.point = null;
      return this.point;
    }
    this.seen = now;
    const x = fit(source.x);
    const y = fit(source.y);
    const was = this.point?.engaged ?? false;
    const engaged =
      source.kind === 'hand'
        ? source.press < (was ? PINCH_OFF : PINCH_ON)
        : source.press > (was ? LEAN_OFF : LEAN_ON);
    this.point = this.point
      ? {
          x: this.point.x + (x - this.point.x) * SMOOTHING,
          y: this.point.y + (y - this.point.y) * SMOOTHING,
          engaged,
        }
      : { x, y, engaged };
    return this.point;
  }
}

/** Why the camera is not in use, or that it is: what the interface shows. */
export function shouldRun(mode: TrackMode, visible: boolean, focused: boolean): boolean {
  return mode !== 'off' && visible && focused;
}

let overlay: TrackMode = 'off';
let points: [TrackPoint | null, TrackPoint | null] = [null, null];
let preview: string | null = null;
let status = 'off';
const followers = [new Follower(), new Follower()];
const listeners = new Set<() => void>();
const notify = () => listeners.forEach((listener) => listener());
const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
};
/** Whatever stops the frames, a held gesture must not stay held. */
function letGo() {
  if (!points[0] && !points[1]) return;
  points = [null, null];
  notify();
}
let lastFrame = 0;
function receive(frame: TrackFrame) {
  if (frame.status) {
    status = frame.status;
    if (status !== 'running') letGo();
    notify();
    return;
  }
  const now = performance.now();
  lastFrame = now;
  const sources = assignSources(frame);
  points = [followers[0].update(sources[0], now), followers[1].update(sources[1], now)];
  if (frame.jpeg) preview = frame.jpeg;
  notify();
}
function clear(next: string) {
  points = [null, null];
  preview = null;
  status = next;
  notify();
}
export const useTrackedPoint = (index: number) =>
  useSyncExternalStore(subscribe, () => points[index]);
/** The tracking mode when the picture should show inside the surfaces, or off. */
export const useTrackingOverlay = () => useSyncExternalStore(subscribe, () => overlay);
export function setTrackingOverlay(mode: TrackMode) {
  overlay = mode;
  notify();
}
export const useTrackingStatus = () => useSyncExternalStore(subscribe, () => status);
export const useTrackingPreview = () => useSyncExternalStore(subscribe, () => preview);

/**
 * Owns the camera from the interface's side. It is on only while asked for, on this screen and in
 * front: leaving, hiding the app or switching to another one stops it, which ends the helper and
 * frees the device, and coming back starts it again. Nothing here survives a launch.
 */
export function useTrackingControl(showPreview: boolean) {
  const [mode, setMode] = useState<TrackMode>('off');
  const [visible, setVisible] = useState(true);
  const [focused, setFocused] = useState(true);
  useEffect(() => {
    if (!native) return;
    const update = () => {
      setVisible(document.visibilityState === 'visible');
      setFocused(document.hasFocus());
    };
    update();
    document.addEventListener('visibilitychange', update);
    window.addEventListener('focus', update);
    window.addEventListener('blur', update);
    return () => {
      document.removeEventListener('visibilitychange', update);
      window.removeEventListener('focus', update);
      window.removeEventListener('blur', update);
    };
  }, []);
  const running = native && shouldRun(mode, visible, focused);
  useEffect(() => {
    if (!running) {
      clear(mode === 'off' ? 'off' : 'paused');
      return;
    }
    let stop: undefined | (() => void);
    let gone = false;
    void listen<TrackFrame>('tracking:frame', (event) => receive(event.payload)).then((off) => {
      if (gone) off();
      else stop = off;
    });
    clear('starting');
    call('tracking_start', { mode }).catch(() => clear('lost'));
    const watchdog = setInterval(() => {
      if (performance.now() - lastFrame > LOST_MS) letGo();
    }, LOST_MS);
    return () => {
      gone = true;
      clearInterval(watchdog);
      stop?.();
      call('tracking_stop').catch(() => {});
    };
  }, [running, mode]);
  useEffect(() => {
    if (running) call('tracking_preview', { on: showPreview }).catch(() => {});
    if (!showPreview) preview = null;
  }, [running, showPreview]);
  return { mode, setMode };
}
