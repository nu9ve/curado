export const DDJ_PAD_MODES = [
  ['cue', 'HOT CUE'],
  ['auto', 'AUTO LOOP'],
  ['manual', 'MANUAL LOOP'],
  ['samples', 'SAMPLER'],
] as const;

export type DdjPadMode = (typeof DDJ_PAD_MODES)[number][0];

export const DDJ_AUTO_LOOP_BEATS = [1, 2, 4, 8] as const;

export const DDJ_MANUAL_PADS = {
  primary: [
    { label: 'IN', kind: 'loopIn', value: 0 },
    { label: 'OUT', kind: 'loopOut', value: 0 },
    { label: 'RELOOP', kind: 'loop', value: 0 },
    { label: '÷2', kind: 'loopScale', value: 0.5 },
  ],
  shifted: [
    { label: '−4', kind: 'beatJump', value: -4 },
    { label: '+4', kind: 'beatJump', value: 4 },
    null,
    { label: '×2', kind: 'loopScale', value: 2 },
  ],
} as const;
