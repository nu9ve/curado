import { expect, test } from 'bun:test';
import { samplePadIntent } from './console-pads';

test('sample pads trigger the four shared sample slots and load only the selected track', () => {
  for (let i = 0; i < 4; i++) {
    expect(samplePadIntent(i, true, 23, false, false)).toEqual({ kind: 'trigger', slot: i + 2 });
    expect(samplePadIntent(i, true, 23, true, false)).toEqual({
      kind: 'load',
      slot: i + 2,
      id: 23,
    });
    expect(samplePadIntent(i, false, 23, false, false)).toEqual({
      kind: 'load',
      slot: i + 2,
      id: 23,
    });
  }
  expect(samplePadIntent(4, true, 23, false, false)).toBeNull();
  expect(samplePadIntent(0, false, null, false, false)).toBeNull();
  expect(samplePadIntent(0, true, null, true, false)).toBeNull();
  expect(samplePadIntent(0, true, 23, false, true)).toBeNull();
});
