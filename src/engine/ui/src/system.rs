use domain::apps::{AppId, Foreground};
use domain::settings::{BacklightDuration, ReadingLamp, Settings};
use embedded_graphics::mono_font::MonoFont;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Line, PrimitiveStyle, Rectangle};

use crate::app_screen::AppScreen;
use crate::controls::{Button, Input};
use crate::text::{self, BODY, HINT, LIST, TITLE};

const NAME: &str = "System";
const SECTION_GAP: i32 = 6;
const COLUMN_GAP: i32 = 16;
const BAR_PADDING: i32 = 3;
const ROW_GAP: i32 = 4;
const BOX_PADDING: i32 = 3;
const CHOICE_GAP: i32 = 4;
const SETTING_GAP: i32 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Setting {
    Backlight,
    ReadingLamp,
}

impl Setting {
    const ALL: [Self; 2] = [Self::Backlight, Self::ReadingLamp];

    fn name(self) -> &'static str {
        match self {
            Self::Backlight => "Backlight",
            Self::ReadingLamp => "Reading lamp",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OfferedApp {
    pub app: AppId,
    pub title: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Row {
    App(OfferedApp),
    Setting(Setting),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mark {
    Chosen,
    Plain,
}

#[derive(Debug, PartialEq, Eq)]
struct Choice {
    label: String,
    is_current: bool,
}

fn backlight_label(duration: BacklightDuration) -> &'static str {
    match duration {
        BacklightDuration::FiveSeconds => "5 s",
        BacklightDuration::TenSeconds => "10 s",
        BacklightDuration::ThirtySeconds => "30 s",
        BacklightDuration::Always => "always",
    }
}

fn reading_lamp_label(lamp: ReadingLamp) -> String {
    match lamp {
        ReadingLamp::Off => "off".into(),
        lamp => format!("{}%", lamp.level().as_percent()),
    }
}

pub struct SystemScreen {
    foreground: Foreground,
    settings: Settings,
    version: &'static str,
    offered: Vec<OfferedApp>,
    chosen_row: usize,
}

impl SystemScreen {
    pub fn new(foreground: Foreground, settings: Settings, version: &'static str, offered: Vec<OfferedApp>) -> Self {
        Self { foreground, settings, version, offered, chosen_row: 0 }
    }

    fn rows(&self) -> Vec<Row> {
        self.offered.iter().map(|&app| Row::App(app)).chain(Setting::ALL.map(Row::Setting)).collect()
    }

    fn mark(&self, row: Row) -> Mark {
        if self.rows().get(self.chosen_row) == Some(&row) {
            Mark::Chosen
        } else {
            Mark::Plain
        }
    }

    fn choices(&self, setting: Setting) -> Vec<Choice> {
        match setting {
            Setting::Backlight => {
                let current = self.settings.backlight();
                BacklightDuration::ALL.map(|d| Choice { label: backlight_label(d).into(), is_current: d == current }).into()
            }
            Setting::ReadingLamp => {
                let current = self.settings.reading_lamp();
                ReadingLamp::ALL.map(|lamp| Choice { label: reading_lamp_label(lamp), is_current: lamp == current }).into()
            }
        }
    }

    fn draw_setting<D: DrawTarget<Color = BinaryColor>>(&self, target: &mut D, setting: Setting, top_left: Point, width: u32) -> i32 {
        let mut top = top_left.y + draw_row(target, setting.name(), top_left, width, self.mark(Row::Setting(setting)));
        top += ROW_GAP;
        let right = top_left.x + width as i32;
        let mut left = top_left.x + BAR_PADDING - BOX_PADDING;
        for choice in self.choices(setting) {
            let size = Size::new(text::width(&choice.label, &LIST) + 2 * BOX_PADDING as u32, LIST.character_size.height + 2 * BOX_PADDING as u32);
            if left + size.width as i32 > right {
                break;
            }
            if choice.is_current {
                let _ = Rectangle::new(Point::new(left, top), size).into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1)).draw(target);
            }
            text::write(target, &choice.label, Point::new(left + BOX_PADDING, top + BOX_PADDING), size.width, &LIST);
            left += size.width as i32 + CHOICE_GAP;
        }
        top += LIST.character_size.height as i32 + 2 * BOX_PADDING;
        top - top_left.y
    }
}

fn draw_row<D: DrawTarget<Color = BinaryColor>>(target: &mut D, label: &str, top_left: Point, width: u32, mark: Mark) -> i32 {
    let height = BODY.character_size.height + 2 * BAR_PADDING as u32;
    let color = match mark {
        Mark::Chosen => {
            let _ = Rectangle::new(top_left, Size::new(width, height)).into_styled(PrimitiveStyle::with_fill(BinaryColor::On)).draw(target);
            BinaryColor::Off
        }
        Mark::Plain => BinaryColor::On,
    };
    let inner = width.saturating_sub(2 * BAR_PADDING as u32);
    text::write_in(target, label, top_left + Point::new(BAR_PADDING, BAR_PADDING), inner, &BODY, color);
    height as i32
}

fn heading<D: DrawTarget<Color = BinaryColor>>(target: &mut D, name: &str, top_left: Point, width: u32, font: &MonoFont<'_>) -> i32 {
    text::write(target, name, top_left + Point::new(BAR_PADDING, 0), width, font);
    font.character_size.height as i32 + ROW_GAP
}

impl<D: DrawTarget<Color = BinaryColor>> AppScreen<D> for SystemScreen {
    fn entered(&mut self) {
        let origin = self.foreground.before_system();
        self.chosen_row = self.rows().iter().position(|row| matches!(row, Row::App(offered) if offered.app == origin)).unwrap_or(0);
    }

    fn on_input(&mut self, input: Input) {
        match input {
            Input::Turn(detents) => {
                self.chosen_row = (self.chosen_row as i64 + i64::from(detents)).rem_euclid(self.rows().len() as i64) as usize;
            }
            Input::Press(Button::Long) => match self.rows().get(self.chosen_row) {
                Some(Row::App(offered)) => self.foreground.bring_to_front(offered.app),
                Some(Row::Setting(Setting::Backlight)) => self.settings.choose_next_backlight(),
                Some(Row::Setting(Setting::ReadingLamp)) => self.settings.choose_next_reading_lamp(),
                None => {}
            },
            Input::Press(Button::Yellow) => self.foreground.close_system(),
            Input::HoldYellowAndLong => {}
        }
    }

    fn draw(&self, target: &mut D, area: Rectangle) {
        let (left, top) = (area.top_left.x, area.top_left.y);
        let right = left + area.size.width as i32;
        let bottom = top + area.size.height as i32;
        let stroke = PrimitiveStyle::with_stroke(BinaryColor::On, 1);
        text::write(target, NAME, area.top_left, area.size.width, &TITLE);
        let version = format!("Cute Display {}", self.version);
        let version_width = text::width(&version, &HINT);
        let version_top = top + (TITLE.character_size.height - HINT.character_size.height) as i32;
        text::write(target, &version, Point::new(right - version_width as i32, version_top), version_width, &HINT);
        let rule = top + TITLE.character_size.height as i32 + SECTION_GAP;
        let _ = Line::new(Point::new(left, rule), Point::new(right - 1, rule)).into_styled(stroke).draw(target);

        let hint_top = bottom - HINT.character_size.height as i32;
        let columns_top = rule + 1 + SECTION_GAP;
        let apps_width = (area.size.width as i32 - COLUMN_GAP) * 2 / 5;
        let settings_left = left + apps_width + COLUMN_GAP;
        let settings_width = (right - settings_left).max(0) as u32;
        let divider = left + apps_width + COLUMN_GAP / 2;
        let _ = Line::new(Point::new(divider, columns_top), Point::new(divider, hint_top - SECTION_GAP)).into_styled(stroke).draw(target);

        let mut apps_top = columns_top + heading(target, "apps", Point::new(left, columns_top), apps_width as u32, &HINT);
        let mut settings_top = columns_top + heading(target, "settings", Point::new(settings_left, columns_top), settings_width, &HINT);
        for row in self.rows() {
            match row {
                Row::App(offered) => {
                    apps_top += draw_row(target, offered.title, Point::new(left, apps_top), apps_width as u32, self.mark(row)) + ROW_GAP;
                }
                Row::Setting(setting) => {
                    settings_top += self.draw_setting(target, setting, Point::new(settings_left, settings_top), settings_width) + SETTING_GAP;
                }
            }
        }
        text::write(target, "wheel: choose   click or long: open or change   yellow: back", Point::new(left, hint_top), area.size.width, &HINT);
    }
}

#[cfg(test)]
mod tests {
    use domain_testing::settings::StubSettingsStore;
    use hal::display::{Frame, HEIGHT, WIDTH};

    use super::*;

    const ALARM: OfferedApp = OfferedApp { app: AppId::new("alarm"), title: "Alarm clock" };
    const WEATHER: OfferedApp = OfferedApp { app: AppId::new("weather"), title: "Weather" };
    const RADAR: OfferedApp = OfferedApp { app: AppId::new("radar"), title: "Radar" };
    const OFFERED: [OfferedApp; 3] = [ALARM, WEATHER, RADAR];

    fn opened_from(app: OfferedApp) -> (SystemScreen, Foreground, Settings) {
        let foreground = Foreground::new(app.app);
        let settings = Settings::load(Box::new(StubSettingsStore));
        foreground.open_system();
        let mut screen = SystemScreen::new(foreground.clone(), settings.clone(), "2026.9.0", OFFERED.into());
        AppScreen::<Frame>::entered(&mut screen);
        (screen, foreground, settings)
    }

    fn input(screen: &mut SystemScreen, input: Input) {
        AppScreen::<Frame>::on_input(screen, input);
    }

    fn move_to(screen: &mut SystemScreen, row: Row) {
        let target = screen.rows().iter().position(|&r| r == row).unwrap() as i32;
        input(screen, Input::Turn(target - screen.chosen_row as i32));
    }

    fn chosen_row(screen: &SystemScreen) -> Option<Row> {
        screen.rows().get(screen.chosen_row).copied()
    }

    #[test]
    fn the_app_it_was_opened_from_is_chosen_first() {
        let (screen, _, _) = opened_from(RADAR);
        assert_eq!(chosen_row(&screen), Some(Row::App(RADAR)));
    }

    #[test]
    fn pressing_on_an_app_brings_it_to_the_front() {
        let (mut screen, foreground, _) = opened_from(WEATHER);
        move_to(&mut screen, Row::App(RADAR));
        input(&mut screen, Input::Press(Button::Long));
        assert_eq!(foreground.app(), RADAR.app);
    }

    #[test]
    fn pressing_on_a_setting_changes_it_and_stays_in_the_system_app() {
        let (mut screen, foreground, settings) = opened_from(WEATHER);
        move_to(&mut screen, Row::Setting(Setting::Backlight));
        input(&mut screen, Input::Press(Button::Long));
        assert_eq!(settings.backlight(), BacklightDuration::ThirtySeconds);
        move_to(&mut screen, Row::Setting(Setting::ReadingLamp));
        input(&mut screen, Input::Press(Button::Long));
        assert_eq!(settings.reading_lamp(), ReadingLamp::TenPercent);
        assert_eq!(foreground.app(), AppId::SYSTEM);
    }

    #[test]
    fn the_yellow_button_goes_back_without_changing_anything() {
        let (mut screen, foreground, settings) = opened_from(RADAR);
        input(&mut screen, Input::Turn(1));
        input(&mut screen, Input::Press(Button::Yellow));
        assert_eq!(foreground.app(), RADAR.app);
        assert_eq!(settings.backlight(), BacklightDuration::default());
    }

    #[test]
    fn every_app_offered_is_listed_before_the_settings() {
        let (screen, _, _) = opened_from(WEATHER);
        let listed: Vec<Row> = screen.rows();
        let apps: Vec<Row> = OFFERED.map(Row::App).into();
        assert_eq!(listed[..apps.len()], apps[..]);
        assert_eq!(listed[apps.len()..], Setting::ALL.map(Row::Setting));
    }

    #[test]
    fn the_chosen_row_wraps_around_both_ways() {
        let (mut screen, _, _) = opened_from(OFFERED[0]);
        input(&mut screen, Input::Turn(-1));
        assert_eq!(chosen_row(&screen), screen.rows().last().copied());
        input(&mut screen, Input::Turn(1));
        assert_eq!(chosen_row(&screen), screen.rows().first().copied());
    }

    #[test]
    fn each_setting_lists_its_values_the_current_one_marked() {
        let (mut screen, _, _) = opened_from(WEATHER);
        move_to(&mut screen, Row::Setting(Setting::ReadingLamp));
        input(&mut screen, Input::Press(Button::Long));
        let marked = |setting| -> Vec<(String, bool)> { screen.choices(setting).into_iter().map(|c| (c.label, c.is_current)).collect() };
        assert_eq!(
            marked(Setting::Backlight),
            [("5 s".into(), false), ("10 s".into(), true), ("30 s".into(), false), ("always".into(), false)]
        );
        assert_eq!(
            marked(Setting::ReadingLamp),
            [("off".into(), false), ("10%".into(), true), ("30%".into(), false), ("50%".into(), false), ("100%".into(), false)]
        );
    }

    const AREA: Rectangle = Rectangle::new(Point::new(8, 8), Size::new(382, 224));

    fn render(screen: &SystemScreen) -> Frame {
        let mut frame = Frame::blank();
        AppScreen::<Frame>::draw(screen, &mut frame, AREA);
        frame
    }

    #[test]
    fn nothing_is_drawn_outside_the_area() {
        let (mut screen, _, _) = opened_from(WEATHER);
        for row in screen.rows() {
            move_to(&mut screen, row);
            let frame = render(&screen);
            for x in 0..i32::from(WIDTH) {
                for y in 0..i32::from(HEIGHT) {
                    if frame.is_ink(x, y) {
                        assert!(AREA.contains(Point::new(x, y)), "{row:?} chosen: ink at ({x},{y})");
                    }
                }
            }
        }
    }

    fn bar_lines_under_the_apps(frame: &Frame) -> usize {
        let left = AREA.top_left.x;
        let right = AREA.top_left.x + AREA.size.width as i32 - 1;
        (AREA.top_left.y..AREA.top_left.y + AREA.size.height as i32)
            .filter(|&y| (left..left + 100).all(|x| frame.is_ink(x, y)) && !frame.is_ink(right, y))
            .count()
    }

    #[test]
    fn the_version_is_on_the_title_line_at_the_right() {
        let (screen, _, _) = opened_from(WEATHER);
        let frame = render(&screen);
        let right = AREA.top_left.x + AREA.size.width as i32;
        let title_line = AREA.top_left.y..AREA.top_left.y + TITLE.character_size.height as i32;
        assert!(title_line.clone().any(|y| (right - 20..right).any(|x| frame.is_ink(x, y))));
    }

    #[test]
    fn the_chosen_app_is_on_an_ink_bar() {
        let (mut screen, _, _) = opened_from(RADAR);
        assert!(bar_lines_under_the_apps(&render(&screen)) > 0);
        move_to(&mut screen, Row::Setting(Setting::Backlight));
        assert_eq!(bar_lines_under_the_apps(&render(&screen)), 0);
    }
}
