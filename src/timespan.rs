use core::marker::PhantomData;

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
    pub fn parts(&self) -> TimeSpanParts {
        let mut jiffies = self.0;
        
        let days = jiffies / TimeSpan::<Clock>::jiffies_per_day();
        jiffies -= days * TimeSpan::<Clock>::jiffies_per_day();

        let hours = jiffies / TimeSpan::<Clock>::jiffies_per_hour();
        jiffies -= hours * TimeSpan::<Clock>::jiffies_per_hour();

        let minutes = jiffies / TimeSpan::<Clock>::jiffies_per_minute();
        jiffies -= minutes * TimeSpan::<Clock>::jiffies_per_minute();

        let seconds = jiffies / TimeSpan::<Clock>::jiffies_per_second();
        jiffies -= seconds * TimeSpan::<Clock>::jiffies_per_second();

        let milliseconds = (jiffies * 1000) / TimeSpan::<Clock>::jiffies_per_second();

        TimeSpanParts {
            days: days as u16,
            hours: hours as u16,
            minutes: minutes as u8,
            seconds: seconds as u8,
            milliseconds: milliseconds as u16,
        }
    }

    pub fn total_seconds(&self) -> u32 {
        (self.0 / TimeSpan::<Clock>::jiffies_per_second()) as u32
    }

    fn jiffies_per_second() -> u64 {
        Clock::freq() as u64
    }

    fn jiffies_per_minute() -> u64 {
        Clock::freq() as u64 * 60
    }

    fn jiffies_per_hour() -> u64 {
        Clock::freq() as u64 * 3600
    }

    fn jiffies_per_day() -> u64 {
        Clock::freq() as u64 * 86400
    }
}