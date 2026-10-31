// Drives a headless Chrome through the DevTools protocol: real input events and saved pictures.
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawn } from 'node:child_process';

const here = import.meta.dir;
const CHROME = process.env.CHROME ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
export const ORIGIN = 'http://127.0.0.1:1420';
export const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

export async function open(width: number, height: number, scale = 1) {
  const profile = mkdtempSync(join(tmpdir(), 'curado-preview-'));
  const port = 9300 + Math.floor(Math.random() * 500);
  const chrome = spawn(
    CHROME,
    [
      '--headless=new',
      `--remote-debugging-port=${port}`,
      `--user-data-dir=${profile}`,
      '--no-first-run',
      '--disable-extensions',
      '--hide-scrollbars',
      'about:blank',
    ],
    { stdio: 'ignore' },
  );
  process.on('exit', () => {
    chrome.kill();
    rmSync(profile, { recursive: true, force: true });
  });
  process.on('uncaughtException', (e) => {
    console.error(e);
    process.exit(1);
  });
  process.on('unhandledRejection', (e) => {
    console.error(e);
    process.exit(1);
  });
  let target: { webSocketDebuggerUrl: string } | undefined;
  for (let i = 0; i < 50 && !target; i++) {
    await sleep(200);
    try {
      const list = (await (await fetch(`http://127.0.0.1:${port}/json`)).json()) as any[];
      target = list.find((t) => t.type === 'page');
    } catch {}
  }
  if (!target) throw new Error('no page');
  const ws = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((r) => (ws.onopen = r));
  let id = 1;
  const waiting = new Map<number, (v: any) => void>();
  const errors: string[] = [];
  ws.onmessage = (m) => {
    const data = JSON.parse(String(m.data));
    if (data.id) waiting.get(data.id)?.(data);
    if (data.method === 'Runtime.exceptionThrown')
      errors.push(
        data.params.exceptionDetails.exception?.description ?? data.params.exceptionDetails.text,
      );
  };
  const send = (method: string, params: object = {}) =>
    new Promise<any>((resolve, reject) => {
      const n = id++;
      waiting.set(n, (d) =>
        d.error ? reject(new Error(`${method}: ${d.error.message}`)) : resolve(d.result),
      );
      ws.send(JSON.stringify({ id: n, method, params }));
    });
  await send('Page.enable');
  await send('Runtime.enable');
  await send('Emulation.setDeviceMetricsOverride', {
    width,
    height,
    deviceScaleFactor: scale,
    mobile: false,
  });
  await send('Emulation.setFocusEmulationEnabled', { enabled: true });
  await send('Page.addScriptToEvaluateOnNewDocument', {
    source: readFileSync(`${here}/mock.js`, 'utf8'),
  });
  const run = async (expression: string) => {
    const r = await send('Runtime.evaluate', {
      expression,
      awaitPromise: true,
      returnByValue: true,
    });
    if (r.exceptionDetails)
      throw new Error(r.exceptionDetails.exception?.description ?? r.exceptionDetails.text);
    return r.result.value;
  };
  const MODS = { alt: 1, ctrl: 2, shift: 8 };
  const mods = (list: (keyof typeof MODS)[] = []) => list.reduce((a, m) => a + MODS[m], 0);
  return {
    send,
    run,
    errors,
    go: async (url: string, wait = 1800) => {
      await send('Page.navigate', { url });
      await sleep(wait);
    },
    /** Center of the first match, in page coordinates. */
    at: (selector: string) =>
      run(
        `(() => { const e = document.querySelector(${JSON.stringify(selector)}); if (!e) return null; const r = e.getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()`,
      ),
    mouse: (
      type: 'mousePressed' | 'mouseMoved' | 'mouseReleased',
      x: number,
      y: number,
      modifiers: (keyof typeof MODS)[] = [],
    ) =>
      send('Input.dispatchMouseEvent', {
        type,
        x,
        y,
        button: type === 'mouseMoved' ? 'none' : 'left',
        buttons: type === 'mouseReleased' ? 0 : 1,
        clickCount: type === 'mouseMoved' ? 0 : 1,
        modifiers: mods(modifiers),
        pointerType: 'mouse',
      }),
    /** A key as a person presses it, down and up. */
    key: async (code: string, key: string, modifiers: (keyof typeof MODS)[] = [], vk = 0) => {
      const common = {
        code,
        key,
        modifiers: mods(modifiers),
        windowsVirtualKeyCode: vk || key.toUpperCase().charCodeAt(0),
      };
      await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', ...common });
      await send('Input.dispatchKeyEvent', { type: 'keyUp', ...common });
      await sleep(30);
    },
    shot: async (path: string, quality = 88) => {
      const { data } = await send('Page.captureScreenshot', { format: 'jpeg', quality });
      writeFileSync(path, Buffer.from(data, 'base64'));
    },
    close: () => {
      ws.close();
      chrome.kill();
    },
  };
}
