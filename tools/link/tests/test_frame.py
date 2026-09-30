import unittest
import zlib

from cute_display_link.frame import BYTES, STRIDE, Frame


def with_ink(*points):
    ink = bytearray(BYTES)
    for x, y in points:
        ink[y * STRIDE + x // 8] |= 0x80 >> (x % 8)
    return Frame(bytes(ink))


class FrameTest(unittest.TestCase):
    def test_ink_is_where_the_bits_say(self):
        frame = with_ink((0, 0), (9, 1))
        self.assertTrue(frame.is_ink(0, 0) and frame.is_ink(9, 1))
        self.assertFalse(frame.is_ink(1, 0) or frame.is_ink(8, 1))

    def test_its_png_is_black_ink_on_white_paper(self):
        png = with_ink((1, 0)).png(zoom=2)
        idat = png.index(b'IDAT')
        length = int.from_bytes(png[idat - 4:idat], 'big')
        rows = zlib.decompress(png[idat + 4:idat + 4 + length])
        first_row = rows[1:1 + 2 * 416]
        self.assertEqual(first_row[:6], bytes([0xff, 0xff, 0x00, 0x00, 0xff, 0xff]))
        self.assertEqual(len(rows), 2 * 240 * (1 + 2 * 416))

    def test_a_frame_is_the_size_of_the_glass(self):
        with self.assertRaises(ValueError):
            Frame(b'\x00' * 10)
