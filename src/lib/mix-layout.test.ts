import { expect, test } from 'bun:test';
import { readMixLayout, saveMixLayout } from './mix-layout';

test('mix layout persists independently and handles unavailable or invalid storage', () => {
  const data = new Map<string, string>();
  const storage = {
    getItem: (key: string) => data.get(key) ?? null,
    setItem: (key: string, value: string) => {
      data.set(key, value);
    },
  };
  expect(readMixLayout(storage)).toBe('classic');
  saveMixLayout(storage, 'console');
  expect(readMixLayout(storage)).toBe('console');
  saveMixLayout(storage, 'classic');
  expect(readMixLayout(storage)).toBe('classic');
  data.set('curado.mix-layout.v1', 'unknown');
  expect(readMixLayout(storage)).toBe('classic');
  expect(
    readMixLayout({
      getItem: () => {
        throw new Error('blocked');
      },
    }),
  ).toBe('classic');
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
