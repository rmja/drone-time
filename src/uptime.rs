use alloc::sync::Arc;
use core::{marker::PhantomData, sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering}};
use drone_core::{fib, thr::prelude::*, thr::ThrToken};

use crate::{JiffiesClock, JiffiesTimer, TimeSpan};

pub struct Uptime<Clock: JiffiesClock, Timer: JiffiesTimer<A>, A> {
    clock: PhantomData<Clock>,
    timer: Timer,
    /// The number of threads simultaneously calling now() and seeing the "pending overflow" flag.
    get_overflows_level: AtomicUsize,
    /// The number of timer overflow interrupts that have occured.
    overflows: AtomicU32,
    /// The next value to use for `overflows`.
    overflows_next: AtomicU32,
    overflows_next_pending: AtomicBool,
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
    pub fn start<TimerInt: ThrToken>(
        timer: Timer,
        timer_int: TimerInt,
        _clock: Clock,
    ) -> Arc<Self> {
        let counter_now = timer.counter();

        let uptime = Arc::new(Self {
            clock: PhantomData,
            timer,
            get_overflows_level: AtomicUsize::new(0),
            overflows: AtomicU32::new(0),
            overflows_next: AtomicU32::new(1),
            overflows_next_pending: AtomicBool::new(false),
            last_counter: AtomicU32::new(counter_now),
            adapter: PhantomData,
        });

        let uptime_weak = Arc::downgrade(&uptime);
        timer_int.add_fn(move || {
            match uptime_weak.upgrade() {
                Some(uptime) => {
                    // now() must be called at least once per timer period so we register it for the overflow interrupt.
                    uptime.now();
                    fib::Yielded(())
                }
                None => fib::Complete(()),
            }
        });

        // Start the underlying timer.
        uptime.timer.start();

        uptime
    }

    /// Sample the uptime counter, returning the non-wrapping time since the uptime was started.
    pub fn now(&self) -> TimeSpan<Clock> {
        // Two things can happen while invoking now()
        // * Any other thread can interrupt and maybe call now()
        // * The underlying timer runs underneath and may wrap during the invocation

        let now = loop {
            let cnt1 = self.timer.counter();
            let overflows = self.get_overflows();
            let cnt2 = self.timer.counter();
            if cnt1 <= cnt2 {
                // There was no timer wrap while `overflows` was obtained.
                break overflows as u64 * Timer::overflow_increment() + cnt2 as u64;
            } else {
                // The underlying timer wrapped, retry
            }
        };

        TimeSpan(now, PhantomData)
    }

    fn get_overflows(&self) -> u32 {
        // Increment the thread-recursion count, and get "our" level
        let level = self.get_overflows_level.fetch_add(1, Ordering::Acquire);

        let overflows = if self.timer.is_pending_overflow() {
            // Get the `overflows_next` value to be assigned to `overflows`
            let overflows_next = self.overflows_next.load(Ordering::Relaxed);
            self.overflows.store(overflows_next, Ordering::Relaxed);

            self.timer.clear_pending_overflow();

            self.overflows_next_pending.store(true, Ordering::Release);

            overflows_next
        } else {
            self.overflows.load(Ordering::Relaxed)
        };

        if level == 0 && self.overflows_next_pending.compare_and_swap(true, false, Ordering::Acquire) {
            // We are the outer-most thread (lowest priority) that have called now() and seen the overflow flag.
            // The flag is now cleared, and so there is a-lot of time until the pending flag is seen again,
            // and we can safely increment the value of `overflow_next`,
            // to be assigned to `overflows` when the next flag is detected the next time.
            self.overflows_next.fetch_add(1, Ordering::Relaxed);
        }

        // Release level.
        self.get_overflows_level.fetch_sub(1, Ordering::Release);

        overflows
    }

    pub fn last_counter(&self) -> u32 {
        self.last_counter.load(Ordering::Relaxed)
    }
}
