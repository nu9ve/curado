import { expect, test } from 'bun:test';
import { stepScale } from './steps';

test('Option is the fine step and Shift the large one, as the suite decided', () => {
  expect(stepScale({ altKey: false, shiftKey: false })).toBe(1);
  expect(stepScale({ altKey: true, shiftKey: false })).toBe(0.1);
  expect(stepScale({ altKey: false, shiftKey: true })).toBe(4);
  expect(stepScale({ altKey: true, shiftKey: true })).toBe(0.1);
});
