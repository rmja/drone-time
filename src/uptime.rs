use alloc::sync::Arc;
use core::{
    marker::PhantomData,
    sync::atomic::{AtomicU32, Ordering},
};
use drone_core::{fib, thr::ThrToken, thr::prelude::*};

use crate::{JiffiesClock, JiffiesTimer, TimeSpan};

pub struct Uptime<Clock: JiffiesClock, Timer: JiffiesTimer<A>, A> {
    clock: PhantomData<Clock>,
    timer: Timer,
    /// The number of timer overflow interrupts that have occured.
    overflows: Arc<AtomicU32>,
    /// The last seen counter value.
    last_counter: AtomicU32,
    adapter: PhantomData<A>,
}

impl<Clock: JiffiesClock, Timer: JiffiesTimer<A>, A> Uptime<Clock, Timer, A> {
    pub fn start<TimerInt: ThrToken>(timer: Timer, timer_int: TimerInt) -> Self {
        let counter_now = timer.counter();

        let uptime = Self {
            clock: PhantomData,
            timer,
            overflows: Arc::new(AtomicU32::new(0)),
            last_counter: AtomicU32::new(counter_now),
            adapter: PhantomData,
        };

        let overflows = uptime.overflows.clone();
        timer_int.add_fn(move || {
            // TODO: Do this check
            // if timer.try_clear_pending_overflow() {
                overflows.fetch_add(1, Ordering::SeqCst);
            // }
            fib::Yielded::<(), !>(())
        });

        uptime
    }

    pub fn now(&self) -> TimeSpan<Clock> {
        // Read the current jiffies counter.
        self.last_counter
            .store(self.timer.counter(), Ordering::SeqCst);

        if self.timer.try_clear_pending_overflow() {
            // There was a pending interrupt that was cleared:
            // 1. Increment overflow.
            // 2. Re-sample counter.
            self.overflows.fetch_add(1, Ordering::SeqCst);
            self.last_counter
                .store(self.timer.counter(), Ordering::SeqCst);
        }

        let increment = Timer::counter_max() as u64 + 1;
        let mut now = self.overflows.load(Ordering::SeqCst) as u64 * increment;
        now += self.last_counter.load(Ordering::SeqCst) as u64;

        TimeSpan(now, PhantomData)
    }

    pub fn last_counter(&self) -> u32 {
        self.last_counter.load(Ordering::SeqCst)
    }
}
