use crate::JiffiesTimer;
use drone_cortexm::{map::periph::sys_tick::SysTickPeriph, reg::prelude::*};

pub struct SysTickDrv(SysTickPeriph);

impl SysTickDrv {
    pub fn init(systick: SysTickPeriph) -> SysTickDrv {
        SysTickDrv(systick)
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
        self.0.scb_icsr_pendstset.read_bit()
    }

    fn clear_pending_overflow(&self) {
        // self.0.scb_icsr_pendstclr.set_bit(); // Is disambiguate
        drone_cortexm::reg::field::WRwRegFieldBitAtomic::set_bit(&self.0.scb_icsr_pendstclr);
    }
}
