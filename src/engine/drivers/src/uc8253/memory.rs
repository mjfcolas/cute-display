//! The controller's image memory is portrait: one row per landscape column, one bit per
//! landscape row, most significant bit first, 1 = white.

use core::ops::RangeInclusive;

use hal::display::{Frame, HEIGHT, WIDTH};

pub const ROW_BYTES: usize = HEIGHT as usize / 8;
pub const ROWS: usize = WIDTH as usize;
pub const BYTES: usize = ROW_BYTES * ROWS;

pub type Image = [u8; BYTES];

pub fn render(frame: &Frame, image: &mut Image) {
    for (x, row) in image.as_chunks_mut::<ROW_BYTES>().0.iter_mut().enumerate() {
        for (i, byte) in row.iter_mut().enumerate() {
            *byte = (0..8).fold(0xff, |b, bit| {
                if frame.is_ink(x as i32, (i * 8 + bit) as i32) { b & !(0x80 >> bit) } else { b }
            });
        }
    }
}

pub fn changed_rows(a: &Image, b: &Image) -> Option<RangeInclusive<usize>> {
    let differs = |(_, (x, y)): &(usize, (&[u8; ROW_BYTES], &[u8; ROW_BYTES]))| x != y;
    let mut rows = a.as_chunks::<ROW_BYTES>().0.iter().zip(b.as_chunks::<ROW_BYTES>().0).enumerate().filter(differs);
    let first = rows.next()?.0;
    let last = rows.next_back().map_or(first, |(i, _)| i);
    Some(first..=last)
}

pub fn rows<'a>(image: &'a Image, range: &RangeInclusive<usize>) -> &'a [u8] {
    image.get(range.start() * ROW_BYTES..(range.end() + 1) * ROW_BYTES).unwrap_or(&[])
}

pub fn rows_mut<'a>(image: &'a mut Image, range: &RangeInclusive<usize>) -> &'a mut [u8] {
    image.get_mut(range.start() * ROW_BYTES..(range.end() + 1) * ROW_BYTES).unwrap_or(&mut [])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn white() -> Box<Image> {
        Box::new([0xff; BYTES])
    }

    fn is_black(image: &Image, row: usize, column: usize) -> bool {
        image[row * ROW_BYTES + column / 8] & (0x80 >> (column % 8)) == 0
    }

    #[test]
    fn a_landscape_pixel_lands_transposed() {
        for (x, y) in [(0, 0), (415, 0), (0, 239), (415, 239), (3, 1)] {
            let mut frame = Frame::blank();
            frame.set_ink(x, y, true);
            let mut image = white();
            render(&frame, &mut image);
            assert!(is_black(&image, x as usize, y as usize), "({x},{y})");
            assert_eq!(image.iter().map(|b| b.count_zeros()).sum::<u32>(), 1, "({x},{y}) smeared");
        }
    }

    #[test]
    fn changed_rows_span_the_first_to_the_last_difference() {
        let a = white();
        assert_eq!(changed_rows(&a, &a), None);
        let mut b = white();
        b[10 * ROW_BYTES] = 0;
        assert_eq!(changed_rows(&a, &b), Some(10..=10));
        b[300 * ROW_BYTES + 29] = 0;
        assert_eq!(changed_rows(&a, &b), Some(10..=300));
        assert_eq!(rows(&b, &(10..=300)).len(), 291 * ROW_BYTES);
    }
}
