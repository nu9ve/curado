import { expect, test } from 'bun:test';
import { MIX_LAYOUTS, readMixLayout, saveMixLayout, suggestedMixLayout } from './mix-layout';

test('mix layout persists independently and handles unavailable or invalid storage', () => {
  const data = new Map<string, string>();
  const storage = {
    getItem: (key: string) => data.get(key) ?? null,
    setItem: (key: string, value: string) => {
      data.set(key, value);
    },
  };
  expect(readMixLayout(storage)).toBe('console');
  for (const layout of MIX_LAYOUTS) {
    saveMixLayout(storage, layout);
    expect(readMixLayout(storage)).toBe(layout);
  }
  data.set('curado.mix-layout.v1', 'unknown');
  expect(readMixLayout(storage)).toBe('console');
  expect(
    readMixLayout({
      getItem: () => {
        throw new Error('blocked');
      },
    }),
  ).toBe('console');
  expect(() =>
    saveMixLayout(
      {
        setItem: () => {
          throw new Error('blocked');
        },
      },
      'console',
    ),
  ).not.toThrow();
});

test('only a connected, recognized DDJ-SB suggests the DDJ-SB view', () => {
  expect(suggestedMixLayout('PIONEER DDJ-SB', 'pioneer-ddj-sb')).toBe('ddj-sb');
  expect(suggestedMixLayout(null, 'pioneer-ddj-sb')).toBeNull();
  expect(suggestedMixLayout('Another device')).toBeNull();
  expect(suggestedMixLayout('Another device', 'another-preset')).toBeNull();
});
