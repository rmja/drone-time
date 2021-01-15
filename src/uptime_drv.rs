use alloc::sync::Arc;
use core::{
    marker::PhantomData,
    sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering},
};
use drone_core::{fib, thr::prelude::*, thr::ThrToken};

use crate::{Tick, TimeSpan, Uptime, UptimeAlarm};

pub struct UptimeDrv<T: Tick, Alarm: UptimeAlarm<A>, A> {
    clock: PhantomData<T>,
    alarm: Alarm,
    /// The number of threads simultaneously calling now() and seeing the "pending overflow" flag.
    get_overflows_level: AtomicUsize,
    /// The number of alarm overflow interrupts that have occured.
    overflows: AtomicU32,
    /// The next value to use for `overflows`.
    overflows_next: AtomicU32,
    overflows_next_pending: AtomicBool,
    adapter: PhantomData<A>,
}

unsafe impl<T: Tick, Alarm: UptimeAlarm<A>, A> Sync for UptimeDrv<T, Alarm, A> {}

impl<T, Alarm, A> UptimeDrv<T, Alarm, A>
where
    T: Tick + Send + 'static,
    Alarm: UptimeAlarm<A> + Send + 'static,
    A: Send + 'static,
{
    /// Start the uptime counter.
    pub fn start<TimerInt: ThrToken>(alarm: Alarm, timer_int: TimerInt, _tick: T) -> Arc<Self> {
        let counter_now = alarm.counter();

        let uptime = Arc::new(Self {
            clock: PhantomData,
            alarm,
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
                    // now() must be called at least once per alarm period so we register it for the overflow interrupt.
                    uptime.now();
                    fib::Yielded(())
                }
                None => fib::Complete(()),
            }
        });

        // Start the underlying alarm.
        uptime.alarm.start();

        uptime
    }

    fn get_overflows(&self) -> u32 {
        // Increment the thread-recursion count, and get "our" level
        let level = self.get_overflows_level.fetch_add(1, Ordering::Acquire);

        let overflows = if self.alarm.is_pending_overflow() {
            // Get the `overflows_next` value to be assigned to `overflows`
            let overflows_next = self.overflows_next.load(Ordering::Relaxed);
            self.overflows.store(overflows_next, Ordering::Relaxed);

            self.alarm.clear_pending_overflow();

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

impl<T, Alarm, A> Uptime<T> for UptimeDrv<T, Alarm, A>
where
    T: Tick + Send + 'static,
    Alarm: UptimeAlarm<A> + Send + 'static,
    A: Send + 'static,
{
    fn counter(&self) -> u32 {
        self.alarm.counter()
    }

    fn now(&self) -> TimeSpan<T> {
        // Two things can happen while invoking now()
        // * Any other thread can interrupt and maybe call now()
        // * The underlying alarm runs underneath and may wrap during the invocation

        let now = loop {
            let cnt1 = self.alarm.counter();
            let overflows = self.get_overflows();
            let cnt2 = self.alarm.counter();
            if cnt1 <= cnt2 {
                // There was no alarm wrap while `overflows` was obtained.
                break overflows as u64 * Alarm::PERIOD + cnt2 as u64;
            } else {
                // The underlying alarm wrapped, retry
            }
        };

        TimeSpan::from_ticks(now as i64)
    }
}
