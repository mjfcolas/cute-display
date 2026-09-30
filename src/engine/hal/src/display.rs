use core::time::Duration;

use embedded_graphics_core::pixelcolor::BinaryColor;
use embedded_graphics_core::prelude::*;

use crate::Fault;

pub const WIDTH: u16 = 416;
pub const HEIGHT: u16 = 240;
/// The case covers every column from here to the right edge.
pub const VISIBLE_WIDTH: u16 = 398;

const BYTES_PER_ROW: usize = WIDTH as usize / 8;
pub const FRAME_BYTES: usize = BYTES_PER_ROW * HEIGHT as usize;

/// [`BinaryColor::On`] is ink.
#[derive(Clone, PartialEq, Eq)]
pub struct Frame {
    ink: Box<[u8; FRAME_BYTES]>,
}

impl Frame {
    pub fn blank() -> Self {
        Self { ink: Box::new([0; FRAME_BYTES]) }
    }

    pub fn is_ink(&self, x: i32, y: i32) -> bool {
        Self::locate(x, y).is_some_and(|(byte, mask)| self.ink.get(byte).is_some_and(|b| b & mask != 0))
    }

    pub fn set_ink(&mut self, x: i32, y: i32, ink: bool) {
        let Some((byte, mask)) = Self::locate(x, y) else {
            return;
        };
        if let Some(b) = self.ink.get_mut(byte) {
            if ink {
                *b |= mask;
            } else {
                *b &= !mask;
            }
        }
    }

    /// Row-major, 52 bytes per row, most significant bit leftmost, 1 = ink.
    pub fn as_bytes(&self) -> &[u8; FRAME_BYTES] {
        &self.ink
    }

    fn locate(x: i32, y: i32) -> Option<(usize, u8)> {
        let inside = (0..i32::from(WIDTH)).contains(&x) && (0..i32::from(HEIGHT)).contains(&y);
        inside.then(|| (y as usize * BYTES_PER_ROW + x as usize / 8, 0x80 >> (x % 8)))
    }
}

impl Default for Frame {
    fn default() -> Self {
        Self::blank()
    }
}

impl OriginDimensions for Frame {
    fn size(&self) -> Size {
        Size::new(u32::from(WIDTH), u32::from(HEIGHT))
    }
}

impl DrawTarget for Frame {
    type Color = BinaryColor;
    type Error = core::convert::Infallible;

    fn draw_iter<I: IntoIterator<Item = Pixel<BinaryColor>>>(&mut self, pixels: I) -> Result<(), Self::Error> {
        for Pixel(p, color) in pixels {
            self.set_ink(p.x, p.y, color.is_on());
        }
        Ok(())
    }

    fn clear(&mut self, color: BinaryColor) -> Result<(), Self::Error> {
        self.ink.fill(if color.is_on() { 0xff } else { 0 });
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Redraw {
    /// The whole glass, on the clean waveform: slow, flashes, leaves no ghost.
    Whole,
    /// Only what changed, on the fast waveform: quick and quiet, ghosts a little.
    Changes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refreshed {
    Whole { took: Duration },
    Columns { count: u16, took: Duration },
    Nothing,
}

pub trait EpaperDisplay {
    /// The display may redraw more than asked, never less.
    fn show(&mut self, frame: &Frame, redraw: Redraw) -> Result<Refreshed, Fault>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ink_round_trips_without_touching_neighbours() {
        let mut f = Frame::blank();
        f.set_ink(7, 0, true);
        f.set_ink(8, 1, true);
        assert!(f.is_ink(7, 0) && f.is_ink(8, 1));
        assert!(!f.is_ink(6, 0) && !f.is_ink(9, 1));
        f.set_ink(7, 0, false);
        assert!(!f.is_ink(7, 0));
    }

    #[test]
    fn off_panel_pixels_are_paper_and_ignored() {
        let mut f = Frame::blank();
        for (x, y) in [(-1, 0), (0, -1), (i32::from(WIDTH), 0), (0, i32::from(HEIGHT))] {
            f.set_ink(x, y, true);
            assert!(!f.is_ink(x, y));
        }
        assert!(f.as_bytes().iter().all(|&b| b == 0));
    }
}
