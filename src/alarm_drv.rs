use core::marker::PhantomData;

use crate::{alarm::*, AlarmTimer};
use async_trait::async_trait;

pub struct AlarmDrv<Timer: AlarmTimer<A>, A> {
    timer: Timer,
    adapter: PhantomData<A>,
}

impl<Timer: AlarmTimer<A>, A: Send> AlarmDrv<Timer, A> {
    pub fn new(timer: Timer) -> Self {
        Self {
            timer,
            adapter: PhantomData,
        }
    }

    fn counter_add(base: u32, duration: u32) -> u32 {
        assert!(base <= Timer::MAX);
        assert!(duration <= Timer::MAX);
        ((base as u64 + duration as u64) % Timer::PERIOD) as u32
    }
}

#[async_trait]
impl<Timer: AlarmTimer<A>, A: Send> Alarm for AlarmDrv<Timer, A> {
    async fn sleep(&mut self, duration: u64) {
        let mut base = self.timer.counter();
        let mut remaining = duration;

        // The maximum delay is half the counters increment.
        // This ensures that we can hit the actual fire time directly when the last timeout is setup.
        let half_period = (Timer::PERIOD / 2) as u32;

        while remaining >= Timer::PERIOD {
            // We can setup the final time
            let compare = Self::counter_add(base, half_period);
            self.timer.next(compare);
            base = compare;
            remaining -= half_period as u64;
        }

        if remaining > 0 {
            let compare = Self::counter_add(base, remaining as u32);
            self.timer.next(compare);
        }
    }
}

#[cfg(test)]
pub mod tests {
    use futures::future;
    use futures_await_test::async_test;

    use crate::{AlarmTimerNext, AlarmTimerStop};

    use super::*;

    struct TestTimer {
        counter: u32,
        running: bool,
        compares: Vec<u32>,
    }

    impl AlarmTimer<TestTimer> for TestTimer {
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

    impl AlarmTimerStop for TestTimer {
        fn stop(&mut self) {
            assert!(self.running);
            self.running = false;
        }
    }

    #[async_test]
    async fn sleep_less_than_a_period() {
        let timer = TestTimer {
            counter: 4,
            running: false,
            compares: Vec::new(),
        };
        let mut alarm = AlarmDrv::new(timer);

        alarm.sleep(9).await;

        assert_eq!(vec![3], alarm.timer.compares);
    }

    #[async_test]
    async fn sleep_a_period() {
        let timer = TestTimer {
            counter: 4,
            running: false,
            compares: Vec::new(),
        };
        let mut alarm = AlarmDrv::new(timer);

        alarm.sleep(10).await;

        assert_eq!(vec![9, 4], alarm.timer.compares);
    }

    #[async_test]
    async fn sleep_more_than_a_period() {
        let timer = TestTimer {
            counter: 4,
            running: false,
            compares: Vec::new(),
        };
        let mut alarm = AlarmDrv::new(timer);

        alarm.sleep(21).await;

        assert_eq!(vec![9, 4, 9, 5], alarm.timer.compares);
    }

    #[test]
    fn sleep_drop() {
        let timer = TestTimer {
            counter: 4,
            running: false,
            compares: Vec::new(),
        };
        let mut alarm = AlarmDrv::new(timer);

        let sleep = alarm.sleep(123);
        drop(sleep);

        assert!(!alarm.timer.running);
    }
}
