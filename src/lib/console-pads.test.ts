import { expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import {
  angleDelta,
  BEAT_DIVISIONS,
  BEAT_FX,
  BEAT_FX_ORDER,
  KEY_PADS,
  neighborCue,
  PAD_FX,
  beatLabel,
  COLOR_FX,
  JUMP_PADS,
  jogSeconds,
  LOOP_PADS,
  nextDivision,
  nextTempoRange,
  PITCH_STEPS,
  pitchLabel,
  ROLL_PADS,
  samplePadIntent,
  tapTempo,
} from './console-pads';

test('sample pads trigger the eight shared sample slots and load only the selected track', () => {
  for (let i = 0; i < 8; i++) {
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
  expect(samplePadIntent(8, true, 23, false, false)).toBeNull();
  expect(samplePadIntent(0, false, null, false, false)).toBeNull();
  expect(samplePadIntent(0, true, null, true, false)).toBeNull();
  expect(samplePadIntent(0, true, 23, false, true)).toBeNull();
});

test('division steps stay in range and read as a musician would write them', () => {
  expect(nextDivision(1, 1)).toBe(2);
  expect(nextDivision(1, -1)).toBe(0.75);
  expect(nextDivision(32, 1)).toBe(32);
  expect(nextDivision(0.0625, -1)).toBe(0.0625);
  expect(nextDivision(3, 1)).toBe(1);
  expect(BEAT_DIVISIONS.map(beatLabel)).toEqual([
    '1/16',
    '1/8',
    '1/4',
    '1/2',
    '3/4',
    '1',
    '2',
    '4',
    '8',
    '16',
    '32',
  ]);
  expect(nextTempoRange(0.06)).toBe(0.1);
  expect(nextTempoRange(0.5)).toBe(0.06);
});

test('platter angles take the short way across the wrap', () => {
  expect(angleDelta(3, -3)).toBeCloseTo(2 * Math.PI - 6);
  expect(angleDelta(-3, 3)).toBeCloseTo(6 - 2 * Math.PI);
  expect(angleDelta(0.1, 0.3)).toBeCloseTo(0.2);
  expect(jogSeconds(2 * Math.PI)).toBeCloseTo(1.8);
  expect(jogSeconds(Math.PI, 2)).toBeCloseTo(1.8);
});

test('tap tempo needs a few taps, averages them and restarts after a pause', () => {
  expect(tapTempo([])).toBeNull();
  expect(tapTempo([0, 500])).toBeNull();
  expect(tapTempo([0, 500, 1000])).toBe(120);
  expect(tapTempo([0, 500, 1000, 5000, 5400, 5800])).toBe(150);
  expect(tapTempo([0, 10, 20])).toBeNull();
  const many = Array.from({ length: 20 }, (_, i) => i * 400);
  expect(tapTempo(many)).toBe(150);
});

test('every pad value is one the engine accepts', () => {
  const loops = [0.0625, 0.125, 0.25, 0.5, 1, 2, 4, 8, 16, 32];
  for (const value of [...LOOP_PADS, ...ROLL_PADS]) expect(loops).toContain(value);
  for (const value of JUMP_PADS) expect(Math.abs(value)).toBeLessThanOrEqual(64);
  expect(LOOP_PADS).toHaveLength(8);
  expect(ROLL_PADS).toHaveLength(8);
  expect(JUMP_PADS).toHaveLength(8);
  expect(COLOR_FX).toHaveLength(6);
  expect(BEAT_FX).toHaveLength(14);
  expect([...BEAT_FX_ORDER].sort((a, b) => a - b)).toEqual(BEAT_FX.map((_, i) => i));
  expect(BEAT_FX_ORDER.map((kind) => BEAT_FX[kind])).toEqual([
    'DELAY',
    'ECHO',
    'PING PONG',
    'SPIRAL',
    'REVERB',
    'TRANS',
    'FILTER',
    'FLANGER',
    'PHASER',
    'PITCH',
    'SLIP ROLL',
    'ROLL',
    'VINYL BRAKE',
    'HELIX',
  ]);
  for (const kind of PAD_FX) expect(BEAT_FX[kind]).toBeDefined();
  expect(KEY_PADS).toHaveLength(8);
  expect(PAD_FX).toHaveLength(8);
});

test('the interface and the engine agree on divisions, intervals and effect count', () => {
  const engine = readFileSync('crates/audio-engine/src/fx.rs', 'utf8');
  const list = (name: string) =>
    engine
      .match(new RegExp(`const ${name}: \\[f\\d\\d; 11\\] =\\s*\\[([^\\]]+)\\]`))![1]
      .split(',')
      .map((v) => v.trim())
      .filter(Boolean)
      .map(Number);
  expect(list('BEAT_DIVISIONS')).toEqual(BEAT_DIVISIONS);
  expect(list('PITCH_STEPS')).toEqual(PITCH_STEPS);
  expect(pitchLabel(32)).toBe('+12 st');
  expect(pitchLabel(0.0625)).toBe('-12 st');
  expect(engine).toContain(`COUNT: u8 = ${BEAT_FX.length}`);
});

test('CALL finds the next saved cue and ignores unset ones and the one under the head', () => {
  const cues = [30, 0, 10, 0, 20, 0, 0, 0];
  expect(neighborCue(cues, 15, 1)).toBe(20);
  expect(neighborCue(cues, 15, -1)).toBe(10);
  expect(neighborCue(cues, 20, 1)).toBe(30);
  expect(neighborCue(cues, 20, -1)).toBe(10);
  expect(neighborCue(cues, 40, 1)).toBeNull();
  expect(neighborCue(cues, 5, -1)).toBeNull();
});
