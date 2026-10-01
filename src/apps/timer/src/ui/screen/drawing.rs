use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use ui::big_digits::CLOCK;
use ui::mark::Mark;
use ui::text::{self, BODY, HINT};
use ui::DrawWithin;

use super::ui_state::TimerPage;

const TAB_PADDING: i32 = 5;
const TAB_GAP: i32 = 14;
const GAP: i32 = 16;

impl DrawWithin for TimerPage {
    fn draw_within<D: DrawTarget<Color = BinaryColor>>(&self, target: &mut D, area: Rectangle) {
        let font_height = BODY.character_size.height as i32;
        let mut left = area.top_left.x;
        for tab in &self.modes {
            let width = text::width(tab.name, &BODY);
            let at = Point::new(left + TAB_PADDING, area.top_left.y + TAB_PADDING);
            if tab.mark == Mark::Chosen {
                let size = Size::new(width + 2 * TAB_PADDING as u32, (font_height + 2 * TAB_PADDING) as u32);
                let _ = Rectangle::new(Point::new(left, area.top_left.y), size).into_styled(PrimitiveStyle::with_fill(BinaryColor::On)).draw(target);
                text::write_in(target, tab.name, at, width, &BODY, BinaryColor::Off);
            } else {
                text::write(target, tab.name, at, width, &BODY);
            }
            left += width as i32 + 2 * TAB_PADDING + TAB_GAP;
        }

        let number = i32::from(self.count.number());
        let unit_width = text::width(self.count.unit(), &BODY);
        let count_width = CLOCK.number_width(number) + unit_width;
        let count_left = area.top_left.x + (area.size.width as i32 - count_width as i32) / 2;
        let count_top = area.top_left.y + font_height + 2 * TAB_PADDING + GAP;
        CLOCK.draw_number(target, number, Point::new(count_left, count_top));
        let unit_at = Point::new(count_left + CLOCK.number_width(number) as i32, count_top + CLOCK.height as i32 - font_height);
        text::write(target, self.count.unit(), unit_at, unit_width, &BODY);

        let status_width = text::width(&self.status, &BODY).min(area.size.width);
        let status_left = area.top_left.x + (area.size.width as i32 - status_width as i32) / 2;
        let status_top = count_top + CLOCK.height as i32 + GAP;
        text::write(target, &self.status, Point::new(status_left, status_top), status_width, &BODY);

        let hint_top = area.top_left.y + area.size.height as i32 - HINT.character_size.height as i32;
        text::write(target, self.hint, Point::new(area.top_left.x, hint_top), area.size.width, &HINT);
    }
}

#[cfg(test)]
mod tests {
    use hal::display::{Frame, HEIGHT, WIDTH};
    use ui_testing::references;

    use super::*;
    use crate::ui::screen::ui_state::{Count, ModeTab};

    const AREA: Rectangle = Rectangle::new(Point::new(8, 8), Size::new(382, 224));

    fn tabs(chosen: &str) -> [ModeTab; 3] {
        ["Timer", "Pomodoro", "Nap"].map(|name| ModeTab { name, mark: if name == chosen { Mark::Chosen } else { Mark::Plain } })
    }

    fn set_page() -> TimerPage {
        TimerPage { modes: tabs("Pomodoro"), count: Count::Minutes(25), status: "Work, then 5 min of rest; 4 rounds".into(), hint: "wheel: minutes   yellow: start   long: mode" }
    }

    fn last_minute_page() -> TimerPage {
        TimerPage { modes: tabs("Timer"), count: Count::Seconds(42), status: "Counting down".into(), hint: "yellow: pause   long: stop" }
    }

    fn render(page: &TimerPage) -> Frame {
        let mut frame = Frame::blank();
        page.draw_within(&mut frame, AREA);
        frame
    }

    #[test]
    fn nothing_is_drawn_outside_the_area() {
        let long = TimerPage { count: Count::Minutes(120), status: "x".repeat(80), ..set_page() };
        for page in [set_page(), last_minute_page(), long] {
            let frame = render(&page);
            for x in 0..i32::from(WIDTH) {
                for y in 0..i32::from(HEIGHT) {
                    if frame.is_ink(x, y) {
                        assert!(AREA.contains(Point::new(x, y)), "{page:?}: ink at ({x},{y})");
                    }
                }
            }
        }
    }

    #[test]
    fn the_page_set_looks_as_its_reference() {
        references::check(&render(&set_page()), env!("CARGO_MANIFEST_DIR"), "set");
    }

    #[test]
    fn the_last_minute_looks_as_its_reference() {
        references::check(&render(&last_minute_page()), env!("CARGO_MANIFEST_DIR"), "last-minute");
    }
}
