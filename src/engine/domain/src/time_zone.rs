use core::iter::Peekable;
use core::str::Chars;

use crate::calendar::{days_in_month, Date, Weekday};
use crate::time::{LocalTime, UtcOffset, UtcTime, SECONDS_PER_DAY};

const HOUR: i32 = 3600;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeZone {
    standard_offset: UtcOffset,
    summer: Option<SummerTime>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SummerTime {
    offset: UtcOffset,
    /// On the wall, in standard time.
    starts: Transition,
    /// On the wall, in summer time.
    ends: Transition,
}

/// The `week`th `weekday` of `month`, 5 being the last, `seconds` after its midnight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Transition {
    month: u8,
    week: u8,
    weekday: Weekday,
    seconds: i32,
}

impl Default for TimeZone {
    fn default() -> Self {
        let last_sunday = |month, hour| Transition { month, week: 5, weekday: Weekday::Sunday, seconds: hour * HOUR };
        let summer = SummerTime { offset: UtcOffset::east(2 * HOUR), starts: last_sunday(3, 2), ends: last_sunday(10, 3) };
        Self { standard_offset: UtcOffset::east(HOUR), summer: Some(summer) }
    }
}

impl TimeZone {
    pub const UTC: Self = Self { standard_offset: UtcOffset::ZERO, summer: None };

    pub fn parse(posix: &str) -> Option<Self> {
        let mut text = Cursor(posix.trim().chars().peekable());
        text.name()?;
        let standard_offset = UtcOffset::from_posix_west(text.duration()?);
        if text.is_done() {
            return Some(Self { standard_offset, summer: None });
        }
        text.name()?;
        let offset = if text.next_is(',') { standard_offset.plus_seconds(HOUR) } else { UtcOffset::from_posix_west(text.duration()?) };
        let starts = text.after(',')?.transition()?;
        let ends = text.after(',')?.transition()?;
        text.is_done().then_some(Self { standard_offset, summer: Some(SummerTime { offset, starts, ends }) })
    }

    pub fn offset_at(&self, time: UtcTime) -> UtcOffset {
        let Some(summer) = self.summer else {
            return self.standard_offset;
        };
        let year = time.at_offset(self.standard_offset).date.year();
        let starts = summer.starts.on_the_wall(year) - i64::from(self.standard_offset.seconds_east());
        let ends = summer.ends.on_the_wall(year) - i64::from(summer.offset.seconds_east());
        let seconds = time.unix_seconds();
        let in_summer = if starts < ends {
            (starts..ends).contains(&seconds)
        } else {
            seconds >= starts || seconds < ends
        };
        if in_summer { summer.offset } else { self.standard_offset }
    }

    pub fn local(&self, time: UtcTime) -> LocalTime {
        time.at_offset(self.offset_at(time))
    }
}

impl Transition {
    fn on_the_wall(self, year: u16) -> i64 {
        let first = Date::new(year, self.month, 1).map_or(0, Date::days_since_epoch);
        let first_weekday = Date::from_days_since_epoch(first).weekday().days_since_sunday();
        let mut day = 1 + (i64::from(self.weekday.days_since_sunday()) - i64::from(first_weekday)).rem_euclid(7)
            + 7 * (i64::from(self.week) - 1);
        while day > i64::from(days_in_month(year, self.month)) {
            day -= 7;
        }
        (first + day - 1) * SECONDS_PER_DAY + i64::from(self.seconds)
    }
}

struct Cursor<'a>(Peekable<Chars<'a>>);

impl Cursor<'_> {
    fn is_done(&mut self) -> bool {
        self.0.peek().is_none()
    }

    fn next_is(&mut self, wanted: char) -> bool {
        self.0.peek() == Some(&wanted)
    }

    fn eat(&mut self, wanted: char) -> bool {
        self.0.next_if_eq(&wanted).is_some()
    }

    fn after(&mut self, wanted: char) -> Option<&mut Self> {
        self.eat(wanted).then_some(self)
    }

    fn name(&mut self) -> Option<()> {
        if self.eat('<') {
            while !self.eat('>') {
                self.0.next()?;
            }
            return Some(());
        }
        let mut letters = 0;
        while self.0.next_if(char::is_ascii_alphabetic).is_some() {
            letters += 1;
        }
        (letters >= 3).then_some(())
    }

    fn number(&mut self) -> Option<i32> {
        let mut number: Option<i32> = None;
        while let Some(digit) = self.0.next_if(char::is_ascii_digit).and_then(|c| c.to_digit(10)) {
            number = Some(number.unwrap_or(0).checked_mul(10)?.checked_add(i32::try_from(digit).ok()?)?);
        }
        number
    }

    fn duration(&mut self) -> Option<i32> {
        let sign = if self.eat('-') {
            -1
        } else {
            self.eat('+');
            1
        };
        let hours = self.number()?;
        let minutes = if self.eat(':') { self.number()? } else { 0 };
        let seconds = if self.eat(':') { self.number()? } else { 0 };
        (hours <= 167 && minutes < 60 && seconds < 60).then_some(sign * (hours * HOUR + minutes * 60 + seconds))
    }

    fn transition(&mut self) -> Option<Transition> {
        self.eat('M').then_some(())?;
        let month = u8::try_from(self.number()?).ok().filter(|m| (1..=12).contains(m))?;
        let week = u8::try_from(self.after('.')?.number()?).ok().filter(|w| (1..=5).contains(w))?;
        let weekday = Weekday::from_days_since_sunday(u8::try_from(self.after('.')?.number()?).ok()?)?;
        let seconds = if self.eat('/') { self.duration()? } else { 2 * HOUR };
        Some(Transition { month, week, weekday, seconds })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::TimeOfDay;

    fn utc(year: u16, month: u8, day: u8, hour: u8, minute: u8) -> UtcTime {
        let date = Date::new(year, month, day).unwrap();
        UtcTime::from_unix_seconds(date.days_since_epoch() * SECONDS_PER_DAY + i64::from(hour) * 3600 + i64::from(minute) * 60)
    }

    fn wall(zone: &TimeZone, time: UtcTime) -> (u8, u8, u8) {
        let local = zone.local(time);
        (local.date.day(), local.time_of_day.hour(), local.time_of_day.minute())
    }

    #[test]
    fn the_default_is_central_european_time() {
        assert_eq!(TimeZone::parse("CET-1CEST,M3.5.0,M10.5.0/3"), Some(TimeZone::default()));
    }

    #[test]
    fn paris_springs_forward_and_falls_back_on_the_last_sundays() {
        let paris = TimeZone::default();
        assert_eq!(wall(&paris, utc(2026, 3, 29, 0, 59)), (29, 1, 59));
        assert_eq!(wall(&paris, utc(2026, 3, 29, 1, 0)), (29, 3, 0));
        assert_eq!(wall(&paris, utc(2026, 10, 25, 0, 59)), (25, 2, 59));
        assert_eq!(wall(&paris, utc(2026, 10, 25, 1, 0)), (25, 2, 0));
        assert_eq!(paris.offset_at(utc(2026, 9, 26, 12, 0)).seconds_east(), 7200);
        assert_eq!(paris.offset_at(utc(2026, 12, 31, 23, 30)).seconds_east(), 3600);
    }

    #[test]
    fn a_week_five_is_the_last_even_in_a_month_with_four() {
        // February 2026 has four Sundays; the last is the 22nd.
        let zone = TimeZone::parse("XXX0YYY,M2.5.0/0,M11.1.0").unwrap();
        assert_eq!(zone.offset_at(utc(2026, 2, 21, 23, 59)).seconds_east(), 0);
        assert_eq!(zone.offset_at(utc(2026, 2, 22, 0, 0)).seconds_east(), 3600);
    }

    #[test]
    fn new_york_is_west_and_changes_on_other_sundays() {
        let new_york = TimeZone::parse("EST5EDT,M3.2.0,M11.1.0").unwrap();
        assert_eq!(new_york.offset_at(utc(2026, 3, 8, 6, 59)).seconds_east(), -5 * 3600);
        assert_eq!(new_york.offset_at(utc(2026, 3, 8, 7, 0)).seconds_east(), -4 * 3600);
        assert_eq!(new_york.offset_at(utc(2026, 11, 1, 5, 59)).seconds_east(), -4 * 3600);
        assert_eq!(new_york.offset_at(utc(2026, 11, 1, 6, 0)).seconds_east(), -5 * 3600);
    }

    #[test]
    fn sydney_has_its_summer_across_the_new_year() {
        let sydney = TimeZone::parse("AEST-10AEDT,M10.1.0,M4.1.0/3").unwrap();
        assert_eq!(sydney.offset_at(utc(2026, 1, 15, 0, 0)).seconds_east(), 11 * 3600);
        assert_eq!(sydney.offset_at(utc(2026, 6, 15, 0, 0)).seconds_east(), 10 * 3600);
        assert_eq!(sydney.offset_at(utc(2026, 12, 15, 0, 0)).seconds_east(), 11 * 3600);
    }

    #[test]
    fn quoted_names_minutes_and_no_summer_time() {
        let tehran = TimeZone::parse("<+0330>-3:30").unwrap();
        assert_eq!(tehran.offset_at(utc(2026, 6, 1, 0, 0)).seconds_east(), 3 * 3600 + 1800);
        assert_eq!(TimeZone::parse("UTC0"), Some(TimeZone::UTC));
        let local = TimeZone::parse(" <-03>3 ").unwrap().local(utc(2026, 1, 1, 1, 30));
        assert_eq!((local.date.year(), local.time_of_day), (2025, TimeOfDay::new(22, 30).unwrap()));
    }

    #[test]
    fn what_it_cannot_read_is_no_time_zone() {
        for text in ["", "CET", "CE-1", "CET-1CEST", "CET-1CEST,J60,J300", "CET-1CEST,M13.5.0,M10.5.0", "CET-1CEST,M3.5.0", "CET-1 CEST", "<CET-1"] {
            assert_eq!(TimeZone::parse(text), None, "{text}");
        }
    }
}
