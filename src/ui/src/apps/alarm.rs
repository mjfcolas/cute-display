//! The alarm clock: the time, large, today's weather beside it and the next alarm; the
//! wake-up time of each day, set with the wheel.

use std::hash::{DefaultHasher, Hash, Hasher};

use domain::alarm::{AlarmClock, AlarmState, SNOOZE};
use domain::apps::App;
use domain::calendar::Weekday;
use domain::clock::Clock;
use domain::time::{LocalTime, TimeOfDay};
use domain::weather::Weather;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, PrimitiveStyle, Rectangle};

use crate::app_screen::AppScreen;
use crate::big_digits;
use crate::calendar_names;
use crate::controls::{Button, Input};
use crate::day_weather;
use crate::text::{self, BODY, HINT, TITLE};

/// What a day with no alarm starts at when it is given one.
const FIRST_TIME: Option<TimeOfDay> = TimeOfDay::new(7, 0);
const ROW_PITCH: i32 = 24;
const DOT_DIAMETER: u32 = 8;
const DOT_GAP: i32 = 10;
/// "Wednesday" and a space.
const DAY_NAME_CHARS: i32 = 10;
const GAP: i32 = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Clock,
    Days { row: usize },
    Hour { day: Weekday, time_to_put_back: Option<TimeOfDay> },
    Minute { day: Weekday, time_to_put_back: Option<TimeOfDay> },
}

pub struct AlarmScreen {
    alarm: AlarmClock,
    clock: Clock,
    weather: Weather,
    mode: Mode,
    /// How many hours after the one under way the weather column starts.
    hours_ahead: usize,
}

impl AlarmScreen {
    pub fn new(alarm: AlarmClock, clock: Clock, weather: Weather) -> Self {
        Self { alarm, clock, weather, mode: Mode::Clock, hours_ahead: 0 }
    }

    fn time_on(&self, day: Weekday) -> Option<TimeOfDay> {
        self.alarm.schedule().time_on(day)
    }

    fn days_row_of(day: Weekday) -> Mode {
        Mode::Days { row: day.days_since_monday() }
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

    /// Stops with the last hours of the forecast in the column.
    fn scroll_hours(&mut self, detents: i32) {
        let known = match (self.weather.report().forecast, self.clock.now()) {
            (Some(forecast), Some(now)) => forecast.hours_from(now).count(),
            _ => 0,
        };
        let last = known.saturating_sub(day_weather::HOURS_SHOWN) as i64;
        let ahead = self.hours_ahead as i64 + i64::from(detents);
        self.hours_ahead = usize::try_from(ahead.clamp(0, last)).unwrap_or(0);
    }

    /// Turning the hour past 23 or below 0 takes the alarm off that day.
    fn turn_hour(&self, day: Weekday, detents: i32) {
        let time = self.time_on(day);
        let position = time.map_or(0, |t| i32::from(t.hour()) + 1);
        let turned = (position + detents).rem_euclid(25);
        let minute = time.map_or(0, TimeOfDay::minute);
        let hour = u8::try_from(turned - 1).ok();
        self.alarm.set_time_on(day, hour.and_then(|hour| TimeOfDay::new(hour, minute)));
    }

    fn turn_minute(&self, day: Weekday, detents: i32) {
        if let Some(time) = self.time_on(day) {
            let minute = (i32::from(time.minute()) + detents).rem_euclid(60);
            self.alarm.set_time_on(day, u8::try_from(minute).ok().and_then(|minute| TimeOfDay::new(time.hour(), minute)));
        }
    }
}

impl<D: DrawTarget<Color = BinaryColor>> AppScreen<D> for AlarmScreen {
    fn app(&self) -> App {
        App::Alarm
    }

    fn entered(&mut self) {
        self.mode = Mode::Clock;
        self.hours_ahead = 0;
    }

    /// The alarm's and the weather's revisions, and the minute on the clock.
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
        match (self.mode, input) {
            (Mode::Clock, input) => self.on_clock_input(input),
            (_, Input::HoldYellowAndLong) => {}
            (Mode::Days { row }, Input::Turn(detents)) => {
                let row = (row as i64 + i64::from(detents)).rem_euclid(Weekday::ALL.len() as i64) as usize;
                self.mode = Mode::Days { row };
            }
            (Mode::Days { row }, Input::Press(Button::Long)) => {
                let day = Weekday::ALL.get(row).copied().unwrap_or(Weekday::Monday);
                let time_to_put_back = self.time_on(day);
                if time_to_put_back.is_none() {
                    self.alarm.set_time_on(day, FIRST_TIME);
                }
                self.mode = Mode::Hour { day, time_to_put_back };
            }
            (Mode::Days { .. }, Input::Press(Button::Yellow)) => self.mode = Mode::Clock,
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
        }
    }

    fn draw(&self, target: &mut D, area: Rectangle) {
        match self.mode {
            Mode::Clock => self.draw_clock(target, area),
            Mode::Days { .. } | Mode::Hour { .. } | Mode::Minute { .. } => self.draw_days(target, area),
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
            AlarmState::Waiting { .. } => "yellow: alarm on/off   long: wake-up times   wheel: hours".into(),
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

    fn draw_days<D: DrawTarget<Color = BinaryColor>>(&self, target: &mut D, area: Rectangle) {
        let enabled = if self.alarm.schedule().enabled { "on" } else { "off" };
        text::write(target, &format!("Wake-up times (alarm {enabled})"), area.top_left, area.size.width, &TITLE);
        let advance = (BODY.character_size.width + BODY.character_spacing) as i32;
        let name_left = area.top_left.x + DOT_DIAMETER as i32 + DOT_GAP;
        let time_left = name_left + DAY_NAME_CHARS * advance;
        let first_top = area.top_left.y + TITLE.character_size.height as i32 + GAP;
        for (n, &day) in Weekday::ALL.iter().enumerate() {
            let top = first_top + n as i32 * ROW_PITCH;
            let chosen = match self.mode {
                Mode::Days { row } => row == n,
                Mode::Hour { day: editing, .. } | Mode::Minute { day: editing, .. } => editing == day,
                Mode::Clock => false,
            };
            if chosen {
                let dot_top = top + (BODY.character_size.height as i32 - DOT_DIAMETER as i32) / 2;
                let _ = Circle::new(Point::new(area.top_left.x, dot_top), DOT_DIAMETER)
                    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                    .draw(target);
            }
            text::write(target, calendar_names::weekday(day), Point::new(name_left, top), (DAY_NAME_CHARS * advance) as u32, &BODY);
            let field = match self.mode {
                Mode::Hour { day: editing, .. } if editing == day => Some(Field::Hour),
                Mode::Minute { day: editing, .. } if editing == day => Some(Field::Minute),
                _ => None,
            };
            let time = wake_up_time(self.time_on(day), field);
            let width = (area.top_left.x + area.size.width as i32 - time_left).max(0) as u32;
            text::write(target, &time, Point::new(time_left, top), width, &BODY);
        }
        let hint = match self.mode {
            Mode::Hour { .. } => "wheel: hour, past 23 is off   long: minutes   yellow: cancel",
            Mode::Minute { .. } => "wheel: minutes   long: done   yellow: cancel",
            Mode::Days { .. } | Mode::Clock => "wheel: day   long: set   yellow: back",
        };
        write_hint(target, hint, area);
    }
}

/// Centred in the room right of the time, between the date and the hint.
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

/// The part of a wake-up time the wheel sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    Hour,
    Minute,
}

/// The field being set is in brackets.
fn wake_up_time(time: Option<TimeOfDay>, field: Option<Field>) -> String {
    match (time, field) {
        (None, Some(Field::Hour)) => "[off]".into(),
        (None, _) => "off".into(),
        (Some(time), Some(Field::Hour)) => format!("[{:02}]:{:02}", time.hour(), time.minute()),
        (Some(time), Some(Field::Minute)) => format!("{:02}:[{:02}]", time.hour(), time.minute()),
        (Some(time), None) => clock_time(time),
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

/// "today", "tomorrow", or the day's name within the week.
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

    use domain::alarm::{AlarmSchedule, AlarmScheduleStore, Ringer, Volume};
    use domain::apps::Foreground;
    use domain::calendar::Date;
    use domain::clock::{TimeKeeper, TimeSource, TimeZoneSource};
    use domain::fetch::Unavailable;
    use domain::place::{GeoPoint, Place, PlaceSource};
    use domain::time::UtcTime;
    use domain::weather::{
        CompassPoint, DayForecast, Degrees, Forecast, ForecastSource, Hectopascals, HourForecast, KilometresPerHour, Millimetres, Percent, Sky,
        Today, Wind,
    };
    use domain::time_zone::TimeZone;
    use hal::display::{Frame, HEIGHT, VISIBLE_WIDTH, WIDTH};

    use super::*;

    struct Nowhere;
    impl AlarmScheduleStore for Nowhere {
        fn load(&mut self) -> Option<AlarmSchedule> {
            None
        }
        fn save(&mut self, _: &AlarmSchedule) {}
    }

    struct Mute;
    impl Ringer for Mute {
        fn ring(&mut self, _: Volume) {}
        fn silence(&mut self) {}
    }

    /// Stands still at the time it is given.
    #[derive(Clone, Default)]
    struct StoppedKeeper(Arc<Mutex<Option<UtcTime>>>);
    impl TimeKeeper for StoppedKeeper {
        fn read(&mut self) -> Option<UtcTime> {
            *self.0.lock().unwrap()
        }
        fn set(&mut self, time: UtcTime) {
            *self.0.lock().unwrap() = Some(time);
        }
    }

    struct Offline;
    impl TimeSource for Offline {
        fn fetch(&mut self) -> Result<UtcTime, Unavailable> {
            Err(Unavailable("offline".into()))
        }
    }

    struct Utc;
    impl TimeZoneSource for Utc {
        fn time_zone(&mut self) -> Result<Option<TimeZone>, Unavailable> {
            Ok(Some(TimeZone::UTC))
        }
    }

    struct Paris;
    impl PlaceSource for Paris {
        fn place(&mut self) -> Option<Place> {
            Some(Place { name: "Paris".into(), point: GeoPoint { latitude: 48.85, longitude: 2.35 } })
        }
    }

    /// Saturday's, from 06:00, rain all day.
    struct Rainy;
    impl ForecastSource for Rainy {
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
        keeper: StoppedKeeper,
    }

    impl Bench {
        fn new() -> Self {
            let keeper = StoppedKeeper::default();
            let clock = Clock::new(Box::new(keeper.clone()), Box::new(Offline), Box::new(Utc));
            let alarm = AlarmClock::new(Box::new(Nowhere), Box::new(Mute), Foreground::new(App::Alarm));
            let weather = Weather::new(Box::new(Paris), Box::new(Rainy));
            let mut screen = AlarmScreen::new(alarm.clone(), clock.clone(), weather.clone());
            AppScreen::<Frame>::entered(&mut screen);
            let bench = Self { screen, alarm, clock, weather, keeper };
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
        assert_eq!(bench.screen.mode, Mode::Days { row: 5 }, "Saturday");
        bench.input(Input::Turn(1));
        bench.press(Button::Long);
        assert_eq!(bench.alarm.schedule().time_on(Weekday::Sunday), TimeOfDay::new(7, 0));
        bench.input(Input::Turn(2));
        bench.press(Button::Long);
        bench.input(Input::Turn(-13));
        bench.press(Button::Long);
        assert_eq!(bench.alarm.schedule().time_on(Weekday::Sunday), TimeOfDay::new(9, 47));
        assert_eq!(bench.screen.mode, Mode::Days { row: 6 });
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
        assert_eq!(bench.screen.mode, Mode::Days { row: 5 });

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
        assert_eq!(bench.screen.mode, Mode::Days { row: 5 }, "an off day has no minutes to set");
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
