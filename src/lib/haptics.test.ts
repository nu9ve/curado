import { expect, test } from 'bun:test';
import { detent, fineFactor, pressureFromForce } from './haptics';

test('each kind of passage feels different', () => {
  expect(detent(0.2, 0.2, 0, 1, 0.5, 10)).toBeNull();
  expect(detent(0.21, 0.22, 0, 1, 0.5, 10)).toBeNull();
  expect(detent(0.29, 0.31, 0, 1, 0, 10)).toBe('level');
  expect(detent(0.49, 0.51, 0, 1, 0.5, 10)).toBe('alignment');
  expect(detent(0.51, 0.49, 0, 1, 0.5, 10)).toBe('alignment');
  expect(detent(0.9, 1, 0, 1, 0.5, 10)).toBe('generic');
  expect(detent(0.1, 0, 0, 1, 0.5, 10)).toBe('generic');
});

test('holding at an end does not keep tapping', () => {
  expect(detent(1, 1, 0, 1, 0.5, 10)).toBeNull();
  expect(detent(0, 0, 0, 1, 0.5, 10)).toBeNull();
});

test('an origin at an end is only the end', () => {
  expect(detent(0.05, 0, 0, 1, 0, 10)).toBe('generic');
  expect(detent(0.49, 0.52, 0, 2, 1, 40)).toBe('level');
  expect(detent(0.99, 1.01, 0, 2, 1, 40)).toBe('alignment');
});

test('harder means finer, and normal clicks are not fine at all', () => {
  expect(pressureFromForce(0)).toBe(0);
  expect(pressureFromForce(1)).toBe(0);
  expect(pressureFromForce(1.5)).toBe(0.5);
  expect(pressureFromForce(3)).toBe(1);
  expect(fineFactor(0, false)).toBe(1);
  expect(fineFactor(1, false)).toBe(4);
  expect(fineFactor(0, true)).toBe(4);
  expect(fineFactor(1, true)).toBe(16);
});
