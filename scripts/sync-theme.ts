import { DEFAULT_THEME, palette } from '../src/lib/theme';
const path = new URL('../src/styles.css', import.meta.url);
const css = await Bun.file(path).text();
const start = css.indexOf('  --accent:');
const end = css.indexOf('\n}', start);
if (start < 0 || end < 0) throw new Error('Default theme token block missing');
const tokens = Object.entries(palette(DEFAULT_THEME).tokens)
  .map(([key, value]) => `  ${key}: ${value};`)
  .join('\n');
const next = css.slice(0, start) + tokens + css.slice(end);
if (next !== css) await Bun.write(path, next);
