//! The screen of each app. Placeholders, until the real apps arrive.

mod counter;
mod echo;
mod ping;

pub use counter::CounterScreen;
pub use echo::EchoScreen;
pub use ping::PingScreen;

use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

use crate::text::{self, BODY, HINT};

/// Lines of body text from the top of `area`, and a hint at its foot.
fn draw_lines<D: DrawTarget<Color = BinaryColor>>(target: &mut D, area: Rectangle, lines: &[&str], hint: &str) {
    for (n, line) in lines.iter().enumerate() {
        let top = area.top_left.y + n as i32 * (BODY.character_size.height as i32 + 6);
        text::write(target, line, Point::new(area.top_left.x, top), area.size.width, &BODY);
    }
    let hint_top = area.top_left.y + area.size.height as i32 - HINT.character_size.height as i32;
    text::write(target, hint, Point::new(area.top_left.x, hint_top), area.size.width, &HINT);
}
