use embedded_graphics::mono_font::iso_8859_1::{FONT_10X20, FONT_6X10, FONT_7X13, FONT_9X18_BOLD};
use embedded_graphics::mono_font::{MonoFont, MonoTextStyle};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Baseline, Text};

pub(crate) const TITLE: MonoFont<'static> = FONT_9X18_BOLD;
pub(crate) const BODY: MonoFont<'static> = FONT_10X20;
pub(crate) const HINT: MonoFont<'static> = FONT_6X10;
pub(crate) const LIST: MonoFont<'static> = FONT_7X13;

pub(crate) fn chars_across(width: u32, font: &MonoFont<'_>) -> usize {
    (width / (font.character_size.width + font.character_spacing)) as usize
}

/// Words into lines of at most `max_chars`; a word longer than a line is split over
/// several, since what is lost off its end may be the part that matters (a file name).
pub(crate) fn wrap(text: &str, max_chars: usize) -> Vec<String> {
    let max_chars = max_chars.max(1);
    let pieces = text.split_whitespace().flat_map(|word| {
        let chars: Vec<char> = word.chars().collect();
        chars.chunks(max_chars).map(|piece| piece.iter().collect::<String>()).collect::<Vec<_>>()
    });
    let mut lines: Vec<String> = Vec::new();
    for word in pieces {
        match lines.last_mut() {
            Some(line) if line.chars().count() + 1 + word.chars().count() <= max_chars => {
                line.push(' ');
                line.push_str(&word);
            }
            _ => lines.push(word),
        }
    }
    lines
}

/// Cut to what fits in `width`. The fonts are Latin-1: '°' and most accents draw, anything
/// further becomes '?'.
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
        .map(|c| if u32::from(c) <= 0xff { c } else { '?' })
        .collect();
    let _ = Text::with_baseline(&shown, top_left, MonoTextStyle::new(font, BinaryColor::On), Baseline::Top).draw(target);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_wrap_into_lines_that_fit() {
        assert_eq!(wrap("no place: put cute-display/radar.conf", 14), ["no place: put", "cute-display/r", "adar.conf"]);
        assert_eq!(wrap("offline: no Wi-Fi", 20), ["offline: no Wi-Fi"]);
        assert!(wrap("", 10).is_empty());
    }
}
