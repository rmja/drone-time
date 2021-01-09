use core::{
    fmt::Debug,
    ops::{Add, Sub},
};

use crate::{Tick, TimeSpan};

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct DateTime(u32);

pub struct DateTimeParts {
    pub year: u16,
    pub month: Month,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl Debug for DateTimeParts {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{}-{:02}-{:02}T{:02}:{:02}:{:02}",
            self.year, self.month as u8, self.day, self.hour, self.minute, self.second
        )
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum Month {
    January = 1,
    February = 2,
    March = 3,
    April = 4,
    May = 5,
    June = 6,
    July = 7,
    August = 8,
    September = 9,
    October = 10,
    November = 11,
    December = 12,
}

impl From<u8> for Month {
    fn from(num: u8) -> Self {
        match num {
            1 => Month::January,
            2 => Month::February,
            3 => Month::March,
            4 => Month::April,
            5 => Month::May,
            6 => Month::June,
            7 => Month::July,
            8 => Month::August,
            9 => Month::September,
            10 => Month::October,
            11 => Month::November,
            12 => Month::December,
            _ => unreachable!(),
        }
    }
}

const EPOCH_YEAR: u16 = 1970;
const SECONDS_PER_MINUTE: u32 = 60;
const SECONDS_PER_HOUR: u32 = 60 * SECONDS_PER_MINUTE;
const SECONDS_PER_DAY: u32 = 24 * SECONDS_PER_HOUR;

impl DateTime {
    pub const EPOCH: DateTime = DateTime::from_unixtimestamp(0);

    /// Create a new `DateTime`.
    pub fn new(year: u16, month: Month, day: u8, hour: u8, minute: u8, second: u8) -> Self {
        let mut days = day as u16 - 1;

        for y in EPOCH_YEAR..year {
            days += days_in_year(y);
        }

        for m in (Month::January as u8)..(month as u8) {
            let m = m.into();
            days += days_in_month(year, m) as u16;
        }

        let seconds = days as u32 * SECONDS_PER_DAY
            + hour as u32 * SECONDS_PER_HOUR
            + minute as u32 * SECONDS_PER_MINUTE
            + second as u32;
        Self(seconds)
    }

    pub const fn from_unixtimestamp(timestamp: u32) -> Self {
        Self(timestamp)
    }

    /// Get the date part without the time component.
    pub const fn date(&self) -> Self {
        Self((self.0 / SECONDS_PER_DAY) * SECONDS_PER_DAY)
    }

    /// Get the different date and time parts.
    pub fn parts(&self) -> DateTimeParts {
        let mut year = EPOCH_YEAR;
        let mut month = 1u8;
        let mut day = 1u8;
        let mut hour = 0u8;
        let mut minute = 0u8;
        let mut seconds = self.0;

        loop {
            let seconds_in_year = days_in_year(year) as u32 * SECONDS_PER_DAY;
            if seconds_in_year <= seconds {
                seconds -= seconds_in_year;
                year += 1;
            } else {
                break;
            }
        }

        loop {
            let seconds_in_month = days_in_month(year, month.into()) as u32 * SECONDS_PER_DAY;
            if seconds_in_month <= seconds {
                seconds -= seconds_in_month;
                month += 1;
            } else {
                break;
            }
        }

        while SECONDS_PER_DAY <= seconds {
            seconds -= SECONDS_PER_DAY;
            day += 1;
        }

        while SECONDS_PER_HOUR <= seconds {
            seconds -= SECONDS_PER_HOUR;
            hour += 1;
        }

        while SECONDS_PER_MINUTE <= seconds {
            seconds -= SECONDS_PER_MINUTE;
            minute += 1;
        }

        DateTimeParts {
            year,
            month: month.into(),
            day,
            hour,
            minute,
            second: seconds as u8,
        }
    }
}

impl<T: Tick> Add<TimeSpan<T>> for DateTime {
    type Output = DateTime;

    fn add(self, rhs: TimeSpan<T>) -> Self::Output {
        DateTime(self.0 + rhs.total_seconds())
    }
}

impl<T: Tick> Sub<TimeSpan<T>> for DateTime {
    type Output = DateTime;

    fn sub(self, rhs: TimeSpan<T>) -> Self::Output {
        DateTime(self.0 - rhs.total_seconds())
    }
}

// fn is_valid_date(year: u16, month: Month, day: u8) -> bool {
//     day >= 1 && day <= days_in_month(year, month)
// }

fn days_in_month(year: u16, month: Month) -> u8 {
    if is_leap_year(year) && month == Month::February {
        29
    } else {
        const DAYS: [u8; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        let index = month as usize - 1;
        DAYS[index]
    }
}

const fn days_in_year(year: u16) -> u16 {
    if is_leap_year(year) {
        366
    } else {
        365
    }
}

const fn is_leap_year(year: u16) -> bool {
    if year % 4 > 0 {
        false
    } else if year % 100 > 0 {
        true
    } else if year % 400 > 0 {
        false
    } else {
        true
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    fn parts() {
        let dt = DateTime::new(1985, Month::August, 28, 1, 2, 3);
        let parts = dt.parts();

        assert_eq!(494038923, dt.0);
        assert_eq!(1985, parts.year);
        assert_eq!(Month::August, parts.month);
        assert_eq!(28, parts.day);
        assert_eq!(1, parts.hour);
        assert_eq!(2, parts.minute);
        assert_eq!(3, parts.second);
    }
}
