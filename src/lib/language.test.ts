import { afterEach, expect, test } from 'bun:test';
import { currentLanguage, setLanguage, tr, validLanguage } from './language';
import { actionLabel } from './keyboard';
import { deckStatus, describeHint } from './hints';
import { DEFAULT_KEYS } from './keyboard';
import type { AppSnapshot } from './types';

afterEach(() => setLanguage('es'));

test('language falls back to Spanish and switches without changing the strings', () => {
  expect(validLanguage(null)).toBe('es');
  expect(validLanguage('fr')).toBe('es');
  setLanguage('en');
  expect(currentLanguage()).toBe('en');
  expect(tr('pista', 'track')).toBe('track');
  setLanguage('es');
  expect(tr('pista', 'track')).toBe('pista');
});

test('English action help and transport status follow the selected language', () => {
  setLanguage('en');
  expect(actionLabel('setCue')).toBe('Save cue');
  expect(deckStatus(null, 0)).toBe('No track');
  const hint = describeHint({ label: '', actions: '0:cue:0' }, DEFAULT_KEYS, {
    mode: 'mix',
    audio: null,
  } as AppSnapshot);
  expect(hint.label).toContain('Hot cue 1');
  expect(hint.summary).toBe('Jump to the hot cue and play.');
  setLanguage('es');
  expect(actionLabel('setCue')).toBe('Guardar cue');
  expect(deckStatus(null, 0)).toBe('Sin pista');
});
