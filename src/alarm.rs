use core::marker::PhantomData;

use crate::{AlarmTimer, Tick, TimeSpan};

/// An alarm is backed by a timer and provides infinite timeout capabilites.
pub struct Alarm<Timer: AlarmTimer<T, A>, T: Tick, A> {
    timer: Timer,
    tick: PhantomData<T>,
    adapter: PhantomData<A>,
}

impl<Timer: AlarmTimer<T, A>, T: Tick, A: Send> Alarm<Timer, T, A> {
    pub const MAX: u32 = Timer::MAX;

    pub fn new(timer: Timer) -> Self {
        Self {
            timer,
            tick: PhantomData,
            adapter: PhantomData,
        }
    }

    pub fn counter(&self) -> u32 {
        self.timer.counter()
    }

    fn counter_add(base: u32, duration: u32) -> u32 {
        assert!(base <= Timer::MAX);
        assert!(duration <= Timer::MAX);
        ((base as u64 + duration as u64) % Timer::PERIOD) as u32
    }

    async fn sleep_from(&mut self, mut base: u32, duration: TimeSpan<T>) {
        let mut remaining = duration.0;

        // The maximum delay is half the counters increment.
        // This ensures that we can hit the actual fire time directly when the last timeout is setup.
        let half_period = (Timer::PERIOD / 2) as u32;

        while remaining >= Timer::PERIOD {
            // We can setup the final time
            let compare = Self::counter_add(base, half_period);
            self.timer.next(compare).await;
            base = compare;
            remaining -= half_period as u64;
        }

        if remaining > 0 {
            let compare = Self::counter_add(base, remaining as u32);
            self.timer.next(compare).await;
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
        let timer = FakeAlarmTimer {
            counter: 4,
            running: false,
            compares: Vec::new(),
        };
        let mut alarm = Alarm::new(timer);

        alarm
            .sleep_from(alarm.counter(), TimeSpan::from_ticks(9))
            .await;

        assert_eq!(vec![3], alarm.timer.compares);
    }

    #[async_test]
    async fn sleep_a_period() {
        let timer = FakeAlarmTimer {
            counter: 4,
            running: false,
            compares: Vec::new(),
        };
        let mut alarm = Alarm::new(timer);

        alarm
            .sleep_from(alarm.counter(), TimeSpan::from_ticks(10))
            .await;

        assert_eq!(vec![9, 4], alarm.timer.compares);
    }

    #[async_test]
    async fn sleep_more_than_a_period() {
        let timer = FakeAlarmTimer {
            counter: 4,
            running: false,
            compares: Vec::new(),
        };
        let mut alarm = Alarm::new(timer);

        alarm
            .sleep_from(alarm.counter(), TimeSpan::from_ticks(21))
            .await;

        assert_eq!(vec![9, 4, 9, 5], alarm.timer.compares);
    }

    #[test]
    fn sleep_drop() {
        let timer = FakeAlarmTimer {
            counter: 4,
            running: false,
            compares: Vec::new(),
        };
        let mut alarm = Alarm::new(timer);

        let sleep = alarm.sleep_from(alarm.counter(), TimeSpan::from_ticks(123));
        drop(sleep);

        assert!(!alarm.timer.running);
    }
}
