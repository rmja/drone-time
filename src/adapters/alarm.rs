use crate::{Tick, TimeSpan};
use async_trait::async_trait;
use core::convert::TryFrom;

/// The alarm timer counter.
/// The counter must be monotonically increasing.
pub trait AlarmCounter<T: Tick, A>: Send + Sync {
    /// Get the current counter value of the timer.
    fn value(&self) -> u32;

    fn spin(&self, cycles: u32);
}

pub enum AlarmTimerMode {
    /// The preferred timer mode.
    /// The timer is always running between between 0 <= counter <= MAX, even when compare value is currently configured.
    AlwaysRunning,
    /// The alternate timer mode to be used when the timer is not always running between 0 <= counter <= MAX.
    /// This mode is less robust when the duration exceeds a period, as jitter is introduced when setting up the next interrupt.
    OneShotOnly,
}

#[async_trait]
pub trait AlarmTimer<T: Tick, A: 'static>: Send {
    /// The maximum counter value.
    const MAX: u32;

    /// The timer period.
    const PERIOD: u64 = Self::MAX as u64 + 1;
    const HALF_PERIOD: u32 = (Self::PERIOD / 2) as u32;

    /// Whether the timer can be assumed to be always running.
    const MODE: AlarmTimerMode = AlarmTimerMode::AlwaysRunning;

    /// Returns a future that resolves when the timer counter is equal to `compare`.
    /// Note that compare is not a duration but an absolute timestamp.
    /// The returned future is resolved immediately if `soon` and
    /// the compare value has already passed with at most `PERIOD/2` ticks.
    ///
    /// This function is only ever called if MODE == AlwaysRunning.
    async fn next(&mut self, _compare: u32, _soon: bool) {
        panic!("next() must be implemented when MODE == AlwaysRunning");
    }

    /// Returns a future that resolves when `duration` time is elapsed.
    ///
    /// This function is only ever called if MODE == OneShotOnly.
    async fn delay(&mut self, _duration: u32) {
        panic!("delay() must be implemented when MODE == OneShotOnly");
    }

    async fn sleep(&mut self, mut base: u32, duration: TimeSpan<T>) {
        let mut remaining = u64::try_from(duration.0).expect("duration must be non negative");
        match Self::MODE {
            AlarmTimerMode::AlwaysRunning => {
                let soon = remaining < Self::PERIOD / 2;

                // The maximum delay is half the counters increment.
                // This ensures that we can hit the actual fire time directly when the last timeout is setup.
                while remaining >= Self::PERIOD {
                    // We cannot setup the final time
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
            AlarmTimerMode::OneShotOnly => {
                assert_eq!(0, base);
                while remaining >= Self::MAX as u64 {
                    self.delay(Self::MAX).await;
                    remaining -= Self::MAX as u64;
                }
                if remaining > 0 {
                    self.delay(remaining as u32).await;
                }
            }
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
    use super::*;

    pub struct Adapter;

    pub struct FakeAlarmCounter(pub(crate) u32);

    pub struct FakeAlarmTimer {
        pub(crate) compares: Vec<u32>,
    }

    pub struct FakeTick;
    impl Tick for FakeTick {
        const FREQ: u32 = 1;
    }

    impl AlarmCounter<FakeTick, Adapter> for FakeAlarmCounter {
        fn value(&self) -> u32 {
            self.0
        }

        fn spin(&self, _cycles: u32) {}
    }

    #[async_trait]
    impl AlarmTimer<FakeTick, Adapter> for FakeAlarmTimer {
        const MAX: u32 = 9;

        async fn next(&mut self, compare: u32, _soon: bool) {
            assert!(compare <= Self::MAX);
            self.compares.push(compare);
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::adapters::alarm::fakes::FakeAlarmTimer;
    use futures_await_test::async_test;

    #[async_test]
    async fn sleep_less_than_a_period() {
        let mut timer = FakeAlarmTimer {
            compares: Vec::new(),
        };

        timer.sleep(4, TimeSpan::from_ticks(9)).await;

        assert_eq!(vec![3], timer.compares);
    }

    #[async_test]
    async fn sleep_a_period() {
        let mut timer = FakeAlarmTimer {
            compares: Vec::new(),
        };

        timer.sleep(4, TimeSpan::from_ticks(10)).await;

        assert_eq!(vec![9, 4], timer.compares);
    }

    #[async_test]
    async fn sleep_more_than_a_period() {
        let mut timer = FakeAlarmTimer {
            compares: Vec::new(),
        };

        timer.sleep(4, TimeSpan::from_ticks(21)).await;

        assert_eq!(vec![9, 4, 9, 5], timer.compares);
    }

    #[test]
    fn sleep_drop() {
        let mut timer = FakeAlarmTimer {
            compares: Vec::new(),
        };

        let sleep = timer.sleep(4, TimeSpan::from_ticks(123));
        drop(sleep);
    }
}
