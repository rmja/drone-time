use crate::JiffiesTimer;
use drone_cortexm::{map::periph::sys_tick::SysTickPeriph, reg::prelude::*};

struct Adapter;

impl JiffiesTimer<Adapter> for SysTickPeriph {
    fn counter(&self) -> u32 {
        // SysTick counts down, but the returned counter value must count up.
        0xFF_FF_FF - self.stk_val.load_bits() as u32
    }

    fn counter_max() -> u32 {
        // SysTick is a 24 bit counter.
        0xFF_FF_FF
    }

    fn try_clear_pending_overflow(&self) -> bool {
        // Disable overflow interrupt.
        self.stk_ctrl.tickint.clear_bit();

        let is_pending = self.scb_icsr_pendstset.read_bit();
        if is_pending {
            // Only clear flag if interrupt was pending.

            // TODO: ACTUALLY CLEAR FLAG
            // self.scb_icsr_pendstclr.set_bit();
        }

        // Re-enable overflow interrupt.
        self.stk_ctrl.tickint.set_bit();

        is_pending
    }
}
