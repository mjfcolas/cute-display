use embedded_graphics::mono_font::ascii::{FONT_10X20, FONT_6X10, FONT_9X18_BOLD};
use embedded_graphics::mono_font::{MonoFont, MonoTextStyle};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Baseline, Text};

pub(crate) const TITLE: MonoFont<'static> = FONT_9X18_BOLD;
pub(crate) const BODY: MonoFont<'static> = FONT_10X20;
pub(crate) const HINT: MonoFont<'static> = FONT_6X10;

pub(crate) fn chars_across(width: u32, font: &MonoFont<'_>) -> usize {
    (width / (font.character_size.width + font.character_spacing)) as usize
}

/// Cut to what fits in `width`; the fonts are ASCII, so anything else becomes '?'.
pub(crate) fn write<D: DrawTarget<Color = BinaryColor>>(
    target: &mut D,
    text: &str,
    top_left: Point,
    width: u32,
    font: &MonoFont<'_>,
) {
    let shown: String = text
        .chars()
        .take(chars_across(width, font))
        .map(|c| if c.is_ascii() { c } else { '?' })
        .collect();
    let _ = Text::with_baseline(&shown, top_left, MonoTextStyle::new(font, BinaryColor::On), Baseline::Top).draw(target);
}
