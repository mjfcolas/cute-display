#!/usr/bin/env python3
"""Turn a hal::display::Frame dump (416x240, row-major, MSB leftmost, 1 = ink) into a PNG.

  python3 tools/fb2png.py frame.fb frame.png [zoom]

Stdlib only.
"""
import struct
import sys
import zlib

WIDTH, HEIGHT = 416, 240
STRIDE = WIDTH // 8


def chunk(tag, data):
    return struct.pack('>I', len(data)) + tag + data + struct.pack('>I', zlib.crc32(tag + data) & 0xffffffff)


def main():
    if not 3 <= len(sys.argv) <= 4:
        sys.exit(__doc__)
    src, dst = sys.argv[1], sys.argv[2]
    zoom = int(sys.argv[3]) if len(sys.argv) == 4 else 2
    fb = open(src, 'rb').read()
    if len(fb) != STRIDE * HEIGHT:
        sys.exit(f'{src}: expected {STRIDE * HEIGHT} bytes, got {len(fb)}')

    raw = bytearray()
    for y in range(HEIGHT):
        row = bytearray()
        for x in range(WIDTH):
            ink = fb[y * STRIDE + x // 8] >> (7 - x % 8) & 1
            row += bytes([0x00 if ink else 0xff]) * zoom
        for _ in range(zoom):
            raw += b'\x00' + row

    header = struct.pack('>IIBBBBB', WIDTH * zoom, HEIGHT * zoom, 8, 0, 0, 0, 0)
    png = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', header) + chunk(b'IDAT', zlib.compress(bytes(raw), 9)) + chunk(b'IEND', b'')
    open(dst, 'wb').write(png)
    print(dst)


if __name__ == '__main__':
    main()
