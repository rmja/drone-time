use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use crate::Tick;

pub trait AlarmTimer<T: Tick, A>: Send {
    /// AlarmTimer stop handler.
    type Stop: AlarmTimerStop;

    /// The maximum counter value.
    const MAX: u32;

    /// The timer period.
    const PERIOD: u64 = Self::MAX as u64 + 1;

    /// Get the current counter value of the timer.
    fn counter(&self) -> u32;

    /// Returns a future that resolves when the timer counter is equal to `compare`.
    /// Note that compare is not a duration but an absolute timestamp.
    fn next(&mut self, compare: u32) -> AlarmTimerNext<'_, Self::Stop>;
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

    impl AlarmTimer<FakeAlarmTimer, FakeAlarmTimer> for FakeAlarmTimer {
        type Stop = Self;
        const MAX: u32 = 9;

        fn counter(&self) -> u32 {
            self.counter
        }

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