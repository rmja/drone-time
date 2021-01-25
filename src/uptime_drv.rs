use alloc::sync::Arc;
use core::{
    marker::PhantomData,
    sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering},
};
use drone_core::{fib, thr::prelude::*, thr::ThrToken};

use crate::{Tick, TimeSpan, Uptime, UptimeCounter, UptimeOverflow};

pub struct UptimeDrv<T: Tick, Cnt: UptimeCounter<T, A>, Ovf: UptimeOverflow<A>, A: Send + Sync> {
    tick: PhantomData<T>,
    counter: Cnt,
    overflow: Ovf,
    /// The number of threads simultaneously calling now() and seeing the "pending overflow" flag.
    get_overflows_level: AtomicUsize,
    /// The number of timer overflow interrupts that have occured.
    overflows: AtomicU32,
    /// The next value to use for `overflows`.
    overflows_next: AtomicU32,
    overflows_next_pending: AtomicBool,
    adapter: PhantomData<A>,
}

impl<T, Cnt, Ovf, A> UptimeDrv<T, Cnt, Ovf, A>
where
    T: Tick + 'static,
    Cnt: UptimeCounter<T, A> + 'static,
    Ovf: UptimeOverflow<A> + 'static,
    A: Send + Sync + 'static,
{
    /// Create a new Uptime driver.
    pub fn new<TimerInt: ThrToken>(
        counter: Cnt,
        overflow: Ovf,
        timer_int: TimerInt,
        _tick: T,
    ) -> Arc<Self> {
        let uptime = Arc::new(Self {
            tick: PhantomData,
            counter,
            overflow,
            get_overflows_level: AtomicUsize::new(0),
            overflows: AtomicU32::new(0),
            overflows_next: AtomicU32::new(1),
            overflows_next_pending: AtomicBool::new(false),
            adapter: PhantomData,
        });

        let uptime_weak = Arc::downgrade(&uptime);
        timer_int.add_fn(move || {
            match uptime_weak.upgrade() {
                Some(uptime) => {
                    // now() must be called at least once per timer period so we register it for the overflow interrupt.
                    if uptime.overflow.is_pending_overflow() {
                        uptime.now();
                    }
                    fib::Yielded(())
                }
                None => fib::Complete(()),
            }
        });
        uptime.overflow.overflow_int_enable();

        uptime
    }

    fn sample(&self) -> (u32, u32) {
        // Two things can happen while invoking now()
        // * Any other thread can interrupt and maybe call now()
        // * The underlying timer runs underneath and may wrap during the invocation

        loop {
            let cnt1 = self.counter.value();
            let overflows = self.get_overflows();
            let cnt2 = self.counter.value();
            if cnt1 <= cnt2 {
                // There was no timer wrap while `overflows` was obtained.
                break (overflows, cnt2);
            } else {
                // The underlying timer wrapped, retry
            }
        }
    }

    fn get_overflows(&self) -> u32 {
        // Increment the thread-recursion count, and get "our" level
        let level = self.get_overflows_level.fetch_add(1, Ordering::Acquire);

        let overflows = if self.overflow.is_pending_overflow() {
            // Get the `overflows_next` value to be assigned to `overflows`
            let overflows_next = self.overflows_next.load(Ordering::Relaxed);
            self.overflows.store(overflows_next, Ordering::Relaxed);

            self.overflow.clear_pending_overflow();

            self.overflows_next_pending.store(true, Ordering::Release);

            overflows_next
        } else {
            self.overflows.load(Ordering::Relaxed)
        };

        if level == 0
            && self
                .overflows_next_pending
                .compare_and_swap(true, false, Ordering::AcqRel)
        {
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
}

impl<T, Cnt, Ovf, A> Uptime<T> for UptimeDrv<T, Cnt, Ovf, A>
where
    T: Tick + 'static,
    Cnt: UptimeCounter<T, A> + 'static,
    Ovf: UptimeOverflow<A> + 'static,
    A: Send + Sync + 'static,
{
    #[inline]
    fn counter(&self) -> u32 {
        self.counter.value()
    }

    #[inline]
    fn now(&self) -> TimeSpan<T> {
        let (overflows, counter) = self.sample();
        let ticks = overflows as u64 * Ovf::PERIOD + counter as u64;
        TimeSpan::from_ticks(ticks as i64)
    }

    fn at(&self, counter: u32) -> TimeSpan<T> {
        let sample = self.sample();
        let ticks = sample.0 as u64 * Ovf::PERIOD + sample.1 as u64;
        let now = TimeSpan::from_ticks(ticks as i64);
        let delta = if counter <= sample.1 {
            (sample.1 - counter) as i64
        }
        else {
            sample.1 as i64 + Ovf::PERIOD as i64 - counter as i64
        };
        now - TimeSpan::from_ticks(delta)
    }
}
