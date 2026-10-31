// Documentation pictures of the real interface with invented tracks.
import { open, ORIGIN, sleep } from './cdp';
import { mkdirSync } from 'node:fs';

const out = process.argv[2] ?? 'docs/assets';
mkdirSync(out, { recursive: true });
const only = process.argv[3]?.split(',');
const page = await open(1600, 1000, 1.5);
await page.go(`${ORIGIN}/`, 500);
const base = {
  version: 2,
  mode: 'dark',
  variant: 'neutral',
  base: '#d66154',
  style: 'curado',
  typography: 'auto',
  texture: 'auto',
  waveform: 'bands',
};
const setup = async (theme: object, workspace: object, query = '', language = 'en') => {
  await page.run(`localStorage.clear();
    localStorage.setItem('curado.language.v1', ${JSON.stringify(language)});
    localStorage.setItem('__shot_theme', ${JSON.stringify(JSON.stringify({ ...base, ...theme }))});
    localStorage.setItem('curado.workspace.v2', ${JSON.stringify(JSON.stringify(workspace))});`);
  await page.go(`${ORIGIN}/${query}`, 2600);
};
const click = (text: string) =>
  page.run(
    `(() => { const e = [...document.querySelectorAll('button')].find((b) => b.textContent.trim().toLowerCase() === ${JSON.stringify(text)} || b.getAttribute('aria-label')?.toLowerCase() === ${JSON.stringify(text)}); e?.click(); return !!e; })()`,
  );
const want = (name: string) => !only || only.includes(name);

if (want('console')) {
  await setup({}, { panelOpen: false, railMix: true });
  await click('console');
  await sleep(1200);
  await page.shot(`${out}/console.jpg`, 80);
}
if (want('gothic')) {
  await setup({ style: 'gothic', base: '#9b7fd6' }, { panelOpen: false, railMix: true });
  await click('console');
  await sleep(1200);
  await page.shot(`${out}/console-gothic.jpg`, 80);
}
if (want('mix')) {
  await setup({}, { panelOpen: true, panel: 'library', railMix: true, split: 0.68 });
  await click('console');
  await sleep(1400);
  await page.shot(`${out}/mix-library.jpg`, 80);
}
if (want('player')) {
  await setup({}, { railPlayer: false }, '?mode=player');
  await sleep(1400);
  await page.shot(`${out}/player.jpg`, 80);
}
if (want('settings')) {
  await setup({}, { railPlayer: false }, '?mode=player');
  await page.run(`document.querySelector('[aria-label="Settings"]')?.click()`);
  await sleep(1000);
  await page.shot(`${out}/settings.jpg`, 80);
}
if (page.errors.length) console.error(page.errors.slice(0, 3));
page.close();
process.exit(page.errors.length ? 1 : 0);
