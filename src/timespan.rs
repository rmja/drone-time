use core::{
    fmt::Debug,
    marker::PhantomData,
    ops::{Add, Sub},
};

use crate::Tick;

pub struct TimeSpan<T: Tick>(pub u64, pub(crate) PhantomData<T>);

impl<T: Tick> Copy for TimeSpan<T> {}

impl<T: Tick> Clone for TimeSpan<T> {
    fn clone(&self) -> Self {
        Self(self.0, PhantomData)
    }
}

#[derive(Debug)]
pub struct TimeSpanParts {
    pub days: u16,
    pub hours: u16,
    pub mins: u8,
    pub secs: u8,
    pub millis: u16,
}

impl<T: Tick> TimeSpan<T> {
    pub const ZERO: Self = Self(0, PhantomData);
    const MAX_secs: u32 = u32::MAX;
    const MAX_millis: u64 = Self::MAX_secs as u64 * 1000;
    const MAX_DAYS: u16 = (Self::MAX_secs / 60 / 60 / 24) as u16;
    const TICKS_PER_SECOND: u64 = T::FREQ as u64;
    const TICKS_PER_MINUTE: u64 = Self::TICKS_PER_SECOND * 60;
    const TICKS_PER_HOUR: u64 = Self::TICKS_PER_MINUTE * 60;
    const TICKS_PER_DAY: u64 = Self::TICKS_PER_HOUR * 24;

    pub fn new(hours: u16, mins: u8, secs: u8) -> Self {
        Self::from_parts(TimeSpanParts {
            days: 0,
            hours,
            mins,
            secs,
            millis: 0,
        })
    }

    pub fn from_parts(parts: TimeSpanParts) -> Self {
        assert!(parts.days <= Self::MAX_DAYS);
        assert!(parts.hours < 24);
        assert!(parts.mins < 60);
        assert!(parts.secs < 60);
        assert!(parts.millis < 1000);

        let ticks = parts.days as u64 * Self::TICKS_PER_DAY
            + parts.hours as u64 * Self::TICKS_PER_HOUR
            + parts.mins as u64 * Self::TICKS_PER_MINUTE
            + parts.secs as u64 * Self::TICKS_PER_SECOND
            + (parts.millis as u64 * Self::TICKS_PER_SECOND) / 1000;
        Self::from_ticks(ticks)
    }

    pub fn from_secs(secs: u32) -> Self {
        Self::from_ticks(secs as u64 * Self::TICKS_PER_SECOND)
    }

    pub fn from_millis(millis: u64) -> Self {
        assert!(millis <= Self::MAX_millis);

        let secs = millis / 1000;
        let sub_secs = millis - secs * 1000;
        let ticks =
            secs * Self::TICKS_PER_SECOND + (sub_secs * 1000 * Self::TICKS_PER_SECOND) / 1000;
        Self::from_ticks(ticks)
    }

    pub fn from_ticks(ticks: u64) -> Self {
        Self(ticks, PhantomData)
    }

    pub fn parts(&self) -> TimeSpanParts {
        let mut ticks = self.0;

        let days = ticks / Self::TICKS_PER_DAY;
        ticks -= days * Self::TICKS_PER_DAY;

        let hours = ticks / Self::TICKS_PER_HOUR;
        ticks -= hours * Self::TICKS_PER_HOUR;

        let mins = ticks / Self::TICKS_PER_MINUTE;
        ticks -= mins * Self::TICKS_PER_MINUTE;

        let secs = ticks / Self::TICKS_PER_SECOND;
        ticks -= secs * Self::TICKS_PER_SECOND;

        // Round to nearest.
        let millis = (ticks * 1000 + Self::TICKS_PER_SECOND / 2) / Self::TICKS_PER_SECOND;

        TimeSpanParts {
            days: days as u16,
            hours: hours as u16,
            mins: mins as u8,
            secs: secs as u8,
            millis: millis as u16,
        }
    }

    pub fn total_secs(&self) -> u32 {
        (self.0 / Self::TICKS_PER_SECOND) as u32
    }

    pub fn total_millis(&self) -> u64 {
        let secs = self.total_secs() as u64;
        let sub_secs = self.0 - secs * Self::TICKS_PER_SECOND;
        // Round to nearest.
        secs * 1000 + (sub_secs * 1000 + Self::TICKS_PER_SECOND / 2) / Self::TICKS_PER_SECOND
    }
}

impl<T: Tick> Default for TimeSpan<T> {
    fn default() -> Self {
        Self::ZERO
    }
}

impl<T: Tick> Into<u64> for TimeSpan<T> {
    fn into(self) -> u64 {
        self.0
    }
}

impl<T: Tick> PartialEq for TimeSpan<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: Tick> PartialOrd for TimeSpan<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<T: Tick> Add for TimeSpan<T> {
    type Output = TimeSpan<T>;

    fn add(self, rhs: Self) -> Self::Output {
        TimeSpan::from_ticks(self.0 + rhs.0)
    }
}

impl<T: Tick> Sub for TimeSpan<T> {
    type Output = TimeSpan<T>;

    fn sub(self, rhs: Self) -> Self::Output {
        TimeSpan::from_ticks(self.0 - rhs.0)
    }
}

impl<T: Tick> Debug for TimeSpan<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let parts = self.parts();
        write!(
            f,
            "{}d{:02}:{:02}:{:02}.{:03}",
            parts.days, parts.hours, parts.mins, parts.secs, parts.millis
        )
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    struct TestTick;

    impl Tick for TestTick {
        const FREQ: u32 = 32768;
    }

    #[test]
    fn parts() {
        let ts = TimeSpan::<TestTick>::from_parts(TimeSpanParts {
            days: 1,
            hours: 2,
            mins: 3,
            secs: 4,
            millis: 5,
        });
        let parts = ts.parts();

        assert_eq!(
            1 * 86400 * 32768 + 2 * 3600 * 32768 + 3 * 60 * 32768 + 4 * 32768 + (5 * 32768) / 1000,
            ts.0
        );
        assert_eq!(1, parts.days);
        assert_eq!(2, parts.hours);
        assert_eq!(3, parts.mins);
        assert_eq!(4, parts.secs);
        assert_eq!(5, parts.millis);
    }

    #[test]
    fn total_secs() {
        let secs = TimeSpan::<TestTick>::from_parts(TimeSpanParts {
            days: 1,
            hours: 2,
            mins: 3,
            secs: 4,
            millis: 5,
        })
        .total_secs();
        assert_eq!(1 * 86400 + 2 * 3600 + 3 * 60 + 4, secs);
    }

    #[test]
    fn total_millis() {
        let millis = TimeSpan::<TestTick>::from_parts(TimeSpanParts {
            days: 1,
            hours: 2,
            mins: 3,
            secs: 4,
            millis: 5,
        })
        .total_millis();
        assert_eq!(
            1 * 86400 * 1000 + 2 * 3600 * 1000 + 3 * 60 * 1000 + 4 * 1000 + 5,
            millis
        );
    }
}
