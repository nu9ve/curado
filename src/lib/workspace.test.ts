import { expect, test } from 'bun:test';
import { clampSplit, DEFAULT_WORKSPACE, MAX_SPLIT, MIN_SPLIT, parseWorkspace } from './workspace';

test('a stored workspace survives bad or partial data field by field', () => {
  for (const text of [null, undefined, '', 'not json', '3', 'null'])
    expect(parseWorkspace(text)).toEqual(DEFAULT_WORKSPACE);
  const mixed = parseWorkspace('{"railMix":false,"panel":"nope","split":"x","info":false}');
  expect(mixed).toEqual({ ...DEFAULT_WORKSPACE, railMix: false, info: false });
  expect(parseWorkspace('{"panel":"tools"}').panel).toBe('tools');
});

test('the divider cannot hide either side', () => {
  expect(clampSplit(0)).toBe(MIN_SPLIT);
  expect(clampSplit(1)).toBe(MAX_SPLIT);
  expect(clampSplit(0.5)).toBe(0.5);
  expect(clampSplit(Number.NaN)).toBe(DEFAULT_WORKSPACE.split);
  expect(parseWorkspace('{"split":9}').split).toBe(MAX_SPLIT);
});

test('mixing starts with the rail and listening with the full sidebar', () => {
  expect(DEFAULT_WORKSPACE.railMix).toBe(true);
  expect(DEFAULT_WORKSPACE.railPlayer).toBe(false);
});
