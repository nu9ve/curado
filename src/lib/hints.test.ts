import { expect, test } from 'bun:test';
import { renderToStaticMarkup } from 'react-dom/server';
import { createElement } from 'react';
import { ContextHelpPanel } from '../components/ContextHelp';
import { describeHint, deckStatus, parseHintAction } from './hints';
import { DEFAULT_KEYS } from './keyboard';
import { EMPTY_SLOT } from './types';
import type { AppSnapshot, Audio } from './types';

const audio = (): Audio => ({
  beatFx: [],
  sampler: 1,
  autoGain: false,
  crossfaderCurve: 1,
  cueMaster: false,
  cueMix: 0,
  cueLevel: 0.7,
  masterDeck: 0,
  slots: Array.from({ length: 6 }, (_, index) => ({ ...EMPTY_SLOT, id: index + 1 })),
  peak: 0,
  rms: 0,
  waveform: [],
  crossfader: 0.5,
  master: 0.75,
  streamError: false,
  callbacks: 0,
  clipped: 0,
  limited: 0,
  limiterDb: 0,
});
const snapshot = (): AppSnapshot => ({
  devices: [],
  mode: 'mix',
  audio: audio(),
  output: null,
  error: null,
  persistenceError: null,
  queue: { count: 0, index: 0, active: false, repeat: 'off', error: null },
  midi: {
    connected: null,
    bindings: [],
    learning: null,
    lastMessage: null,
    error: null,
    dropped: 0,
  },
});

test('hints identify the exact deck and cue with current remapped shortcuts', () => {
  const keys = DEFAULT_KEYS.map((key) =>
    key.code === 'Digit7' && !key.shift && !key.ctrl && !key.alt ? { ...key, code: 'KeyB' } : key,
  );
  const hint = describeHint({ label: 'CUE 1', actions: '1:cue:0 1:setCue:0' }, keys, snapshot());
  expect(hint.label).toBe('Deck B · Hot cue 1');
  expect(hint.summary).toContain('reproduce');
  expect(hint.shortcuts).toContain('B');
  expect(hint.shortcuts).toContain('⇧ 7');
  expect(hint.shortcuts).not.toContain('7');
  expect(hint.tone).toBe('b');
  expect(describeHint({ label: '', actions: '0:cue:2' }, keys, snapshot()).label).toBe(
    'Deck A · Hot cue 3',
  );
});

test('parameter hints show all mapped values but cue hints keep their cue number', () => {
  const s = snapshot();
  const hint = describeHint(
    { label: 'CROSSFADER', actions: '0:crossfader:0', slider: true },
    DEFAULT_KEYS,
    s,
  );
  expect(hint.shortcuts).toEqual(['⌃ ←', '⌃ ↓', '⌃ →']);
  expect(describeHint({ label: '', actions: '1:cue:1' }, DEFAULT_KEYS, s).shortcuts).toEqual(['8']);
  s.mode = 'player';
  expect(describeHint({ label: '', actions: '0:cueBoth:0' }, DEFAULT_KEYS, s).status).toContain(
    'Mezcla',
  );
});

test('transport diagnostics distinguish stopped decks from excluded output without claiming audibility', () => {
  const a = audio();
  expect(deckStatus(null, 1)).toBe('Sin pista');
  expect(deckStatus(a, 1)).toBe('En pausa');
  a.slots[1].duration = 1;
  a.slots[1].position = 1;
  expect(deckStatus(a, 1)).toBe('Fin de pista');
  a.slots[1].playing = true;
  a.crossfader = 0;
  expect(deckStatus(a, 1)).toBe('Fuera del crossfader');
  expect(deckStatus(a, 2)).toBe('En pausa');
  a.slots[2].playing = true;
  expect(deckStatus(a, 2)).toBe('Reproduciendo');
  a.crossfader = 0.5;
  a.slots[1].gain = 0;
  expect(deckStatus(a, 1)).toBe('Canal a cero');
  a.master = 0;
  expect(deckStatus(a, 1)).toBe('Master a cero');
  a.streamError = true;
  expect(deckStatus(a, 1)).toBe('Error de salida');
});

test('unknown controls use their own label/title and never borrow a neighbouring action', () => {
  const s = snapshot();
  const hint = describeHint(
    { label: 'Retirar', title: 'Retirar sin borrar el archivo', disabled: true },
    DEFAULT_KEYS,
    s,
  );
  expect(hint.label).toBe('Retirar');
  expect(hint.summary).toBe('Retirar sin borrar el archivo');
  expect(hint.status).toContain('no disponible');
  expect(hint.shortcuts).toEqual([]);
  for (const value of ['wrong', '6:cue:0', '0:cue:NaN', '0.5:cue:0'])
    expect(parseHintAction(value)).toBeUndefined();
});

test('help disclosure stays keyboard accessible and starts with hidden details', () => {
  const collapsed = renderToStaticMarkup(
    createElement(ContextHelpPanel, { hint: null, expanded: false, onExpand: () => {} }),
  );
  expect(collapsed).toContain('aria-expanded="false"');
  expect(collapsed).toContain('aria-controls="context-help-details"');
  expect(collapsed).toContain('hidden=""');
  const expanded = renderToStaticMarkup(
    createElement(ContextHelpPanel, { hint: null, expanded: true, onExpand: () => {} }),
  );
  expect(expanded).toContain('aria-expanded="true"');
  expect(expanded).not.toContain('hidden=""');
});
