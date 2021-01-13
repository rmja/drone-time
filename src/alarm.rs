use core::{cell::RefCell, future::Future, marker::PhantomData, pin::Pin, sync::atomic::{AtomicBool, Ordering}, task::{Context, Poll, Waker}};

use crate::{AlarmTimer, Tick, TimeSpan};
use alloc::{collections::VecDeque, rc::Rc, sync::Arc};
use drone_core::sync::Mutex;
use futures::prelude::*;

/// An alarm is backed by a timer and provides infinite timeout capabilites and multiple simultaneously running timeouts.
pub struct Alarm<Timer: AlarmTimer<T, A>, T: Tick, A> {
    timer: Timer,
    subscriptions: Arc<Mutex<VecDeque<Arc<SubscriptionState<T>>>>>,
    // state: Arc<Mutex<State<T, A>>>,
    tick: PhantomData<T>,
    adapter: PhantomData<A>,
}

// struct State<'a, T: Tick, A> {
//     running: Option<Pin<Box<dyn Future<Output = ()>>>>,
//     adapter: PhantomData<A>,
// }

pub struct SubscriptionState<T: Tick>  {
    remaining: TimeSpan<T>,
    dropped: AtomicBool,
    waker: Mutex<Option<Waker>>,
}

pub struct SubscriptionGuard<T: Tick> {
    subscriptions: Arc<Mutex<VecDeque<Arc<SubscriptionState<T>>>>>,
    inner: Arc<SubscriptionState<T>>,
    tick: PhantomData<T>,
}

impl<T: Tick + 'static> Future for SubscriptionGuard<T> {
    type Output = ();

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Self::Output> {
        let waker = self.inner.waker.try_lock();
        if let Some(mut waker) = waker {
            let inner = self.inner.clone();
            if inner.remaining.0 == 0 {
                // Timeout has already occured.
                Poll::Ready(())
            }
            else {
                // Copy the waker to the subscription so that we can wake it when it is time.
                *waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
        else {
            // Wake immediately to retry lock.
            cx.waker().clone().wake();
            Poll::Pending
        }
    }
}

impl<T: Tick> Drop for SubscriptionGuard<T> {
    fn drop(&mut self) {
        self.inner.dropped.store(true, Ordering::Release);
    }
}

impl<Timer: AlarmTimer<T, A>, T: Tick + 'static, A: Send> Alarm<Timer, T, A> {
    pub const MAX: u32 = Timer::MAX;

    /// Create a new `Alarm` backed by a hardware timer.
    pub fn new(timer: Timer) -> Self {
        Self {
            timer,
            subscriptions: Arc::new(Mutex::new(VecDeque::new())),
            tick: PhantomData,
            adapter: PhantomData,
            // state: Arc::new(Mutex::new(State {
            //     running: None,
            //     subscriptions: VecDeque::new(),
            //     adapter: PhantomData,
            // }))
        }
    }

    /// Get the current counter value of the 
    pub fn counter(&self) -> u32 {
        self.timer.counter()
    }

    /// Get a future that completes after a delay of length `duration`.
    pub fn sleep(&mut self, duration: TimeSpan<T>) -> impl Future<Output = ()> {
        self.sleep_from(self.counter(), duration)
    }

    /// Get a future that completes after a delay of length `duration` relative to the counter value `base`.
    pub fn sleep_from(&mut self, base: u32, duration: TimeSpan<T>) -> impl Future<Output = ()> {
        let index = 2;
        let sub = Arc::new(SubscriptionState {
            remaining: duration,
            dropped: AtomicBool::new(false),
            waker: Mutex::new(None),
        });

        let index = self.get_insert_index(duration);
        self.subscriptions.try_lock().unwrap().insert(index, sub.clone());

        // if index == 0 {
        //     self.set_running(base, duration);
        // }

        SubscriptionGuard {
            subscriptions: self.subscriptions.clone(),
            inner: sub,
            tick: PhantomData,
        }
    }

    fn get_insert_index(&self, remaining: TimeSpan<T>) -> usize {
        let mut index = 0;
        for sub in self.subscriptions.try_lock().unwrap().iter() {
            if remaining < sub.remaining {
                break;
            }
            index += 1;
        }
        index
    }

    // fn set_running(&mut self, base: u32, duration: TimeSpan<T>) {
    //     let state = self.state.clone();
    //     let future = self.sleep_timer(base, duration).then(move |f| {
    //         let mut state = state.try_lock().unwrap();
    //         // Set the remaining time for each subscription.
    //         for s in state.subscriptions.iter_mut() {
    //             s.remaining -= duration;

    //             if s.remaining.0 == 0 {
    //                 state.subscriptions.pop_front();

    //                 // Wake the future for the subscription.
    //                 s.waker.unwrap().wake();
    //             }
    //         }

    //         let next = state.subscriptions.front();
    //         if let Some(next) = next {
    //             let base = Self::counter_add(base, (duration.0 % Timer::PERIOD) as u32);
    //             // self.set_running(base, next.remaining);
    //         }

    //         future::ready(())
    //     });

    //     // self.running = Some(Box::pin(future));
    // }

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
