import { useSyncExternalStore } from 'react';

export type Language = 'es' | 'en';
const storageKey = 'curado.language.v1';
const listeners = new Set<() => void>();

export function validLanguage(value: unknown): Language {
  return value === 'en' ? 'en' : 'es';
}

let language: Language = (() => {
  try {
    return validLanguage(globalThis.localStorage?.getItem(storageKey));
  } catch {
    return 'es';
  }
})();

export function currentLanguage(): Language {
  return language;
}

export function useLanguage(): Language {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    currentLanguage,
    () => 'es',
  );
}

export function setLanguage(value: Language): void {
  language = validLanguage(value);
  try {
    globalThis.localStorage?.setItem(storageKey, language);
  } catch {
    // The current session remains usable when web storage is unavailable.
  }
  if (typeof document !== 'undefined') document.documentElement.lang = language;
  listeners.forEach((listener) => listener());
}

/** Keep both strings explicit; interpolate user data outside the catalog. */
export function tr(spanish: string, english: string): string {
  return language === 'en' ? english : spanish;
}

if (typeof document !== 'undefined') document.documentElement.lang = language;
