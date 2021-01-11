use crate::{AlarmTimer, AlarmTimerNext, AlarmTimerStop, UptimeAlarm};
use core::convert::TryFrom;
use core::marker::PhantomData;
use drone_cortexm::reg::prelude::*;
use drone_cortexm::thr::IntToken;
use drone_cortexm::{fib, thr::prelude::*};
use drone_stm32_map::periph::tim::{
    general::{
        traits::*, GeneralTimMap, GeneralTimPeriph, TimCr1Cms,
        TimCr1Dir,
    },
};

use super::ch::TimCh;

pub struct GeneralTimDrv<Tim: GeneralTimMap, Int: IntToken, Ch: TimCh<Tim>> {
    tim: GeneralTimPeriph<Tim>,
    tim_int: Int,
    channel: PhantomData<Ch>,
}

unsafe impl<Tim: GeneralTimMap, Int: IntToken, Ch: TimCh<Tim>> Sync
    for GeneralTimDrv<Tim, Int, Ch>
{
}

impl<Tim: GeneralTimMap, Int: IntToken, Ch: TimCh<Tim>> GeneralTimDrv<Tim, Int, Ch> {
    pub(crate) fn new(tim: GeneralTimPeriph<Tim>, tim_int: Int) -> Self {
        Self { tim, tim_int, channel: PhantomData}
    }
}

pub trait NewGeneralCh1<Tim: GeneralTimMap, Int: IntToken> {
    fn new_ch1(tim: GeneralTimPeriph<Tim>, tim_int: Int) -> Self;
}

pub trait NewGeneralCh2<Tim: GeneralTimMap, Int: IntToken> {
    fn new_ch2(tim: GeneralTimPeriph<Tim>, tim_int: Int) -> Self;
}

pub trait NewGeneralCh3<Tim: GeneralTimMap, Int: IntToken> {
    fn new_ch3(tim: GeneralTimPeriph<Tim>, tim_int: Int) -> Self;
}

pub trait NewGeneralCh4<Tim: GeneralTimMap, Int: IntToken> {
    fn new_ch4(tim: GeneralTimPeriph<Tim>, tim_int: Int) -> Self;
}

impl<Tim: GeneralTimMap + TimCr1Dir + TimCr1Cms, Int: IntToken, Ch: TimCh<Tim>>
    UptimeAlarm<GeneralTimDrv<Tim, Int, Ch>> for GeneralTimDrv<Tim, Int, Ch>
{
    fn start(&self) {
        self.tim.tim_cr1.modify_reg(|r, v| {
            r.udis().clear(v); // Enable counter overflow event generation
            r.urs().set(v); // Only counter overflow generates an update interrupt
            r.opm().clear(v); // Counter is not stopped at update event
            r.dir().clear(v); // Count up
            r.cms().write(v, 0b00); // Count up or down depending on the direction bit (i.e. count up)
            r.arpe().set(v) // Use buffered auto reload value
        });

        // Set the auto-reload register to a full period.
        // This defines the timer to be a 16 bit timer.
        self.tim.tim_arr.arr().write_bits(0xFFFF);

        // Re-initialize the counter and generate an update of the registers.
        self.tim.tim_egr.ug().set_bit();

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
        // TODO: Find a better way to construct all-ones, something like:
        // let mut val: Tim::TimSrVal = u32::MAX;

        // rc_w0: Clear flag by writing a 0, 1 has no effect.
        let mut val = unsafe { Tim::STimSr::val_from(u32::MAX) };
        self.tim.tim_sr.uif().clear(&mut val);
        self.tim.tim_sr.store_val(val);
    }
}

impl<Tim: GeneralTimMap + TimCr1Dir + TimCr1Cms, Int: IntToken, Ch: TimCh<Tim> + Send>
    AlarmTimer<GeneralTimDrv<Tim, Int, Ch>> for GeneralTimDrv<Tim, Int, Ch>
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

        Ch::set_compare(&self.tim, u16::try_from(compare).unwrap());
        Ch::enable_interrupt(&self.tim);

        AlarmTimerNext::new(self, future)
    }
}

impl<Tim: GeneralTimMap + TimCr1Dir + TimCr1Cms, Int: IntToken, Ch: TimCh<Tim> + Send>
    AlarmTimerStop for GeneralTimDrv<Tim, Int, Ch>
{
    fn stop(&mut self) {
        // Disable capture/compare interrupt.
        Ch::disable_interrupt(&self.tim);
    }
}
