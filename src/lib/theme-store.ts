import { useSyncExternalStore } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { call, native } from './bridge';
import { DEFAULT_THEME, palette, validateTheme, nativeWindowTheme } from './theme';
import type { ThemeSettings } from './theme';
const CACHE = 'curado.theme.v1';
let theme = (() => {
  try {
    return validateTheme(JSON.parse(localStorage.getItem(CACHE) ?? 'null'));
  } catch {
    return { ...DEFAULT_THEME };
  }
})();
const media = window.matchMedia('(prefers-color-scheme: dark)');
let revision = 0;
let windowMode: string | undefined;
let windowTimer: ReturnType<typeof setTimeout> | undefined;
let saveTimer: ReturnType<typeof setTimeout> | undefined;
let saveChain = Promise.resolve();
const listeners = new Set<() => void>();
const subscribe = (cb: () => void) => {
  listeners.add(cb);
  return () => {
    listeners.delete(cb);
  };
};
export function useTheme() {
  return useSyncExternalStore(subscribe, () => theme);
}
export function currentPalette() {
  return palette(theme, media.matches);
}
export function applyTheme() {
  const p = currentPalette();
  for (const [key, value] of Object.entries(p.tokens))
    document.documentElement.style.setProperty(key, value);
  document.documentElement.dataset.theme = p.dark ? 'dark' : 'light';
  document.documentElement.dataset.variant = theme.variant;
  document.documentElement.style.colorScheme = p.dark ? 'dark' : 'light';
  const meta = document.querySelector('meta[name="theme-color"]');
  meta?.setAttribute('content', p.bg);
  if (native) {
    const win = getCurrentWindow();
    const mode = nativeWindowTheme(theme);
    const key = mode ?? 'system';
    if (windowMode !== key) {
      windowMode = key;
      void win.setTheme(mode).catch(() => {});
    }
    clearTimeout(windowTimer);
    windowTimer = setTimeout(() => {
      void win.setBackgroundColor(p.bg).catch(() => {});
    }, 80);
  }
  listeners.forEach((fn) => fn());
}
export async function connectTheme() {
  if (!native) return;
  const before = revision;
  const stored = await call<ThemeSettings>('get_theme');
  if (before !== revision) return;
  theme = validateTheme(stored);
  try {
    localStorage.setItem(CACHE, JSON.stringify(theme));
  } catch {
    /* Native SQLite is authoritative. */
  }
  applyTheme();
}
export function updateTheme(next: ThemeSettings, onError: (e: unknown) => void) {
  theme = validateTheme(next);
  revision++;
  try {
    localStorage.setItem(CACHE, JSON.stringify(theme));
  } catch {
    /* Preferences remain usable when web storage is disabled. */
  }
  applyTheme();
  clearTimeout(saveTimer);
  if (native) {
    const value = { ...theme };
    saveTimer = setTimeout(() => {
      saveChain = saveChain.catch(() => {}).then(() => call<void>('save_theme', { theme: value }));
      void saveChain.catch(onError);
    }, 180);
  }
}
media.addEventListener('change', () => {
  if (theme.mode === 'system') {
    theme = { ...theme };
    applyTheme();
  }
});
// Apply before the first React paint; native persistence is loaded asynchronously.
applyTheme();
