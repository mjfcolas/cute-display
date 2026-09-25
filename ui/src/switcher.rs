//! The list of apps, open over the one in front until one is chosen. Where its dot
//! stands is screen state: the app in front only changes once a choice is made.

use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, PrimitiveStyle, Rectangle};

use crate::text::{self, BODY, HINT};

const ROW_PITCH: i32 = 26;
const DOT_DIAMETER: u32 = 8;
const DOT_GAP: i32 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Switcher {
    highlighted: usize,
    choices: usize,
}

impl Switcher {
    /// `None` without anything to choose from.
    pub fn open(highlighted: usize, choices: usize) -> Option<Self> {
        (choices > 0).then(|| Self { highlighted: highlighted.min(choices - 1), choices })
    }

    pub fn highlighted(&self) -> usize {
        self.highlighted
    }

    /// Whether the dot moved. The list wraps around.
    pub fn turn(&mut self, detents: i32) -> bool {
        let before = self.highlighted;
        let choices = self.choices as i64;
        self.highlighted = (self.highlighted as i64 + i64::from(detents)).rem_euclid(choices) as usize;
        self.highlighted != before
    }

    pub fn draw<D: DrawTarget<Color = BinaryColor>>(&self, target: &mut D, area: Rectangle, titles: &[&str]) {
        let text_left = area.top_left.x + DOT_DIAMETER as i32 + DOT_GAP;
        let text_width = area.size.width.saturating_sub(DOT_DIAMETER + DOT_GAP as u32);
        for (n, title) in titles.iter().enumerate() {
            let top = area.top_left.y + n as i32 * ROW_PITCH;
            if n == self.highlighted {
                let dot_top = top + (BODY.character_size.height as i32 - DOT_DIAMETER as i32) / 2;
                let _ = Circle::new(Point::new(area.top_left.x, dot_top), DOT_DIAMETER)
                    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                    .draw(target);
            }
            text::write(target, title, Point::new(text_left, top), text_width, &BODY);
        }
        let hint_top = area.top_left.y + area.size.height as i32 - HINT.character_size.height as i32;
        text::write(target, "wheel: choose   press: open   long: back", Point::new(area.top_left.x, hint_top), area.size.width, &HINT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn there_is_no_switcher_without_choices() {
        assert!(Switcher::open(0, 0).is_none());
    }

    #[test]
    fn the_dot_wraps_around_both_ways() {
        let mut switcher = Switcher::open(0, 3).unwrap();
        assert!(switcher.turn(-1));
        assert_eq!(switcher.highlighted(), 2);
        assert!(switcher.turn(4));
        assert_eq!(switcher.highlighted(), 0);
        assert!(!switcher.turn(3));
    }
}
