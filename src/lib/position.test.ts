import { expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { livePosition, STALE_MS } from './position';

test('a live position stands only while reports keep coming', () => {
  const live = { values: [12.5, 40], at: 1000 };
  expect(livePosition(live, 0, 12, 1010)).toBe(12.5);
  expect(livePosition(live, 1, 39, 1010)).toBe(40);
  expect(livePosition(live, 0, 12, 1000 + STALE_MS + 1)).toBe(12);
  expect(livePosition(live, 5, 3, 1010)).toBe(3);
  expect(livePosition(null, 0, 7, 1010)).toBe(7);
});

test('position comes from the engine and nothing advances it on its own', () => {
  const source = readFileSync('src/lib/position.ts', 'utf8');
  expect(source).not.toMatch(/setInterval|setTimeout/);
  expect(readFileSync('src/lib/bridge.ts', 'utf8')).toContain("'engine:position'");
  expect(readFileSync('src-tauri/src/lib.rs', 'utf8')).toContain('"engine:position"');
});
