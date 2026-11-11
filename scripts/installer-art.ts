/**
 * Windows installer art, colors and display font, generated from the app's own theme.
 * Nothing here is drawn by hand: the colors come from `palette()`, the grain from the
 * Bayer matrix in `dither.ts` and the mark from `LOGO_CELLS`. The only source image is
 * the nu9ve signature (`src-tauri/windows/nu9ve.png`, ink coverage in gray).
 *
 *   bun scripts/installer-art.ts                 write the files into src-tauri/windows
 *   bun scripts/installer-art.ts --check         fail if a committed file is out of date
 *   bun scripts/installer-art.ts --preview DIR   also write PNG previews of the bitmaps
 */
import { deflateSync, inflateSync } from 'node:zlib';
import { LOGO_CELLS } from '../src/components/Logo';
import { bayer } from '../src/lib/dither';
import { DEFAULT_THEME, contrast, palette, rgb } from '../src/lib/theme';

type RGB = [number, number, number];
const bytes = (hex: string) => rgb(hex).map((v) => Math.round(v * 255)) as RGB;
const OUT = new URL('../src-tauri/windows/', import.meta.url);
const FONT = new URL(
  '../node_modules/@fontsource/jersey-15/files/jersey-15-latin-400-normal.woff',
  import.meta.url,
);
export const FONT_FILE = 'Jersey15-Regular.ttf';

/** The installer's colors: the default curado theme, dark. */
export function installerColors() {
  const p = palette(DEFAULT_THEME);
  return {
    chrome: p.chrome,
    sheet: p.panel,
    text: p.text,
    text2: p.tokens['--text-secondary'],
    muted: p.tokens['--text-muted'],
    accent: p.accent,
    accentText: p.tokens['--accent-text'],
    gradient: p.gradient,
    secondary: p.secondary,
    danger: p.tokens['--danger'],
  };
}

class Canvas {
  readonly data: Uint8Array;
  constructor(
    readonly w: number,
    readonly h: number,
    fill: RGB,
  ) {
    this.data = new Uint8Array(w * h * 3);
    for (let i = 0; i < w * h; i++) this.data.set(fill, i * 3);
  }
  set(x: number, y: number, c: RGB) {
    if (x >= 0 && y >= 0 && x < this.w && y < this.h) this.data.set(c, (y * this.w + x) * 3);
  }
  get(x: number, y: number): RGB {
    const i = (y * this.w + x) * 3;
    return [this.data[i], this.data[i + 1], this.data[i + 2]];
  }
  /** One art pixel of `unit` real pixels: the installer scales by whole steps at 150 % and 200 %. */
  block(x: number, y: number, unit: number, c: RGB) {
    for (let dy = 0; dy < unit; dy++)
      for (let dx = 0; dx < unit; dx++) this.set(x * unit + dx, y * unit + dy, c);
  }
}

const lerp = (a: RGB, b: RGB, t: number): RGB => [
  Math.round(a[0] + (b[0] - a[0]) * t),
  Math.round(a[1] + (b[1] - a[1]) * t),
  Math.round(a[2] + (b[2] - a[2]) * t),
];
/** Ordered dither of `v` in [0, 1] over a ramp of flat colors. */
const ramp = (colors: RGB[], v: number, x: number, y: number) =>
  colors[
    Math.max(0, Math.min(colors.length - 1, Math.floor(v * (colors.length - 1) + bayer(x, y))))
  ];
const glow = (x: number, y: number, cx: number, cy: number, radius: number) =>
  Math.max(0, 1 - Math.hypot(x - cx, y - cy) / radius) ** 2;

/** The c-ring: accent to gradient end along the diagonal, as in the app's logo, but dithered. */
function mark(canvas: Canvas, unit: number, ox: number, oy: number, cell: number, a: RGB, b: RGB) {
  const size = cell * 16;
  for (const [cx, cy] of LOGO_CELLS)
    for (let dy = 0; dy < cell; dy++)
      for (let dx = 0; dx < cell; dx++) {
        const x = ox + cx * cell + dx;
        const y = oy + cy * cell + dy;
        const g = (x - ox + (y - oy)) / (2 * size);
        canvas.block(x, y, unit, g * 1.25 - 0.15 > bayer(x, y) ? b : a);
      }
}

/** Sidebar of the welcome and finish pages, 164×314: Modern UI's size at 100 %. */
export function sidebar(signature: Gray): Canvas {
  const c = installerColors();
  const unit = 2;
  const w = 82;
  const h = 157;
  const chrome = bytes(c.chrome);
  const canvas = new Canvas(w * unit, h * unit, chrome);
  const warm = [0, 0.1, 0.2, 0.32, 0.46].map((t) => lerp(chrome, bytes(c.accent), t));
  const cool = [0, 0.1, 0.22, 0.34].map((t) => lerp(chrome, bytes(c.secondary), t));
  const ring = lerp(chrome, bytes(c.accent), 0.62);
  const sheet = bytes(c.sheet);
  const cell = 3;
  const mx = (w - cell * 16) / 2;
  const my = 26;
  const centerX = mx + cell * 8;
  const centerY = my + cell * 8;
  for (let y = 0; y < h; y++)
    for (let x = 0; x < w; x++) {
      // Main-color glow from the top left, B-color glow from the bottom right: the app's chrome.
      const a = glow(x, y, 6, 4, 96) * 0.95;
      const b = glow(x, y, w + 6, h - 34, 78) * 0.9;
      let color = a >= b ? ramp(warm, a, x, y) : ramp(cool, b, x, y);
      // Grooves: dotted rings leaving the record, thinning out with distance.
      const r = Math.hypot(x + 0.5 - centerX, y + 0.5 - centerY);
      const groove = (r - 31) / 7;
      if (groove >= 0 && groove - Math.floor(groove) < 1 / 7) {
        const fade = Math.max(0, 1 - Math.floor(groove) / 9) * 0.7;
        if (fade > bayer(x, y)) color = ring;
      }
      // The chrome dissolves into the sheet at its right edge.
      const edge = (x - (w - 12)) / 12;
      if (edge > 0 && edge * 0.9 > bayer(x, y)) color = sheet;
      canvas.block(x, y, unit, color);
    }
  mark(canvas, unit, mx, my, cell, bytes(c.accent), bytes(c.gradient));
  stamp(canvas, signature, 82, canvas.h - 40, bytes(c.muted));
  return canvas;
}

/** Header of the progress and maintenance pages, 150×57, at the right of the header. */
export function header(): Canvas {
  const c = installerColors();
  const chrome = bytes(c.chrome);
  const canvas = new Canvas(150, 57, chrome);
  const warm = [0, 0.1, 0.2, 0.32].map((t) => lerp(chrome, bytes(c.accent), t));
  const unit = 2;
  for (let y = 0; y < 29; y++)
    for (let x = 0; x < 75; x++)
      canvas.block(x, y, unit, ramp(warm, glow(x, y, 60, 14, 46) * 0.9, x, y));
  mark(canvas, unit, 51, 6, 1, bytes(c.accent), bytes(c.gradient));
  return canvas;
}

// ---- nu9ve signature --------------------------------------------------------

interface Gray {
  w: number;
  h: number;
  data: Uint8Array;
}

/** Decodes an 8-bit grayscale, non-interlaced PNG: the format of `nu9ve.png`. */
export function decodeGrayPng(bytes: Uint8Array): Gray {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  let offset = 8;
  let w = 0;
  let h = 0;
  const idat: Uint8Array[] = [];
  while (offset < bytes.length) {
    const length = view.getUint32(offset);
    const type = String.fromCharCode(...bytes.subarray(offset + 4, offset + 8));
    const body = bytes.subarray(offset + 8, offset + 8 + length);
    if (type === 'IHDR') {
      w = view.getUint32(offset + 8);
      h = view.getUint32(offset + 12);
      if (body[8] !== 8 || body[9] !== 0 || body[12] !== 0)
        throw new Error('nu9ve.png must be 8-bit grayscale, not interlaced');
    } else if (type === 'IDAT') idat.push(body);
    offset += 12 + length;
  }
  const raw = inflateSync(Buffer.concat(idat));
  const data = new Uint8Array(w * h);
  for (let y = 0; y < h; y++) {
    const filter = raw[y * (w + 1)];
    for (let x = 0; x < w; x++) {
      const left = x ? data[y * w + x - 1] : 0;
      const up = y ? data[(y - 1) * w + x] : 0;
      const corner = x && y ? data[(y - 1) * w + x - 1] : 0;
      const p = left + up - corner;
      const paeth =
        Math.abs(p - left) <= Math.abs(p - up) && Math.abs(p - left) <= Math.abs(p - corner)
          ? left
          : Math.abs(p - up) <= Math.abs(p - corner)
            ? up
            : corner;
      const predicted = [0, left, up, (left + up) >> 1, paeth][filter];
      data[y * w + x] = (raw[y * (w + 1) + 1 + x] + predicted) & 255;
    }
  }
  return { w, h, data };
}

/** Draws the signature `width` pixels wide, centred on the canvas, by area average. */
function stamp(canvas: Canvas, ink: Gray, width: number, top: number, color: RGB) {
  const scale = ink.w / width;
  const height = Math.round(ink.h / scale);
  const left = Math.round((canvas.w - width) / 2);
  for (let y = 0; y < height; y++)
    for (let x = 0; x < width; x++) {
      let sum = 0;
      let n = 0;
      for (let sy = Math.floor(y * scale); sy < Math.min(ink.h, Math.ceil((y + 1) * scale)); sy++)
        for (
          let sx = Math.floor(x * scale);
          sx < Math.min(ink.w, Math.ceil((x + 1) * scale));
          sx++
        ) {
          sum += ink.data[sy * ink.w + sx];
          n++;
        }
      const coverage = n ? sum / n / 255 : 0;
      if (coverage > 0.02)
        canvas.set(left + x, top + y, lerp(canvas.get(left + x, top + y), color, coverage));
    }
}

// ---- Files ------------------------------------------------------------------

/** 24-bit bottom-up BMP, the only format Modern UI loads. */
export function bmp(canvas: Canvas): Uint8Array {
  const row = (canvas.w * 3 + 3) & ~3;
  const out = new Uint8Array(54 + row * canvas.h);
  const view = new DataView(out.buffer);
  out.set([0x42, 0x4d]);
  view.setUint32(2, out.length, true);
  view.setUint32(10, 54, true);
  view.setUint32(14, 40, true);
  view.setInt32(18, canvas.w, true);
  view.setInt32(22, canvas.h, true);
  view.setUint16(26, 1, true);
  view.setUint16(28, 24, true);
  view.setUint32(34, row * canvas.h, true);
  view.setInt32(38, 3780, true);
  view.setInt32(42, 3780, true);
  for (let y = 0; y < canvas.h; y++)
    for (let x = 0; x < canvas.w; x++) {
      const [r, g, b] = canvas.get(x, y);
      out.set([b, g, r], 54 + (canvas.h - 1 - y) * row + x * 3);
    }
  return out;
}

function png(canvas: Canvas): Uint8Array {
  const table = Array.from({ length: 256 }, (_, n) => {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    return c >>> 0;
  });
  const chunk = (type: string, body: Uint8Array) => {
    const out = new Uint8Array(12 + body.length);
    const view = new DataView(out.buffer);
    view.setUint32(0, body.length);
    out.set(
      [...type].map((ch) => ch.charCodeAt(0)),
      4,
    );
    out.set(body, 8);
    let crc = 0xffffffff;
    for (const byte of out.subarray(4, 8 + body.length))
      crc = table[(crc ^ byte) & 255] ^ (crc >>> 8);
    view.setUint32(8 + body.length, (crc ^ 0xffffffff) >>> 0);
    return out;
  };
  const head = new Uint8Array(13);
  const view = new DataView(head.buffer);
  view.setUint32(0, canvas.w);
  view.setUint32(4, canvas.h);
  head.set([8, 2, 0, 0, 0], 8);
  const raw = new Uint8Array((canvas.w * 3 + 1) * canvas.h);
  for (let y = 0; y < canvas.h; y++)
    raw.set(
      canvas.data.subarray(y * canvas.w * 3, (y + 1) * canvas.w * 3),
      y * (canvas.w * 3 + 1) + 1,
    );
  return Buffer.concat([
    new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk('IHDR', head),
    chunk('IDAT', deflateSync(raw)),
    chunk('IEND', new Uint8Array()),
  ]);
}

/**
 * Rebuilds the TrueType file inside a WOFF 1 container. GDI loads .ttf, not .woff, and
 * Fontsource ships only web formats; the font and its license are unchanged.
 */
export function woffToSfnt(woff: Uint8Array): { sfnt: Uint8Array; family: string } {
  const view = new DataView(woff.buffer, woff.byteOffset, woff.byteLength);
  if (view.getUint32(0) !== 0x774f4646) throw new Error('Not a WOFF file');
  const count = view.getUint16(12);
  const tables = Array.from({ length: count }, (_, i) => {
    const o = 44 + i * 20;
    const [offset, compressed, length] = [
      view.getUint32(o + 4),
      view.getUint32(o + 8),
      view.getUint32(o + 12),
    ];
    const body = woff.subarray(offset, offset + compressed);
    return {
      tag: woff.subarray(o, o + 4),
      checksum: view.getUint32(o + 16),
      data: compressed < length ? new Uint8Array(inflateSync(body)) : body,
    };
  });
  let size = 12 + 16 * count;
  for (const t of tables) size += (t.data.length + 3) & ~3;
  const sfnt = new Uint8Array(size);
  const out = new DataView(sfnt.buffer);
  const selector = Math.floor(Math.log2(count));
  out.setUint32(0, view.getUint32(4));
  out.setUint16(4, count);
  out.setUint16(6, 2 ** selector * 16);
  out.setUint16(8, selector);
  out.setUint16(10, count * 16 - 2 ** selector * 16);
  let offset = 12 + 16 * count;
  tables.forEach((t, i) => {
    const o = 12 + i * 16;
    sfnt.set(t.tag, o);
    out.setUint32(o + 4, t.checksum);
    out.setUint32(o + 8, offset);
    out.setUint32(o + 12, t.data.length);
    sfnt.set(t.data, offset);
    offset += (t.data.length + 3) & ~3;
  });
  // Family name (name id 1, Windows platform, UTF-16BE): what CreateFont must ask for.
  const name = tables.find((t) => String.fromCharCode(...t.tag) === 'name')!.data;
  const names = new DataView(name.buffer, name.byteOffset, name.byteLength);
  let family = '';
  for (let i = 0; i < names.getUint16(2); i++) {
    const o = 6 + i * 12;
    if (names.getUint16(o) !== 3 || names.getUint16(o + 6) !== 1) continue;
    const start = names.getUint16(4) + names.getUint16(o + 10);
    for (let k = 0; k < names.getUint16(o + 8); k += 2)
      family += String.fromCharCode(names.getUint16(start + k));
    break;
  }
  if (!family) throw new Error('Font family name not found');
  return { sfnt, family };
}

/** Colors for NSIS: `RRGGBB` for SetCtlColors and `0x00BBGGRR` for Win32 messages. */
export function paletteNsh(family: string): string {
  const names: Record<string, string> = {
    chrome: 'CHROME',
    sheet: 'SHEET',
    text: 'TEXT',
    text2: 'TEXT_2',
    muted: 'MUTED',
    accent: 'ACCENT',
    accentText: 'ACCENT_TEXT',
    danger: 'DANGER',
  };
  const colors = installerColors() as Record<string, string>;
  const lines = [
    '; Generated by scripts/installer-art.ts from palette(DEFAULT_THEME). Do not edit:',
    '; change the theme, then run `bun scripts/installer-art.ts`.',
  ];
  for (const [key, name] of Object.entries(names)) {
    const [r, g, b] = bytes(colors[key]);
    const hex = (n: number) => n.toString(16).padStart(2, '0').toUpperCase();
    lines.push(`!define NU_${name} "${hex(r)}${hex(g)}${hex(b)}"`);
    lines.push(`!define NU_${name}_REF 0x00${hex(b)}${hex(g)}${hex(r)}`);
  }
  lines.push(`!define NU_DISPLAY_FONT "${family}"`, `!define NU_DISPLAY_FONT_FILE "${FONT_FILE}"`);
  return lines.join('\n') + '\n';
}

export async function build() {
  const signature = decodeGrayPng(await Bun.file(new URL('nu9ve.png', OUT)).bytes());
  const { sfnt, family } = woffToSfnt(await Bun.file(FONT).bytes());
  const side = sidebar(signature);
  const head = header();
  return {
    files: new Map<string, Uint8Array>([
      ['sidebar.bmp', bmp(side)],
      ['header.bmp', bmp(head)],
      [FONT_FILE, sfnt],
      ['palette.nsh', new TextEncoder().encode(paletteNsh(family))],
    ]),
    previews: new Map<string, Uint8Array>([
      ['sidebar.png', png(side)],
      ['header.png', png(head)],
    ]),
  };
}

/** Text over each surface it is drawn on; the installer has no other pairs. */
export function contrastReport() {
  const c = installerColors();
  return {
    text: Math.min(contrast(c.text, c.sheet), contrast(c.text, c.chrome)),
    text2: Math.min(contrast(c.text2, c.sheet), contrast(c.text2, c.chrome)),
    muted: Math.min(contrast(c.muted, c.sheet), contrast(c.muted, c.chrome)),
    accentText: Math.min(contrast(c.accentText, c.sheet), contrast(c.accentText, c.chrome)),
    signature: contrast(c.muted, c.chrome),
  };
}

if (import.meta.main) {
  const { files, previews } = await build();
  const check = process.argv.includes('--check');
  const stale: string[] = [];
  for (const [name, bytes] of files) {
    const file = Bun.file(new URL(name, OUT));
    const same =
      (await file.exists()) && Buffer.from(await file.bytes()).equals(Buffer.from(bytes));
    if (same) continue;
    if (check) stale.push(name);
    else {
      await Bun.write(file, bytes);
      console.log(`wrote src-tauri/windows/${name} (${bytes.length} bytes)`);
    }
  }
  const at = process.argv.indexOf('--preview');
  if (at > 0)
    for (const [name, bytes] of previews) await Bun.write(`${process.argv[at + 1]}/${name}`, bytes);
  if (stale.length) {
    console.error(`Out of date: ${stale.join(', ')}. Run: bun scripts/installer-art.ts`);
    process.exit(1);
  }
  console.log(check ? 'Installer art is up to date.' : 'Installer art ready.', contrastReport());
}
