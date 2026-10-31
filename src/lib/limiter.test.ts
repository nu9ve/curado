import { expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { createElement } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { LimiterLamp } from '../components/Controls';
import { describeHint } from './hints';
import { setLanguage } from './language';
import { limiterLevel } from './limiter';
import type { AppSnapshot } from './types';

const lamp = (reduction: number) => renderToStaticMarkup(createElement(LimiterLamp, { reduction }));

test('the lamp stays off for inaudible reduction and warns when the mix is flattened', () => {
  expect(limiterLevel(0)).toBe('idle');
  expect(limiterLevel(0.09)).toBe('idle');
  expect(limiterLevel(0.1)).toBe('active');
  expect(limiterLevel(5.9)).toBe('active');
  expect(limiterLevel(6)).toBe('hot');
  expect(limiterLevel(Number.NaN)).toBe('idle');
});

test('the lamp exposes level and reduction to assistive technology', () => {
  setLanguage('es');
  const idle = lamp(0);
  expect(idle).toContain('data-level="idle"');
  expect(idle).toContain('aria-valuetext="Sin reducción"');
  expect(idle).not.toContain('−');
  const hot = lamp(7.25);
  expect(hot).toContain('data-level="hot"');
  expect(hot).toContain('role="meter"');
  expect(hot).toContain('aria-valuenow="7.3"');
  expect(hot).toContain('−7.3');
});

test('both mix layouts show the lamp from the engine snapshot', () => {
  for (const file of ['src/components/Mixer.tsx', 'src/components/HardwareConsole.tsx']) {
    expect(readFileSync(file, 'utf8')).toContain(
      '<LimiterLamp reduction={audio?.limiterDb ?? 0} />',
    );
  }
});

test('footer help explains the lamp in both languages', () => {
  for (const [language, expected] of [
    ['es', 'limitador'],
    ['en', 'limiter'],
  ] as const) {
    setLanguage(language);
    const hint = describeHint({ label: 'LIM', help: 'limiter' }, [], {} as AppSnapshot);
    expect(hint.summary).toContain(expected);
  }
  setLanguage('es');
});
