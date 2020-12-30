use crate::{CountDown, JiffiesTimer};
use drone_cortexm::{map::periph::sys_tick::SysTickPeriph, reg::prelude::*};

struct Adapter;

impl JiffiesTimer<CountDown, Adapter> for SysTickPeriph {
    fn counter(&self) -> u32 {
        self.stk_val.load_bits() as u32
    }

    fn counter_max() -> u32 {
        0xFF_FF_FF // SysTick is a 24 bit counter
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
