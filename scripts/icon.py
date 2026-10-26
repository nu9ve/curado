"""Reproducible curado icon and SVG mark from geometry. No external assets or dependencies.

The mark matches src/components/Logo.tsx: a 16×16 pixel ring opened to the right
(a lowercase "c" and a record) with a centre spindle. The ring is filled with an
ordered (Bayer) dither between coral and amber over a smooth dark squircle.

    python3 scripts/icon.py --output /tmp/curado-icon.png --svg docs/assets/logo.svg
    bun tauri icon /tmp/curado-icon.png
"""
from pathlib import Path
import argparse
import math
import struct
import zlib

parser = argparse.ArgumentParser()
parser.add_argument('--color', default='#d66154')
parser.add_argument('--light', default='#f2b36d')
parser.add_argument('--output', default='/tmp/curado-icon.png')
parser.add_argument('--svg', default=None)
args = parser.parse_args()


def hex_rgb(value):
    return tuple(bytes.fromhex(value.lstrip('#')))


CORAL, LIGHT = hex_rgb(args.color), hex_rgb(args.light)
BAYER = [0, 32, 8, 40, 2, 34, 10, 42, 48, 16, 56, 24, 50, 18, 58, 26, 12, 44, 4, 36, 14, 46, 6, 38,
         60, 28, 52, 20, 62, 30, 54, 22, 3, 35, 11, 43, 1, 33, 9, 41, 51, 19, 59, 27, 49, 17, 57, 25,
         15, 47, 7, 39, 13, 45, 5, 37, 63, 31, 55, 23, 61, 29, 53, 21]
GRID = 16


def cells():
    out = []
    for y in range(GRID):
        for x in range(GRID):
            dx, dy = x + 0.5 - GRID / 2, y + 0.5 - GRID / 2
            r = math.hypot(dx, dy)
            gap = dx > 1 and abs(dy) < 3
            if (4.3 <= r <= 7.7 and not gap) or (abs(dx) < 1 and abs(dy) < 1):
                out.append((x, y))
    return out


CELLS = set(cells())
W = 1024
# macOS icon grid: 824 px squircle centred on a 1024 canvas.
BOX, MARGIN = 824, 100
MARK = 560
CELL = MARK / GRID
OX = OY = (W - MARK) / 2


def squircle(x, y):
    nx = (x - W / 2) / (BOX / 2)
    ny = (y - W / 2) / (BOX / 2)
    return abs(nx) ** 5 + abs(ny) ** 5 <= 1


def coverage(x, y):
    hits = sum(squircle(x + ox, y + oy) for ox in (0.25, 0.75) for oy in (0.25, 0.75))
    return hits / 4


def lerp(a, b, t):
    return tuple(round(a[i] + (b[i] - a[i]) * t) for i in range(3))


raw = bytearray()
for y in range(W):
    raw.append(0)
    for x in range(W):
        a = coverage(x, y)
        if a == 0:
            raw.extend((0, 0, 0, 0))
            continue
        t = (y - MARGIN) / BOX
        color = lerp((38, 27, 26), (13, 10, 10), max(0, min(1, t)))
        glow = max(0, 1 - math.hypot(x - 300, y - 260) / 620) ** 2 * 0.35
        color = lerp(color, CORAL, glow)
        cx, cy = int((x - OX) // CELL), int((y - OY) // CELL)
        if 0 <= x - OX < MARK and 0 <= y - OY < MARK and (cx, cy) in CELLS:
            # Diagonal gradient, quantised by an 8×8 Bayer matrix in 8 px blocks.
            g = ((x - OX) + (y - OY)) / (2 * MARK)
            threshold = (BAYER[((y // 8) & 7) * 8 + ((x // 8) & 7)] + 0.5) / 64
            color = LIGHT if g * 1.25 - 0.15 > threshold else CORAL
        elif y - MARGIN < 3 and a == 1:
            color = lerp(color, (255, 255, 255), 0.08)
        raw.extend((*color, round(255 * a)))


def chunk(kind, data):
    return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))


Path(args.output).write_bytes(
    b'\x89PNG\r\n\x1a\n'
    + chunk(b'IHDR', struct.pack('>IIBBBBB', W, W, 8, 6, 0, 0, 0))
    + chunk(b'IDAT', zlib.compress(bytes(raw), 9))
    + chunk(b'IEND', b'')
)

if args.svg:
    rects = ''.join(f'<rect x="{x}" y="{y}" width="1.02" height="1.02"/>' for x, y in sorted(CELLS))
    Path(args.svg).parent.mkdir(parents=True, exist_ok=True)
    Path(args.svg).write_text(
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" shape-rendering="crispEdges">'
        f'<defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="{args.color}"/>'
        f'<stop offset="1" stop-color="{args.light}"/></linearGradient></defs>'
        f'<g fill="url(#g)">{rects}</g></svg>\n'
    )
