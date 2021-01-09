use crate::{AlarmTimer, AlarmTimerNext, AlarmTimerStop, UptimeAlarm};
use drone_cortexm::reg::prelude::*;
use drone_cortexm::thr::IntToken;
use drone_cortexm::{fib, reg::prelude::*, thr::prelude::*};
use drone_stm32_map::periph::tim::{
    advanced::AdvancedTimMap,
    basic::BasicTimMap,
    general::{
        traits::*, GeneralTimMap, GeneralTimPeriph, TimCcr2, TimCcr3, TimCcr4, TimCr1Cms,
        TimCr1Dir, TimDierCc2Ie, TimDierCc3Ie, TimDierCc4Ie,
    },
};
use core::convert::TryFrom;
use core::marker::PhantomData;

pub struct Stm32GeneralTimDrv<Tim: GeneralTimMap, Int: IntToken, Ch: TimCh<Tim>> {
    tim: GeneralTimPeriph<Tim>,
    tim_int: Int,
    channel: PhantomData<Ch>,
}

impl<Tim: GeneralTimMap, Int: IntToken, Ch: TimCh<Tim>> Stm32GeneralTimDrv<Tim, Int, Ch> {
    pub fn new(tim: GeneralTimPeriph<Tim>, tim_int: Int, _channel: Ch) -> Self {
        Self {
            tim,
            tim_int,
            channel: PhantomData,
        }
    }
}

unsafe impl<Tim: GeneralTimMap, Int: IntToken, Ch: TimCh<Tim>> Sync for Stm32GeneralTimDrv<Tim, Int, Ch> {}

impl<Tim: GeneralTimMap + TimCr1Dir + TimCr1Cms, Int: IntToken, Ch: TimCh<Tim>>
    UptimeAlarm<Stm32GeneralTimDrv<Tim, Int, Ch>> for Stm32GeneralTimDrv<Tim, Int, Ch>
{
    fn start(&self) {
        self.tim.rcc_busenr_timen.set_bit();

        self.tim.tim_cr1.modify_reg(|r, v| {
            r.udis().clear(v); // Enable counter overflow event generation
            r.urs().set(v); // Only counter overflow generates an update interrupt
            r.opm().clear(v); // Counter is not stopped at update event
            r.dir().clear(v); // Count up
            r.cms().write(v, 0b00); // Count up or down depending on the direction bit (i.e. count up)
            r.arpe().clear(v) // Use unbuffered auto reload value
        });

        // Set the auto-reload register to a full period
        // self.tim.tim_arr.arr().write_bits(0xFFFF);

        // Re-initialize the counter and generate an update of the registers
        // self.tim.tim_egr.ug().set_bit();

        // Start the counter.
        self.tim.tim_cr1.cen().set_bit();
    }

    fn counter(&self) -> u32 {
        self.tim.tim_cnt.cnt().read_bits() as u32
    }

    fn counter_max() -> u32 {
        0xFFFF
    }

    fn is_pending_overflow(&self) -> bool {
        self.tim.tim_sr.uif().read_bit()
    }

    fn clear_pending_overflow(&self) {
        // TODO: Make this more nicely.
        // Clear bit by writing a 0, 1 has no effect
        self.tim.tim_sr.store_bits(0xFFFE);
    }
}

impl<Tim: GeneralTimMap + TimCr1Dir + TimCr1Cms, Int: IntToken, Ch: TimCh<Tim> + Send>
    AlarmTimer<Stm32GeneralTimDrv<Tim, Int, Ch>> for Stm32GeneralTimDrv<Tim, Int, Ch>
{
    type Stop = Self;

    fn counter(&self) -> u32 {
        self.tim.tim_cnt.cnt().read_bits() as u32
    }

    fn counter_max() -> u32 {
        0xFFFF
    }

    /// Returns a future that resolves when the timer counter is equal to `compare`.
    /// Note that compare is not a duration but an absolute timestamp.
    fn next(&mut self, compare: u32) -> AlarmTimerNext<'_, Self::Stop> {
        let future = Box::pin(self.tim_int.add_future(fib::new_fn(move || {
            // let mut ctrl_val = ctrl.load();
            // if ctrl_val.countflag() {
            //     ctrl.store_val(disable(&mut ctrl_val).val());
            //     unsafe { set_bit(&pendstclr) };
            //     fib::Complete(())
            // } else {
            //     fib::Yielded(())
            // }
            if true {
                fib::Complete(())
            } else {
                fib::Yielded(())
            }
        })));

        Ch::set_ccr(&self.tim, u16::try_from(compare).unwrap());
        Ch::enable_interrupt(&self.tim);

        AlarmTimerNext::new(self, future)
    }
}

impl<Tim: GeneralTimMap + TimCr1Dir + TimCr1Cms, Int: IntToken, Ch: TimCh<Tim> + Send> AlarmTimerStop
    for Stm32GeneralTimDrv<Tim, Int, Ch>
{
    fn stop(&mut self) {
        // Disable capture/compare interrupt.
        Ch::disable_interrupt(&self.tim);
    }
}

pub trait TimCh<Tim: GeneralTimMap> {
    /// Set compare register for the channel.
    fn set_ccr(tim: &GeneralTimPeriph<Tim>, ccr: u16);
    fn enable_interrupt(tim: &GeneralTimPeriph<Tim>);
    fn disable_interrupt(tim: &GeneralTimPeriph<Tim>);
}

pub struct TimCh1;
pub struct TimCh2;
pub struct TimCh3;
pub struct TimCh4;

impl<Tim: GeneralTimMap> TimCh<Tim> for TimCh1 {
    fn set_ccr(tim: &GeneralTimPeriph<Tim>, ccr: u16) {
        tim.tim_ccr1.store_bits(ccr as u32);
    }

    fn enable_interrupt(tim: &GeneralTimPeriph<Tim>) {
        tim.tim_dier.modify_reg(|r, v| r.cc1ie().set(v));
    }

    fn disable_interrupt(tim: &GeneralTimPeriph<Tim>) {
        tim.tim_dier.modify_reg(|r, v| r.cc1ie().clear(v));
    }
}

impl<Tim: GeneralTimMap + TimCcr2 + TimDierCc2Ie> TimCh<Tim> for TimCh2 {
    fn set_ccr(tim: &GeneralTimPeriph<Tim>, ccr: u16) {
        tim.tim_ccr2.store_bits(ccr as u32);
    }

    fn enable_interrupt(tim: &GeneralTimPeriph<Tim>) {
        tim.tim_dier.modify_reg(|r, v| r.cc2ie().set(v));
    }

    fn disable_interrupt(tim: &GeneralTimPeriph<Tim>) {
        tim.tim_dier.modify_reg(|r, v| r.cc2ie().clear(v));
    }
}

impl<Tim: GeneralTimMap + TimCcr3 + TimDierCc3Ie> TimCh<Tim> for TimCh3 {
    fn set_ccr(tim: &GeneralTimPeriph<Tim>, ccr: u16) {
        tim.tim_ccr3.store_bits(ccr as u32);
    }

    fn enable_interrupt(tim: &GeneralTimPeriph<Tim>) {
        tim.tim_dier.modify_reg(|r, v| r.cc3ie().set(v));
    }

    fn disable_interrupt(tim: &GeneralTimPeriph<Tim>) {
        tim.tim_dier.modify_reg(|r, v| r.cc3ie().clear(v));
    }
}

impl<Tim: GeneralTimMap + TimCcr4 + TimDierCc4Ie> TimCh<Tim> for TimCh4 {
    fn set_ccr(tim: &GeneralTimPeriph<Tim>, ccr: u16) {
        tim.tim_ccr4.store_bits(ccr as u32);
    }

    fn enable_interrupt(tim: &GeneralTimPeriph<Tim>) {
        tim.tim_dier.modify_reg(|r, v| r.cc4ie().set(v));
    }

    fn disable_interrupt(tim: &GeneralTimPeriph<Tim>) {
        tim.tim_dier.modify_reg(|r, v| r.cc4ie().clear(v));
    }
}
