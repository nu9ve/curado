import { expect, test } from 'bun:test';
import { actionLabel } from './keyboard';
import {
  bindingKey,
  detectedLabel,
  djWizard,
  learnTargets,
  midiKeyLabel,
  padWizard,
  targetId,
  targetLabel,
} from './midi';
import type { MidiBinding } from './types';

test('controls read as a MIDI manual writes them', () => {
  expect(midiKeyLabel({ kind: 'note', channel: 0, number: 0x0b })).toBe('CH 1 · NOTE 0x0B');
  expect(midiKeyLabel({ kind: 'cc', channel: 6, number: 0x1f })).toBe('CH 7 · CC 0x1F');
  expect(midiKeyLabel({ kind: 'pitchBend', channel: 0, number: 0 })).toBe('CH 1 · PITCH BEND');
  const binding = { status: 0xb0, channel: 1, number: 0x13 } as MidiBinding;
  expect(bindingKey(binding)).toEqual({ kind: 'cc', channel: 1, number: 0x13 });
  const fader = detectedLabel({
    key: { kind: 'cc', channel: 0, number: 0x13 },
    lsb: 0x33,
    mode: { type: 'absolute' },
    bits: 14,
    min: 0,
    max: 16383,
    count: 200,
    mirrors: [],
  });
  expect(fader).toContain('14');
  expect(fader).toContain('0–16383');
});

test('every learnable action has a name and appears once', () => {
  const targets = learnTargets();
  expect(new Set(targets.map(targetId)).size).toBe(targets.length);
  for (const t of targets) expect(actionLabel(t.kind)).not.toBe(t.kind);
  expect(targetLabel({ slot: 1, kind: 'cue', value: 2 })).toBe('Deck B · Hot cue 3');
  expect(targetLabel({ slot: 4, kind: 'trigger', value: 0 })).toBe('Sample 3 · Disparar sample');
});

test('the guided lists only ask for actions the list can learn', () => {
  const kinds = new Set(learnTargets().map((t) => `${t.slot}:${t.kind}`));
  for (const step of [...djWizard(), ...padWizard()]) {
    expect(kinds.has(`${step.action.slot}:${step.action.kind}`)).toBe(true);
    expect(step.ask.length).toBeGreaterThan(5);
  }
  // CUE and pad 1 both jump to hot cue 1, as on the hardware, so steps differ by what they ask.
  const dj = djWizard();
  expect(new Set(dj.map((s) => s.ask)).size).toBe(dj.length);
});

test('every learnable action is one the engine or MIDI itself takes', async () => {
  const engine = await Bun.file(
    new URL('../../crates/audio-engine/src/lib.rs', import.meta.url),
  ).text();
  const midi = await Bun.file(new URL('../../src-tauri/src/midi.rs', import.meta.url)).text();
  const validate = engine.slice(
    engine.indexOf('pub fn validate'),
    engine.indexOf('pub enum Command'),
  );
  const engineKinds = new Set([...validate.matchAll(/^\s*"([a-zA-Z]+)"/gm)].map((m) => m[1]));
  const own = midi.slice(midi.indexOf('pub fn validate_target'), midi.indexOf('fn wanted'));
  const midiKinds = new Set([...own.matchAll(/"([a-zA-Z]+)"/g)].map((m) => m[1]));
  for (const t of learnTargets())
    expect(engineKinds.has(t.kind) || midiKinds.has(t.kind)).toBe(true);
});
