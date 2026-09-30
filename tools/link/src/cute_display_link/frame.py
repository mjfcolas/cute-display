"""What the glass shows, as src/engine/hal/src/display.rs lays it out: 416 × 240, a row
after another, the leftmost pixel in the most significant bit, 1 for ink."""
import struct
import sys
import zlib
from dataclasses import dataclass

WIDTH, HEIGHT = 416, 240
STRIDE = WIDTH // 8
BYTES = STRIDE * HEIGHT
PNG_SIGNATURE = b'\x89PNG\r\n\x1a\n'


@dataclass(frozen=True)
class Frame:
    ink: bytes

    def __post_init__(self):
        if len(self.ink) != BYTES:
            raise ValueError(f'a frame is {BYTES} bytes, not {len(self.ink)}')

    def is_ink(self, x, y):
        return bool(self.ink[y * STRIDE + x // 8] >> (7 - x % 8) & 1)

    def png(self, zoom=1):
        """Greyscale, black ink on white paper."""
        raw = bytearray()
        for y in range(HEIGHT):
            row = bytearray()
            for x in range(WIDTH):
                row += bytes([0x00 if self.is_ink(x, y) else 0xff]) * zoom
            for _ in range(zoom):
                raw += b'\x00' + row
        header = struct.pack('>IIBBBBB', WIDTH * zoom, HEIGHT * zoom, 8, 0, 0, 0, 0)
        return PNG_SIGNATURE + _chunk(b'IHDR', header) + _chunk(b'IDAT', zlib.compress(bytes(raw), 9)) + _chunk(b'IEND', b'')


def _chunk(tag, data):
    return struct.pack('>I', len(data)) + tag + data + struct.pack('>I', zlib.crc32(tag + data) & 0xffffffff)


def main():
    """A frame dump to a PNG: `python -m cute_display_link.frame frame.fb frame.png [zoom]`."""
    if not 3 <= len(sys.argv) <= 4:
        sys.exit(main.__doc__)
    source, destination = sys.argv[1], sys.argv[2]
    zoom = int(sys.argv[3]) if len(sys.argv) == 4 else 2
    with open(source, 'rb') as f:
        frame = Frame(f.read())
    with open(destination, 'wb') as f:
        f.write(frame.png(zoom))
    print(destination)


if __name__ == '__main__':
    main()
