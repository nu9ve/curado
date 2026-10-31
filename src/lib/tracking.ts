import { useEffect, useState, useSyncExternalStore } from 'react';
import { listen } from '@tauri-apps/api/event';
import { call, native } from './bridge';

export type TrackMode = 'off' | 'hands' | 'face' | 'all';
export interface TrackFrame {
  hands?: { x: number; y: number; pinch: number; size: number }[];
  face?: { x: number; y: number; size: number } | null;
  jpeg?: string | null;
  status?: string | null;
}
export interface TrackPoint {
  x: number;
  y: number;
  /** Nearness, with 0.5 as where a pinch began or a face just leaning in. */
  z: number;
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
/** A pinch's own distance is the middle, since no two people or cameras agree on an absolute. */
const Z_RANGE = 1.25;
/** From just leaning in to about as near as a face stays in view. */
const FACE_NEAR = 0.15;
const Z_SMOOTHING = 0.3;
const unit = (value: number) => Math.max(0, Math.min(1, value));
const fit = (value: number) => Math.max(0, Math.min(1, (value - MARGIN) / (1 - MARGIN * 2)));

/**
 * The picture comes unmirrored, so the side a person sees their hand on is the opposite one:
 * flipping it makes the left hand reach the left surface, like a mirror. Each half of the view is
 * a whole surface, so a hand reaches all of its own without crossing into the other's. A face has
 * one surface and the whole view.
 */
export function assignSources(
  frame: TrackFrame,
  mode: TrackMode = 'hands',
): [Source | null, Source | null] {
  if (frame.face && mode !== 'all') {
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
      size: hand.size,
      kind: 'hand',
    };
  }
  return sides;
}
/** A face beside hands has all of the view to itself. */
export function faceSource(frame: TrackFrame): Source | null {
  if (!frame.face) return null;
  return { x: 1 - frame.face.x, y: frame.face.y, press: frame.face.size, kind: 'face' };
}
/** Where a point lands in the whole picture, in percent, from its own surface. */
export function inView(index: number, point: TrackPoint, mode: TrackMode) {
  const across = MARGIN + point.x * (1 - 2 * MARGIN);
  return {
    left: (mode === 'face' || index > 1 ? across : (index + across) / 2) * 100,
    bottom: (MARGIN + point.y * (1 - 2 * MARGIN)) * 100,
  };
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
  /** A hand's apparent size, which grows as it comes nearer. */
  size?: number;
  kind: 'hand' | 'face';
}
/** Turns noisy sightings into a steady point, and into a clean press and release. */
export class Follower {
  private point: TrackPoint | null = null;
  private seen = 0;
  private startSize = 0;
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
    if (engaged && !was) this.startSize = source.size ?? 0;
    const z =
      source.kind === 'face'
        ? unit(0.5 + ((source.press - LEAN_ON) / FACE_NEAR) * 0.5)
        : engaged && this.startSize > 0 && source.size
          ? unit(0.5 + (source.size / this.startSize - 1) * Z_RANGE)
          : 0.5;
    this.point = this.point
      ? {
          x: this.point.x + (x - this.point.x) * SMOOTHING,
          y: this.point.y + (y - this.point.y) * SMOOTHING,
          z: engaged && !was ? z : this.point.z + (z - this.point.z) * Z_SMOOTHING,
          engaged,
        }
      : { x, y, z, engaged };
    return this.point;
  }
}

/** Why the camera is not in use, or that it is: what the interface shows. */
export function shouldRun(mode: TrackMode, visible: boolean, focused: boolean): boolean {
  return mode !== 'off' && visible && focused;
}

let overlay: TrackMode = 'off';
let points: [TrackPoint | null, TrackPoint | null] = [null, null];
let facePoint: TrackPoint | null = null;
let runningMode: TrackMode = 'off';
const faceFollower = new Follower();
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
  if (!points[0] && !points[1] && !facePoint) return;
  points = [null, null];
  facePoint = null;
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
  const sources = assignSources(frame, runningMode);
  points = [followers[0].update(sources[0], now), followers[1].update(sources[1], now)];
  facePoint = runningMode === 'all' ? faceFollower.update(faceSource(frame), now) : null;
  if (frame.jpeg) preview = frame.jpeg;
  notify();
}
function clear(next: string) {
  points = [null, null];
  facePoint = null;
  preview = null;
  status = next;
  notify();
}
export const useTrackedFace = () => useSyncExternalStore(subscribe, () => facePoint);
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
    runningMode = mode;
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
