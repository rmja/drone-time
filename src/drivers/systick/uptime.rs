use core::sync::atomic::{AtomicBool, Ordering};

use crate::{Tick, UptimeCounter, UptimeOverflow};
use alloc::sync::Arc;
use drone_cortexm::{map::periph::sys_tick::SysTickPeriph, reg::prelude::*};

use super::{Adapter, diverged::SysTickDiverged};

/// A cortex SysTick uptime driver.
pub struct SysTickUptimeDrv {
    /// The timer counter.
    pub counter: SysTickCounterDrv,
    /// The overflow control.
    pub overflow: SysTickOverflowDrv,
}
pub struct SysTickCounterDrv(Arc<SysTickDiverged>);
pub struct SysTickOverflowDrv(Arc<SysTickDiverged>, AtomicBool);

impl SysTickUptimeDrv {
    pub fn new(systick: SysTickPeriph) -> Self {
        // Configure reload value.
        systick.stk_load.store(|r| r.write_reload(0xFFFFFF));

        let systick = Arc::new(SysTickDiverged::new(systick));
        Self {
            counter: SysTickCounterDrv(systick.clone()),
            overflow: SysTickOverflowDrv(systick, AtomicBool::new(false)),
        }
    }

    /// Start the counter in a multi-shot way
    pub fn start(&mut self) {
        self.counter.0.stk_ctrl.store(|r| r.set_enable());
    }
}


impl<T: Tick> UptimeCounter<T, Adapter> for SysTickCounterDrv {
    fn value(&self) -> u32 {
        // SysTick counts down, but the returned counter value must count up.
        0xFFFFFF - self.0.stk_val.load_bits() as u32
    }
}

impl UptimeOverflow<Adapter> for SysTickOverflowDrv {
    const MAX: u32 = 0xFFFFFF; // SysTick is a 24 bit counter.

    fn overflow_int_enable(&self) {
        // Counting down to 0 triggers the SysTick interrupt
        self.0.stk_ctrl.modify(|r| r.set_tickint());
    }

    fn is_pending_overflow(&self) -> bool {
        // SysTick is inherently racy as reading the COUNTFLAG that tells whether the timer has overflowed,
        // _also clears the flag_. That means that any other interrupting thread cannot read flag.

        interrupt::critical(|_| {
            // Read the COUNTFLAG value.
            // This reads the register and returns 1 if timer counted to 0 _since last time this was read_,
            // i.e. the flag is actually cleared by reading it.
            let is_pending = self.0.stk_ctrl.countflag.read_bit();

            // Store the flag in case that is_pending_overflow() is called multiple times for the overflow.
            self.1.compare_and_swap(false, is_pending, Ordering::AcqRel) || is_pending
        })
    }

    fn clear_pending_overflow(&self) {
        self.1.store(false, Ordering::Release);
    }
}

mod interrupt {
    #[cfg(not(feature = "std"))]
    use core::sync::atomic::{compiler_fence, Ordering};

    /// Disables all interrupts
    #[inline]
    pub fn disable() {
        #[cfg(feature = "std")]
        unimplemented!();
        #[cfg(not(feature = "std"))]
        {
            unsafe { asm!("cpsid i") };
            compiler_fence(Ordering::SeqCst);
        }
    }

    /// Enables all (not masked) interrupts
    ///
    /// # Safety
    ///
    /// - Do not call this function inside a critical section.
    #[inline]
    pub unsafe fn enable() {
        #[cfg(feature = "std")]
        unimplemented!();
        #[cfg(not(feature = "std"))]
        {
            compiler_fence(Ordering::SeqCst);
            asm!("cpsie i");
        }
    }

    #[inline]
    fn primask() -> u32 {
        #[cfg(feature = "std")]
        unimplemented!();
        #[cfg(not(feature = "std"))]
        {
            let r: usize;
            unsafe { asm!("mrs {}, PRIMASK", out(reg) r) };
            r as u32
        }
    }

    /// Critical section token.
    pub struct CriticalSection;

    /// Execute the closure `f` in an interrupt-free context.
    #[inline]
    pub fn critical<F, R>(f: F) -> R
    where
        F: FnOnce(&CriticalSection) -> R,
    {
        let pm = primask();

        // Disable interrupts - they may already be disabled if this is a nested critical section.
        disable();

        let cs = CriticalSection;
        let r = f(&cs);

        // Only enable interrupt if interrupts were active when entering.
        if pm & 1 == 0 {
            unsafe { enable() }
        }

        r
    }
}
