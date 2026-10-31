import { expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { assignSources, Follower, LOST_MS, shouldRun } from './tracking';
import type { Source } from './tracking';

const hand = (x: number, pinch = 1): Source => ({ x, y: 0.5, press: pinch, kind: 'hand' });
const face = (size: number): Source => ({ x: 0.5, y: 0.5, press: size, kind: 'face' });

test('the camera only runs when wanted, in view and in front', () => {
  expect(shouldRun('hands', true, true)).toBe(true);
  expect(shouldRun('face', true, true)).toBe(true);
  expect(shouldRun('off', true, true)).toBe(false);
  expect(shouldRun('hands', false, true)).toBe(false);
  expect(shouldRun('hands', true, false)).toBe(false);
});

test('hands go to the surface on their side, as in a mirror', () => {
  const pair = (x: number) => ({ x, y: 0.5, pinch: 1, size: 0.2 });
  // Raw x 0.8 is on the left of a mirrored picture.
  const [left, right] = assignSources({ hands: [pair(0.2), pair(0.8)] });
  expect(left?.x).toBeCloseTo(0.2);
  expect(right?.x).toBeCloseTo(0.8);
  expect(assignSources({ hands: [pair(0.8)] })[0]).not.toBeNull();
  expect(assignSources({ hands: [pair(0.8)] })[1]).toBeNull();
  expect(assignSources({ hands: [pair(0.2)] })[0]).toBeNull();
  expect(assignSources({ hands: [pair(0.2)] })[1]).not.toBeNull();
  expect(assignSources({})).toEqual([null, null]);
  expect(assignSources({ face: { x: 0.3, y: 0.4, size: 0.3 } })[0]?.kind).toBe('face');
});

test('a pinch presses and lets go at different points, so it does not flicker', () => {
  const follower = new Follower();
  expect(follower.update(hand(0.5, 1), 0)?.engaged).toBe(false);
  expect(follower.update(hand(0.5, 0.5), 10)?.engaged).toBe(false);
  expect(follower.update(hand(0.5, 0.4), 20)?.engaged).toBe(true);
  expect(follower.update(hand(0.5, 0.6), 30)?.engaged).toBe(true);
  expect(follower.update(hand(0.5, 0.8), 40)?.engaged).toBe(false);
});

test('leaning in presses for a face', () => {
  const follower = new Follower();
  expect(follower.update(face(0.2), 0)?.engaged).toBe(false);
  expect(follower.update(face(0.3), 10)?.engaged).toBe(true);
  expect(follower.update(face(0.25), 20)?.engaged).toBe(true);
  expect(follower.update(face(0.2), 30)?.engaged).toBe(false);
});

test('a hand out of sight lets go soon, not on a single missed frame', () => {
  const follower = new Follower();
  follower.update(hand(0.5, 0.2), 0);
  expect(follower.update(null, LOST_MS - 1)?.engaged).toBe(true);
  expect(follower.update(null, LOST_MS + 1)).toBeNull();
});

test('movement is steadied and stays in range at the edges of the view', () => {
  const follower = new Follower();
  follower.update(hand(0.5), 0);
  const next = follower.update(hand(0.9), 10);
  expect(next!.x).toBeGreaterThan(0.5);
  expect(next!.x).toBeLessThan(1);
  for (let i = 0; i < 40; i++) follower.update(hand(1.4), i * 10 + 20);
  expect(follower.update(hand(1.4), 500)!.x).toBeCloseTo(1, 2);
});

test('the camera is opt-in, off on launch, and needs a statement of its use', () => {
  const source = readFileSync('src/lib/tracking.ts', 'utf8');
  expect(source).toContain("useState<TrackMode>('off')");
  expect(source).not.toContain('localStorage');
  const plist = readFileSync('src-tauri/Info.plist', 'utf8');
  expect(plist).toContain('NSCameraUsageDescription');
  const capture = readFileSync('src-tauri/native/CuradoTracker.swift', 'utf8');
  // Nothing goes anywhere but out on its own pipe to the app.
  expect(capture).not.toMatch(/URLSession|URLRequest|\.write\(to:/);
  expect(capture).toContain('alwaysDiscardsLateVideoFrames');
});
