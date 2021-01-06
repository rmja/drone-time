use core::{fmt::Debug, marker::PhantomData};

use crate::JiffiesClock;

pub struct TimeSpan<Clock: JiffiesClock>(pub u64, pub(crate) PhantomData<Clock>);

pub struct TimeSpanParts {
    pub days: u16,
    pub hours: u16,
    pub minutes: u8,
    pub seconds: u8,
    pub milliseconds: u16,
}

impl<Clock: JiffiesClock> TimeSpan<Clock> {
    pub const ZERO: Self = Self(0, PhantomData);
    const MAX_SECONDS: u32 = u32::MAX;
    const MAX_MILLISECONDS: u64 = Self::MAX_SECONDS as u64 * 1000;
    const MAX_DAYS: u16 = (Self::MAX_SECONDS / 60 / 60 / 24) as u16;

    pub fn new(parts: TimeSpanParts) -> Self {
        assert!(parts.days <= Self::MAX_DAYS);
        assert!(parts.hours < 24);
        assert!(parts.minutes < 60);
        assert!(parts.seconds < 60);
        assert!(parts.milliseconds < 1000);

        let ticks = parts.days as u64 * Self::jiffies_per_day()
            + parts.hours as u64 * Self::jiffies_per_hour()
            + parts.minutes as u64 * Self::jiffies_per_minute()
            + parts.seconds as u64 * Self::jiffies_per_second()
            + (parts.milliseconds as u64 * Self::jiffies_per_second()) / 1000;
        Self(ticks, PhantomData)
    }

    pub fn from_seconds(seconds: u32) -> Self {
        assert!(seconds <= Self::MAX_SECONDS);

        Self(seconds as u64 * Self::jiffies_per_second(), PhantomData)
    }

    pub fn from_milliseconds(milliseconds: u64) -> Self {
        assert!(milliseconds <= Self::MAX_MILLISECONDS);

        let seconds = milliseconds / 1000;
        let sub_seconds = milliseconds - seconds * 1000;
        let ticks = seconds * Self::jiffies_per_second()
            + (sub_seconds * 1000 * Self::jiffies_per_second()) / 1000;
        Self(ticks, PhantomData)
    }

    pub fn parts(&self) -> TimeSpanParts {
        let mut ticks = self.0;

        let days = ticks / Self::jiffies_per_day();
        ticks -= days * Self::jiffies_per_day();

        let hours = ticks / Self::jiffies_per_hour();
        ticks -= hours * Self::jiffies_per_hour();

        let minutes = ticks / Self::jiffies_per_minute();
        ticks -= minutes * Self::jiffies_per_minute();

        let seconds = ticks / Self::jiffies_per_second();
        ticks -= seconds * Self::jiffies_per_second();

        // Round to nearest.
        let milliseconds =
            (ticks * 1000 + Self::jiffies_per_second() / 2) / Self::jiffies_per_second();

        TimeSpanParts {
            days: days as u16,
            hours: hours as u16,
            minutes: minutes as u8,
            seconds: seconds as u8,
            milliseconds: milliseconds as u16,
        }
    }

    pub fn total_seconds(&self) -> u32 {
        (self.0 / Self::jiffies_per_second()) as u32
    }

    pub fn total_milliseconds(&self) -> u64 {
        let seconds = self.total_seconds() as u64;
        let sub_seconds = self.0 - seconds * Self::jiffies_per_second();
        // Round to nearest.
        seconds * 1000
            + (sub_seconds * 1000 + Self::jiffies_per_second() / 2) / Self::jiffies_per_second()
    }

    fn jiffies_per_second() -> u64 {
        Clock::freq() as u64
    }

    fn jiffies_per_minute() -> u64 {
        Self::jiffies_per_second() * 60
    }

    fn jiffies_per_hour() -> u64 {
        Self::jiffies_per_minute() * 60
    }

    fn jiffies_per_day() -> u64 {
        Self::jiffies_per_hour() * 24
    }
}

impl<Clock: JiffiesClock> Into<u64> for TimeSpan<Clock> {
    fn into(self) -> u64 {
        self.0
    }
}

impl<Clock: JiffiesClock> PartialEq for TimeSpan<Clock> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<Clock: JiffiesClock> PartialOrd for TimeSpan<Clock> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<Clock: JiffiesClock> Debug for TimeSpan<Clock> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let parts = self.parts();
        write!(
            f,
            "{}d{:02}:{:02}:{:02}.{:03}",
            parts.days, parts.hours, parts.minutes, parts.seconds, parts.milliseconds
        )
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    struct TestClock;

    impl JiffiesClock for TestClock {
        fn freq() -> u32 {
            32768
        }
    }

    #[test]
    fn parts() {
        let ts = TimeSpan::<TestClock>::new(TimeSpanParts {
            days: 1,
            hours: 2,
            minutes: 3,
            seconds: 4,
            milliseconds: 5,
        });
        let parts = ts.parts();

        assert_eq!(
            1 * 86400 * 32768 + 2 * 3600 * 32768 + 3 * 60 * 32768 + 4 * 32768 + (5 * 32768) / 1000,
            ts.0
        );
        assert_eq!(1, parts.days);
        assert_eq!(2, parts.hours);
        assert_eq!(3, parts.minutes);
        assert_eq!(4, parts.seconds);
        assert_eq!(5, parts.milliseconds);
    }

    #[test]
    fn total_seconds() {
        let seconds = TimeSpan::<TestClock>::new(TimeSpanParts {
            days: 1,
            hours: 2,
            minutes: 3,
            seconds: 4,
            milliseconds: 5,
        })
        .total_seconds();
        assert_eq!(1 * 86400 + 2 * 3600 + 3 * 60 + 4, seconds);
    }

    #[test]
    fn total_milliseconds() {
        let milliseconds = TimeSpan::<TestClock>::new(TimeSpanParts {
            days: 1,
            hours: 2,
            minutes: 3,
            seconds: 4,
            milliseconds: 5,
        })
        .total_milliseconds();
        assert_eq!(
            1 * 86400 * 1000 + 2 * 3600 * 1000 + 3 * 60 * 1000 + 4 * 1000 + 5,
            milliseconds
        );
    }
}
