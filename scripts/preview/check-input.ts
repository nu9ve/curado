// Keyboard and pointer together, with real input events against the interface and a stand-in engine.
import { open, ORIGIN, sleep } from './cdp';

const page = await open(1600, 1000);
await page.go(`${ORIGIN}/`);
await page.run(`localStorage.clear()`);
await page.go(`${ORIGIN}/`);
await page.run(`document.querySelector('[data-action="0:toggle:0"]') ? 1 : 0`);
const actions = () =>
  page.run(
    `window.__calls.filter(c => c[0] === 'audio_action').map(c => c[1].action.slot + ':' + c[1].action.kind + ':' + c[1].action.value)`,
  ) as Promise<string[]>;
const clear = () => page.run(`window.__calls.length = 0`);
let failed = 0;
const check = (name: string, ok: boolean, detail?: unknown) => {
  if (!ok) failed++;
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${name}${ok ? '' : ' ' + JSON.stringify(detail)}`);
};
const has = (list: string[], text: string) => list.some((a) => a.startsWith(text));
const F = ['KeyF', 'f'] as const;
const J = ['KeyJ', 'j'] as const;
const knobSel = '.rx-body .rx-mini-knob .knob';
const knob = await page.at(knobSel);
check('console present', !!knob, knob);

// 1. A key in the middle of turning a knob.
await clear();
await page.mouse('mouseMoved', knob.x, knob.y);
await page.mouse('mousePressed', knob.x, knob.y);
await page.mouse('mouseMoved', knob.x, knob.y - 20);
await sleep(60);
await page.key(...F);
await page.mouse('mouseMoved', knob.x, knob.y - 40);
await sleep(60);
let list = await actions();
check('F plays while turning a knob', has(list, '0:toggle'), list);

// 2. Shift held for fine movement, F still plays; a real combination keeps its meaning.
await clear();
await page.key(...J, ['shift']);
list = await actions();
check('Shift+J while dragging still plays', has(list, '1:toggle'), list);
await clear();
await page.key('Digit1', '1', ['shift']);
list = await actions();
check('Shift+1 while dragging saves a cue', has(list, '0:setCue'), list);
await page.mouse('mouseReleased', knob.x, knob.y - 40);
await sleep(60);
await clear();

// 3. After letting go, with the focus the click left behind.
const focus = await page.run(
  `document.activeElement?.getAttribute('role') ?? document.activeElement?.tagName`,
);
console.log('     focus after the drag:', focus);
await page.key(...J);
await page.key('Space', ' ', [], 32);
await page.key('ArrowDown', 'ArrowDown', [], 40);
list = await actions();
check('J and Space work after the knob', has(list, '1:toggle') && has(list, '0:toggleBoth'), list);
check('arrows did not move the knob', !list.some((a) => /master|cue|trim/.test(a)), list);
check(
  'without a pointer down, Shift+F means nothing',
  await (async () => {
    await clear();
    await page.key(...F, ['shift']);
    return (await actions()).length === 0;
  })(),
);

// 4. Slider dragged, with several keys while it moves.
const fader = await page.at('.rx-cross .slider');
await clear();
await page.mouse('mouseMoved', fader.x, fader.y);
await page.mouse('mousePressed', fader.x, fader.y);
await page.mouse('mouseMoved', fader.x + 15, fader.y);
await page.key(...F);
await page.key(...J);
await page.key('Digit2', '2');
await page.mouse('mouseMoved', fader.x + 30, fader.y);
await sleep(60);
list = await actions();
check(
  'three keys during a crossfade',
  has(list, '0:toggle') && has(list, '1:toggle') && has(list, '0:cue:1'),
  list,
);
await page.mouse('mouseReleased', fader.x + 30, fader.y);

// 5. A menu picked with the mouse does not hold the keyboard either.
const dial = await page.at('.rx-top select, .rx-body select');
if (dial) {
  await page.mouse('mousePressed', dial.x, dial.y);
  await page.mouse('mouseReleased', dial.x, dial.y);
  await sleep(80);
  await page.run(
    `document.activeElement.blur?.(); document.querySelector('.rx-body select').focus()`,
  );
  await clear();
  await page.key(...F);
  list = await actions();
  check('F works with a menu focused by the mouse', has(list, '0:toggle'), list);
}

// 6. Typing stays typing, and any press gives the keyboard back.
const search = await page.at('.mix-panel input[type="search"], .mix-panel input');
check('there is a field to type in', !!search, search);
await page.mouse('mousePressed', search.x, search.y);
await page.mouse('mouseReleased', search.x, search.y);
await sleep(60);
await clear();
await page.key(...F);
list = await actions();
check('F in a field does not play', !has(list, '0:toggle'), list);
const k2 = await page.at(knobSel);
await page.mouse('mousePressed', k2.x, k2.y);
await page.mouse('mouseReleased', k2.x, k2.y);
await sleep(60);
await clear();
await page.key(...F);
list = await actions();
check('a press on a knob gives the keyboard back', has(list, '0:toggle'), list);

// 7. Tab still works: a knob reached by keyboard keeps its arrows, and letters still play.
await page.run(`document.activeElement?.blur?.()`);
await page.send('Input.dispatchKeyEvent', {
  type: 'rawKeyDown',
  code: 'Tab',
  key: 'Tab',
  windowsVirtualKeyCode: 9,
});
await page.send('Input.dispatchKeyEvent', {
  type: 'keyUp',
  code: 'Tab',
  key: 'Tab',
  windowsVirtualKeyCode: 9,
});
await sleep(50);
const tabbed = await page.run(
  `(() => { const k = document.querySelector(${JSON.stringify(knobSel)}); k.focus(); return k.matches(':focus-visible'); })()`,
);
console.log('     knob focused by script after Tab, focus-visible:', tabbed);
if (tabbed) {
  await clear();
  await page.key('ArrowUp', 'ArrowUp', [], 38);
  list = await actions();
  check('arrows turn a knob reached with Tab', list.length > 0 && !has(list, '0:listMove'), list);
  await clear();
  await page.key(...F);
  list = await actions();
  check('letters still play from there', has(list, '0:toggle'), list);
}
await page.run(`document.activeElement?.blur?.()`);

// 8. Auto loop.
await clear();
await page.key('KeyB', 'b');
await page.key('KeyN', 'n');
await page.key('KeyB', 'b', ['shift']);
await page.key('KeyN', 'n', ['ctrl']);
list = await actions();
check(
  'B and N make loops, Shift and Control change length',
  ['0:autoLoop', '1:autoLoop', '0:loopScale:0.5', '1:loopScale:2'].every((a) => has(list, a)),
  list,
);
const chip = await page.at('.rx-band-left .rx-autoloop [data-action="0:loopBeats:8"]');
await clear();
await page.mouse('mousePressed', chip.x, chip.y);
await page.mouse('mouseReleased', chip.x, chip.y);
await sleep(50);
list = await actions();
check('a length button sends it', has(list, '0:loopBeats:8'), list);
const layout = await page.run(`(() => {
  const box = (e) => e.getBoundingClientRect();
  const hit = (a, b) => !(a.right <= b.left || b.right <= a.left || a.bottom <= b.top || b.bottom <= a.top);
  const out = [];
  for (const strip of document.querySelectorAll('.rx-autoloop')) {
    const s = box(strip);
    for (const e of document.querySelectorAll('.rx-band > *, .rx-browser, .rx-screen')) {
      if (e === strip || e.contains(strip)) continue;
      if (hit(s, box(e))) out.push(strip.getAttribute('aria-label') + ' overlaps ' + (e.className || e.tagName));
    }
    const band = box(strip.parentElement);
    if (s.left < band.left || s.right > band.right || s.top < band.top || s.bottom > band.bottom) out.push(strip.getAttribute('aria-label') + ' leaves its band');
  }
  const cut = [...document.querySelectorAll('.rx-autoloop button')].filter((e) => e.scrollWidth > e.clientWidth + 1).map((e) => e.textContent);
  return { out, cut };
})()`);
check('the strips touch nothing', layout.out.length === 0 && layout.cut.length === 0, layout);
const covered = await page.run(`(() => {
  const out = [];
  for (const e of document.querySelectorAll('.rx-body button, .rx-body [role="slider"]')) {
    const r = e.getBoundingClientRect();
    if (!r.width || !r.height) continue;
    const top = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
    if (!top || !(e === top || e.contains(top) || top.contains(e) && top.closest('button') === e))
      out.push((e.getAttribute('aria-label') || e.textContent.trim()).slice(0, 24) + ' under ' + (top?.className || top?.tagName));
  }
  return out;
})()`);
check('no control lies under another', covered.length === 0, covered);
check('no page errors', page.errors.length === 0, page.errors.slice(0, 3));
console.log(failed ? `${failed} FAILED` : 'all passed');
page.close();
process.exit(failed ? 1 : 0);
