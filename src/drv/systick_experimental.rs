use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::UptimeTimer;
use drone_cortexm::{map::periph::sys_tick::SysTickPeriph, reg::prelude::*};

pub struct SysTickDrv(SysTickPeriph, AtomicBool, AtomicUsize);

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

    #[inline(never)]
    fn is_pending_overflow(&self) -> bool {
        // SysTick is inherently racy as reading the COUNTFLAG that tells whether the timer has overflowed,
        // _also clears the flag_. That means that any other interrupting thread cannot read flag.

        // Pseudo code - does not compile:
        const SYSTICK_CTRL: usize = 0xE000E010;
        const SP_TO_R0_OFFSET: usize = 2;
        unsafe { asm!("nop") };
        // Set register to an impossible SYSTICK_CTRL value.
        // This register is reserved for the SYSTICK_CTRL,
        // so that any thread that may interrupt us can go and read its value
        // as an offset of our stack pointer which we will save in a moment.
        let mut r0 = 0;
        let my_sp: usize; // Get this threads stack pointer.
        unsafe { asm!("mov {}, SP", out(reg) my_sp) };
        let preempted_sp = self.2.compare_and_swap(0, my_sp, Ordering::AcqRel);
        if preempted_sp != 0 {
            // We have preempted at least one thread.
            // Lets see if the other thread has yet read SYSTICK_CTRL into r0.
            let preempted_r0 = unsafe { core::ptr::read_volatile((preempted_sp as *const usize).add(SP_TO_R0_OFFSET)) };
            if preempted_r0 == 0 {
                // The preempted thread has not yet read SYSTICK_CTRL - its value is invalid.
                // Steal the stack pointer atomic from the preempted thread
                // as other interrupting threads should read our r0,
                // and not the one we have just found to be invalid.
                self.2.store(my_sp, Ordering::Release);
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

        if self.2.compare_and_swap(my_sp, preempted_sp, Ordering::AcqRel) == 0 {
            // The R0 value is valid
            let countflag = r0 & 0x10000 > 0; // Mask out the COUNTFLAG.
            self.1.compare_and_swap(false, countflag, Ordering::AcqRel)
        }
        else {
            self.1.load(Ordering::Acquire)
        }
    }

    fn clear_pending_overflow(&self) {
        self.1.store(false, Ordering::Release);
    }
}
