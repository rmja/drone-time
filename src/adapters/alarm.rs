use crate::{Tick, TimeSpan};
use async_trait::async_trait;
use core::convert::TryFrom;

pub trait AlarmCounter<T: Tick, A>
where
    Self: Sync,
{
    /// Get the current counter value of the timer.
    fn value(&self) -> u32;
}

#[async_trait]
pub trait AlarmTimer<T: Tick + 'static, A: 'static>: Send {
    /// The maximum counter value.
    const MAX: u32;

    /// The timer period.
    const PERIOD: u64 = Self::MAX as u64 + 1;
    const HALF_PERIOD: u32 = (Self::PERIOD / 2) as u32;

    /// Returns a future that resolves when the timer counter is equal to `compare`.
    /// Note that compare is not a duration but an absolute timestamp.
    /// The returned future is resolved immediately if `soon` and
    /// the compare value has already passed with at most `PERIOD/2` ticks.
    async fn next(&mut self, compare: u32, soon: bool);

    async fn sleep(&mut self, mut base: u32, duration: TimeSpan<T>) {
        let mut remaining = u64::try_from(duration.0).expect("duration must be non negative");
        let soon = remaining < Self::PERIOD / 2;

        // The maximum delay is half the counters increment.
        // This ensures that we can hit the actual fire time directly when the last timeout is setup.
        while remaining >= Self::PERIOD {
            // We can setup the final time
            let compare = Self::counter_add(base, Self::HALF_PERIOD);
            self.next(compare, false).await;
            base = compare;
            remaining -= Self::HALF_PERIOD as u64;
        }

        if remaining > 0 {
            let compare = Self::counter_add(base, remaining as u32);
            self.next(compare, soon).await;
        }
    }

    fn counter_add(base: u32, duration: u32) -> u32 {
        assert!(base <= Self::MAX);
        assert!(duration <= Self::MAX);
        ((base as u64 + duration as u64) % Self::PERIOD) as u32
    }
}

#[cfg(test)]
pub mod fakes {
    use futures::future;

    use super::*;

    pub struct FakeDrv;

    pub struct FakeAlarmCounter(pub(crate) u32);

    pub struct FakeAlarmTimer {
        pub(crate) running: bool,
        pub(crate) compares: Vec<u32>,
    }

    impl Tick for FakeDrv {
        const FREQ: u32 = 1;
    }

    impl AlarmCounter<FakeDrv, FakeDrv> for FakeAlarmCounter {
        fn value(&self) -> u32 {
            self.0
        }
    }

    impl AlarmTimer<FakeDrv, FakeDrv> for FakeAlarmTimer {
        type Stop = Self;
        const MAX: u32 = 9;

        fn next(&mut self, compare: u32) -> AlarmTimerNext<'_, Self::Stop> {
            assert!(compare <= Self::MAX);
            assert!(!self.running);
            self.compares.push(compare);
            self.running = true;
            let fut = Box::pin(future::ready(()));
            AlarmTimerNext::new(self, fut)
        }
    }

    impl AlarmTimerStop for FakeAlarmTimer {
        fn stop(&mut self) {
            assert!(self.running);
            self.running = false;
        }
    }
}

#[cfg(test)]
pub mod tests {
    use futures::future;
    use futures_await_test::async_test;

    use crate::adapters::alarm::fakes::FakeAlarmTimer;

    use super::*;

    #[async_test]
    async fn sleep_less_than_a_period() {
        let mut timer = FakeAlarmTimer {
            running: false,
            compares: Vec::new(),
        };

        timer.sleep(4, TimeSpan::from_ticks(9)).await;

        assert_eq!(vec![3], timer.compares);
    }

    #[async_test]
    async fn sleep_a_period() {
        let mut timer = FakeAlarmTimer {
            running: false,
            compares: Vec::new(),
        };

        timer.sleep(4, TimeSpan::from_ticks(10)).await;

        assert_eq!(vec![9, 4], timer.compares);
    }

    #[async_test]
    async fn sleep_more_than_a_period() {
        let mut timer = FakeAlarmTimer {
            running: false,
            compares: Vec::new(),
        };

        timer.sleep(4, TimeSpan::from_ticks(21)).await;

        assert_eq!(vec![9, 4, 9, 5], timer.compares);
    }

    #[test]
    fn sleep_drop() {
        let mut timer = FakeAlarmTimer {
            running: false,
            compares: Vec::new(),
        };

        let sleep = timer.sleep(4, TimeSpan::from_ticks(123));
        drop(sleep);

        assert!(!timer.running);
    }
}
