"""Reproducible application icon from geometry; default brand coral, no external assets."""
from pathlib import Path
import struct
import zlib
import math
import argparse
parser=argparse.ArgumentParser()
parser.add_argument('--color', default='#d66154')
parser.add_argument('--output', default='/tmp/curado-icon.png')
args=parser.parse_args()
color=tuple(bytes.fromhex(args.color.lstrip('#')))+(255,)
w=512
raw=bytearray()
for y in range(w):
    raw.append(0)
    for x in range(w):
        dx=x-w/2;dy=y-w/2
        c=(24,20,19,255)
        if dx*dx+dy*dy<210**2:c=color
        if dx*dx+dy*dy<150**2:c=(24,20,19,255)
        if abs(dx)<103 and abs(dy-45*math.sin(dx/21))<10:c=color
        raw.extend(c)
def chunk(t,d):
    return struct.pack('>I',len(d))+t+d+struct.pack('>I',zlib.crc32(t+d))
Path(args.output).write_bytes(b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',w,w,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(raw))+chunk(b'IEND',b''))
