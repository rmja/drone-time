use core::{cell::RefCell, future::Future, marker::PhantomData, pin::Pin, task::{Context, Poll, Waker}};

use crate::{AlarmTimer, Tick, TimeSpan};
use alloc::{collections::VecDeque, rc::Rc, sync::Arc};
use drone_core::sync::Mutex;
use futures::prelude::*;

/// An alarm is backed by a timer and provides infinite timeout capabilites and multiple simultaneously running timeouts.
pub struct Alarm<Timer: AlarmTimer<T, A>, T: Tick, A> {
    timer: Timer,
    subscriptions: VecDeque<Arc<Mutex<Box<Subscription<T>>>>>,
    // state: Arc<Mutex<State<T, A>>>,
    tick: PhantomData<T>,
    adapter: PhantomData<A>,
}

// struct State<'a, T: Tick, A> {
//     running: Option<Pin<Box<dyn Future<Output = ()>>>>,
//     adapter: PhantomData<A>,
// }

pub struct Subscription<T: Tick>  {
    remaining: TimeSpan<T>,
    waker: Option<Waker>,
}

pub struct SubscriptionGuard<T: Tick> {
    // subscriptions: Option<&'a RefCell<VecDeque<Arc<Mutex<Box<Subscription<T>>>>>>>,
    inner: Arc<Mutex<Box<Subscription<T>>>>,
    tick: PhantomData<T>,
}

impl<T: Tick + 'static> Future for SubscriptionGuard<T> {
    type Output = ();

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Self::Output> {
        let mut inner = self.inner.try_lock().unwrap();
        if inner.remaining.0 == 0 {
            // Timeout has already occured.
            Poll::Ready(())
        }
        else {
            // Copy the waker to the subscription so that we can wake it when it is time.
            inner.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

impl<T: Tick> Drop for SubscriptionGuard<T> {
    fn drop(&mut self) {
        // let subs = self.inner.subscriptions.unwrap().borrow_mut();
        // let index = subs.into_iter().position(|x| core::ptr::eq(x, self)).unwrap();
        // subs.remove(index);
    }
}

impl<Timer: AlarmTimer<T, A>, T: Tick + 'static, A: Send> Alarm<Timer, T, A> {
    pub const MAX: u32 = Timer::MAX;

    /// Create a new `Alarm` backed by a hardware timer.
    pub fn new(timer: Timer) -> Self {
        Self {
            timer,
            subscriptions: VecDeque::new(),
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
        let sub = Arc::new(Mutex::new(Box::new(Subscription {
            remaining: duration,
            waker: None,
        })));

        let index = Self::get_insert_index(&self.subscriptions, duration);
        self.subscriptions.insert(index, sub.clone());

        // let subs = self.subscriptions;
        // sub.subscriptions = Some(&subs);

        // if index == 0 {
        //     self.set_running(base, duration);
        // }

        SubscriptionGuard {
            // subscriptions: Some(&self.subscriptions),
            inner: sub,
            tick: PhantomData,
        }
    }

    fn get_insert_index(subscriptions: &VecDeque<Arc<Mutex<Box<Subscription<T>>>>>, remaining: TimeSpan<T>) -> usize {
        let mut index = 0;
        for sub in subscriptions.iter() {
            if remaining < sub.try_lock().unwrap().remaining {
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
