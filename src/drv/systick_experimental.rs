use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::UptimeTimer;
use drone_cortexm::{map::periph::sys_tick::SysTickPeriph, reg::prelude::*};

pub struct SysTickDrv(SysTickPeriph, AtomicBool);

impl SysTickDrv {
    pub fn new(systick: SysTickPeriph) -> Self {
        Self(systick, AtomicBool::new(false), AtomicUsize::new(0))
    }
}

unsafe impl Sync for SysTickDrv {}

impl UptimeTimer<SysTickDrv> for SysTickDrv {
    fn start(&self) {
        // Enable timer
        self.0.stk_load.store(|r| r.write_reload(0xFF_FF_FF));

        self.0.stk_ctrl.store(|r| {
            r.set_tickint() // Counting down to 0 triggers the SysTick interrupt
                .set_enable() // Start the counter in a multi-shot way
        });
    }

    fn counter(&self) -> u32 {
        // SysTick counts down, but the returned counter value must count up.
        0xFF_FF_FF - self.0.stk_val.load_bits() as u32
    }

    fn counter_max() -> u32 {
        // SysTick is a 24 bit counter.
        0xFF_FF_FF
    }

    [inline(never)]
    fn is_pending_overflow(&self) -> bool {
        // SysTick is inherently racy as reading the COUNTFLAG that tells whether the timer has overflowed,
        // _also clears the flag_. That means that any other interrupting thread cannot read flag.

        // Pseudo code - does not compile:
        let r0 = 0; // Set register to an impossible SYSTICK_CTRL value.
        let my_sp = SP; // Get this threads stack pointer.
        let preempted_sp = self.2.compare_and_swap(0, my_sp, Ordering::AcqRel);
        if preempted_sp != 0 {
            // We have preempted at least one thread.
            // Lets see if the other thread has yet read SYSTICK_CTRL into r0.
            let preempted_r0 = *(preempted_sp + SP_TO_R0_OFFSET);
            if preempted_r0 == 0 {
                // The preempted thread has not yet read SYSTICK_CTRL - its value is invalid.
                // Steal the stack pointer atomic as other interrupting threads should read
                // our r0, and not the one we have just found to be invalid.
                preempted_sp = self.2.store(my_sp);
                r0 = SYSTICK_CTRL;
            }
            else {
                // The preempted thread had SYSTICK_CTRL read to R0 on its stack.
                r0 = preempted_r0;
            }
        }
        else {
            // We have not preempted any other threads.
            r0 = SYSTICK_CTRL;
        }

        let is_pending = r0 & 0x10000 > 0; // Mask out the COUNTFLAG.
        let is_pending = self.1.compare_and_swap(false, is_pending, Ordering::AcqRel) || is_pending;

        // The flag is now safely stored.
        self.2.store(0);

        is_pending
    }

    fn clear_pending_overflow(&self) {
        self.1.store(false, Ordering::Release);
    }
}

mod interrupt {
    use core::sync::atomic::{Ordering, compiler_fence};

    /// Disables all interrupts
    #[inline]
    pub fn disable() {
        #[cfg(feature = "std")]
        return unimplemented!();
        unsafe { asm!("cpsid i") };
        compiler_fence(Ordering::SeqCst);
    }

    /// Enables all (not masked) interrupts
    ///
    /// # Safety
    ///
    /// - Do not call this function inside a critical section.
    #[inline]
    pub unsafe fn enable() {
        #[cfg(feature = "std")]
        return unimplemented!();
        compiler_fence(Ordering::SeqCst);
        unsafe { asm!("cpsie i") };
    }

    #[inline]
    fn primask() -> u32 {
        #[cfg(feature = "std")]
        return unimplemented!();
        let r: usize;
        unsafe { asm!("mrs {}, PRIMASK", out(reg) r) };
        r as u32
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