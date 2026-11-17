import { actionLabel } from './keyboard';
import { tr } from './language';
import type { Action, MidiBinding, MidiDetected, MidiKey, MidiMode } from './types';

const hex = (n: number) => `0x${n.toString(16).toUpperCase().padStart(2, '0')}`;

/** How a MIDI manual writes a control: channels from 1, numbers in hex. */
export function midiKeyLabel(key: MidiKey) {
  const what =
    key.kind === 'note'
      ? `NOTE ${hex(key.number)}`
      : key.kind === 'cc'
        ? `CC ${hex(key.number)}`
        : 'PITCH BEND';
  return `CH ${key.channel + 1} · ${what}`;
}

export function bindingKey(binding: MidiBinding): MidiKey {
  const kind = binding.status === 0x90 ? 'note' : binding.status === 0xe0 ? 'pitchBend' : 'cc';
  return { kind, channel: binding.channel, number: binding.number };
}

export const sameKey = (a: MidiKey, b: MidiKey) =>
  a.kind === b.kind && a.channel === b.channel && a.number === b.number;

/** What kind of control it is, in words. */
export function modeLabel(mode: MidiMode | null, lsb: number | null, kind?: MidiKey['kind']) {
  if (!mode || mode.type === 'auto')
    return kind === 'note' ? tr('botón', 'button') : tr('sin identificar', 'not identified');
  if (mode.type === 'button') return tr('botón', 'button');
  if (mode.type === 'relative') return tr('encoder relativo', 'relative encoder');
  return lsb !== null || kind === 'pitchBend'
    ? tr('fader/perilla 14 bits', 'fader/knob 14-bit')
    : tr('fader/perilla 7 bits', 'fader/knob 7-bit');
}

export function detectedLabel(d: MidiDetected) {
  const range = d.mode.type === 'absolute' ? ` · ${d.min}–${d.max}` : '';
  const mirrors = d.mirrors.length
    ? ` · ${tr('repite en', 'repeated on')} ${d.mirrors.map(midiKeyLabel).join(', ')}`
    : '';
  return `${midiKeyLabel(d.key)} · ${modeLabel(d.mode, d.lsb, d.key.kind)}${range}${mirrors}`;
}

const DECKS = ['A', 'B'];

/** Actions whose value is which hot cue. */
export const CUE_KINDS = ['cue', 'setCue', 'clearCue'];

/** What a learned control does, with its deck or sample; `numbered` adds which cue or length. */
export function targetLabel(action: Action, numbered = true) {
  const { slot, kind, value } = action;
  if (kind === 'trigger') return `Sample ${slot - 1} · ${actionLabel(kind)}`;
  if (['crossfader', 'master', 'cueMix', 'cueLevel', 'panic', 'listMove'].includes(kind))
    return `Mixer · ${actionLabel(kind)}`;
  if (kind === 'sampler') return `Mixer · ${actionLabel(kind)}`;
  if (slot === 2 && kind.startsWith('beatFx')) return `Master · ${actionLabel(kind)}`;
  const deck = `Deck ${DECKS[slot] ?? slot + 1}`;
  if (numbered && CUE_KINDS.includes(kind)) return `${deck} · ${actionLabel(kind)} ${value + 1}`;
  if (numbered && kind === 'loopBeats') return `${deck} · ${actionLabel(kind)} ${value}`;
  return `${deck} · ${actionLabel(kind)}`;
}

/** The value an action is learned with: which cue, which length, which row step. */
export function learnValue(kind: string, cue = 0) {
  if (CUE_KINDS.includes(kind)) return cue;
  if (['rate', 'roll', 'listMove', 'loopBeats'].includes(kind)) return 1;
  // Toggles: MIDI flips them from what the engine reports, whatever was learned.
  if (['sync', 'keyLock', 'channelCue', 'slip', 'reverse', 'beatFxOn', 'quantize'].includes(kind))
    return 1;
  return 0;
}

const DECK_KINDS = [
  'toggle',
  'cue',
  'setCue',
  'clearCue',
  'sync',
  'keyLock',
  'channelCue',
  'gain',
  'rate',
  'high',
  'mid',
  'low',
  'color',
  'filter',
  'highPass',
  'echo',
  'trim',
  'colorParam',
  'jogTouch',
  'jog',
  'loop',
  'loopIn',
  'loopOut',
  'autoLoop',
  'loopBeats',
  'beatFxOn',
  'beatFxDepth',
  'slip',
  'reverse',
  'roll',
];

/** Everything a control can be learned for, in the order the list shows it. */
export function learnTargets(): Action[] {
  const deck = [0, 1].flatMap((slot) =>
    DECK_KINDS.map((kind) => ({ slot, kind, value: learnValue(kind) })),
  );
  const samples = [2, 3, 4, 5, 6, 7, 8, 9].map((slot) => ({ slot, kind: 'trigger', value: 0 }));
  const mixer: Action[] = [
    { slot: 0, kind: 'crossfader', value: 0 },
    { slot: 0, kind: 'master', value: 0 },
    { slot: 0, kind: 'cueMix', value: 0 },
    { slot: 0, kind: 'cueLevel', value: 0 },
    { slot: 2, kind: 'sampler', value: 0 },
    { slot: 2, kind: 'beatFxOn', value: 1 },
    { slot: 2, kind: 'beatFxDepth', value: 0 },
    { slot: 0, kind: 'listMove', value: 1 },
    { slot: 0, kind: 'listLoad', value: 0 },
    { slot: 1, kind: 'listLoad', value: 0 },
    { slot: 0, kind: 'panic', value: 0 },
  ];
  return [...deck, ...samples, ...mixer];
}

export const targetId = (a: Action) => `${a.slot}:${a.kind}:${a.value}`;

export interface WizardStep {
  /** What to touch, in the person's words. */
  ask: string;
  action: Action;
}

/** The guided setup: the controls most DJ controllers share, one at a time. */
export function djWizard(): WizardStep[] {
  const deck = (slot: number): WizardStep[] => {
    const d = `Deck ${DECKS[slot]}`;
    const a = (kind: string, value = learnValue(kind)) => ({ slot, kind, value });
    return [
      { ask: tr(`${d}: presiona PLAY`, `${d}: press PLAY`), action: a('toggle') },
      { ask: tr(`${d}: presiona CUE`, `${d}: press CUE`), action: a('cue', 0) },
      { ask: tr(`${d}: presiona SYNC`, `${d}: press SYNC`), action: a('sync') },
      {
        ask: tr(`${d}: sube y baja el fader de canal`, `${d}: move the channel fader`),
        action: a('gain'),
      },
      {
        ask: tr(`${d}: mueve el pitch/tempo`, `${d}: move the pitch/tempo fader`),
        action: a('rate'),
      },
      { ask: tr(`${d}: gira el EQ de agudos`, `${d}: turn the high EQ`), action: a('high') },
      { ask: tr(`${d}: gira el EQ de medios`, `${d}: turn the mid EQ`), action: a('mid') },
      { ask: tr(`${d}: gira el EQ de graves`, `${d}: turn the low EQ`), action: a('low') },
      { ask: tr(`${d}: gira el filtro`, `${d}: turn the filter`), action: a('color') },
      {
        ask: tr(`${d}: presiona el cue de audífonos`, `${d}: press the headphone cue`),
        action: a('channelCue'),
      },
      {
        ask: tr(`${d}: toca el jog sin girarlo`, `${d}: touch the jog without turning it`),
        action: a('jogTouch'),
      },
      { ask: tr(`${d}: gira el jog`, `${d}: turn the jog`), action: a('jog') },
      ...[0, 1, 2, 3].map((cue) => ({
        ask: tr(`${d}: pad ${cue + 1} en modo hot cue`, `${d}: pad ${cue + 1} in hot cue mode`),
        action: a('cue', cue),
      })),
    ];
  };
  return [
    ...deck(0),
    ...deck(1),
    {
      ask: tr('Mueve el crossfader', 'Move the crossfader'),
      action: { slot: 0, kind: 'crossfader', value: 0 },
    },
    {
      ask: tr('Gira el selector de la lista', 'Turn the browse knob'),
      action: { slot: 0, kind: 'listMove', value: 1 },
    },
    {
      ask: tr('Presiona LOAD del deck A', 'Press LOAD for deck A'),
      action: { slot: 0, kind: 'listLoad', value: 0 },
    },
    {
      ask: tr('Presiona LOAD del deck B', 'Press LOAD for deck B'),
      action: { slot: 1, kind: 'listLoad', value: 0 },
    },
    {
      ask: tr(
        'Gira la mezcla de audífonos (si envía MIDI)',
        'Turn the headphones mix (if it sends MIDI)',
      ),
      action: { slot: 0, kind: 'cueMix', value: 0 },
    },
    {
      ask: tr('Gira el master (si envía MIDI)', 'Turn the master (if it sends MIDI)'),
      action: { slot: 0, kind: 'master', value: 0 },
    },
  ];
}

/** For a keyboard or a pad controller: samples, then hot cues. */
export function padWizard(): WizardStep[] {
  return [
    ...[2, 3, 4, 5, 6, 7, 8, 9].map((slot) => ({
      ask: tr(`Tecla o pad para el sample ${slot - 1}`, `Key or pad for sample ${slot - 1}`),
      action: { slot, kind: 'trigger', value: 0 },
    })),
    ...[0, 1].flatMap((slot) =>
      [0, 1, 2, 3].map((cue) => ({
        ask: tr(
          `Tecla o pad para el hot cue ${cue + 1} del deck ${DECKS[slot]}`,
          `Key or pad for deck ${DECKS[slot]} hot cue ${cue + 1}`,
        ),
        action: { slot, kind: 'cue', value: cue },
      })),
    ),
  ];
}
