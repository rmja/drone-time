use core::{
    fmt::Debug,
    marker::PhantomData,
    ops::{Add, AddAssign, Sub, SubAssign},
};

use crate::Tick;

pub struct TimeSpan<T: Tick>(pub i64, PhantomData<T>);

impl<T: Tick> Copy for TimeSpan<T> {}

impl<T: Tick> Clone for TimeSpan<T> {
    fn clone(&self) -> Self {
        Self(self.0, PhantomData)
    }
}

#[derive(Debug)]
pub struct TimeSpanParts {
    pub days: i16,
    pub hours: i8,
    pub mins: i8,
    pub secs: i8,
    pub millis: i16,
}

impl<T: Tick> TimeSpan<T> {
    pub const ZERO: Self = Self(0, PhantomData);
    pub const MAX: Self = Self(i64::MAX, PhantomData);
    pub const MIN: Self = Self(i64::MIN, PhantomData);
    const MAX_MILLIS: i64 = Self::MAX_SECS as i64 * 1000;
    const MAX_SECS: i32 = i32::MAX;
    const MAX_DAYS: i16 = (Self::MAX_SECS / 60 / 60 / 24) as i16;
    const MIN_MILLIS: i64 = Self::MIN_SECS as i64 * 1000;
    const MIN_SECS: i32 = i32::MIN;
    const MIN_DAYS: i16 = (Self::MIN_SECS / 60 / 60 / 24) as i16;
    const TICKS_PER_SEC: i64 = T::FREQ as i64;
    const TICKS_PER_MIN: i64 = Self::TICKS_PER_SEC * 60;
    const TICKS_PER_HOUR: i64 = Self::TICKS_PER_MIN * 60;
    const TICKS_PER_DAY: i64 = Self::TICKS_PER_HOUR * 24;

    /// Create a new `TimeSpan` from `hours`, `mins`, and `secs`.
    pub const fn new_time(hours: i8, mins: i8, secs: i8) -> Self {
        Self::from_parts(TimeSpanParts {
            days: 0,
            hours,
            mins,
            secs,
            millis: 0,
        })
    }

    /// Create a new `TimeSpan` from individual components.
    pub const fn from_parts(parts: TimeSpanParts) -> Self {
        assert!(parts.days >= Self::MIN_DAYS && parts.days <= Self::MAX_DAYS);
        assert!(parts.hours > -24 && parts.hours < 24);
        assert!(parts.mins > -60 && parts.mins < 60);
        assert!(parts.secs > -60 && parts.secs < 60);
        assert!(parts.millis > -1000 && parts.millis < 1000);

        let ticks = parts.days as i64 * Self::TICKS_PER_DAY
            + parts.hours as i64 * Self::TICKS_PER_HOUR
            + parts.mins as i64 * Self::TICKS_PER_MIN
            + parts.secs as i64 * Self::TICKS_PER_SEC
            + (parts.millis as i64 * Self::TICKS_PER_SEC) / 1000;
        Self::from_ticks(ticks)
    }

    /// Create a new `TimeSpan` from the specified number of _whole_ days.
    pub const fn from_days(days: i16) -> Self {
        Self::from_ticks(days as i64 * Self::TICKS_PER_DAY)
    }

    /// Create a new `TimeSpan` from the specified number of _whole_ hours.
    pub const fn from_hours(hours: i32) -> Self {
        Self::from_ticks(hours as i64 * Self::TICKS_PER_HOUR)
    }

    /// Create a new `TimeSpan` from the specified number of _whole_ mins.
    pub const fn from_mins(mins: i32) -> Self {
        Self::from_ticks(mins as i64 * Self::TICKS_PER_MIN)
    }

    /// Create a new `TimeSpan` from the specified number of _whole_ seconds.
    pub const fn from_secs(secs: i32) -> Self {
        Self::from_ticks(secs as i64 * Self::TICKS_PER_SEC)
    }

    /// Create a new `TimeSpan` from the specified number of _whole_ milliseconds.
    // TODO: Maybe add Rounding
    pub const fn from_millis(millis: i64) -> Self {
        assert!(millis >= Self::MIN_MILLIS && millis <= Self::MAX_MILLIS);

        let secs = millis / 1000;
        let sub_secs = millis - secs * 1000;
        let ticks = secs * Self::TICKS_PER_SEC + (sub_secs * 1000 * Self::TICKS_PER_SEC) / 1000;
        Self::from_ticks(ticks)
    }

    /// Create a new `TimeSpan` from the specified number of _whole_ microseconds.
    // TODO: Maybe add Rounding
    pub const fn from_micros(micros: i64) -> Self {
        let millis = micros / 1000;
        let sub_millis = micros - millis * 1000;
        let ticks =
            (millis * Self::TICKS_PER_SEC) / 1000 + (sub_millis * Self::TICKS_PER_SEC) / 1_000_000;
        Self::from_ticks(ticks)
    }

    /// Create a new `TimeSpan` from the specified number of _whole_ nanoseconds.
    // TODO: Maybe add Rounding
    pub const fn from_nanos(nanos: i64) -> Self {
        let micros = nanos / 1000;
        let sub_micros = nanos - micros * 1000;
        let ticks = (micros * Self::TICKS_PER_SEC) / 1_000_000
            + (sub_micros * Self::TICKS_PER_SEC) / 1_000_000_000;
        Self::from_ticks(ticks)
    }

    /// Create a new `TimeSpan` from the specified number of _whole_ ticks.
    pub const fn from_ticks(ticks: i64) -> Self {
        Self(ticks, PhantomData)
    }

    /// Get the individual components of a `TimeSpan`.
    pub fn parts(&self) -> TimeSpanParts {
        let mut ticks = self.0;

        let days = ticks / Self::TICKS_PER_DAY;
        ticks -= days * Self::TICKS_PER_DAY;

        let hours = ticks / Self::TICKS_PER_HOUR;
        ticks -= hours * Self::TICKS_PER_HOUR;

        let mins = ticks / Self::TICKS_PER_MIN;
        ticks -= mins * Self::TICKS_PER_MIN;

        let secs = ticks / Self::TICKS_PER_SEC;
        ticks -= secs * Self::TICKS_PER_SEC;

        // Round to nearest.
        let millis = (ticks * 1000 + Self::TICKS_PER_SEC / 2) / Self::TICKS_PER_SEC;

        TimeSpanParts {
            days: days as i16,
            hours: hours as i8,
            mins: mins as i8,
            secs: secs as i8,
            millis: millis as i16,
        }
    }

    /// Get the absolute duration of a `TimeSpan`.
    pub fn abs(&self) -> Self {
        assert!(self.0 != Self::MIN.0, "Overflow!");
        if self.0 >= 0 {
            Self(self.0, PhantomData)
        } else {
            Self(-self.0, PhantomData)
        }
    }

    /// Get the number of _whole_ seconds in the `TimeSpan`.
    pub fn as_secs(&self) -> i32 {
        (self.0 / Self::TICKS_PER_SEC) as i32
    }

    /// Get the number of _whole_ milliseconds in the `TimeSpan`.
    pub fn as_millis(&self) -> i64 {
        let secs = self.as_secs() as i64;
        let sub_secs = self.0 - secs * Self::TICKS_PER_SEC;
        // Round to nearest.
        secs * 1000 + (sub_secs * 1000 + Self::TICKS_PER_SEC / 2) / Self::TICKS_PER_SEC
    }
}

impl<T: Tick> Default for TimeSpan<T> {
    fn default() -> Self {
        Self::ZERO
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

impl<T: Tick> AddAssign for TimeSpan<T> {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl<T: Tick> Sub for TimeSpan<T> {
    type Output = TimeSpan<T>;

    fn sub(self, rhs: Self) -> Self::Output {
        TimeSpan::from_ticks(self.0 - rhs.0)
    }
}

impl<T: Tick> SubAssign for TimeSpan<T> {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
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
    fn abs() {
        let ts = TimeSpan::<TestTick>::from_ticks(-10);

        assert_eq!(-ts.0, ts.abs().0);
    }

    #[test]
    fn as_secs() {
        let secs = TimeSpan::<TestTick>::from_parts(TimeSpanParts {
            days: 1,
            hours: 2,
            mins: 3,
            secs: 4,
            millis: 5,
        })
        .as_secs();
        assert_eq!(1 * 86400 + 2 * 3600 + 3 * 60 + 4, secs);
    }

    #[test]
    fn as_millis() {
        let millis = TimeSpan::<TestTick>::from_parts(TimeSpanParts {
            days: 1,
            hours: 2,
            mins: 3,
            secs: 4,
            millis: 5,
        })
        .as_millis();
        assert_eq!(
            1 * 86400 * 1000 + 2 * 3600 * 1000 + 3 * 60 * 1000 + 4 * 1000 + 5,
            millis
        );
    }
}
