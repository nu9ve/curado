// Pieces, tags, the quick select and banks through the real interface, with a stand-in engine.
import { open, ORIGIN, sleep } from './cdp';

const page = await open(1600, 1000);
await page.go(`${ORIGIN}/`);
await page.run(`localStorage.clear(); localStorage.setItem('curado.language.v1', 'en')`);
await page.go(`${ORIGIN}/`);
let failed = 0;
const check = (name: string, ok: boolean, detail?: unknown) => {
  if (!ok) failed++;
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${name}${ok ? '' : ' ' + JSON.stringify(detail)}`);
};
const calls = (name: string) =>
  page.run(
    `window.__calls.filter((c) => c[0] === ${JSON.stringify(name)}).map((c) => c[1])`,
  ) as Promise<any[]>;
const clear = () => page.run(`window.__calls.length = 0`);
const button = async (scope: string, label: string, shift = false) => {
  const ok = await page.run(
    `(() => { const b = [...document.querySelectorAll(${JSON.stringify(scope + ' button')})].find((e) => e.textContent.trim().startsWith(${JSON.stringify(label)})); if (!b) return false; b.dispatchEvent(new MouseEvent('click', { bubbles: true, shiftKey: ${shift} })); return true; })()`,
  );
  await sleep(120);
  return ok;
};
const rows = () =>
  page.run(`document.querySelectorAll('.track-table tbody tr').length`) as Promise<number>;

const all = await rows();
check(
  'stems and loops are listed with their kind',
  (await page.run(`document.querySelectorAll('.piece-kind').length`)) > 0,
);
await button('.piece-filters', 'stems');
const stems = await rows();
check('filter by kind', stems > 0 && stems < all, [stems, all]);
await button('.piece-filters', 'samples');
const samples = await rows();
check('samples are the marked ones', samples > 0 && samples < all, samples);
await button('.piece-filters', 'all');
await button('.piece-filters', 'dub');
check('filter by tag', (await rows()) < all);
await button('.piece-filters', '+ save search');
await page.run(
  `(() => { const i = document.querySelector('.piece-filters input'); const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set; set.call(i, 'favorites of mine'); i.dispatchEvent(new Event('input', { bubbles: true })); i.form.requestSubmit(); })()`,
);
await sleep(120);
const saved = await calls('edit_library');
check(
  'a search saves as a collection',
  saved.some((c) => c.edit.kind === 'saveCollection' && c.edit.filter.tags[0] === 'dub'),
  saved,
);
await button('.piece-filters', 'dub');

// A selected piece: mark, tag, separate.
await page.run(`document.querySelector('.track-table tbody tr:nth-child(2)').click()`);
await sleep(100);
await clear();
await button('.selection-toolbar', 'sample');
let edits = await calls('edit_library');
check('marks as a sample', edits.at(-1)?.edit.tag === 'sample', edits);
await page.run(
  `(() => { const i = document.querySelector('.piece-add-tag input'); const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set; set.call(i, 'Vocal Chop'); i.dispatchEvent(new Event('input', { bubbles: true })); i.form.requestSubmit(); })()`,
);
await sleep(120);
edits = await calls('edit_library');
check('adds a tag', edits.at(-1)?.edit.kind === 'addTag', edits);
await button('.piece-stems', 'Separate');
const started = await calls('stems_start');
check('asks for stems', started[0]?.model === 'htdemucs', started);

// Lost files show, and can be looked for.
await page.run(`localStorage.setItem('__missing', '[1]')`);
await page.go(`${ORIGIN}/`);
check(
  'a lost file is marked',
  (await page.run(`document.querySelectorAll('.track-table tr.is-lost').length`)) >= 1,
);
await clear();
await button('.library-health', 'Find moved');
await sleep(150);
check('and looked for', (await calls('relink_search')).length === 1);
await page.run(`localStorage.removeItem('__missing')`);

// Quick select on the console.
check('list opens', await button('.rx-samples', 'LIST'));
const listed = () =>
  page.run(`document.querySelectorAll('.rx-quick-load').length`) as Promise<number>;
const marked = await listed();
check('it starts with what is marked', marked > 0, marked);
await button('.rx-quick-filters', 'SAMPLE');
const unmarked = await listed();
check('turning that off shows more', unmarked > marked, [unmarked, marked]);
await button('.rx-quick-filters', 'LENGTH');
await button('.rx-quick-filters', '≤5');
await button('.rx-quick-filters', '≤15');
const shorter = await page.run(
  `document.querySelector('.rx-quick-filters button:nth-child(2)').textContent`,
);
check('duration steps through its limits', String(shorter).startsWith('≤'), shorter);
await button('.rx-quick-filters', '≤30');
await button('.rx-quick-pads', '6');
await clear();
await page.run(`document.querySelector('.rx-quick-load').click()`);
await sleep(150);
const loaded = await calls('load_track');
check('a row goes to the chosen pad', loaded[0]?.slot === 7, loaded);
const fits = await page.run(
  `(() => { const q = document.querySelector('.rx-quick').getBoundingClientRect(); const m = document.querySelector('.rx-samples').getBoundingClientRect(); const cut = [...document.querySelectorAll('.rx-quick-filters button, .rx-sample-banks button')].filter((b) => b.getBoundingClientRect().right > m.right + 0.5).map((b) => b.textContent); return { inside: q.bottom <= m.bottom + 0.5, cut }; })()`,
);
check('it fits its panel', fits.inside && fits.cut.length === 0, fits);
await button('.rx-samples', 'LIST');

// Banks.
await clear();
await button('.rx-sample-banks', '1');
await sleep(150);
const recalled = await calls('load_track');
check(
  'a bank loads its pads',
  recalled.length === 2 && recalled[0].slot === 2 && recalled[1].slot === 3,
  recalled,
);
await clear();
await button('.rx-sample-banks', '2', true);
const banked = await calls('edit_library');
check(
  'Shift saves the pads to a bank',
  banked[0]?.edit.kind === 'saveBank' &&
    banked[0].edit.index === 1 &&
    banked[0].edit.tracks.length === 8,
  banked,
);
await clear();
await page.key('BracketRight', ']', ['alt'], 221);
await sleep(150);
check(
  'Option and a bracket change bank',
  (await calls('load_track')).length >= 0 &&
    (await page.run(
      `document.querySelector('.rx-sample-banks button[aria-pressed="true"]').textContent`,
    )) === '3',
);

// Saving the loop playing, by button and key.
await clear();
await page.run(
  `window.__state.audio.slots[0].loopIn = 8; window.__state.audio.slots[0].loopOut = 12; window.__emit()`,
);
await sleep(80);
await button('.rx-band-left .rx-autoloop', '+');
check('the + saves the loop', (await calls('save_loop'))[0]?.slot === 0);
await clear();
await page.key('Comma', ',', ['ctrl'], 188);
await sleep(100);
check('Control and reloop too, on each deck', (await calls('save_loop'))[0]?.slot === 1);

const covered = await page.run(
  `(() => { const out = []; for (const e of document.querySelectorAll('.rx-autoloop button, .rx-sample-banks button')) { const r = e.getBoundingClientRect(); const top = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2); if (!top || !(e === top || e.contains(top))) out.push(e.textContent); } return out; })()`,
);
check('nothing covers the new buttons', covered.length === 0, covered);
check('no page errors', page.errors.length === 0, page.errors.slice(0, 3));
console.log(failed ? `${failed} FAILED` : 'all passed');
page.close();
process.exit(failed ? 1 : 0);
