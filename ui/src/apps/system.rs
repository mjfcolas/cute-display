//! The system app: the other apps to choose from, and the device's settings.

use domain::apps::{App, Foreground};
use domain::settings::{BacklightDuration, ReadingLamp, Settings};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, PrimitiveStyle, Rectangle};

use crate::app_screen::{title, AppScreen};
use crate::controls::{Control, Input};
use crate::text::{self, BODY, HINT};

const ROW_PITCH: i32 = 24;
const SETTINGS_GAP: i32 = 8;
const DOT_DIAMETER: u32 = 8;
const DOT_GAP: i32 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Row {
    App(App),
    Backlight,
    ReadingLamp,
}

/// Every app it offers, then the settings.
fn rows() -> Vec<Row> {
    App::LAUNCHABLE.iter().map(|&app| Row::App(app)).chain([Row::Backlight, Row::ReadingLamp]).collect()
}

pub struct SystemScreen {
    foreground: Foreground,
    settings: Settings,
    /// The row the wheel is on.
    dot: usize,
}

impl SystemScreen {
    pub fn new(foreground: Foreground, settings: Settings) -> Self {
        Self { foreground, settings, dot: 0 }
    }

    fn label(&self, row: Row) -> String {
        match row {
            Row::App(app) => title(app).into(),
            Row::Backlight => format!(
                "Backlight: {}",
                match self.settings.backlight() {
                    BacklightDuration::FiveSeconds => "5 s",
                    BacklightDuration::TenSeconds => "10 s",
                    BacklightDuration::ThirtySeconds => "30 s",
                    BacklightDuration::Always => "always",
                }
            ),
            Row::ReadingLamp => match self.settings.reading_lamp() {
                ReadingLamp::Off => "Reading lamp: off".into(),
                lamp => format!("Reading lamp: {} %", lamp.level().as_percent()),
            },
        }
    }
}

impl<D: DrawTarget<Color = BinaryColor>> AppScreen<D> for SystemScreen {
    fn app(&self) -> App {
        App::System
    }

    /// The dot starts on the app the system app was opened from.
    fn entered(&mut self) {
        let origin = Row::App(self.foreground.before_system());
        self.dot = rows().iter().position(|&row| row == origin).unwrap_or(0);
    }

    fn on_input(&mut self, input: Input) {
        match input {
            Input::Turn(detents) => {
                self.dot = (self.dot as i64 + i64::from(detents)).rem_euclid(rows().len() as i64) as usize;
            }
            Input::Press(Control::Wheel) => match rows().get(self.dot) {
                Some(Row::App(app)) => self.foreground.bring_to_front(*app),
                Some(Row::Backlight) => self.settings.choose_next_backlight(),
                Some(Row::ReadingLamp) => self.settings.choose_next_reading_lamp(),
                None => {}
            },
            Input::Press(Control::Long) => self.foreground.close_system(),
            Input::Press(Control::Yellow) => {}
        }
    }

    fn draw(&self, target: &mut D, area: Rectangle) {
        let text_left = area.top_left.x + DOT_DIAMETER as i32 + DOT_GAP;
        let text_width = area.size.width.saturating_sub(DOT_DIAMETER + DOT_GAP as u32);
        let mut top = area.top_left.y;
        for (n, row) in rows().iter().enumerate() {
            if n > 0 && matches!(row, Row::Backlight) {
                top += SETTINGS_GAP;
            }
            if n == self.dot {
                let dot_top = top + (BODY.character_size.height as i32 - DOT_DIAMETER as i32) / 2;
                let _ = Circle::new(Point::new(area.top_left.x, dot_top), DOT_DIAMETER)
                    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                    .draw(target);
            }
            text::write(target, &self.label(*row), Point::new(text_left, top), text_width, &BODY);
            top += ROW_PITCH;
        }
        let hint_top = area.top_left.y + area.size.height as i32 - HINT.character_size.height as i32;
        text::write(target, "wheel: choose   press: open or change   long: back", Point::new(area.top_left.x, hint_top), area.size.width, &HINT);
    }
}

#[cfg(test)]
mod tests {
    use domain::settings::{SettingsRecord, SettingsStore};
    use hal::display::Frame;

    use super::*;

    struct Nowhere;

    impl SettingsStore for Nowhere {
        fn load(&mut self) -> Option<SettingsRecord> {
            None
        }
        fn save(&mut self, _: &SettingsRecord) {}
    }

    fn opened_from(app: App) -> (SystemScreen, Foreground, Settings) {
        let foreground = Foreground::new(app);
        let settings = Settings::load(Box::new(Nowhere));
        foreground.open_system();
        let mut screen = SystemScreen::new(foreground.clone(), settings.clone());
        AppScreen::<Frame>::entered(&mut screen);
        (screen, foreground, settings)
    }

    fn input(screen: &mut SystemScreen, input: Input) {
        AppScreen::<Frame>::on_input(screen, input);
    }

    #[test]
    fn the_dot_starts_on_the_app_it_was_opened_from() {
        let (screen, _, _) = opened_from(App::Echo);
        assert_eq!(rows().get(screen.dot).copied(), Some(Row::App(App::Echo)));
    }

    #[test]
    fn pressing_on_an_app_brings_it_to_the_front() {
        let (mut screen, foreground, _) = opened_from(App::Counter);
        input(&mut screen, Input::Turn(2));
        input(&mut screen, Input::Press(Control::Wheel));
        assert_eq!(foreground.app(), App::Ping);
    }

    #[test]
    fn pressing_on_a_setting_changes_it_and_stays_in_the_system_app() {
        let (mut screen, foreground, settings) = opened_from(App::Counter);
        input(&mut screen, Input::Turn(3));
        input(&mut screen, Input::Press(Control::Wheel));
        assert_eq!(settings.backlight(), BacklightDuration::ThirtySeconds);
        input(&mut screen, Input::Turn(1));
        input(&mut screen, Input::Press(Control::Wheel));
        assert_eq!(settings.reading_lamp(), ReadingLamp::TenPercent);
        assert_eq!(foreground.app(), App::System);
    }

    #[test]
    fn the_long_button_goes_back_without_changing_anything() {
        let (mut screen, foreground, settings) = opened_from(App::Echo);
        input(&mut screen, Input::Turn(1));
        input(&mut screen, Input::Press(Control::Long));
        assert_eq!(foreground.app(), App::Echo);
        assert_eq!(settings.backlight(), BacklightDuration::default());
    }

    #[test]
    fn the_dot_wraps_around_both_ways() {
        let (mut screen, _, _) = opened_from(App::Counter);
        input(&mut screen, Input::Turn(-1));
        assert_eq!(rows().get(screen.dot).copied(), Some(Row::ReadingLamp));
        input(&mut screen, Input::Turn(1));
        assert_eq!(rows().get(screen.dot).copied(), Some(Row::App(App::Counter)));
    }
}
