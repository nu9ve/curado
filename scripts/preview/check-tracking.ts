// Hands and face at once, from invented camera frames, through the real interface.
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
const actions = () =>
  page.run(
    `window.__calls.filter(c => c[0] === 'audio_action').map(c => c[1].action.slot + ':' + c[1].action.kind + ':' + c[1].action.value)`,
  ) as Promise<string[]>;
const clear = () => page.run(`window.__calls.length = 0`);
const button = async (scope: string, label: string) => {
  const ok = await page.run(
    `(() => { const b = [...document.querySelectorAll(${JSON.stringify(scope + ' button')})].find((e) => e.textContent.trim() === ${JSON.stringify(label)}); b?.click(); return !!b; })()`,
  );
  await sleep(120);
  return ok;
};
/** Sends frames for a while, as the helper would at 15 a second. */
const frames = async (frame: object, count = 6) => {
  for (let i = 0; i < count; i++) {
    await page.run(`window.__track(${JSON.stringify(frame)})`);
    await sleep(40);
  }
};
const hand = (x: number, y: number, pinch: number, size = 0.15) => ({ x, y, pinch, size });
const leaning = { x: 0.5, y: 0.7, size: 0.34 };
const away = { x: 0.5, y: 0.7, size: 0.15 };

check('ALL is offered', await button('.rx-camera', 'ALL'));
check(
  'the helper was asked for both',
  (
    await page.run(`JSON.stringify(window.__calls.filter(c => c[0] === 'tracking_start').at(-1))`)
  ).includes('"all"'),
);
await frames({ status: 'running' }, 1);

// A hand pinching on the left works XY 1, with the face away doing nothing.
await clear();
await frames({ hands: [hand(0.75, 0.5, 0.2)], face: away });
let list = await actions();
check(
  'left hand works its own channel',
  list.some((a) => a.startsWith('0:color:')) && !list.some((a) => a.startsWith('1:')),
  list,
);
check('a far face does nothing', !list.some((a) => a.includes('beatFx')), list);

// Closer or farther than where the pinch began: the third axis.
await clear();
await frames({ hands: [hand(0.75, 0.5, 0.2, 0.26)], face: away }, 10);
list = await actions();
check(
  'closer opens the high pass',
  list.some((a) => a.startsWith('0:highPass:') && +a.split(':')[2] > 0.3),
  list,
);
await clear();
await frames({ hands: [hand(0.75, 0.5, 0.2, 0.08)], face: away }, 12);
list = await actions();
check(
  'farther closes the low pass',
  list.some((a) => a.startsWith('0:filter:') && +a.split(':')[2] < 0.7),
  list,
);

// The face leaning in, with that side active, goes to that channel's Beat FX.
await clear();
await frames({ hands: [hand(0.75, 0.5, 0.2)], face: leaning });
list = await actions();
check('face follows the active side', list.includes('0:beatFxOn:1'), list);

// Letting the hand go moves the face to the master, turning off what it left.
await clear();
await frames({ hands: [hand(0.75, 0.5, 0.9)], face: leaning });
list = await actions();
check(
  'with no hand active the face takes the master',
  list.includes('0:beatFxOn:0') && list.includes('2:beatFxOn:1'),
  list,
);

// Three at once: both hands and the face, each on its own.
await clear();
await frames({ hands: [hand(0.75, 0.5, 0.2), hand(0.25, 0.5, 0.2)], face: leaning });
list = await actions();
check(
  'both hands and the face, apart',
  list.some((a) => a.startsWith('0:color:')) &&
    list.some((a) => a.startsWith('1:color:')) &&
    list.includes('2:beatFxOn:1'),
  list,
);

// A hand on an effect of its own leaves the unit to it, and the face to the master.
await clear();
await button('.rx-xy:first-child, .rx-top-left .rx-xy', 'STUTTER');
await frames({ hands: [hand(0.75, 0.5, 0.2)], face: leaning });
list = await actions();
check(
  'a hand on Stutter keeps the unit',
  list.includes('0:beatFx:14') && list.includes('2:beatFxOn:1'),
  list,
);
await button('.rx-top-left .rx-xy', 'COLOR');

// The face's own effect is chosen in the panel.
await clear();
await button('.rx-face-fx', 'RECYCLE');
await frames({ hands: [], face: leaning });
list = await actions();
check('the face uses the effect chosen', list.includes('2:beatFx:15'), list);

// Losing everything lets everything go, without anyone asking.
await clear();
await sleep(500);
list = await actions();
check('losing sight turns it off', list.includes('2:beatFxOn:0'), list);

const cut = await page.run(
  `[...document.querySelectorAll('.rx-camera button, .rx-face-fx button, .rx-xy-modes button')].filter((b) => b.scrollWidth > b.clientWidth + 1 || b.getBoundingClientRect().right > b.closest('.rx-top').getBoundingClientRect().right).map((b) => b.textContent)`,
);
check('every new button fits', cut.length === 0, cut);
check('no page errors', page.errors.length === 0, page.errors.slice(0, 3));
console.log(failed ? `${failed} FAILED` : 'all passed');
page.close();
process.exit(failed ? 1 : 0);
