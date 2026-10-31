import { expect, test } from 'bun:test';
import { createTrigger, triggerDepth } from './trigger-fx';

const setup = () => {
  const sent: [string, number][] = [];
  const waiting: (() => void)[] = [];
  const trigger = createTrigger(
    (run) => {
      waiting.push(run);
      return waiting.length;
    },
    () => waiting.pop(),
  );
  return {
    sent,
    waiting,
    trigger,
    send: (kind: string, value: number) => sent.push([kind, value]),
  };
};
const idle = { kind: 7, depth: 0.8, on: false };

test('pressing harder means more, from a level that is already audible', () => {
  expect(triggerDepth(0)).toBe(0.35);
  expect(triggerDepth(1)).toBe(1);
  expect(triggerDepth(5)).toBe(1);
});

test('holding runs it, letting go stops it and later gives the unit back', () => {
  const { sent, waiting, trigger, send } = setup();
  trigger.press(3, false, idle, send);
  expect(sent).toEqual([
    ['beatFx', 3],
    ['beatFxDepth', 0.35],
    ['beatFxOn', 1],
  ]);
  sent.length = 0;
  trigger.lift(3, send);
  expect(sent).toEqual([['beatFxOn', 0]]);
  expect(trigger.active()).toBeNull();
  waiting.pop()!();
  expect(sent.slice(1)).toEqual([
    ['beatFx', 7],
    ['beatFxDepth', 0.8],
  ]);
});

test('a unit that was already on stays on, with its own effect', () => {
  const { sent, trigger, send } = setup();
  trigger.press(3, false, { kind: 7, depth: 0.8, on: true }, send);
  sent.length = 0;
  trigger.lift(3, send);
  expect(sent).toEqual([
    ['beatFx', 7],
    ['beatFxDepth', 0.8],
  ]);
});

test('Shift keeps it until pressed again', () => {
  const { sent, trigger, send } = setup();
  trigger.press(3, true, idle, send);
  trigger.lift(3, send);
  expect(trigger.active()).toBe(3);
  sent.length = 0;
  trigger.press(3, false, { ...idle, kind: 3, on: true }, send);
  expect(sent).toEqual([['beatFxOn', 0]]);
  expect(trigger.active()).toBeNull();
});

test('a quick second pad keeps what to give back, and no late restore cuts it', () => {
  const { sent, waiting, trigger, send } = setup();
  trigger.press(3, false, idle, send);
  trigger.lift(3, send);
  trigger.press(4, false, { kind: 3, depth: 0.35, on: false }, send);
  expect(waiting).toHaveLength(0);
  sent.length = 0;
  trigger.lift(4, send);
  waiting.pop()!();
  expect(sent.slice(1)).toEqual([
    ['beatFx', 7],
    ['beatFxDepth', 0.8],
  ]);
});
