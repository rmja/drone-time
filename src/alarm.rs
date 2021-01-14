use core::{future::Future, marker::PhantomData, pin::Pin, sync::atomic::{AtomicUsize, Ordering}, task::{Context, Poll, Waker}};

use crate::{AlarmTimer, Tick, TimeSpan, atomic_box::AtomicBox, atomic_option_box::AtomicOptionBox};
use alloc::{collections::VecDeque, sync::Arc};
use drone_core::sync::Mutex;
use futures::prelude::*;

/// An alarm is backed by a timer and provides infinite timeout capabilites and multiple simultaneously running timeouts.
pub struct Alarm<Timer: AlarmTimer<T, A>, T: Tick, A> {
    timer: Timer,
    state: Arc<Mutex<State<T>>>,
    tick: PhantomData<T>,
    adapter: PhantomData<A>,
}

struct State<T: Tick> {
    running: Option<Pin<Box<dyn Future<Output = ()>>>>,
    subscriptions: VecDeque<Subscription<T>>,
}

pub struct Subscription<T: Tick> {
    remaining: TimeSpan<T>,
    state: Arc<SubscriptionState>,
}

pub struct SubscriptionState {
    state: AtomicUsize,
    waker: AtomicOptionBox<Waker>,
}

pub struct SubscriptionGuard {
    inner: Arc<SubscriptionState>,
}

const ADDED: usize = 0;
const WAKEABLE: usize = 1;
const COMPLETED: usize = 2;
const DROPPED: usize = 3;

impl Future for SubscriptionGuard {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner = self.inner.clone();

        let waker = cx.waker().clone();

        // Copy the waker to the subscription so that we can wake it when it is time.
        inner.waker.store(Some(Box::new(waker)), Ordering::AcqRel);

        let old = inner.state.swap(WAKEABLE, Ordering::AcqRel);
        assert!(old != DROPPED);
        if old == COMPLETED {
            // Timeout has already occured.
            inner.waker.take(Ordering::AcqRel);
            inner.state.store(COMPLETED, Ordering::Release);
            Poll::Ready(())
        }
        else {
            Poll::Pending
        }
    }
}

impl Drop for SubscriptionGuard {
    fn drop(&mut self) {
        self.inner.state.store(DROPPED, Ordering::Release);
    }
}

impl<Timer: AlarmTimer<T, A>, T: Tick + 'static, A: Send> Alarm<Timer, T, A> {
    pub const MAX: u32 = Timer::MAX;

    /// Create a new `Alarm` backed by a hardware timer.
    pub fn new(timer: Timer) -> Self {
        Self {
            timer,
            tick: PhantomData,
            state: Arc::new(Mutex::new(State {
                 running: None,
                 subscriptions: VecDeque::new(),
            })),
            adapter: PhantomData,
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
        let sub_state = Arc::new(SubscriptionState {
            state: AtomicUsize::new(ADDED),
            waker: AtomicOptionBox::new(None),
        });
        let sub = Subscription {
            remaining: duration,
            state: sub_state.clone(),
        };

        let mut state = self.state.try_lock().unwrap();
        let index = state.get_insert_index(duration);
        state.remove_dropped();
        state.subscriptions.insert(index, sub);
        drop(state);

        if index == 0 {
            self.set_running(base, duration);
        }

        SubscriptionGuard { inner: sub_state }
    }

    fn set_running(&mut self, base: u32, duration: TimeSpan<T>) {
        let state = self.state.clone();
        let future = self.sleep_timer(base, duration).then(move |_| {
            let mut state = state.try_lock().unwrap();

            state.remove_dropped();

            // Set the remaining time for each subscription.
            for s in state.subscriptions.iter_mut() {
                s.remaining -= duration;

                if s.remaining.0 == 0 {
                    // Wake the future for the subscription.
                    let old = s.state.state.compare_and_swap(WAKEABLE, COMPLETED, Ordering::AcqRel);
                    if old == WAKEABLE {
                        let waker = s.state.waker.take(Ordering::AcqRel).unwrap();
                        waker.wake();
                    }
                }
            }

            state.subscriptions.retain(|x| x.remaining.0 > 0);

            let next = state.subscriptions.front();
            if let Some(next) = next {
                let base = Self::counter_add(base, (duration.0 % Timer::PERIOD) as u32);
                // self.set_running(base, next.remaining);
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

impl<T: Tick> State<T> {
    fn get_insert_index(&self, remaining: TimeSpan<T>) -> usize {
        let mut index = 0;
        for sub in self.subscriptions.iter() {
            if remaining < sub.remaining {
                break;
            }
            index += 1;
        }
        index
    }

    fn remove_dropped(&mut self) {
        self.subscriptions
            .retain(|x| x.state.state.load(Ordering::Relaxed) != DROPPED);
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
