use core::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::atomic::{AtomicU8, Ordering},
    task::{Context, Poll, Waker},
};

use crate::{AlarmCounter, AlarmTimer, Tick, TimeSpan};
use alloc::{collections::VecDeque, sync::Arc};
use atomicbox::AtomicOptionBox;
use drone_core::sync::Mutex;
use futures::prelude::*;

pub trait Alarm<T: Tick>: Send {
    /// Get the current counter value of the underlying hardware timer.
    fn counter(&self) -> u32;

    /// Spin a number of clock cycles.
    fn spin(&self, cycles: u32);

    /// Spin a number of nanoseconds.
    fn burn_nanos(&self, mut nanos: u32) {
        debug_assert_ne!(
            0,
            T::CPU_FREQ,
            "The Tick::CPU_FREQ must be defined to support cycle by nanoseconds."
        );

        while nanos > 1000000 {
            self.spin((nanos * (T::CPU_FREQ / 1000000)) / 1000);
            nanos -= 1000;
        }
        self.spin((nanos * (T::CPU_FREQ / 1000000)) / 1000);
    }

    /// Get a future that completes after a delay of length `duration`.
    fn sleep(&self, duration: TimeSpan<T>) -> SubscriptionGuard {
        self.sleep_from(self.counter(), duration)
    }

    /// Get a future that completes after a delay of length `duration` relative to the counter value `base`.
    fn sleep_from(&self, base: u32, duration: TimeSpan<T>) -> SubscriptionGuard;
}

/// An alarm is backed by a single hardware timer and provides infinite timeout capabilites and multiple simultaneously running timeouts.
pub struct AlarmDrv<Cnt: AlarmCounter<T, A> + 'static, Tim: AlarmTimer<T, A>, T: Tick, A: 'static> {
    counter: Cnt,
    timer: Arc<Mutex<Tim>>,
    running: Arc<AtomicOptionBox<Pin<Box<dyn Future<Output = ()>>>>>,
    subscriptions: Arc<Mutex<VecDeque<Subscription<T>>>>,
    adapter: PhantomData<A>,
}

pub struct Subscription<T: Tick> {
    /// The remaining duration until the future is resolved.
    remaining: TimeSpan<T>,
    /// Shared state related to the subscription.
    state: Arc<SubscriptionState>,
}

/// The state related to a subscription.
/// It is basically an enum where `waker` is only defined if state is `WAKEABLE`.
struct SubscriptionState {
    /// The subscription state (PENDING, ADDED, WAKEABLE, COMPLETED, DROPPED).
    value: AtomicU8,
    /// The waker to be invoked when the future should complete.
    waker: AtomicOptionBox<Waker>,
}

pub struct SubscriptionGuard {
    appender: AtomicOptionBox<Pin<Box<dyn Future<Output = ()>>>>,
    running: Arc<AtomicOptionBox<Pin<Box<dyn Future<Output = ()>>>>>,
    state: Arc<SubscriptionState>,
}

impl SubscriptionState {
    const PENDING_ADD: u8 = 0;
    const ADDED: u8 = 1;
    const WAKEABLE: u8 = 2;
    const COMPLETED: u8 = 3;
    const DROPPED: u8 = 4;
}

impl Future for SubscriptionGuard {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let state = self.state.clone();

        if state.value.compare_exchange(
            SubscriptionState::PENDING_ADD,
            SubscriptionState::ADDED,
            Ordering::AcqRel,
            Ordering::Relaxed,
        ).is_ok()
        {
            let mut appender = self.appender.take(Ordering::AcqRel).unwrap();

            if appender.poll_unpin(cx).is_pending() {
                self.appender.store(Some(appender), Ordering::AcqRel);
                state
                    .value
                    .store(SubscriptionState::PENDING_ADD, Ordering::Release);
                return Poll::Pending;
            }
        }

        // Always poll the underlying timer sleep future - it won't start otherwise.
        let running = self.running.clone();
        if let Some(mut future) = running.take(Ordering::AcqRel) {
            if future.poll_unpin(cx).is_pending() {
                // The timer is currently running.
                // Set the future back if not assigned to some earlier timeout.
                running.try_store(future, Ordering::Release);
            }
        }

        let waker = cx.waker().clone();

        // Copy the waker to the subscription so that we can wake it when it is time.
        state.waker.store(Some(Box::new(waker)), Ordering::AcqRel);

        // We can now update the state to WAKEABLE now when the waker is reliably stored for the subscription.
        let old = state
            .value
            .swap(SubscriptionState::WAKEABLE, Ordering::AcqRel);
        assert!(old != SubscriptionState::DROPPED);
        if old == SubscriptionState::COMPLETED {
            // Timeout has already occured.

            // Set the state back to COMPLETED.
            state
                .value
                .store(SubscriptionState::COMPLETED, Ordering::Release);

            // Remove the waker that we just assigned - it turns out that it was not needed as we are about to return `Ready`.
            state.waker.take(Ordering::AcqRel);

            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

impl Drop for SubscriptionGuard {
    fn drop(&mut self) {
        self.state
            .value
            .store(SubscriptionState::DROPPED, Ordering::Release);
    }
}

impl<
        Cnt: AlarmCounter<T, A> + 'static,
        Tim: AlarmTimer<T, A> + 'static,
        T: Tick,
        A: Send + 'static,
    > AlarmDrv<Cnt, Tim, T, A>
{
    pub const MAX: u32 = Tim::MAX;

    /// Create a new `AlarmDrv` backed by a hardware timer.
    pub fn new(counter: Cnt, timer: Tim, _tick: T) -> Self {
        Self {
            counter,
            timer: Arc::new(Mutex::new(timer)),
            running: Arc::new(AtomicOptionBox::new(None)),
            subscriptions: Arc::new(Mutex::new(VecDeque::new())),
            adapter: PhantomData,
        }
    }

    async fn create_future(
        timer: Arc<Mutex<Tim>>,
        running: Arc<AtomicOptionBox<Pin<Box<dyn Future<Output = ()>>>>>,
        subscriptions: Arc<Mutex<VecDeque<Subscription<T>>>>,
        base: u32,
        duration: TimeSpan<T>,
    ) {
        let mut t = timer
            .try_lock()
            .expect("The timer must not be running when setting up a new timeout.");
        let timer = timer.clone();
        t.sleep(base, duration)
            .then(move |_| {
                let subscriptions = subscriptions.clone();
                async move {
                    let mut subs = subscriptions.lock().await;
                    // Remove all subscriptions that are in the `DROPPED` state.
                    subs.remove_dropped();

                    // Set the remaining time for each subscription.
                    for s in subs.iter_mut() {
                        s.remaining -= duration;

                        if s.remaining.0 == 0 {
                            // Wake the future for the subscription.
                            let old = s
                                .state
                                .value
                                .swap(SubscriptionState::COMPLETED, Ordering::AcqRel);
                            if old == SubscriptionState::WAKEABLE {
                                let waker = s.state.waker.take(Ordering::AcqRel).unwrap();
                                waker.wake();
                            } else if old == SubscriptionState::DROPPED {
                                s.state
                                    .value
                                    .store(SubscriptionState::DROPPED, Ordering::Release);
                            }
                        }
                    }

                    // Remove all subscriptions that have remaining == 0.
                    subs.retain(|x| x.remaining.0 > 0);

                    if let Some(next) = subs.front() {
                        // Create a future for the next subscription in line.

                        let base = Tim::counter_add(base, (duration.0 as u64 % Tim::PERIOD) as u32);
                        let duration = next.remaining;

                        let future = Self::create_future(
                            timer,
                            running.clone(),
                            subscriptions.clone(),
                            base,
                            duration,
                        );
                        running.store(Some(Box::new(future.boxed_local())), Ordering::AcqRel);
                    } else {
                        running.take(Ordering::AcqRel);
                    }
                }
            })
            .await;
    }
}

impl<
        Cnt: AlarmCounter<T, A> + 'static,
        Tim: AlarmTimer<T, A> + 'static,
        T: Tick,
        A: Send + 'static,
    > Alarm<T> for AlarmDrv<Cnt, Tim, T, A>
{
    fn counter(&self) -> u32 {
        self.counter.value()
    }

    #[inline]
    fn spin(&self, cycles: u32) {
        self.counter.spin(cycles);
    }

    fn sleep_from(&self, base: u32, duration: TimeSpan<T>) -> SubscriptionGuard {
        let sub_state = Arc::new(SubscriptionState {
            value: AtomicU8::new(SubscriptionState::PENDING_ADD),
            waker: AtomicOptionBox::new(None),
        });

        let sub = Subscription {
            remaining: duration,
            state: sub_state.clone(),
        };

        let timer = self.timer.clone();
        let running = self.running.clone();
        let subscriptions = self.subscriptions.clone();
        let appender = async move {
            let mut subs = subscriptions.lock().await;

            // Remove all subscriptions that are in the `DROPPED` state.
            subs.remove_dropped();

            // Find the position where the new subscription should be added and insert.
            let index = subs.get_insert_index(duration);
            subs.insert(index, sub);

            if index == 0 {
                // It turns out that this subscription is the next in line.

                let future = Self::create_future(
                    timer.clone(),
                    running.clone(),
                    subscriptions.clone(),
                    base,
                    duration,
                );

                let running = running.clone();
                running.store(Some(Box::new(future.boxed_local())), Ordering::AcqRel);
            }
        };

        SubscriptionGuard {
            appender: AtomicOptionBox::new(Some(Box::new(appender.boxed_local()))),
            running: self.running.clone(),
            state: sub_state,
        }
    }
}

trait VecDequeExt<T: Tick> {
    fn get_insert_index(&self, remaining: TimeSpan<T>) -> usize;
    fn remove_dropped(&mut self);
}

impl<T: Tick> VecDequeExt<T> for VecDeque<Subscription<T>> {
    fn get_insert_index(&self, remaining: TimeSpan<T>) -> usize {
        let mut index = 0;
        for sub in self.iter() {
            if remaining < sub.remaining {
                break;
            }
            index += 1;
        }
        index
    }

    fn remove_dropped(&mut self) {
        self.retain(|x| x.state.value.load(Ordering::Relaxed) != SubscriptionState::DROPPED);
    }
}

#[cfg(test)]
pub mod tests {
    use std::thread::spawn;

    use futures::future;
    use futures_await_test::async_test;

    use crate::adapters::alarm::fakes::{FakeAlarmCounter, FakeAlarmTimer, FakeTick};

    use super::*;

    #[async_test]
    async fn whoot() {
        let counter = FakeAlarmCounter(4);
        let timer = FakeAlarmTimer {
            compares: Vec::new(),
        };
        let alarm = AlarmDrv::new(counter, timer, FakeTick);

        let fires = Mutex::new(Vec::new());
        let t1 = alarm.sleep(TimeSpan::from_ticks(2)).then(|_| async {
            let mut fires = fires.lock().await;
            fires.push(2);
        });
        let t2 = alarm.sleep(TimeSpan::from_ticks(1)).then(|_| async {
            let mut fires = fires.lock().await;
            fires.push(1);
        });
        let t3 = alarm.sleep(TimeSpan::from_ticks(3)).then(|_| async {
            let mut fires = fires.lock().await;
            fires.push(3);
        });

        future::join3(t1, t2, t3).await;

        // TODO: Find a way for the fake to actually schedule in correct order.
        assert_eq!(vec![1, 2, 3], fires.into_inner());
    }
}
