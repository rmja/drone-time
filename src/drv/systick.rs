use core::sync::atomic::{AtomicBool, Ordering};

use crate::JiffiesTimer;
use drone_cortexm::{map::periph::sys_tick::SysTickPeriph, processor::interrupt, reg::prelude::*};

pub struct SysTickDrv(SysTickPeriph, AtomicBool);

impl SysTickDrv {
    pub fn init(systick: SysTickPeriph) -> SysTickDrv {
        SysTickDrv(systick, AtomicBool::new(false))
    }
}

impl JiffiesTimer<SysTickDrv> for SysTickDrv {
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

    fn is_pending_overflow(&self) -> bool {
        interrupt::critical(|_| {
            // SysTick is inherently racy so we need to disable interrupts while reading COUNTFLAG and storing its value.

            // Read the COUNTFLAG value - this reads the register and returns 1 if timer counted to 0 _since last time this was read_.
            let is_pending = self.0.stk_ctrl.countflag.read_bit();

            // Store the flag in case that is_pending_overflow() is called multiple times for the overflow.
            self.1
                .compare_and_swap(false, is_pending, Ordering::Acquire)
                || is_pending
        })
    }

    fn clear_pending_overflow(&self) {
        // The flag is auto-cleared, so there is no reason to do it manually
        // drone_cortexm::reg::field::WRwRegFieldBitAtomic::set_bit(&self.0.scb_icsr_pendstclr); // self.0.scb_icsr_pendstclr.set_bit();

        self.1.store(false, Ordering::Release);
    }
}
