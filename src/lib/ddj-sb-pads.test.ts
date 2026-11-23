import { expect, test } from 'bun:test';
import preset from '../../crates/midi/presets/pioneer-ddj-sb.json';
import { DDJ_AUTO_LOOP_BEATS, DDJ_MANUAL_PADS } from './ddj-sb-pads';

const bindings = new Map(preset.mappings.map(({ control, action }) => [control, action]));

test('DDJ-SB loop pads use the stock deck targets, lengths and secondary actions', () => {
  for (const index of [0, 1]) {
    const deck = `deck-${index + 1}`;
    const expectBinding = (id: string, kind: string, value: number) =>
      expect(bindings.get(`${deck}-${id}`)).toEqual({ slot: index, kind, value });

    DDJ_AUTO_LOOP_BEATS.forEach((beats, pad) => {
      expectBinding(`auto-loop-pad-${pad + 1}`, 'loopBeats', beats);
      expectBinding(`shift-auto-loop-pad-${pad + 1}`, 'autoLoop', 0);
    });

    DDJ_MANUAL_PADS.primary.forEach(({ kind, value }, pad) =>
      expectBinding(`manual-loop-pad-${pad + 1}`, kind, value),
    );
    DDJ_MANUAL_PADS.shifted.forEach((action, pad) => {
      if (action) expectBinding(`manual-loop-shift-pad-${pad + 1}`, action.kind, action.value);
      else expect(bindings.has(`${deck}-manual-loop-shift-pad-${pad + 1}`)).toBe(false);
    });
  }
});
