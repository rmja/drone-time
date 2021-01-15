use crate::{AlarmCounter, AlarmTimer, AlarmTimerNext, AlarmTimerStop, Tick, UptimeTimer};
use core::{convert::TryFrom, marker::PhantomData};
use drone_cortexm::{fib, reg::prelude::*, thr::prelude::*};
use drone_stm32_map::periph::tim::general::{
    traits::*, GeneralTimMap, GeneralTimPeriph, TimCr1Cms, TimCr1Dir,
};

use super::gen_ch::TimCh;

pub struct GeneralTimDrv<
    Tim: GeneralTimMap + TimCr1Dir + TimCr1Cms,
    Int: IntToken,
    Ch: TimCh<Tim>,
    T: Tick,
> {
    uptime_timer: UptimeTimerDrv<Tim>,
    alarm_counter: AlarmCounterDrv<Tim>,
    alarm_timer: AlarmTimerDrv<Tim, Int, Ch>,
    tick: PhantomData<T>,
}

pub struct UptimeTimerDrv<Tim: GeneralTimMap + TimCr1Dir + TimCr1Cms> {
    tim_cr1: Tim::STimCr1,
    tim_sr: Tim::CTimSr,
    tim_arr: Tim::STimArr,
    tim_egr: Tim::STimEgr,
    tim_cnt: Tim::CTimCnt,
}

pub struct AlarmCounterDrv<Tim: GeneralTimMap>(Tim::CTimCnt);

pub struct AlarmTimerDrv<Tim: GeneralTimMap, Int: IntToken, Ch: TimCh<Tim>> {
    tim_int: Int,
    tim_dier: Tim::CTimDier,
    tim_sr: Tim::CTimSr,
    tim_ccr: Ch::STimCcr,
}

pub struct GeneralTimDiverged<Tim: GeneralTimMap, Ch: TimCh<Tim>> {
    pub(crate) tim_cr1: Tim::STimCr1,
    pub(crate) tim_dier: Tim::CTimDier,
    pub(crate) tim_sr: Tim::CTimSr,
    pub(crate) tim_arr: Tim::STimArr,
    pub(crate) tim_egr: Tim::STimEgr,
    pub(crate) tim_cnt: Tim::CTimCnt,
    pub(crate) tim_ccr: Ch::STimCcr,
}

unsafe impl<Tim: GeneralTimMap + TimCr1Dir + TimCr1Cms, Int: IntToken, Ch: TimCh<Tim>, T: Tick> Sync
    for GeneralTimDrv<Tim, Int, Ch, T>
{
}

impl<
        Tim: GeneralTimMap + TimCr1Dir + TimCr1Cms,
        Int: IntToken,
        Ch: TimCh<Tim> + Send + 'static,
        T: Tick + 'static,
    > GeneralTimDrv<Tim, Int, Ch, T>
{
    pub(crate) fn new(tim: GeneralTimPeriph<Tim>, tim_int: Int, _tick: T) -> Self {
        let tim = Ch::new_diverged(tim);
        Self {
            uptime_timer: UptimeTimerDrv {
                tim_cr1: tim.tim_cr1,
                tim_sr: tim.tim_sr,
                tim_arr: tim.tim_arr,
                tim_egr: tim.tim_egr,
                tim_cnt: tim.tim_cnt,
            },
            alarm_counter: AlarmCounterDrv(tim.tim_cnt),
            alarm_timer: AlarmTimerDrv {
                tim_int,
                tim_dier: tim.tim_dier,
                tim_sr: tim.tim_sr,
                tim_ccr: tim.tim_ccr,
            },
            tick: PhantomData,
        }
    }

    pub fn split(
        self,
    ) -> (
        impl UptimeTimer<T, Self>,
        impl AlarmCounter<T, Self>,
        impl AlarmTimer<T, Self>,
    ) {
        (self.uptime_timer, self.alarm_counter, self.alarm_timer)
    }
}

pub trait NewGeneralCh1<Tim: GeneralTimMap, Int: IntToken, T: Tick> {
    fn new_ch1(tim: GeneralTimPeriph<Tim>, tim_int: Int, tick: T) -> Self;
}

pub trait NewGeneralCh2<Tim: GeneralTimMap, Int: IntToken, T: Tick> {
    fn new_ch2(tim: GeneralTimPeriph<Tim>, tim_int: Int, tick: T) -> Self;
}

pub trait NewGeneralCh3<Tim: GeneralTimMap, Int: IntToken, T: Tick> {
    fn new_ch3(tim: GeneralTimPeriph<Tim>, tim_int: Int, tick: T) -> Self;
}

pub trait NewGeneralCh4<Tim: GeneralTimMap, Int: IntToken, T: Tick> {
    fn new_ch4(tim: GeneralTimPeriph<Tim>, tim_int: Int, tick: T) -> Self;
}

impl<Tim: GeneralTimMap + TimCr1Dir + TimCr1Cms, T: Tick, A> UptimeTimer<T, A>
    for UptimeTimerDrv<Tim>
{
    const MAX: u32 = 0xFFFF;

    fn start(&self) {
        self.tim_cr1.modify_reg(|r, v| {
            r.udis().clear(v); // Enable counter overflow event generation
            r.urs().set(v); // Only counter overflow generates an update interrupt
            r.opm().clear(v); // Counter is not stopped at update event
            r.dir().clear(v); // Count up
            r.cms().write(v, 0b00); // Count up or down depending on the direction bit (i.e. count up)
            r.arpe().set(v) // Use buffered auto reload value
        });

        // Set the auto-reload register to a full period.
        // This defines the timer to be a 16 bit timer.
        self.tim_arr.arr().write_bits(0xFFFF);

        // Re-initialize the counter and generate an update of the registers.
        self.tim_egr.ug().set_bit();

        // Start the counter.
        self.tim_cr1.cen().set_bit();
    }

    fn counter(&self) -> u32 {
        self.tim_cnt.cnt().read_bits() as u32
    }

    fn is_pending_overflow(&self) -> bool {
        self.tim_sr.uif().read_bit()
    }

    fn clear_pending_overflow(&self) {
        // TODO: Find a better way to construct all-ones, something like:
        // let mut val: Tim::TimSrVal = u32::MAX;

        // rc_w0: Clear flag by writing a 0, 1 has no effect.
        let mut val = unsafe { Tim::STimSr::val_from(u32::MAX) };
        self.tim_sr.uif().clear(&mut val);
        self.tim_sr.store_val(val);
    }
}

impl<Tim: GeneralTimMap, T: Tick, A> AlarmCounter<T, A> for AlarmCounterDrv<Tim> {
    fn value(&self) -> u32 {
        self.0.cnt().read_bits() as u32
    }
}

impl<
        Tim: GeneralTimMap + TimCr1Dir + TimCr1Cms,
        Int: IntToken,
        Ch: TimCh<Tim> + Send + 'static,
        T: Tick + 'static,
        A: 'static,
    > AlarmTimer<T, A> for AlarmTimerDrv<Tim, Int, Ch>
{
    type Stop = Self;
    const MAX: u32 = 0xFFFF;

    fn next(&mut self, compare: u32) -> AlarmTimerNext<'_, Self::Stop> {
        let tim_sr = self.tim_sr;
        let tim_dier = self.tim_dier;
        let future = Box::pin(self.tim_int.add_future(fib::new_fn(move || {
            if Ch::is_pending(tim_sr) {
                Ch::clear_pending(tim_sr);
                Ch::disable_interrupt(tim_dier);
                fib::Complete(())
            } else {
                fib::Yielded(())
            }
        })));

        Ch::set_compare(&self.tim_ccr, u16::try_from(compare).unwrap());
        Ch::enable_interrupt(self.tim_dier);

        AlarmTimerNext::new(self, future)
    }
}

impl<Tim: GeneralTimMap + TimCr1Dir + TimCr1Cms, Int: IntToken, Ch: TimCh<Tim> + Send>
    AlarmTimerStop for AlarmTimerDrv<Tim, Int, Ch>
{
    fn stop(&mut self) {
        // Disable capture/compare interrupt.
        Ch::disable_interrupt(self.tim_dier);
    }
}
