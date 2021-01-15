use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{Tick, TimeSpan};
use async_trait::async_trait;
use core::convert::TryFrom;

pub trait AlarmCounter<T: Tick, A> where Self: Sync {
    /// Get the current counter value of the timer.
    fn value(&self) -> u32;
}

#[async_trait]
pub trait AlarmTimer<T: Tick + 'static, A: 'static>: Send {
    /// AlarmTimer stop handler.
    type Stop: AlarmTimerStop;

    /// The maximum counter value.
    const MAX: u32;

    /// The timer period.
    const PERIOD: u64 = Self::MAX as u64 + 1;

    /// Returns a future that resolves when the timer counter is equal to `compare`.
    /// Note that compare is not a duration but an absolute timestamp.
    fn next(&mut self, compare: u32) -> AlarmTimerNext<'_, Self::Stop>;

    async fn sleep(&mut self, mut base: u32, duration: TimeSpan<T>) {
        let mut remaining = u64::try_from(duration.0).expect("duration must be non negative");

        // The maximum delay is half the counters increment.
        // This ensures that we can hit the actual fire time directly when the last timeout is setup.
        let half_period = (Self::PERIOD / 2) as u32;

        while remaining >= Self::PERIOD {
            // We can setup the final time
            let compare = Self::counter_add(base, half_period);
            self.next(compare).await;
            base = compare;
            remaining -= half_period as u64;
        }

        if remaining > 0 {
            let compare = Self::counter_add(base, remaining as u32);
            self.next(compare).await;
        }
    }

    fn counter_add(base: u32, duration: u32) -> u32 {
        assert!(base <= Self::MAX);
        assert!(duration <= Self::MAX);
        ((base as u64 + duration as u64) % Self::PERIOD) as u32
    }
}

/// AlarmTimer stop handler.
pub trait AlarmTimerStop: Send {
    /// Stop the timer.
    fn stop(&mut self);
}

pub struct AlarmTimerNext<'a, T: AlarmTimerStop> {
    stop: &'a mut T,
    future: Pin<Box<dyn Future<Output = ()> + Send + 'a>>,
}

impl<'a, T: AlarmTimerStop> AlarmTimerNext<'a, T> {
    /// Creates a new [`AlarmTimerNext`].
    pub fn new(stop: &'a mut T, future: Pin<Box<dyn Future<Output = ()> + Send + 'a>>) -> Self {
        Self { stop, future }
    }
}

impl<'a, T: AlarmTimerStop> Future for AlarmTimerNext<'a, T> {
    type Output = ();

    #[inline]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        self.future.as_mut().poll(cx)
    }
}

impl<'a, T: AlarmTimerStop> Drop for AlarmTimerNext<'a, T> {
    #[inline]
    fn drop(&mut self) {
        self.stop.stop();
    }
}

#[cfg(test)]
pub mod fakes {
    use futures::future;

    use super::*;

    pub struct FakeAlarmTimer {
        pub(crate) counter: u32,
        pub(crate) running: bool,
        pub(crate) compares: Vec<u32>,
    }

    impl Tick for FakeAlarmTimer {
        const FREQ: u32 = 1;
    }

    impl AlarmCounter<FakeAlarmTimer, FakeAlarmTimer> for FakeAlarmTimer {
        fn value(&self) -> u32 {
            self.counter
        }
    }

    impl AlarmTimer<FakeAlarmTimer, FakeAlarmTimer> for FakeAlarmTimer {
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
            counter: 4,
            running: false,
            compares: Vec::new(),
        };

        timer.sleep(timer.value(), TimeSpan::from_ticks(9)).await;

        assert_eq!(vec![3], timer.compares);
    }

    #[async_test]
    async fn sleep_a_period() {
        let mut timer = FakeAlarmTimer {
            counter: 4,
            running: false,
            compares: Vec::new(),
        };

        timer.sleep(timer.value(), TimeSpan::from_ticks(10)).await;

        assert_eq!(vec![9, 4], timer.compares);
    }

    #[async_test]
    async fn sleep_more_than_a_period() {
        let mut timer = FakeAlarmTimer {
            counter: 4,
            running: false,
            compares: Vec::new(),
        };

        timer.sleep(timer.value(), TimeSpan::from_ticks(21)).await;

        assert_eq!(vec![9, 4, 9, 5], timer.compares);
    }

    #[test]
    fn sleep_drop() {
        let mut timer = FakeAlarmTimer {
            counter: 4,
            running: false,
            compares: Vec::new(),
        };

        let sleep = timer.sleep(timer.value(), TimeSpan::from_ticks(123));
        drop(sleep);

        assert!(!timer.running);
    }
}
