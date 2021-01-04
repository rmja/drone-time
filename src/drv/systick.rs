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

    fn has_pending_overflow(&self) -> bool {
        self.scb_icsr_pendstset.read_bit()
    }

    fn clear_pending_overflow(&self) {
        // self.scb_icsr_pendstclr.set_bit();
    }
}
