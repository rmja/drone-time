use core::{
    cell::RefCell,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
    task::{Context, Poll, Waker},
};

use crate::{
    atomic_box::AtomicBox, atomic_option_box::AtomicOptionBox, AlarmTimer, Tick, TimeSpan,
};
use alloc::{collections::VecDeque, sync::Arc};
use drone_core::sync::Mutex;
use futures::prelude::*;

/// An alarm is backed by a timer and provides infinite timeout capabilites and multiple simultaneously running timeouts.
pub struct Alarm<Timer: AlarmTimer<T, A>, T: Tick + 'static, A: 'static> {
    timer: RefCell<Timer>,
    state: Arc<Mutex<SharedState<Timer, T, A>>>,
}

struct SharedState<Timer: AlarmTimer<T, A>, T: Tick + 'static, A: 'static> {
    running: Option<Pin<Box<dyn Future<Output = ()>>>>,
    subscriptions: VecDeque<Subscription<T>>,
    adapter: PhantomData<A>,
    timer: PhantomData<Timer>,
}

pub struct Subscription<T: Tick> {
    remaining: TimeSpan<T>,
    inner: Arc<SubscriptionState>,
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
        } else {
            Poll::Pending
        }
    }
}

impl Drop for SubscriptionGuard {
    fn drop(&mut self) {
        self.inner.state.store(DROPPED, Ordering::Release);
    }
}

impl<Timer: AlarmTimer<T, A> + 'static, T: Tick + 'static, A: Send + 'static> Alarm<Timer, T, A> {
    pub const MAX: u32 = Timer::MAX;

    /// Create a new `Alarm` backed by a hardware timer.
    pub fn new(timer: Timer) -> Self {
        Self {
            timer: RefCell::new(timer),
            state: Arc::new(Mutex::new(SharedState {
                // timer: RefCell::new(timer),
                running: None,
                subscriptions: VecDeque::new(),
                adapter: PhantomData,
                timer: PhantomData,
            })),
        }
    }

    /// Get the current counter value of the
    pub fn counter(&self) -> u32 {
        // self.state.try_lock().unwrap().timer.borrow().counter()
        123
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
            inner: sub_state.clone(),
        };

        let arc = self.state.clone();
        let mut shared = arc.try_lock().unwrap();
        let index = shared.get_insert_index(duration);
        shared.remove_dropped();
        shared.subscriptions.insert(index, sub);
        if index == 0 {
            let mut timer = self.timer.borrow_mut();
            // let future =
            //     SharedState::create_running(&mut timer, self.state.clone(), base, duration);
            // shared.running = Some(Box::pin(future));
        }

        SubscriptionGuard { inner: sub_state }
    }
}

impl<Timer: AlarmTimer<T, A>, T: Tick + 'static, A: 'static> SharedState<Timer, T, A> {
    async fn create_running(
        timer: &mut Timer,
        arc: Arc<Mutex<SharedState<Timer, T, A>>>,
        base: u32,
        duration: TimeSpan<T>,
    ) {
        timer
            .sleep(base, duration)
            .then(move |_| {
                let mut shared = arc.try_lock().unwrap();

                shared.remove_dropped();

                // Set the remaining time for each subscription.
                for s in shared.subscriptions.iter_mut() {
                    s.remaining -= duration;

                    if s.remaining.0 == 0 {
                        // Wake the future for the subscription.
                        let old =
                            s.inner
                                .state
                                .compare_and_swap(WAKEABLE, COMPLETED, Ordering::AcqRel);
                        if old == WAKEABLE {
                            let waker = s.inner.waker.take(Ordering::AcqRel).unwrap();
                            waker.wake();
                        }
                    }
                }

                shared.subscriptions.retain(|x| x.remaining.0 > 0);

                if let Some(next) = shared.subscriptions.front() {
                    let base = Timer::counter_add(base, (duration.0 % Timer::PERIOD) as u32);
                    let duration = next.remaining;
                    // shared.running = Some(shared.create_running(arc.clone(), base, duration));
                }

                future::ready(())
            })
            .await;
    }

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
            .retain(|x| x.inner.state.load(Ordering::Relaxed) != DROPPED);
    }
}

#[cfg(test)]
pub mod tests {
    use futures::future;
    use futures_await_test::async_test;

    use crate::adapters::alarm::fakes::FakeAlarmTimer;

    use super::*;

    #[async_test]
    async fn whoot() {
        let timer = FakeAlarmTimer {
            counter: 4,
            running: false,
            compares: Vec::new(),
        };
        let mut alarm = Alarm::new(timer);

        let t1 = alarm.sleep(TimeSpan::from_ticks(2));
        let t2 = alarm.sleep(TimeSpan::from_ticks(1));
        let t3 = alarm.sleep(TimeSpan::from_ticks(3));

        future::join3(t1, t2, t3).await;
    }
}
