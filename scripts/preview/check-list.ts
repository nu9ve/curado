// The track list from the keyboard, after clicking in it, with real input events.
import { open, ORIGIN, sleep } from './cdp';

const page = await open(1600, 1000);
await page.go(`${ORIGIN}/`);
await page.run(`localStorage.clear()`);
await page.go(`${ORIGIN}/`);
let failed = 0;
const check = (name: string, ok: boolean, detail?: unknown) => {
  if (!ok) failed++;
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${name}${ok ? '' : ' ' + JSON.stringify(detail)}`);
};
const chosen = () =>
  page.run(`document.querySelector('.track-table tr.selected strong')?.textContent ?? null`);
const titles = () =>
  page.run(
    `[...document.querySelectorAll('.track-table tbody strong')].slice(0, 4).map((e) => e.textContent)`,
  );
const loads = () =>
  page.run(
    `window.__calls.filter((c) => c[0] === 'load_track').map((c) => c[1].slot + ':' + c[1].id)`,
  );
const row = await page.at('.track-table tbody tr:nth-child(3)');
await page.mouse('mouseMoved', row.x, row.y);
await page.mouse('mousePressed', row.x, row.y);
await page.mouse('mouseReleased', row.x, row.y);
await sleep(80);
const first = await chosen();
console.log(
  '     focus:',
  await page.run(
    `document.activeElement.tagName + ' visible=' + document.activeElement.matches(':focus-visible')`,
  ),
);
await page.key('ArrowDown', 'ArrowDown', [], 40);
await sleep(50);
const second = await chosen();
console.log(
  '     focus after a key:',
  await page.run(
    `document.activeElement.tagName + ' visible=' + document.activeElement.matches(':focus-visible')`,
  ),
);
check('arrow down moves the selection after a click', second !== first, [first, second]);
await page.key('ArrowLeft', 'ArrowLeft', ['shift'], 37);
await page.key('ArrowRight', 'ArrowRight', ['shift'], 39);
await sleep(80);
const loaded = await loads();
check(
  'Shift + arrows load into each deck',
  loaded.some((l: string) => l.startsWith('0:')) && loaded.some((l: string) => l.startsWith('1:')),
  loaded,
);

// Sorting by a column, and the keys following what is shown.
const header = async (name: string) => {
  const at = await page.run(
    `(() => { const b = [...document.querySelectorAll('.track-table .sort-head')].find((e) => e.textContent.startsWith(${JSON.stringify(name)})); const r = b.getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()`,
  );
  await page.mouse('mouseMoved', at.x, at.y);
  await page.mouse('mousePressed', at.x, at.y);
  await page.mouse('mouseReleased', at.x, at.y);
  await sleep(80);
};
const bpms = () =>
  page.run(
    `[...document.querySelectorAll('.track-table tbody tr')].map((r) => parseFloat(r.querySelector('td.mono')?.textContent)).filter(Number.isFinite)`,
  ) as Promise<number[]>;
const ordered = (list: number[], sign: number) =>
  list.every((v, i) => !i || (v - list[i - 1]) * sign >= 0);
const original = await titles();
await header('BPM');
check('BPM sorts upward', ordered(await bpms(), 1), await bpms());
await header('BPM');
check('and reverses', ordered(await bpms(), -1), await bpms());
await header('BPM');
check(
  'and goes back to the list order',
  JSON.stringify(await titles()) === JSON.stringify(original),
  await titles(),
);
await header('BPM');
const top = await page.at('.track-table tbody tr:nth-child(1)');
await page.mouse('mouseMoved', top.x, top.y);
await page.mouse('mousePressed', top.x, top.y);
await page.mouse('mouseReleased', top.x, top.y);
await sleep(60);
await page.key('ArrowDown', 'ArrowDown', [], 40);
await sleep(60);
const shownSecond = (await page.run(
  `document.querySelector('.track-table tbody tr:nth-child(2) strong').textContent`,
)) as string;
check('arrows follow the sorted order', (await chosen()) === shownSecond, [
  await chosen(),
  shownSecond,
]);
await page.run(`window.__calls.length = 0`);
await page.key('ArrowRight', 'ArrowRight', ['shift'], 39);
await sleep(80);
check('and loading takes the track shown', (await loads()).length === 1, await loads());
const look = await page.run(
  `(() => { const b = document.querySelector('.track-table .sort-head'); const c = getComputedStyle(b); return { bg: c.backgroundColor, border: c.borderTopWidth, height: b.getBoundingClientRect().height }; })()`,
);
check(
  'headers still read as headers',
  look.border === '0px' && look.height < 22 && /transparent|, 0\)/.test(look.bg),
  look,
);
// The player shows the number column, which also goes back to the list order.
await page.run(`localStorage.clear()`);
await page.go(`${ORIGIN}/?mode=player`);
const inPlayer = await titles();
await header('key');
check(
  'key sorts in the player',
  JSON.stringify(await titles()) !== JSON.stringify(inPlayer),
  inPlayer,
);
await header('#');
check(
  '# goes back to the list order',
  JSON.stringify(await titles()) === JSON.stringify(inPlayer),
  await titles(),
);
check('no page errors', page.errors.length === 0, page.errors.slice(0, 3));
console.log(failed ? `${failed} FAILED` : 'all passed');
page.close();
process.exit(failed ? 1 : 0);
