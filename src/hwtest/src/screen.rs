//! Two pages: the report, as two columns of checks under a title, and a checkerboard to
//! judge the panel itself. The report is framed by the visible edge of the glass, so a
//! missing side means the geometry is wrong.

use embedded_graphics::mono_font::ascii::{FONT_6X10, FONT_7X13, FONT_9X18_BOLD};
use embedded_graphics::mono_font::{MonoFont, MonoTextStyle};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Line, PrimitiveStyle, Rectangle};
use embedded_graphics::text::{Baseline, Text};
use hal::display::{Frame, HEIGHT, VISIBLE_WIDTH, WIDTH};

use crate::report::{Check, Report, Verdict};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Page {
    Report(Report),
    TestPattern,
}

impl Page {
    pub fn is_same_kind(&self, other: &Page) -> bool {
        core::mem::discriminant(self) == core::mem::discriminant(other)
    }
}

/// What the report page says around the checks.
#[derive(Clone, Copy, Debug)]
pub struct Legend {
    pub title: &'static str,
    pub help: &'static str,
    pub build: &'static str,
}

const MARGIN: i32 = 12;
const RIGHT: i32 = VISIBLE_WIDTH as i32 - 1;
const BOTTOM: i32 = HEIGHT as i32 - 1;

const TITLE_FONT: MonoFont<'static> = FONT_9X18_BOLD;
const CHECK_FONT: MonoFont<'static> = FONT_7X13;
const FOOTER_FONT: MonoFont<'static> = FONT_6X10;

const TITLE_RULE_Y: i32 = MARGIN + 20;
const CHECKS_TOP: i32 = TITLE_RULE_Y + 6;
const CHECK_PITCH: i32 = 16;
const FOOTER_PITCH: i32 = 11;
const FOOTER_TOP: i32 = BOTTOM - MARGIN - 2 * FOOTER_PITCH + 3;
const FOOTER_RULE_Y: i32 = FOOTER_TOP - 4;
pub const CHECKS_PER_COLUMN: usize = ((FOOTER_RULE_Y - CHECKS_TOP) / CHECK_PITCH) as usize;

const COLUMN_GAP: i32 = 8;
const COLUMN_WIDTH: i32 = (RIGHT - 2 * MARGIN - COLUMN_GAP) / 2;
const MARK_SIDE: i32 = 9;
const MARK_GAP: i32 = 5;
const SUBJECT_CHARS: usize = 7;
const CHECK_CHARS: usize = chars_across(COLUMN_WIDTH - MARK_SIDE - MARK_GAP, &CHECK_FONT);
pub const READING_CHARS: usize = CHECK_CHARS - SUBJECT_CHARS - 1;

const _: () = assert!(CHECKS_PER_COLUMN >= 10 && READING_CHARS >= 16);

const CHECKERBOARD_SQUARE: i32 = 16;

const fn advance(font: &MonoFont<'_>) -> i32 {
    (font.character_size.width + font.character_spacing) as i32
}

const fn chars_across(width: i32, font: &MonoFont<'_>) -> usize {
    (width / advance(font)) as usize
}

pub fn draw(page: &Page, legend: &Legend) -> Frame {
    let mut frame = Frame::blank();
    match page {
        Page::Report(report) => draw_report(&mut frame, report, legend),
        Page::TestPattern => draw_checkerboard(&mut frame),
    }
    frame
}

fn draw_report(frame: &mut Frame, report: &Report, legend: &Legend) {
    let stroke = PrimitiveStyle::with_stroke(BinaryColor::On, 1);
    let _ = Rectangle::with_corners(Point::zero(), Point::new(RIGHT, BOTTOM)).into_styled(stroke).draw(frame);

    let width = RIGHT - 2 * MARGIN;
    write(frame, legend.title, Point::new(MARGIN, MARGIN), &TITLE_FONT, chars_across(width, &TITLE_FONT));
    for y in [TITLE_RULE_Y, FOOTER_RULE_Y] {
        let _ = Line::new(Point::new(MARGIN, y), Point::new(RIGHT - MARGIN, y)).into_styled(stroke).draw(frame);
    }

    draw_column(frame, &report.hardware, MARGIN);
    draw_column(frame, &report.controls, MARGIN + COLUMN_WIDTH + COLUMN_GAP);

    for (n, line) in [legend.help, legend.build].iter().enumerate() {
        let at = Point::new(MARGIN, FOOTER_TOP + n as i32 * FOOTER_PITCH);
        write(frame, line, at, &FOOTER_FONT, chars_across(width, &FOOTER_FONT));
    }
}

fn draw_column(frame: &mut Frame, checks: &[Check], left: i32) {
    for (n, check) in checks.iter().take(CHECKS_PER_COLUMN).enumerate() {
        let top = CHECKS_TOP + n as i32 * CHECK_PITCH;
        draw_mark(frame, check.verdict, Point::new(left, top + 2));
        let subject_at = Point::new(left + MARK_SIDE + MARK_GAP, top);
        write(frame, check.subject, subject_at, &CHECK_FONT, SUBJECT_CHARS);
        let reading_at = subject_at + Point::new((SUBJECT_CHARS as i32 + 1) * advance(&CHECK_FONT), 0);
        write(frame, &check.reading, reading_at, &CHECK_FONT, READING_CHARS);
    }
}

/// Filled: pass. Crossed: fail. Hollow: pending. None: a plain reading.
fn draw_mark(frame: &mut Frame, verdict: Verdict, corner: Point) {
    let square = Rectangle::new(corner, Size::new(MARK_SIDE as u32, MARK_SIDE as u32));
    let stroke = PrimitiveStyle::with_stroke(BinaryColor::On, 1);
    match verdict {
        Verdict::Pass => {
            let _ = square.into_styled(PrimitiveStyle::with_fill(BinaryColor::On)).draw(frame);
        }
        Verdict::Fail => {
            let _ = square.into_styled(stroke).draw(frame);
            let far = corner + Point::new(MARK_SIDE - 1, MARK_SIDE - 1);
            let _ = Line::new(corner, far).into_styled(stroke).draw(frame);
            let _ = Line::new(Point::new(corner.x, far.y), Point::new(far.x, corner.y)).into_styled(stroke).draw(frame);
        }
        Verdict::Pending => {
            let _ = square.into_styled(stroke).draw(frame);
        }
        Verdict::Reading => {}
    }
}

/// Cut to `max_chars`; the fonts are ASCII, so anything else becomes '?'.
fn write(frame: &mut Frame, text: &str, top_left: Point, font: &MonoFont<'_>, max_chars: usize) {
    let shown: String = text.chars().take(max_chars).map(|c| if c.is_ascii() { c } else { '?' }).collect();
    let _ = Text::with_baseline(&shown, top_left, MonoTextStyle::new(font, BinaryColor::On), Baseline::Top).draw(frame);
}

fn draw_checkerboard(frame: &mut Frame) {
    for y in 0..i32::from(HEIGHT) {
        for x in 0..i32::from(WIDTH) {
            frame.set_ink(x, y, (x / CHECKERBOARD_SQUARE + y / CHECKERBOARD_SQUARE) % 2 == 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOO_LONG: &str = "a reading far too long to ever fit in a single column of this screen";

    fn crowded() -> Frame {
        let check = Check { subject: "SUBJECT TOO LONG", verdict: Verdict::Fail, reading: TOO_LONG.into() };
        let report = Report { hardware: vec![check.clone(); 20], controls: vec![check; 20] };
        draw(&Page::Report(report), &Legend { title: TOO_LONG, help: TOO_LONG, build: TOO_LONG })
    }

    #[test]
    fn nothing_is_drawn_behind_the_case() {
        let frame = crowded();
        for x in i32::from(VISIBLE_WIDTH)..i32::from(WIDTH) {
            for y in 0..i32::from(HEIGHT) {
                assert!(!frame.is_ink(x, y), "ink at ({x},{y})");
            }
        }
    }

    #[test]
    fn the_columns_never_touch() {
        let frame = crowded();
        for x in MARGIN + COLUMN_WIDTH..MARGIN + COLUMN_WIDTH + COLUMN_GAP {
            for y in CHECKS_TOP..FOOTER_RULE_Y {
                assert!(!frame.is_ink(x, y), "ink in the gap at ({x},{y})");
            }
        }
    }

    #[test]
    fn the_visible_edge_is_framed_on_all_four_sides() {
        let report = Report { hardware: vec![], controls: vec![] };
        let frame = draw(&Page::Report(report), &Legend { title: "", help: "", build: "" });
        assert!(frame.is_ink(0, 100) && frame.is_ink(RIGHT, 100));
        assert!(frame.is_ink(200, 0) && frame.is_ink(200, BOTTOM));
    }

    #[test]
    fn each_verdict_has_its_own_mark() {
        let ink = |verdict| {
            let mut frame = Frame::blank();
            draw_mark(&mut frame, verdict, Point::zero());
            (0..MARK_SIDE).flat_map(|y| (0..MARK_SIDE).map(move |x| (x, y))).filter(|&(x, y)| frame.is_ink(x, y)).count()
        };
        assert_eq!(ink(Verdict::Reading), 0);
        assert!(ink(Verdict::Pending) < ink(Verdict::Fail));
        assert!(ink(Verdict::Fail) < ink(Verdict::Pass));
    }
}
