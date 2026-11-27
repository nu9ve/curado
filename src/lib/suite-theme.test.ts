import { describe, expect, test } from 'bun:test';
import themeVectors from '../../vendor/nu9ve-core/data/theme-vectors-v1.json';
import eraVectors from '../../vendor/nu9ve-core/data/theme-era-vectors-v1.json';
import {
  ROLE_NAMES,
  SUITE_SKINS,
  defaultDark,
  forcedDark,
  suiteRoles,
  suiteRolesFromBase,
  type SuiteEra,
  type SuiteSkin,
} from './suite-theme';

describe('suite theme roles (nu9ve-theme-v1)', () => {
  test('every role of the 110 shared vectors, through HSV as the vectors say', () => {
    expect(themeVectors.cases.length).toBe(110);
    for (const { skin, dark, base, resolvedDark, roles } of themeVectors.cases) {
      const result = suiteRoles(skin as SuiteSkin, dark, base);
      const input = `${skin} / ${dark ? 'dark' : 'light'} / ${base}`;
      expect(result.dark, input).toBe(resolvedDark);
      for (const name of ROLE_NAMES)
        expect(result.roles[name], `${input} ${name}`).toBe(roles[name]);
    }
  });
  test('every role of the 44 tinted era vectors', () => {
    expect(eraVectors.cases.length).toBe(44);
    for (const { skin, dark, base, era, resolvedDark, roles } of eraVectors.cases) {
      const result = suiteRolesFromBase(skin as SuiteSkin, dark, base, era as SuiteEra);
      const input = `${skin} / ${era} / ${dark ? 'dark' : 'light'} / ${base}`;
      expect(result.dark, input).toBe(resolvedDark);
      for (const name of ROLE_NAMES)
        expect(result.roles[name], `${input} ${name}`).toBe(
          (roles as Record<string, string>)[name],
        );
    }
  });
  test('period skins fix their mode only in their original era', () => {
    expect(SUITE_SKINS).toHaveLength(5);
    expect(forcedDark('classicMac')).toBe(false);
    expect(forcedDark('classicMac', 'tinted')).toBeNull();
    expect(forcedDark('gothic', 'tinted')).toBe(true);
    expect(forcedDark('glass')).toBeNull();
    expect(defaultDark('classicMac', 'tinted')).toBe(false);
    expect(defaultDark('win9x', 'tinted')).toBeNull();
  });
});
