use core::sync::atomic::{AtomicUsize, Ordering};

use crate::UptimeAlarm;
use drone_cortexm::{map::periph::sys_tick::SysTickPeriph, reg::prelude::*};

pub struct SysTickDrv(SysTickPeriph, AtomicUsize);

impl SysTickDrv {
    pub fn new(systick: SysTickPeriph) -> Self {
        Self(systick, AtomicUsize::new(0))
    }
}

unsafe impl Sync for SysTickDrv {}

impl UptimeAlarm<SysTickDrv> for SysTickDrv {
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

    #[inline(never)]
    fn is_pending_overflow(&self) -> bool {
        // SysTick is inherently racy as reading the COUNTFLAG that tells whether the timer has overflowed,
        // _also clears the flag_. That means that any other interrupting thread cannot read flag.

        // Pseudo code - does not compile:
        const SYSTICK_CTRL: usize = 0xE000E010;
        const SP_TO_R0_OFFSET: usize = 2;
        let atomic = &self.1;
        unsafe { asm!("nop") };
        // Set register r0 to an impossible SYSTICK_CTRL value.
        // This register is reserved for the SYSTICK_CTRL,
        // so that any thread that may interrupt us can go and read its value
        // as an offset of our stack pointer which we will save in a moment.
        let mut r0 = 0;
        let my_sp: usize; // Get this threads stack pointer.
        unsafe { asm!("mov {}, SP", out(reg) my_sp) };
        // Store our stack pointer in the atomic. It can contain three different kinds of value:
        // ==0 - The latched overflow flag is not set and no other thread is currently reading SYSTICK_CTRL.
        // ==1 - The latched overflow flag is currently set.
        // >=2 - The stack pointer of the thread that is currently reading SYSTICK_CTRL.
        let preempted_sp = atomic.compare_and_swap(0, my_sp, Ordering::AcqRel);
        if preempted_sp >= 2 {
            // We have preempted at least one thread.
            // Lets see if the other thread has yet read SYSTICK_CTRL into r0.
            let preempted_r0 = unsafe {
                core::ptr::read_volatile(
                    ((preempted_sp & 0xFFFFFFFE) as *const usize).add(SP_TO_R0_OFFSET),
                )
            };
            if preempted_r0 == 0 {
                // The preempted thread has not yet read SYSTICK_CTRL - its value is invalid.
                // Steal the stack pointer atomic from the preempted thread
                // as other interrupting threads should read our r0,
                // and not the one we have just found to be invalid.
                atomic.compare_and_swap(preempted_sp, my_sp, Ordering::Release);
                r0 = SYSTICK_CTRL;
            } else {
                // The preempted thread had SYSTICK_CTRL read to R0 on its stack.
                // Use that value instead of the just cleared SYSTICK_CTRL.
                r0 = preempted_r0;
            }
        } else {
            // We have not preempted any other threads.
            r0 = SYSTICK_CTRL;
        }

        if r0 & 0x10000 > 0 {
            // COUNTFLAG is set in R0
            // Store the flag instead of our stack pointer, but only do so if we were not interrupted.
            // If we were, we must not set the latched flag, as the preempting thread may have
            // cleared the flag already.
            atomic.compare_and_swap(my_sp, 1, Ordering::Release);
        }

        // The truth is now in the latched atomic - re-read it.
        atomic.load(Ordering::Acquire) == 1
    }

    fn clear_pending_overflow(&self) {
        self.1.store(0, Ordering::Release);
    }
}
