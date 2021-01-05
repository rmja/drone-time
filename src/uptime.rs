use core::{
    marker::PhantomData,
    sync::atomic::{AtomicU32, AtomicUsize, Ordering},
};
use drone_core::{fib, thr::prelude::*, thr::ThrToken};

use crate::{JiffiesClock, JiffiesTimer, TimeSpan};

pub struct Uptime<Clock: JiffiesClock, Timer: JiffiesTimer<A>, A> {
    clock: PhantomData<Clock>,
    timer: Timer,
    /// The number of threads simultaneously calling now() and seeing the "pending overflow" flag.
    pending_seen: AtomicUsize,
    /// The number of timer overflow interrupts that have occured.
    overflows: AtomicU32,
    /// The next value to use for `overflows`.
    overflows_next: AtomicU32,
    /// The last seen counter value.
    last_counter: AtomicU32,
    adapter: PhantomData<A>,
}

unsafe impl<Clock: JiffiesClock, Timer: JiffiesTimer<A>, A> Sync for Uptime<Clock, Timer, A> {}

impl<
        Clock: JiffiesClock + 'static + Send,
        Timer: JiffiesTimer<A> + 'static + Send,
        A: 'static + Send,
    > Uptime<Clock, Timer, A>
{
    /// Start the uptime counter.
    pub fn start<TimerInt: ThrToken>(timer: Timer, timer_int: TimerInt, _clock: Clock) -> Self {
        let counter_now = timer.counter();

        let uptime = Self {
            clock: PhantomData,
            timer,
            pending_seen: AtomicUsize::new(0),
            overflows: AtomicU32::new(0),
            overflows_next: AtomicU32::new(1),
            last_counter: AtomicU32::new(counter_now),
            adapter: PhantomData,
        };

        // Start the underlying timer.
        uptime.timer.start();

        timer_int.add_fn(|| {
            // now() must be called at least once per timer period
            // uptime.now();
            fib::Yielded::<(), !>(())
        });

        uptime
    }

    /// Sample the uptime counter, returning the non-wrapping time since the uptime was started.
    pub fn now(&self) -> TimeSpan<Clock> {
        // Two things can happen while invoking now()
        // * Any other thread can interrupt and maybe call now()
        // * The underlying timer runs underneath and may wrap during the invocation

        // Increment the thread-recursion count, and get "our" level
        let level = self.pending_seen.fetch_add(1, Ordering::Acquire);

        let now = loop {
            let cnt = self.timer.counter();

            let overflows = if self.timer.is_pending_overflow() {
                // Get the `overflows_next` value to be assigned to `overflows`
                let overflows = self.overflows_next.load(Ordering::Relaxed);
                self.overflows.store(overflows, Ordering::Relaxed);

                self.timer.clear_pending_overflow();

                if level == 0 {
                    // We are the outer-most thread (lowest priority) that have called now() and seen the overflow flag.
                    // The flag is now cleared, and so there is a-lot of time until the pending flag is seen again,
                    // and we can safely increment the value of `overflow_next`,
                    // to be assigned to `overflows` when the next flag is detected the next time.
                    self.overflows_next.fetch_add(1, Ordering::Relaxed);
                }

                overflows
            } else {
                self.overflows.load(Ordering::Relaxed)
            };

            if cnt <= self.timer.counter() {
                // There was no timer wrap while `overflows` was obtained.
                let increment = Timer::counter_max() as u64 + 1;
                break overflows as u64 * increment + cnt as u64;
            } else {
                // The underlying timer wrapped, retry
            }
        };

        // Release level.
        self.pending_seen.fetch_sub(1, Ordering::Release);

        TimeSpan(now, PhantomData)
    }

    pub fn last_counter(&self) -> u32 {
        self.last_counter.load(Ordering::Relaxed)
    }
}
