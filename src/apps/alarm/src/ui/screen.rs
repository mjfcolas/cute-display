use std::hash::{DefaultHasher, Hash, Hasher};

use domain::calendar::Weekday;
use domain::clock::Clock;
use domain::time::{LocalTime, TimeOfDay};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, PrimitiveStyle, Rectangle};
use forecast::day_weather;
use forecast::Weather;
use ui::big_digits;
use ui::calendar_names;
use ui::controls::{Button, Input};
use ui::text::{self, BODY, HINT, TITLE};
use ui::AppScreen;

use crate::domain::alarm_clock::{AlarmClock, AlarmState, Ringtone, SNOOZE};

const FIRST_TIME: Option<TimeOfDay> = TimeOfDay::new(7, 0);
const RINGTONE_ROW: usize = Weekday::ALL.len();
const ROWS: usize = RINGTONE_ROW + 1;
const ROW_PITCH: i32 = 22;
const DOT_DIAMETER: u32 = 8;
const DOT_GAP: i32 = 10;
/// "Wednesday" and a space.
const DAY_NAME_CHARS: i32 = 10;
const GAP: i32 = 12;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Mode {
    Clock,
    Settings { row: usize },
    Hour { day: Weekday, time_to_put_back: Option<TimeOfDay> },
    Minute { day: Weekday, time_to_put_back: Option<TimeOfDay> },
    Ringtone { to_put_back: Ringtone },
}

pub struct AlarmScreen {
    alarm: AlarmClock,
    clock: Clock,
    weather: Weather,
    mode: Mode,
    hours_ahead: usize,
    /// Those to choose from, read when the choice opens.
    ringtones: Vec<Ringtone>,
}

impl AlarmScreen {
    pub fn new(alarm: AlarmClock, clock: Clock, weather: Weather) -> Self {
        Self { alarm, clock, weather, mode: Mode::Clock, hours_ahead: 0, ringtones: Vec::new() }
    }

    fn time_on(&self, day: Weekday) -> Option<TimeOfDay> {
        self.alarm.schedule().time_on(day)
    }

    fn days_row_of(day: Weekday) -> Mode {
        Mode::Settings { row: day.days_since_monday() }
    }

    /// Ringing or snoozed, every other input is ignored so that nothing half asleep turns
    /// the alarm off.
    fn on_clock_input(&mut self, input: Input) {
        match (self.alarm.state(), input) {
            (AlarmState::Ringing | AlarmState::Snoozed { .. }, Input::HoldYellowAndLong) => self.alarm.stop(),
            (AlarmState::Ringing, Input::Press(Button::Long)) => self.alarm.snooze(),
            (AlarmState::Ringing | AlarmState::Snoozed { .. }, _) => {}
            (AlarmState::Waiting { .. }, Input::Press(Button::Yellow)) => self.alarm.switch_on_or_off(),
            (AlarmState::Waiting { .. }, Input::Press(Button::Long)) => {
                let today = self.clock.now().map_or(Weekday::Monday, |now| now.date.weekday());
                self.mode = Self::days_row_of(today);
            }
            (AlarmState::Waiting { .. }, Input::Turn(detents)) => self.scroll_hours(detents),
            (AlarmState::Waiting { .. }, _) => {}
        }
    }

    fn scroll_hours(&mut self, detents: i32) {
        let known = match (self.weather.report().forecast, self.clock.now()) {
            (Some(forecast), Some(now)) => forecast.hours_from(now).count(),
            _ => 0,
        };
        let last = known.saturating_sub(day_weather::HOURS_SHOWN) as i64;
        let ahead = self.hours_ahead as i64 + i64::from(detents);
        self.hours_ahead = usize::try_from(ahead.clamp(0, last)).unwrap_or(0);
    }

    fn turn_hour(&self, day: Weekday, detents: i32) {
        let time = self.time_on(day);
        let position = time.map_or(0, |t| i32::from(t.hour()) + 1);
        let turned = (position + detents).rem_euclid(25);
        let minute = time.map_or(0, TimeOfDay::minute);
        let hour = u8::try_from(turned - 1).ok();
        self.alarm.set_time_on(day, hour.and_then(|hour| TimeOfDay::new(hour, minute)));
    }

    fn choose_ringtone(&mut self) {
        self.ringtones = self.alarm.ringtones();
        self.mode = Mode::Ringtone { to_put_back: self.alarm.ringtone() };
        self.alarm.preview();
    }

    fn turn_ringtone(&self, detents: i32) {
        let current = self.alarm.ringtone();
        let at = self.ringtones.iter().position(|ringtone| *ringtone == current).unwrap_or(0) as i64;
        let turned = (at + i64::from(detents)).rem_euclid(self.ringtones.len().max(1) as i64);
        if let Some(ringtone) = self.ringtones.get(turned as usize) {
            self.alarm.set_ringtone(ringtone.clone());
        }
    }

    fn close_ringtone(&mut self) {
        self.alarm.end_preview();
        self.mode = Mode::Settings { row: RINGTONE_ROW };
    }

    fn turn_minute(&self, day: Weekday, detents: i32) {
        if let Some(time) = self.time_on(day) {
            let minute = (i32::from(time.minute()) + detents).rem_euclid(60);
            self.alarm.set_time_on(day, u8::try_from(minute).ok().and_then(|minute| TimeOfDay::new(time.hour(), minute)));
        }
    }
}

impl<D: DrawTarget<Color = BinaryColor>> AppScreen<D> for AlarmScreen {
    fn entered(&mut self) {
        self.mode = Mode::Clock;
        self.hours_ahead = 0;
    }

    fn left(&mut self) {
        if let Mode::Ringtone { .. } = self.mode {
            self.alarm.end_preview();
        }
    }

    fn version(&self) -> u64 {
        let minute = self.clock.now().map_or(0, |now| now.seconds_since_epoch().div_euclid(60));
        let mut hasher = DefaultHasher::new();
        (self.alarm.revision(), self.weather.report().revision, minute).hash(&mut hasher);
        hasher.finish()
    }

    fn on_input(&mut self, input: Input) {
        if !matches!(self.alarm.state(), AlarmState::Waiting { .. }) {
            self.mode = Mode::Clock;
        }
        match (self.mode.clone(), input) {
            (Mode::Clock, input) => self.on_clock_input(input),
            (_, Input::HoldYellowAndLong) => {}
            (Mode::Settings { row }, Input::Turn(detents)) => {
                let row = (row as i64 + i64::from(detents)).rem_euclid(ROWS as i64) as usize;
                self.mode = Mode::Settings { row };
            }
            (Mode::Settings { row: RINGTONE_ROW }, Input::Press(Button::Long)) => self.choose_ringtone(),
            (Mode::Settings { row }, Input::Press(Button::Long)) => {
                let day = Weekday::ALL.get(row).copied().unwrap_or(Weekday::Monday);
                let time_to_put_back = self.time_on(day);
                if time_to_put_back.is_none() {
                    self.alarm.set_time_on(day, FIRST_TIME);
                }
                self.mode = Mode::Hour { day, time_to_put_back };
            }
            (Mode::Settings { .. }, Input::Press(Button::Yellow)) => self.mode = Mode::Clock,
            (Mode::Hour { day, .. }, Input::Turn(detents)) => self.turn_hour(day, detents),
            (Mode::Hour { day, time_to_put_back }, Input::Press(Button::Long)) if self.time_on(day).is_some() => {
                self.mode = Mode::Minute { day, time_to_put_back };
            }
            (Mode::Hour { day, .. }, Input::Press(Button::Long)) => self.mode = Self::days_row_of(day),
            (Mode::Minute { day, .. }, Input::Turn(detents)) => self.turn_minute(day, detents),
            (Mode::Minute { day, .. }, Input::Press(Button::Long)) => self.mode = Self::days_row_of(day),
            (Mode::Hour { day, time_to_put_back } | Mode::Minute { day, time_to_put_back }, Input::Press(Button::Yellow)) => {
                self.alarm.set_time_on(day, time_to_put_back);
                self.mode = Self::days_row_of(day);
            }
            (Mode::Ringtone { .. }, Input::Turn(detents)) => self.turn_ringtone(detents),
            (Mode::Ringtone { .. }, Input::Press(Button::Long)) => self.close_ringtone(),
            (Mode::Ringtone { to_put_back }, Input::Press(Button::Yellow)) => {
                self.alarm.set_ringtone(to_put_back);
                self.close_ringtone();
            }
        }
    }

    fn draw(&self, target: &mut D, area: Rectangle) {
        match self.mode {
            Mode::Clock => self.draw_clock(target, area),
            Mode::Settings { .. } | Mode::Hour { .. } | Mode::Minute { .. } | Mode::Ringtone { .. } => self.draw_settings(target, area),
        }
    }
}

impl AlarmScreen {
    fn draw_clock<D: DrawTarget<Color = BinaryColor>>(&self, target: &mut D, area: Rectangle) {
        let now = self.clock.now();
        let top = area.top_left.y;
        let right = area.top_left.x + area.size.width as i32;
        let digits_top = top + BODY.character_size.height as i32 + 2 * GAP;
        let forecast = self.weather.report().forecast;
        let mut date_width = area.size.width;
        if let (Some(forecast), Some(now)) = (&forecast, now) {
            if let Some(today) = forecast.day(now.date) {
                day_weather::draw_range(target, today, Point::new(right, top));
                date_width = date_width.saturating_sub(day_weather::range_width(today) + GAP as u32);
            }
            day_weather::draw_hours(target, forecast.hours_from(now).skip(self.hours_ahead), hours_column(area));
        }
        let date = now.map_or_else(|| "The time is not known yet".into(), |now| long_date(&now));
        text::write(target, &date, area.top_left, date_width, &BODY);
        big_digits::CLOCK.draw_time(target, now.map(|now| now.time_of_day), Point::new(area.top_left.x, digits_top));

        let line_top = digits_top + big_digits::CLOCK.height as i32 + 2 * GAP;
        text::write(target, &self.alarm_line(now), Point::new(area.top_left.x, line_top), big_digits::CLOCK.time_width(), &BODY);
        let hint = match self.alarm.state() {
            AlarmState::Ringing => format!("long: snooze {} min   hold yellow and long: stop", SNOOZE.as_secs() / 60),
            AlarmState::Snoozed { .. } => "hold yellow and long: stop".into(),
            AlarmState::Waiting { .. } => "yellow: alarm on/off   long: settings   wheel: hours".into(),
        };
        write_hint(target, &hint, area);
    }

    fn alarm_line(&self, now: Option<LocalTime>) -> String {
        match self.alarm.state() {
            AlarmState::Ringing => "Good morning!".into(),
            AlarmState::Snoozed { until } => format!("Snoozing until {}", clock_time(until.time_of_day)),
            AlarmState::Waiting { .. } if !self.alarm.schedule().enabled => "Alarm off".into(),
            AlarmState::Waiting { next: Some(next) } => match now {
                Some(now) => format!("Alarm {} at {}", day_from(now, next), clock_time(next.time_of_day)),
                None => format!("Alarm at {}", clock_time(next.time_of_day)),
            },
            AlarmState::Waiting { next: None } if now.is_none() => "Alarm on".into(),
            AlarmState::Waiting { next: None } => "No wake-up time set".into(),
        }
    }

    fn draw_settings<D: DrawTarget<Color = BinaryColor>>(&self, target: &mut D, area: Rectangle) {
        let enabled = if self.alarm.schedule().enabled { "on" } else { "off" };
        text::write(target, &format!("Alarm settings (alarm {enabled})"), area.top_left, area.size.width, &TITLE);
        let advance = (BODY.character_size.width + BODY.character_spacing) as i32;
        let name_left = area.top_left.x + DOT_DIAMETER as i32 + DOT_GAP;
        let value_left = name_left + DAY_NAME_CHARS * advance;
        let value_width = (area.top_left.x + area.size.width as i32 - value_left).max(0) as u32;
        let first_top = area.top_left.y + TITLE.character_size.height as i32 + GAP;
        let row_top = |row: usize| first_top + row as i32 * ROW_PITCH;
        let chosen_row = match self.mode {
            Mode::Settings { row } => Some(row),
            Mode::Hour { day, .. } | Mode::Minute { day, .. } => Some(day.days_since_monday()),
            Mode::Ringtone { .. } => Some(RINGTONE_ROW),
            Mode::Clock => None,
        };
        if let Some(row) = chosen_row {
            let dot_top = row_top(row) + (BODY.character_size.height as i32 - DOT_DIAMETER as i32) / 2;
            let _ = Circle::new(Point::new(area.top_left.x, dot_top), DOT_DIAMETER)
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(target);
        }
        for (n, &day) in Weekday::ALL.iter().enumerate() {
            text::write(target, calendar_names::weekday(day), Point::new(name_left, row_top(n)), (DAY_NAME_CHARS * advance) as u32, &BODY);
            let field = match self.mode {
                Mode::Hour { day: editing, .. } if editing == day => Some(Field::Hour),
                Mode::Minute { day: editing, .. } if editing == day => Some(Field::Minute),
                _ => None,
            };
            text::write(target, &wake_up_time(self.time_on(day), field), Point::new(value_left, row_top(n)), value_width, &BODY);
        }
        let top = row_top(RINGTONE_ROW);
        text::write(target, "Ringtone", Point::new(name_left, top), (DAY_NAME_CHARS * advance) as u32, &BODY);
        let ringtone = self.alarm.ringtone();
        let ringtone = match self.mode {
            Mode::Ringtone { .. } => format!("[{}]", ringtone_name(&ringtone)),
            _ => ringtone_name(&ringtone).into(),
        };
        text::write(target, &ringtone, Point::new(value_left, top), value_width, &BODY);
        let hint = match self.mode {
            Mode::Hour { .. } => "wheel: hour, past 23 is off   long: minutes   yellow: cancel",
            Mode::Minute { .. } => "wheel: minutes   long: done   yellow: cancel",
            Mode::Ringtone { .. } => "wheel: ringtone, playing softly   long: done   yellow: cancel",
            Mode::Settings { .. } | Mode::Clock => "wheel: day or ringtone   long: set   yellow: back",
        };
        write_hint(target, hint, area);
    }
}

fn hours_column(area: Rectangle) -> Rectangle {
    let right = area.top_left.x + area.size.width as i32;
    let room_beside_time = right - area.top_left.x - big_digits::CLOCK.time_width() as i32;
    let left = right - (room_beside_time + day_weather::HOURS_WIDTH as i32) / 2;
    let below_date = area.top_left.y + BODY.character_size.height as i32;
    let top = below_date + (hint_top(area) - below_date - day_weather::HOURS_HEIGHT as i32) / 2;
    Rectangle::new(Point::new(left, top), Size::new(day_weather::HOURS_WIDTH, day_weather::HOURS_HEIGHT))
}

fn hint_top(area: Rectangle) -> i32 {
    area.top_left.y + area.size.height as i32 - HINT.character_size.height as i32
}

fn write_hint<D: DrawTarget<Color = BinaryColor>>(target: &mut D, hint: &str, area: Rectangle) {
    text::write(target, hint, Point::new(area.top_left.x, hint_top(area)), area.size.width, &HINT);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    Hour,
    Minute,
}

fn wake_up_time(time: Option<TimeOfDay>, field: Option<Field>) -> String {
    match (time, field) {
        (None, Some(Field::Hour)) => "[off]".into(),
        (None, _) => "off".into(),
        (Some(time), Some(Field::Hour)) => format!("[{:02}]:{:02}", time.hour(), time.minute()),
        (Some(time), Some(Field::Minute)) => format!("{:02}:[{:02}]", time.hour(), time.minute()),
        (Some(time), None) => clock_time(time),
    }
}

fn ringtone_name(ringtone: &Ringtone) -> &str {
    match ringtone {
        Ringtone::Chime => "Chime",
        Ringtone::Recorded(name) => name.rsplit_once('.').map_or(name.as_str(), |(stem, _)| stem),
    }
}

fn clock_time(time: TimeOfDay) -> String {
    format!("{:02}:{:02}", time.hour(), time.minute())
}

fn long_date(now: &LocalTime) -> String {
    let date = now.date;
    let month = calendar_names::month(date.month()).unwrap_or_default();
    format!("{} {} {month}", calendar_names::weekday(date.weekday()), date.day())
}

fn day_from(now: LocalTime, then: LocalTime) -> String {
    match then.date.days_since_epoch() - now.date.days_since_epoch() {
        0 => "today".into(),
        1 => "tomorrow".into(),
        _ => calendar_names::weekday(then.date.weekday()).into(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use domain::apps::Foreground;
    use domain::calendar::Date;
    use domain::clock::TimeKeeper;
    use domain::fetch::Unavailable;
    use domain::place::Place;
    use domain::time::UtcTime;
    use domain_testing::place::{paris, StubPlace};
    use domain_testing::time::FakeTimeKeeper;
    use forecast::{
        CompassPoint, DayForecast, Degrees, Forecast, ForecastSource, Hectopascals, HourForecast, KilometresPerHour, Millimetres, Percent, Sky,
        Today, Wind,
    };
    use hal::display::{Frame, HEIGHT, VISIBLE_WIDTH, WIDTH};

    use super::*;
    use crate::domain::alarm_clock::{AlarmSettings, AlarmSettingsStore, Ringer, Volume};

    struct StubAlarmSettingsStore;
    impl AlarmSettingsStore for StubAlarmSettingsStore {
        fn load(&mut self) -> Option<AlarmSettings> {
            None
        }
        fn save(&mut self, _: &AlarmSettings) {}
    }

    #[derive(Clone, Default)]
    struct StubRinger(Arc<Mutex<Option<Ringtone>>>);
    impl Ringer for StubRinger {
        fn recordings(&self) -> Vec<String> {
            vec!["Zen.mp3".into(), "Lost Ark.mp3".into()]
        }
        fn ring(&mut self, ringtone: &Ringtone, _: Volume) {
            *self.0.lock().unwrap() = Some(ringtone.clone());
        }
        fn silence(&mut self) {
            *self.0.lock().unwrap() = None;
        }
    }

    impl StubRinger {
        fn playing(&self) -> Option<Ringtone> {
            self.0.lock().unwrap().clone()
        }
    }

    fn zen() -> Ringtone {
        Ringtone::Recorded("Zen.mp3".into())
    }

    fn lost_ark() -> Ringtone {
        Ringtone::Recorded("Lost Ark.mp3".into())
    }

    struct StubForecastSource;
    impl ForecastSource for StubForecastSource {
        fn fetch(&mut self, _: &Place) -> Result<Forecast, Unavailable> {
            let saturday = Date::new(2026, 9, 26).unwrap();
            let six = LocalTime { date: saturday, time_of_day: TimeOfDay::new(6, 0).unwrap(), second: 0 }.seconds_since_epoch();
            let hours = (0..24)
                .map(|n| HourForecast {
                    start: LocalTime::from_seconds_since_epoch(six + n * 3600),
                    sky: Sky::Rain,
                    temperature: Degrees(12),
                    rain_chance: Percent::new(80),
                    precipitation: Some(Millimetres::from_tenths(12)),
                })
                .collect();
            let week = vec![DayForecast {
                date: saturday,
                sky: Sky::Rain,
                low: Degrees(9),
                high: Degrees(14),
                rain_chance: Percent::new(90),
                sunrise: TimeOfDay::new(7, 41),
                sunset: TimeOfDay::new(19, 41),
            }];
            let today = Today {
                sky: Sky::Rain,
                now: Degrees(11),
                feels_like: Degrees(9),
                humidity: Percent::saturating(92),
                pressure: Hectopascals(1004),
                wind: Wind { speed: KilometresPerHour(24), from: CompassPoint::West },
            };
            Ok(Forecast { today, hours, week })
        }
    }

    /// Saturday 26 September 2026, 07:00 UTC.
    fn saturday_at_seven() -> UtcTime {
        UtcTime::from_unix_seconds(Date::new(2026, 9, 26).unwrap().days_since_epoch() * 86_400 + 7 * 3600)
    }

    struct Bench {
        screen: AlarmScreen,
        alarm: AlarmClock,
        clock: Clock,
        weather: Weather,
        keeper: FakeTimeKeeper,
        ringer: StubRinger,
    }

    impl Bench {
        fn new() -> Self {
            let keeper = FakeTimeKeeper::default();
            let clock = keeper.offline_clock();
            let ringer = StubRinger::default();
            let alarm = AlarmClock::new(Box::new(StubAlarmSettingsStore), Box::new(ringer.clone()), Foreground::new(crate::ID));
            let weather = Weather::new(Box::new(StubPlace::at(paris())), Box::new(StubForecastSource));
            let mut screen = AlarmScreen::new(alarm.clone(), clock.clone(), weather.clone());
            AppScreen::<Frame>::entered(&mut screen);
            let bench = Self { screen, alarm, clock, weather, keeper, ringer };
            bench.at(saturday_at_seven());
            bench
        }

        fn at(&self, time: UtcTime) {
            self.keeper.clone().set(time);
            self.clock.tick(std::time::Instant::now());
            if let Some(now) = self.clock.now() {
                self.alarm.tick(now);
            }
        }

        fn input(&mut self, input: Input) {
            AppScreen::<Frame>::on_input(&mut self.screen, input);
        }

        fn press(&mut self, control: Button) {
            self.input(Input::Press(control));
        }

        fn render(&self) -> Frame {
            let mut frame = Frame::blank();
            let visible = Rectangle::new(Point::zero(), Size::new(VISIBLE_WIDTH.into(), HEIGHT.into()));
            AppScreen::draw(&self.screen, &mut frame, visible.offset(-8));
            frame
        }
    }

    #[test]
    fn the_yellow_button_turns_the_alarm_on_and_off() {
        let mut bench = Bench::new();
        assert!(!bench.alarm.schedule().enabled);
        bench.press(Button::Yellow);
        assert!(bench.alarm.schedule().enabled);
        bench.press(Button::Yellow);
        assert!(!bench.alarm.schedule().enabled);
    }

    #[test]
    fn a_day_is_set_hour_then_minutes_starting_from_today() {
        let mut bench = Bench::new();
        bench.press(Button::Long);
        assert_eq!(bench.screen.mode, Mode::Settings { row: 5 }, "Saturday");
        bench.input(Input::Turn(1));
        bench.press(Button::Long);
        assert_eq!(bench.alarm.schedule().time_on(Weekday::Sunday), TimeOfDay::new(7, 0));
        bench.input(Input::Turn(2));
        bench.press(Button::Long);
        bench.input(Input::Turn(-13));
        bench.press(Button::Long);
        assert_eq!(bench.alarm.schedule().time_on(Weekday::Sunday), TimeOfDay::new(9, 47));
        assert_eq!(bench.screen.mode, Mode::Settings { row: 6 });
        bench.press(Button::Yellow);
        assert_eq!(bench.screen.mode, Mode::Clock);
        assert!(!bench.alarm.schedule().enabled, "going back is not switching on or off");
    }

    #[test]
    fn going_back_while_setting_a_day_puts_its_time_back() {
        let mut bench = Bench::new();
        bench.alarm.set_time_on(Weekday::Saturday, TimeOfDay::new(6, 30));
        bench.press(Button::Long);
        bench.press(Button::Long);
        bench.input(Input::Turn(3));
        bench.press(Button::Long);
        bench.input(Input::Turn(10));
        bench.press(Button::Yellow);
        assert_eq!(bench.alarm.schedule().time_on(Weekday::Saturday), TimeOfDay::new(6, 30));
        assert_eq!(bench.screen.mode, Mode::Settings { row: 5 });

        bench.input(Input::Turn(1));
        bench.press(Button::Long);
        bench.press(Button::Yellow);
        assert_eq!(bench.alarm.schedule().time_on(Weekday::Sunday), None, "a day that had none has none again");
    }

    #[test]
    fn turning_the_hour_past_either_end_takes_the_day_off() {
        let mut bench = Bench::new();
        bench.press(Button::Long);
        bench.press(Button::Long);
        bench.input(Input::Turn(-8));
        assert_eq!(bench.alarm.schedule().time_on(Weekday::Saturday), None);
        bench.input(Input::Turn(-1));
        assert_eq!(bench.alarm.schedule().time_on(Weekday::Saturday), TimeOfDay::new(23, 0));
        bench.input(Input::Turn(1));
        bench.press(Button::Long);
        assert_eq!(bench.screen.mode, Mode::Settings { row: 5 }, "an off day has no minutes to set");
    }

    #[test]
    fn the_wheel_goes_round_the_ringtones_each_playing_softly_until_done() {
        let mut bench = Bench::new();
        bench.press(Button::Long);
        bench.input(Input::Turn(2));
        assert_eq!(bench.screen.mode, Mode::Settings { row: RINGTONE_ROW }, "after Sunday");
        bench.press(Button::Long);
        assert_eq!(bench.ringer.playing(), Some(Ringtone::Chime));
        bench.input(Input::Turn(-1));
        assert_eq!((bench.alarm.ringtone(), bench.ringer.playing()), (lost_ark(), Some(lost_ark())));
        bench.input(Input::Turn(2));
        assert_eq!(bench.alarm.ringtone(), zen());
        bench.press(Button::Long);
        assert_eq!((bench.alarm.ringtone(), bench.ringer.playing()), (zen(), None));
        assert_eq!(bench.screen.mode, Mode::Settings { row: RINGTONE_ROW });
    }

    #[test]
    fn going_back_while_choosing_a_ringtone_puts_it_back() {
        let mut bench = Bench::new();
        bench.press(Button::Long);
        bench.input(Input::Turn(2));
        bench.press(Button::Long);
        bench.input(Input::Turn(1));
        bench.press(Button::Yellow);
        assert_eq!((bench.alarm.ringtone(), bench.ringer.playing()), (Ringtone::Chime, None));
    }

    #[test]
    fn leaving_the_app_while_choosing_a_ringtone_stops_it() {
        let mut bench = Bench::new();
        bench.press(Button::Long);
        bench.input(Input::Turn(2));
        bench.press(Button::Long);
        bench.input(Input::Turn(1));
        AppScreen::<Frame>::left(&mut bench.screen);
        assert_eq!((bench.alarm.ringtone(), bench.ringer.playing()), (zen(), None));
    }

    #[test]
    fn a_ringtone_is_named_without_its_extension() {
        assert_eq!(ringtone_name(&lost_ark()), "Lost Ark");
        assert_eq!(ringtone_name(&Ringtone::Chime), "Chime");
    }

    #[test]
    fn ringing_the_long_button_snoozes_and_holding_both_stops() {
        let mut bench = Bench::new();
        bench.alarm.set_time_on(Weekday::Saturday, TimeOfDay::new(7, 1));
        bench.alarm.switch_on();
        bench.at(UtcTime::from_unix_seconds(saturday_at_seven().unix_seconds() + 60));
        assert_eq!(bench.alarm.state(), AlarmState::Ringing);
        bench.press(Button::Yellow);
        bench.input(Input::Turn(3));
        assert_eq!(bench.alarm.state(), AlarmState::Ringing, "the yellow button and the wheel do nothing");
        assert!(bench.alarm.schedule().enabled);
        bench.press(Button::Long);
        assert!(matches!(bench.alarm.state(), AlarmState::Snoozed { .. }));
        bench.press(Button::Long);
        bench.press(Button::Yellow);
        assert!(matches!(bench.alarm.state(), AlarmState::Snoozed { .. }), "snoozing, only holding both stops");
        bench.input(Input::HoldYellowAndLong);
        assert!(matches!(bench.alarm.state(), AlarmState::Waiting { .. }));
        assert_eq!(bench.screen.mode, Mode::Clock);
    }

    #[test]
    fn the_screen_changes_with_the_minute_and_the_schedule() {
        let bench = Bench::new();
        let version = || AppScreen::<Frame>::version(&bench.screen);
        let before = version();
        bench.at(UtcTime::from_unix_seconds(saturday_at_seven().unix_seconds() + 30));
        assert_eq!(version(), before);
        bench.at(UtcTime::from_unix_seconds(saturday_at_seven().unix_seconds() + 60));
        let a_minute_later = version();
        assert_ne!(a_minute_later, before);
        bench.alarm.switch_on();
        assert_ne!(version(), a_minute_later);
    }

    #[test]
    fn the_weather_shows_once_forecast_and_the_screen_changes_with_it() {
        let bench = Bench::new();
        let (version, without) = (AppScreen::<Frame>::version(&bench.screen), bench.render());
        bench.weather.refresh_if_due(std::time::Instant::now());
        assert_ne!(AppScreen::<Frame>::version(&bench.screen), version);
        assert!(bench.render() != without);
    }

    #[test]
    fn the_wheel_moves_through_the_hours_and_stops_at_either_end() {
        let mut bench = Bench::new();
        bench.input(Input::Turn(3));
        assert_eq!(bench.screen.hours_ahead, 0, "no forecast, nothing to move through");
        bench.weather.refresh_if_due(std::time::Instant::now());
        let first_hours = bench.render();
        bench.input(Input::Turn(3));
        assert_eq!(bench.screen.hours_ahead, 3);
        assert!(bench.render() != first_hours);
        bench.input(Input::Turn(-10));
        assert_eq!(bench.screen.hours_ahead, 0);
        bench.input(Input::Turn(100));
        assert_eq!(bench.screen.hours_ahead, 23 - day_weather::HOURS_SHOWN, "the last hours still fill the column");
        AppScreen::<Frame>::entered(&mut bench.screen);
        assert_eq!(bench.screen.hours_ahead, 0, "back to the hour under way");
    }

    #[test]
    fn the_next_alarm_is_named_by_its_day() {
        let bench = Bench::new();
        bench.alarm.set_time_on(Weekday::Sunday, TimeOfDay::new(8, 30));
        assert_eq!(bench.screen.alarm_line(bench.clock.now()), "Alarm off");
        bench.alarm.switch_on();
        assert_eq!(bench.screen.alarm_line(bench.clock.now()), "Alarm tomorrow at 08:30");
        bench.alarm.set_time_on(Weekday::Sunday, None);
        bench.alarm.set_time_on(Weekday::Tuesday, TimeOfDay::new(6, 5));
        assert_eq!(bench.screen.alarm_line(bench.clock.now()), "Alarm Tuesday at 06:05");
    }

    #[test]
    fn every_mode_draws_within_the_glass() {
        let mut bench = Bench::new();
        bench.weather.refresh_if_due(std::time::Instant::now());
        let mut frames = vec![bench.render()];
        for _ in 0..3 {
            bench.press(Button::Long);
            frames.push(bench.render());
        }
        bench.press(Button::Long);
        bench.input(Input::Turn(2));
        bench.press(Button::Long);
        frames.push(bench.render());
        for (n, frame) in frames.iter().enumerate() {
            assert!(frames.iter().skip(n + 1).all(|other| other != frame), "mode {n} looks like a later one");
            for x in i32::from(VISIBLE_WIDTH)..i32::from(WIDTH) {
                for y in 0..i32::from(HEIGHT) {
                    assert!(!frame.is_ink(x, y), "ink at ({x},{y})");
                }
            }
        }
    }
}
