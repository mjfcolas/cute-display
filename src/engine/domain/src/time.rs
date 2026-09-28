//! Instants, and the time on the wall.

use crate::calendar::Date;

pub(crate) const SECONDS_PER_DAY: i64 = 86_400;

/// Seconds since 1970-01-01 00:00:00 UTC, leap seconds aside.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct UtcTime(i64);

impl UtcTime {
    pub const fn from_unix_seconds(seconds: i64) -> Self {
        Self(seconds)
    }

    pub fn unix_seconds(self) -> i64 {
        self.0
    }

    /// The time on the wall where the clocks are `offset` from UTC.
    pub fn at_offset(self, offset: UtcOffset) -> LocalTime {
        LocalTime::from_seconds_since_epoch(self.0 + i64::from(offset.seconds_east()))
    }
}

/// How far the clocks on the wall are ahead of UTC; behind in the west.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UtcOffset(i32);

impl UtcOffset {
    pub const ZERO: Self = Self(0);

    pub const fn east(seconds: i32) -> Self {
        Self(seconds)
    }

    /// POSIX counts the other way: `5` is New York, five hours behind.
    pub const fn from_posix_west(seconds: i32) -> Self {
        Self(-seconds)
    }

    pub fn seconds_east(self) -> i32 {
        self.0
    }

    pub fn plus_seconds(self, seconds: i32) -> Self {
        Self(self.0 + seconds)
    }
}

/// An hour and a minute of the day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimeOfDay {
    hour: u8,
    minute: u8,
}

impl TimeOfDay {
    pub const MIDNIGHT: Self = Self { hour: 0, minute: 0 };

    /// `None` for an hour above 23 or a minute above 59.
    pub const fn new(hour: u8, minute: u8) -> Option<Self> {
        if hour < 24 && minute < 60 { Some(Self { hour, minute }) } else { None }
    }

    pub fn hour(self) -> u8 {
        self.hour
    }

    pub fn minute(self) -> u8 {
        self.minute
    }

    pub fn minutes_since_midnight(self) -> u16 {
        u16::from(self.hour) * 60 + u16::from(self.minute)
    }
}

/// The date and time on the wall where the device is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LocalTime {
    pub date: Date,
    pub time_of_day: TimeOfDay,
    pub second: u8,
}

impl LocalTime {
    /// Seconds since 1970-01-01 00:00:00 on the wall.
    pub fn seconds_since_epoch(self) -> i64 {
        self.date.days_since_epoch() * SECONDS_PER_DAY
            + i64::from(self.time_of_day.minutes_since_midnight()) * 60
            + i64::from(self.second)
    }

    pub fn from_seconds_since_epoch(seconds: i64) -> Self {
        let second_of_day = seconds.rem_euclid(SECONDS_PER_DAY);
        let narrow = |n: i64| u8::try_from(n).unwrap_or(0);
        Self {
            date: Date::from_days_since_epoch(seconds.div_euclid(SECONDS_PER_DAY)),
            time_of_day: TimeOfDay::new(narrow(second_of_day / 3600), narrow(second_of_day / 60 % 60))
                .unwrap_or(TimeOfDay::MIDNIGHT),
            second: narrow(second_of_day % 60),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_offset_moves_the_wall_clock_across_midnight() {
        let late = UtcTime::from_unix_seconds(1_790_467_200 - 1); // 2026-09-26 23:59:59 UTC
        let paris = late.at_offset(UtcOffset::east(2 * 3600));
        assert_eq!(paris.date, Date::new(2026, 9, 27).unwrap());
        assert_eq!(paris.time_of_day, TimeOfDay::new(1, 59).unwrap());
        assert_eq!(paris.second, 59);
        let new_york = UtcTime::from_unix_seconds(1_790_467_200).at_offset(UtcOffset::from_posix_west(4 * 3600));
        assert_eq!((new_york.date.day(), new_york.time_of_day.hour()), (26, 20));
    }

    #[test]
    fn wall_seconds_come_back() {
        for seconds in [0, 59, 86_399, 86_400, 1_790_407_815, -1] {
            assert_eq!(LocalTime::from_seconds_since_epoch(seconds).seconds_since_epoch(), seconds);
        }
    }

    #[test]
    fn a_time_of_day_stays_within_the_day() {
        assert_eq!(TimeOfDay::new(24, 0), None);
        assert_eq!(TimeOfDay::new(23, 60), None);
        assert_eq!(TimeOfDay::new(23, 59).map(TimeOfDay::minutes_since_midnight), Some(1439));
    }
}
