mod drawing;
mod ui_state;

use std::hash::{DefaultHasher, Hash, Hasher};

use domain::calendar::Weekday;
use domain::clock::Clock;
use domain::time::{LocalTime, TimeOfDay};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use forecast::day_weather;
use forecast::Weather;
use ui::calendar_names;
use ui::controls::{Button, Input};
use ui::mark::Mark;
use ui::AppScreen;

pub use ui_state::{AlarmUiState, ClockPage, SettingRow, SettingsPage};

use crate::domain::alarm_clock::{AlarmClock, AlarmState, Ringtone, SNOOZE};

const FIRST_TIME: Option<TimeOfDay> = TimeOfDay::new(7, 0);
const RINGTONE_ROW: usize = Weekday::ALL.len();
const ROWS: usize = RINGTONE_ROW + 1;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Mode {
    Clock,
    Settings(SettingsStep),
}

/// Where the settings page is: choosing a row, or setting what is on it.
#[derive(Clone, Debug, PartialEq, Eq)]
enum SettingsStep {
    Choosing { row: usize },
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

    pub fn ui_state(&self) -> AlarmUiState {
        match &self.mode {
            Mode::Clock => AlarmUiState::Clock(self.clock_page()),
            Mode::Settings(step) => AlarmUiState::Settings(Box::new(self.settings_page(step))),
        }
    }

    fn clock_page(&self) -> ClockPage {
        let now = self.clock.now();
        let forecast = self.weather.report().forecast;
        let (today, hours) = match (&forecast, now) {
            (Some(forecast), Some(now)) => (
                forecast.day(now.date).cloned(),
                forecast.hours_from(now).skip(self.hours_ahead).take(day_weather::HOURS_SHOWN).cloned().collect(),
            ),
            _ => (None, Vec::new()),
        };
        let hint = match self.alarm.state() {
            AlarmState::Ringing => format!("long: snooze {} min   hold yellow and long: stop", SNOOZE.as_secs() / 60),
            AlarmState::Snoozed { .. } => "hold yellow and long: stop".into(),
            AlarmState::Waiting { .. } => "yellow: alarm on/off   long: settings   wheel: hours".into(),
        };
        ClockPage {
            date: now.map_or_else(|| "The time is not known yet".into(), |now| long_date(&now)),
            time: now.map(|now| now.time_of_day),
            today,
            hours,
            alarm: self.alarm_line(now),
            hint,
        }
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

    fn settings_page(&self, step: &SettingsStep) -> SettingsPage {
        let enabled = if self.alarm.schedule().enabled { "on" } else { "off" };
        let chosen_row = match step {
            SettingsStep::Choosing { row } => *row,
            SettingsStep::Hour { day, .. } | SettingsStep::Minute { day, .. } => day.days_since_monday(),
            SettingsStep::Ringtone { .. } => RINGTONE_ROW,
        };
        let mark = |row: usize| if row == chosen_row { Mark::Chosen } else { Mark::Plain };
        let days = Weekday::ALL.map(|day| {
            let field = match step {
                SettingsStep::Hour { day: editing, .. } if *editing == day => Some(Field::Hour),
                SettingsStep::Minute { day: editing, .. } if *editing == day => Some(Field::Minute),
                _ => None,
            };
            SettingRow { name: calendar_names::weekday(day), value: wake_up_time(self.time_on(day), field), mark: mark(day.days_since_monday()) }
        });
        let ringtone = self.alarm.ringtone();
        let ringtone = match step {
            SettingsStep::Ringtone { .. } => format!("[{}]", ringtone_name(&ringtone)),
            _ => ringtone_name(&ringtone).into(),
        };
        let hint = match step {
            SettingsStep::Choosing { .. } => "wheel: day or ringtone   long: set   yellow: back",
            SettingsStep::Hour { .. } => "wheel: hour, past 23 is off   long: minutes   yellow: cancel",
            SettingsStep::Minute { .. } => "wheel: minutes   long: done   yellow: cancel",
            SettingsStep::Ringtone { .. } => "wheel: ringtone, playing softly   long: done   yellow: cancel",
        };
        SettingsPage {
            title: format!("Alarm settings (alarm {enabled})"),
            days,
            ringtone: SettingRow { name: "Ringtone", value: ringtone, mark: mark(RINGTONE_ROW) },
            hint,
        }
    }

    fn time_on(&self, day: Weekday) -> Option<TimeOfDay> {
        self.alarm.schedule().time_on(day)
    }

    fn choosing(row: usize) -> Mode {
        Mode::Settings(SettingsStep::Choosing { row })
    }

    fn days_row_of(day: Weekday) -> Mode {
        Self::choosing(day.days_since_monday())
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

    fn on_settings_input(&mut self, step: SettingsStep, input: Input) {
        match (step, input) {
            (_, Input::HoldYellowAndLong) => {}
            (SettingsStep::Choosing { row }, Input::Turn(detents)) => {
                self.mode = Self::choosing((row as i64 + i64::from(detents)).rem_euclid(ROWS as i64) as usize);
            }
            (SettingsStep::Choosing { row: RINGTONE_ROW }, Input::Press(Button::Long)) => self.choose_ringtone(),
            (SettingsStep::Choosing { row }, Input::Press(Button::Long)) => {
                let day = Weekday::ALL.get(row).copied().unwrap_or(Weekday::Monday);
                let time_to_put_back = self.time_on(day);
                if time_to_put_back.is_none() {
                    self.alarm.set_time_on(day, FIRST_TIME);
                }
                self.mode = Mode::Settings(SettingsStep::Hour { day, time_to_put_back });
            }
            (SettingsStep::Choosing { .. }, Input::Press(Button::Yellow)) => self.mode = Mode::Clock,
            (SettingsStep::Hour { day, .. }, Input::Turn(detents)) => self.turn_hour(day, detents),
            (SettingsStep::Hour { day, time_to_put_back }, Input::Press(Button::Long)) if self.time_on(day).is_some() => {
                self.mode = Mode::Settings(SettingsStep::Minute { day, time_to_put_back });
            }
            (SettingsStep::Hour { day, .. }, Input::Press(Button::Long)) => self.mode = Self::days_row_of(day),
            (SettingsStep::Minute { day, .. }, Input::Turn(detents)) => self.turn_minute(day, detents),
            (SettingsStep::Minute { day, .. }, Input::Press(Button::Long)) => self.mode = Self::days_row_of(day),
            (
                SettingsStep::Hour { day, time_to_put_back } | SettingsStep::Minute { day, time_to_put_back },
                Input::Press(Button::Yellow),
            ) => {
                self.alarm.set_time_on(day, time_to_put_back);
                self.mode = Self::days_row_of(day);
            }
            (SettingsStep::Ringtone { .. }, Input::Turn(detents)) => self.turn_ringtone(detents),
            (SettingsStep::Ringtone { .. }, Input::Press(Button::Long)) => self.close_ringtone(),
            (SettingsStep::Ringtone { to_put_back }, Input::Press(Button::Yellow)) => {
                self.alarm.set_ringtone(to_put_back);
                self.close_ringtone();
            }
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
        self.mode = Mode::Settings(SettingsStep::Ringtone { to_put_back: self.alarm.ringtone() });
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
        self.mode = Self::choosing(RINGTONE_ROW);
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
        if let Mode::Settings(SettingsStep::Ringtone { .. }) = self.mode {
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
        match self.mode.clone() {
            Mode::Clock => self.on_clock_input(input),
            Mode::Settings(step) => self.on_settings_input(step, input),
        }
    }

    fn draw(&self, target: &mut D, area: Rectangle) {
        drawing::draw(&self.ui_state(), target, area);
    }
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
    use hal::display::Frame;

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

        fn shows_clock(&self) -> bool {
            matches!(self.screen.ui_state(), AlarmUiState::Clock(_))
        }

        fn clock_page(&self) -> ClockPage {
            match self.screen.ui_state() {
                AlarmUiState::Clock(page) => page,
                AlarmUiState::Settings(page) => panic!("the settings are shown: {page:?}"),
            }
        }

        fn settings_page(&self) -> SettingsPage {
            match self.screen.ui_state() {
                AlarmUiState::Settings(page) => *page,
                AlarmUiState::Clock(page) => panic!("the clock is shown: {page:?}"),
            }
        }

        fn chosen_row(&self) -> &'static str {
            let chosen: Vec<&'static str> = self.setting_rows().filter(|row| row.mark == Mark::Chosen).map(|row| row.name).collect();
            let [row] = chosen[..] else { panic!("rows chosen: {chosen:?}") };
            row
        }

        fn value_of(&self, name: &str) -> String {
            self.setting_rows().find(|row| row.name == name).map(|row| row.value).unwrap()
        }

        fn setting_rows(&self) -> impl Iterator<Item = SettingRow> {
            let page = self.settings_page();
            page.days.into_iter().chain([page.ringtone])
        }

        fn first_hour_shown(&self) -> Option<u8> {
            self.clock_page().hours.first().map(|hour| hour.start.time_of_day.hour())
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
        assert_eq!(bench.chosen_row(), "Saturday");
        bench.input(Input::Turn(1));
        bench.press(Button::Long);
        assert_eq!(bench.alarm.schedule().time_on(Weekday::Sunday), TimeOfDay::new(7, 0));
        bench.input(Input::Turn(2));
        bench.press(Button::Long);
        bench.input(Input::Turn(-13));
        bench.press(Button::Long);
        assert_eq!(bench.alarm.schedule().time_on(Weekday::Sunday), TimeOfDay::new(9, 47));
        assert_eq!(bench.chosen_row(), "Sunday");
        bench.press(Button::Yellow);
        assert!(bench.shows_clock());
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
        assert_eq!(bench.chosen_row(), "Saturday");

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
        assert_eq!(bench.chosen_row(), "Saturday", "an off day has no minutes to set");
    }

    #[test]
    fn the_wheel_goes_round_the_ringtones_each_playing_softly_until_done() {
        let mut bench = Bench::new();
        bench.press(Button::Long);
        bench.input(Input::Turn(2));
        assert_eq!(bench.chosen_row(), "Ringtone", "after Sunday");
        bench.press(Button::Long);
        assert_eq!(bench.ringer.playing(), Some(Ringtone::Chime));
        bench.input(Input::Turn(-1));
        assert_eq!((bench.alarm.ringtone(), bench.ringer.playing()), (lost_ark(), Some(lost_ark())));
        bench.input(Input::Turn(2));
        assert_eq!(bench.alarm.ringtone(), zen());
        bench.press(Button::Long);
        assert_eq!((bench.alarm.ringtone(), bench.ringer.playing()), (zen(), None));
        assert_eq!(bench.chosen_row(), "Ringtone");
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
        assert!(bench.shows_clock());
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
        let (version, before) = (AppScreen::<Frame>::version(&bench.screen), bench.clock_page());
        assert_eq!((before.today, before.hours.len()), (None, 0));
        bench.weather.refresh_if_due(std::time::Instant::now());
        assert_ne!(AppScreen::<Frame>::version(&bench.screen), version);
        let after = bench.clock_page();
        assert_eq!(after.today.map(|today| today.high), Some(Degrees(14)));
        assert_eq!(after.hours.len(), day_weather::HOURS_SHOWN);
    }

    #[test]
    fn the_wheel_moves_through_the_hours_and_stops_at_either_end() {
        let mut bench = Bench::new();
        bench.input(Input::Turn(3));
        bench.weather.refresh_if_due(std::time::Instant::now());
        assert_eq!(bench.first_hour_shown(), Some(7), "no forecast, nothing to move through");
        bench.input(Input::Turn(3));
        assert_eq!(bench.first_hour_shown(), Some(10));
        bench.input(Input::Turn(-10));
        assert_eq!(bench.first_hour_shown(), Some(7));
        bench.input(Input::Turn(100));
        assert_eq!(bench.first_hour_shown(), Some(23), "the last hours still fill the column");
        assert_eq!(bench.clock_page().hours.len(), day_weather::HOURS_SHOWN);
        AppScreen::<Frame>::entered(&mut bench.screen);
        assert_eq!(bench.first_hour_shown(), Some(7), "back to the hour under way");
    }

    #[test]
    fn the_next_alarm_is_named_by_its_day() {
        let bench = Bench::new();
        bench.alarm.set_time_on(Weekday::Sunday, TimeOfDay::new(8, 30));
        assert_eq!(bench.clock_page().alarm, "Alarm off");
        bench.alarm.switch_on();
        assert_eq!(bench.clock_page().alarm, "Alarm tomorrow at 08:30");
        bench.alarm.set_time_on(Weekday::Sunday, None);
        bench.alarm.set_time_on(Weekday::Tuesday, TimeOfDay::new(6, 5));
        assert_eq!(bench.clock_page().alarm, "Alarm Tuesday at 06:05");
    }

    #[test]
    fn the_part_of_a_value_being_set_is_in_brackets() {
        let mut bench = Bench::new();
        bench.alarm.set_time_on(Weekday::Saturday, TimeOfDay::new(6, 30));
        bench.press(Button::Long);
        assert_eq!(bench.value_of("Saturday"), "06:30");
        bench.press(Button::Long);
        assert_eq!(bench.value_of("Saturday"), "[06]:30");
        bench.press(Button::Long);
        assert_eq!(bench.value_of("Saturday"), "06:[30]");
        bench.press(Button::Long);
        bench.input(Input::Turn(1));
        bench.press(Button::Long);
        bench.input(Input::Turn(-8));
        assert_eq!(bench.value_of("Sunday"), "[off]");
        bench.press(Button::Long);
        assert_eq!(bench.value_of("Sunday"), "off");
        bench.input(Input::Turn(1));
        bench.press(Button::Long);
        assert_eq!(bench.value_of("Ringtone"), "[Chime]");
        assert_eq!(bench.settings_page().title, "Alarm settings (alarm off)");
    }

    #[test]
    fn ringing_the_hint_says_how_to_snooze_and_stop() {
        let bench = Bench::new();
        assert_eq!(bench.clock_page().hint, "yellow: alarm on/off   long: settings   wheel: hours");
        bench.alarm.set_time_on(Weekday::Saturday, TimeOfDay::new(7, 1));
        bench.alarm.switch_on();
        bench.at(UtcTime::from_unix_seconds(saturday_at_seven().unix_seconds() + 60));
        assert_eq!(bench.clock_page().hint, format!("long: snooze {} min   hold yellow and long: stop", SNOOZE.as_secs() / 60));
        assert_eq!(bench.clock_page().alarm, "Good morning!");
    }
}
