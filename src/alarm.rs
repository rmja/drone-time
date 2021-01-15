use core::{
    cell::RefCell,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
    task::{Context, Poll, Waker},
};

use crate::{AlarmCounter, AlarmTimer, Tick, TimeSpan};
use alloc::{collections::VecDeque, sync::Arc};
use atomicbox::AtomicOptionBox;
use drone_core::sync::Mutex;
use futures::prelude::*;

/// An alarm is backed by a timer and provides infinite timeout capabilites and multiple simultaneously running timeouts.
pub struct Alarm<
    Counter: AlarmCounter<T, A> + 'static,
    Timer: AlarmTimer<T, A>,
    T: Tick + 'static,
    A: 'static,
> {
    counter: Counter,
    timer: Arc<RefCell<Timer>>,
    running: Arc<AtomicOptionBox<Pin<Box<dyn Future<Output = ()>>>>>,
    state: Arc<Mutex<SharedState<Timer, T, A>>>,
}

struct SharedState<Timer: AlarmTimer<T, A>, T: Tick + 'static, A: 'static> {
    subscriptions: VecDeque<Subscription<T>>,
    adapter: PhantomData<A>,
    timer: PhantomData<Timer>,
}

pub struct Subscription<T: Tick> {
    /// The remaining duration until the future is resolved.
    remaining: TimeSpan<T>,
    /// Shared state related to the subscription.
    shared: Arc<SubscriptionShared>,
}

/// The state related to a subscription.
/// It is basically an enum where `waker` is only defined if state is `WAKEABLE`.
struct SubscriptionShared {
    /// The subscription state (ADDED, WAKEABLE, COMPLETED, DROPPED).
    state: AtomicUsize,
    /// The waker to be invoked when the future should complete.
    waker: AtomicOptionBox<Waker>,
}

const ADDED: usize = 0;
const WAKEABLE: usize = 1;
const COMPLETED: usize = 2;
const DROPPED: usize = 3;

pub struct SubscriptionGuard {
    running: Arc<AtomicOptionBox<Pin<Box<dyn Future<Output = ()>>>>>,
    shared: Arc<SubscriptionShared>,
}

impl Future for SubscriptionGuard {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let running = self.running.clone();
        if let Some(mut future) = running.take(Ordering::AcqRel) {
            if future.poll_unpin(cx).is_pending() {
                // The timer is currently running - there is no chance that we could have completed.
                // FIXME: Only set if None - it may be that the timer has scheduled a new future, and we do not want to override that one.
                running.swap(Some(future), Ordering::AcqRel);

                // return Poll::Pending;
            }
        }

        let shared = self.shared.clone();
        let waker = cx.waker().clone();

        // Copy the waker to the subscription so that we can wake it when it is time.
        // TODO: Use store() when available in atomicbox.
        // shared.waker.store(Some(Box::new(waker)), Ordering::AcqRel);
        let _ = shared.waker.swap(Some(Box::new(waker)), Ordering::AcqRel);

        // We can now update the state to WAKEABLE now when the waker is reliably stored for the subscription.
        let old = shared.state.swap(WAKEABLE, Ordering::AcqRel);
        assert!(old != DROPPED);
        if old == COMPLETED {
            // Timeout has already occured.

            // Set the state back to COMPLETED.
            shared.state.store(COMPLETED, Ordering::Release);

            // Remove the waker that we just assigned - it turns out that it was not needed as we are about to return `Ready`.
            shared.waker.take(Ordering::AcqRel);

            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

impl Drop for SubscriptionGuard {
    fn drop(&mut self) {
        self.shared.state.store(DROPPED, Ordering::Release);
    }
}

impl<
        Counter: AlarmCounter<T, A> + 'static,
        Timer: AlarmTimer<T, A> + 'static,
        T: Tick + 'static,
        A: Send + 'static,
    > Alarm<Counter, Timer, T, A>
{
    pub const MAX: u32 = Timer::MAX;

    /// Create a new `Alarm` backed by a hardware timer.
    pub fn new(counter: Counter, timer: Timer) -> Self {
        Self {
            counter,
            timer: Arc::new(RefCell::new(timer)),
            running: Arc::new(AtomicOptionBox::new(None)),
            state: Arc::new(Mutex::new(SharedState {
                subscriptions: VecDeque::new(),
                adapter: PhantomData,
                timer: PhantomData,
            })),
        }
    }

    /// Get the current counter value of the underlying hardware timer.
    pub fn counter(&self) -> u32 {
        self.counter.value()
    }

    /// Get a future that completes after a delay of length `duration`.
    pub fn sleep(&mut self, duration: TimeSpan<T>) -> impl Future<Output = ()> {
        self.sleep_from(self.counter(), duration)
    }

    /// Get a future that completes after a delay of length `duration` relative to the counter value `base`.
    pub fn sleep_from(&mut self, base: u32, duration: TimeSpan<T>) -> impl Future<Output = ()> {
        let sub_state = Arc::new(SubscriptionShared {
            state: AtomicUsize::new(ADDED),
            waker: AtomicOptionBox::new(None),
        });
        let sub = Subscription {
            remaining: duration,
            shared: sub_state.clone(),
        };

        let mutex = self.state.clone();
        let mut shared = mutex.try_lock().unwrap();

        // Remove all subscriptions that are in the `DROPPED` state.
        shared.remove_dropped();

        // Find the position where the new subscription should be added and insert.
        let index = shared.get_insert_index(duration);
        shared.subscriptions.insert(index, sub);

        if index == 0 {
            // It turns out that this subscription is the next in line.

            let future = Self::create_future(self.timer.clone(), self.running.clone(), self.state.clone(), base, duration);

            let running = self.running.clone();
            running.swap(Some(Box::new(future.boxed_local())), Ordering::AcqRel);
        }

        SubscriptionGuard { running: self.running.clone(), shared: sub_state }
    }

    async fn create_future(
        timer: Arc<RefCell<Timer>>,
        running: Arc<AtomicOptionBox<Pin<Box<dyn Future<Output = ()>>>>>,
        state: Arc<Mutex<SharedState<Timer, T, A>>>,
        base: u32,
        duration: TimeSpan<T>,
    ) {
        let mut t = timer.borrow_mut();
        let timer = timer.clone();
        t.sleep(base, duration)
            .then(move |_| {
                let mut shared = state.try_lock().unwrap();

                // Remove all subscriptions that are in the `DROPPED` state.
                shared.remove_dropped();

                // Set the remaining time for each subscription.
                for s in shared.subscriptions.iter_mut() {
                    s.remaining -= duration;

                    if s.remaining.0 == 0 {
                        // Wake the future for the subscription.
                        let old = s.shared.state.swap(COMPLETED, Ordering::AcqRel);
                        if old == WAKEABLE {
                            let waker = s.shared.waker.take(Ordering::AcqRel).unwrap();
                            waker.wake();
                        } else if old == DROPPED {
                            s.shared.state.store(DROPPED, Ordering::Release);
                        }
                    }
                }

                // Remove all subscriptions that have remaining == 0.
                shared.subscriptions.retain(|x| x.remaining.0 > 0);

                if let Some(next) = shared.subscriptions.front() {
                    // Create a future for the next subscription in line.

                    let base = Timer::counter_add(base, (duration.0 as u64 % Timer::PERIOD) as u32);
                    let duration = next.remaining;

                    let future = Self::create_future(timer, running.clone(), state.clone(), base, duration);
                    running.swap(Some(Box::new(future.boxed_local())), Ordering::AcqRel);
                } else {
                    running.take(Ordering::AcqRel);
                }

                future::ready(())
            }).await;
    }
}

impl<Timer: AlarmTimer<T, A>, T: Tick + 'static, A: 'static> SharedState<Timer, T, A> {
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
            .retain(|x| x.shared.state.load(Ordering::Relaxed) != DROPPED);
    }
}

// #[cfg(test)]
// pub mod tests {
//     use futures::future;
//     use futures_await_test::async_test;

//     use crate::adapters::alarm::fakes::FakeAlarmTimer;

//     use super::*;

//     #[async_test]
//     async fn whoot() {
//         let timer = FakeAlarmTimer {
//             counter: 4,
//             running: false,
//             compares: Vec::new(),
//         };
//         let mut alarm = Alarm::new(timer);

//         let t1 = alarm.sleep(TimeSpan::from_ticks(2));
//         let t2 = alarm.sleep(TimeSpan::from_ticks(1));
//         let t3 = alarm.sleep(TimeSpan::from_ticks(3));

//         future::join3(t1, t2, t3).await;
//     }
// }
