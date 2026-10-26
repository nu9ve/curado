/**
 * Ordered (Bayer 8×8) dither textures, generated once per theme change.
 * Each texture is a tiny PNG data URL: CSS scales it with `image-rendering: pixelated`
 * so the pixels stay crisp. Allowed by the CSP (`img-src 'self' data:`).
 */
const BAYER = [
  0, 32, 8, 40, 2, 34, 10, 42, 48, 16, 56, 24, 50, 18, 58, 26, 12, 44, 4, 36, 14, 46, 6, 38, 60, 28,
  52, 20, 62, 30, 54, 22, 3, 35, 11, 43, 1, 33, 9, 41, 51, 19, 59, 27, 49, 17, 57, 25, 15, 47, 7,
  39, 13, 45, 5, 37, 63, 31, 55, 23, 61, 29, 53, 21,
];
/** Threshold in [0, 1) for a pixel; exported for tests and canvas renderers. */
export const bayer = (x: number, y: number) => (BAYER[(y & 7) * 8 + (x & 7)] + 0.5) / 64;

type Shape = (x: number, y: number, w: number, h: number) => number;
const shapes: Record<string, { w: number; h: number; f: Shape }> = {
  // Dense at the leading edge, dissolving toward the end.
  x: { w: 64, h: 8, f: (x, _y, w) => 1 - x / (w - 1) },
  y: { w: 8, h: 64, f: (_x, y, _w, h) => 1 - y / (h - 1) },
  // Uniform low-density grain for materials.
  grain: { w: 8, h: 8, f: () => 0.14 },
  half: { w: 8, h: 8, f: () => 0.5 },
};

export function ditherUrl(color: string, shape: keyof typeof shapes, strength = 1): string {
  if (typeof document === 'undefined') return 'none';
  const { w, h, f } = shapes[shape];
  const canvas = document.createElement('canvas');
  canvas.width = w;
  canvas.height = h;
  const ctx = canvas.getContext('2d');
  if (!ctx) return 'none';
  ctx.fillStyle = color;
  for (let y = 0; y < h; y++)
    for (let x = 0; x < w; x++)
      if (f(x, y, w, h) * strength > bayer(x, y)) ctx.fillRect(x, y, 1, 1);
  return `url("${canvas.toDataURL('image/png')}")`;
}
