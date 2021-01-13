use core::{future::Future, marker::PhantomData, pin::Pin};

use crate::{AlarmTimer, Tick, TimeSpan};
use alloc::{collections::VecDeque, sync::Arc};
use drone_core::sync::Mutex;
use futures::prelude::*;

/// An alarm is backed by a timer and provides infinite timeout capabilites and multiple simultaneously running timeouts.
pub struct Alarm<Timer: AlarmTimer<T, A>, T: Tick, A> {
    timer: Timer,
    running: Option<Pin<Box<dyn Future<Output = ()>>>>,
    subscriptions: Arc<Mutex<VecDeque<Subscription<T>>>>,
    adapter: PhantomData<A>,
}

struct Subscription<T: Tick> {
    remaining: TimeSpan<T>,
}

struct SubscriptionFuture;

impl Future for SubscriptionFuture {
    type Output = ();

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        todo!()
    }
}

impl<Timer: AlarmTimer<T, A>, T: Tick, A: Send> Alarm<Timer, T, A> {
    pub const MAX: u32 = Timer::MAX;

    pub fn new(timer: Timer) -> Self {
        Self {
            timer,
            running: None,
            subscriptions: Arc::new(Mutex::new(VecDeque::new())),
            adapter: PhantomData,
        }
    }

    pub fn counter(&self) -> u32 {
        self.timer.counter()
    }

    pub fn sleep(&mut self, duration: TimeSpan<T>) -> impl Future<Output = ()> {
        self.sleep_from(self.counter(), duration)
    }

    pub fn sleep_from(&mut self, base: u32, duration: TimeSpan<T>) -> impl Future<Output = ()> {
        let sub = Subscription {
            remaining: duration,
        };
        let mut subscriptions = self.subscriptions.try_lock().unwrap();
        let index = Self::get_insert_index(&subscriptions, duration);
        subscriptions.insert(index, sub);

        if index == 0 {
            let a = subscriptions.front().unwrap();
            // self.set_running(base, a);
        }

        SubscriptionFuture
    }

    fn get_insert_index(subs: &VecDeque<Subscription<T>>, remaining: TimeSpan<T>) -> usize {
        let mut index = 0;
        for sub in subs {
            if remaining < sub.remaining {
                break;
            }
            index += 1;
        }
        index
    }

    fn set_running(&mut self, base: u32, sub: &Subscription<T>) {
        let remaining = sub.remaining;

        let subs = self.subscriptions.clone();
        let future = self.sleep_timer(base, remaining).then(move |f| {
            let mut subs = subs.try_lock().unwrap();
            // Set the remaining time for each subscription.
            for s in subs.iter_mut() {
                s.remaining -= remaining;
            }

            // Wake all futures for subscriptions with remaining == 0.
            while !subs.is_empty() {
                let s = subs.front().unwrap();
                if s.remaining.0 == 0 {
                    subs.pop_front();

                    // Wake the future for the subscription.
                } else {
                    break;
                }
            }

            let next = subs.front();
            if let Some(next) = next {
                // self.set_running(next)
            }

            future::ready(())
        });

        // self.running = Some(Box::pin(future));
    }

    fn counter_add(base: u32, duration: u32) -> u32 {
        assert!(base <= Timer::MAX);
        assert!(duration <= Timer::MAX);
        ((base as u64 + duration as u64) % Timer::PERIOD) as u32
    }

    async fn sleep_timer(&mut self, mut base: u32, duration: TimeSpan<T>) {
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
    async fn sleep_timer_less_than_a_period() {
        let timer = FakeAlarmTimer {
            counter: 4,
            running: false,
            compares: Vec::new(),
        };
        let mut alarm = Alarm::new(timer);

        alarm
            .sleep_timer(alarm.counter(), TimeSpan::from_ticks(9))
            .await;

        assert_eq!(vec![3], alarm.timer.compares);
    }

    #[async_test]
    async fn sleep_timer_a_period() {
        let timer = FakeAlarmTimer {
            counter: 4,
            running: false,
            compares: Vec::new(),
        };
        let mut alarm = Alarm::new(timer);

        alarm
            .sleep_timer(alarm.counter(), TimeSpan::from_ticks(10))
            .await;

        assert_eq!(vec![9, 4], alarm.timer.compares);
    }

    #[async_test]
    async fn sleep_timer_more_than_a_period() {
        let timer = FakeAlarmTimer {
            counter: 4,
            running: false,
            compares: Vec::new(),
        };
        let mut alarm = Alarm::new(timer);

        alarm
            .sleep_timer(alarm.counter(), TimeSpan::from_ticks(21))
            .await;

        assert_eq!(vec![9, 4, 9, 5], alarm.timer.compares);
    }

    #[test]
    fn sleep_timer_drop() {
        let timer = FakeAlarmTimer {
            counter: 4,
            running: false,
            compares: Vec::new(),
        };
        let mut alarm = Alarm::new(timer);

        let sleep = alarm.sleep_timer(alarm.counter(), TimeSpan::from_ticks(123));
        drop(sleep);

        assert!(!alarm.timer.running);
    }
}
