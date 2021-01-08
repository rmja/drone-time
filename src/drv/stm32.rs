use crate::UptimeTimer;
use drone_cortexm::reg::prelude::*;
use drone_stm32_map::periph::tim::{
    advanced::AdvancedTimMap,
    basic::BasicTimMap,
    general::{traits::*, GeneralTimMap, GeneralTimPeriph, TimCr1Cms, TimCr1Dir},
};

pub struct Stm32GeneralTimDrv<Tim: GeneralTimMap>(GeneralTimPeriph<Tim>);

impl<Tim: GeneralTimMap> Stm32GeneralTimDrv<Tim> {
    pub fn new(tim: GeneralTimPeriph<Tim>) -> Self {
        Self(tim)
    }
}

impl<Tim: GeneralTimMap + TimCr1Dir + TimCr1Cms> UptimeTimer<Stm32GeneralTimDrv<Tim>>
    for Stm32GeneralTimDrv<Tim>
{
    fn start(&self) {
        self.0.rcc_busenr_timen.set_bit();

        self.0.tim_cr1.modify_reg(|r, v| {
            r.udis().clear(v); // Enable counter overflow event generation
            r.urs().set(v); // Only counter overflow generates an update interrupt
            r.opm().clear(v); // Counter is not stopped at update event
            r.dir().clear(v); // Count up
            r.cms().write(v, 0b00); // Count up or down depending on the direction bit (i.e. count up)
            r.arpe().set(v) // Use buffered auto reload value (it is always 0xFFFF)
        });

        // Set the auto-reload register to a full period
        self.0.tim_arr.arr().write_bits(0xFFFF);

        // Re-initialize the counter and generate an update of the registers
        self.0.tim_egr.ug().set_bit();

        // Start the counter.
        self.0.tim_cr1.cen().set_bit();
    }

    fn counter(&self) -> u32 {
        self.0.tim_cnt.cnt().read_bits() as u32
    }

    fn counter_max() -> u32 {
        0xFFFF
    }

    fn is_pending_overflow(&self) -> bool {
        self.0.tim_sr.uif().read_bit()
    }

    fn clear_pending_overflow(&self) {
        // TODO: Make this more nicely.
        // Clear bit by writing a 0, 1 has no effect
        self.0.tim_sr.store_bits(0xFFFE);
    }
}
