import { useEffect, useRef, useState } from 'react';
import { call, native } from './bridge';

export type CoverSource =
  { kind: 'track'; id: number } | { kind: 'device'; device: string; track: number };
const MAX_KEPT = 600;
/** Reading covers from a USB one or two at a time keeps it from competing with a track loading. */
const AT_ONCE = 2;
const THUMB = { small: 96, large: 320 };
export const coverKey = (source: CoverSource, size: number) =>
  source.kind === 'track' ? `t:${source.id}:${size}` : `d:${source.device}:${source.track}:${size}`;

/** What a cover turned into, with a track without one remembered too. */
const known = new Map<string, string | null>();
const pending: { key: string; source: CoverSource; size: number; done: () => void }[] = [];
let running = 0;
export function rememberCover<T>(store: Map<string, T>, key: string, value: T, max: number) {
  // Oldest first in a Map, so the first key is the one to forget.
  store.delete(key);
  store.set(key, value);
  if (store.size > max) store.delete(store.keys().next().value as string);
}
/** Small enough to keep hundreds of, whatever size the file carried. */
async function shrink(bytes: ArrayBuffer, size: number): Promise<string | null> {
  if (!bytes.byteLength) return null;
  const image = await createImageBitmap(new Blob([bytes]));
  const canvas = document.createElement('canvas');
  canvas.width = canvas.height = size;
  const side = Math.min(image.width, image.height);
  canvas
    .getContext('2d')
    ?.drawImage(
      image,
      (image.width - side) / 2,
      (image.height - side) / 2,
      side,
      side,
      0,
      0,
      size,
      size,
    );
  image.close();
  return canvas.toDataURL('image/jpeg', 0.8);
}
function fetchCover(source: CoverSource) {
  return source.kind === 'track'
    ? call<ArrayBuffer>('track_cover', { id: source.id })
    : call<ArrayBuffer>('device_cover', { device: source.device, track: source.track });
}
function pump() {
  while (running < AT_ONCE && pending.length) {
    // The newest asked is what is on screen now; a fast scroll leaves older ones behind.
    const next = pending.pop()!;
    if (known.has(next.key)) {
      next.done();
      continue;
    }
    running++;
    fetchCover(next.source)
      .then((bytes) => shrink(bytes, next.size))
      .catch(() => null)
      .then((image) => {
        rememberCover(known, next.key, image, MAX_KEPT);
        next.done();
        running--;
        pump();
      });
  }
}
/** One observer however many rows a USB's list has. */
const waiting = new WeakMap<Element, () => void>();
let observer: IntersectionObserver | null = null;
function whenVisible(element: Element, visible: () => void) {
  observer ??= new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue;
        observer?.unobserve(entry.target);
        waiting.get(entry.target)?.();
      }
    },
    { rootMargin: '120px' },
  );
  waiting.set(element, visible);
  observer.observe(element);
  return () => observer?.unobserve(element);
}
/** The cover of a track once its place on screen is visible, or null while there is none. */
export function useCover(source: CoverSource | null, large = false) {
  const element = useRef<HTMLElement & HTMLDivElement & HTMLSpanElement>(null);
  const size = large ? THUMB.large : THUMB.small;
  const key = source ? coverKey(source, size) : null;
  const [image, setImage] = useState<string | null>(null);
  useEffect(() => {
    setImage(key ? (known.get(key) ?? null) : null);
    if (!native || !source || !key || known.has(key) || !element.current) return;
    let current = true;
    const entry = {
      key,
      source,
      size,
      done: () => {
        if (current) setImage(known.get(key) ?? null);
      },
    };
    const stop = whenVisible(element.current, () => {
      pending.push(entry);
      pump();
    });
    return () => {
      current = false;
      stop();
      // Scrolled past before its turn came: nobody is waiting for this one any more.
      const queued = pending.indexOf(entry);
      if (queued >= 0) pending.splice(queued, 1);
    };
    // The key says everything the source does.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  return { element, image };
}
