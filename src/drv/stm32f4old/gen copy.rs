use crate::{AlarmCounter, AlarmTimer, AlarmTimerNext, AlarmTimerStop, Tick, UptimeTimer};
use core::{convert::TryFrom, marker::PhantomData};
use alloc::sync::Arc;
use drone_core::token::Token;
use drone_cortexm::{fib, reg::prelude::*, thr::prelude::*};
use drone_stm32_map::periph::tim::general::{
    traits::*, GeneralTimMap, GeneralTimPeriph, TimCr1Cms, TimCr1Dir,
};
use futures::future;

use super::gen_ch::TimCh;

pub struct GeneralTimDrv<
    Tim: GeneralTimMap + TimCr1Dir + TimCr1Cms,
    UpTim: UptimeTimer<T, Self> + 'static,
    AlCnt: AlarmCounter<T, Self> + 'static,
    AlTim: AlarmTimer<T, Self> + 'static,
    T: Tick + 'static,
> {
    tim: Arc<GeneralTimDiverged<Tim>>,
    pub uptime_timer: UpTim,
    pub alarm_counter: AlCnt,
    pub alarm_timer: AlTim,
    tick: PhantomData<T>,
}

pub struct UptimeTimerDrv<Tim: GeneralTimMap, T: Tick>(Arc<GeneralTimDiverged<Tim>>, PhantomData<T>);

pub struct AlarmCounterDrv<Tim: GeneralTimMap, T: Tick>(Arc<GeneralTimDiverged<Tim>>, PhantomData<T>);

pub struct AlarmTimerDrv<Tim: GeneralTimMap, Int: IntToken, Ch: TimCh<Tim>, T: Tick>{
    tim: Arc<GeneralTimDiverged<Tim>>,
    tim_int: Int,
    ch: PhantomData<Ch>,
    tick: PhantomData<T>,
}

pub struct GeneralTimDiverged<Tim: GeneralTimMap> {
    pub(crate) tim_cr1: Tim::STimCr1,
    pub(crate) tim_dier: Tim::CTimDier,
    pub(crate) tim_sr: Tim::CTimSr,
    pub(crate) tim_arr: Tim::STimArr,
    pub(crate) tim_egr: Tim::STimEgr,
    pub(crate) tim_cnt: Tim::CTimCnt,
}

impl<
        Tim: GeneralTimMap + TimCr1Dir + TimCr1Cms,
        Int: IntToken,
        Ch: TimCh<Tim> + Send,
        T: Tick + 'static,
    > GeneralTimDrv<Tim, UptimeTimerDrv<Tim, T>, AlarmCounterDrv<Tim, T>, AlarmTimerDrv<Tim, Int, Ch, T>, T>
{
    pub(crate) fn new(tim: GeneralTimPeriph<Tim>, tim_int: Int, _tick: T) -> Self {
        let tim = Arc::new(GeneralTimDiverged {
            tim_cr1: tim.tim_cr1,
            tim_dier: tim.tim_dier.into_copy(),
            tim_sr: tim.tim_sr.into_copy(),
            tim_arr: tim.tim_arr,
            tim_egr: tim.tim_egr,
            tim_cnt: tim.tim_cnt.into_copy(),
        });
        Self {
            tim: tim.clone(),
            uptime_timer: UptimeTimerDrv(tim.clone(), PhantomData),
            alarm_counter: AlarmCounterDrv(tim.clone(), PhantomData),
            alarm_timer: AlarmTimerDrv {
                tim,
                tim_int,
                ch: PhantomData,
                tick: PhantomData,
            },
            tick: PhantomData,
        }
    }

    pub fn start(&self) {
        self.tim.tim_cr1.modify_reg(|r, v| {
            r.udis().clear(v); // Enable counter overflow event generation
            r.urs().set(v); // Only counter overflow generates an update interrupt
            r.opm().clear(v); // Counter is not stopped at update event
            r.dir().clear(v); // Count up
            r.cms().write(v, 0b00); // Count up or down depending on the direction bit (i.e. count up)
            r.arpe().set(v) // Use buffered auto reload value
        });

        self.tim.tim_dier.modify_reg(|r, v| {
            r.uie().set(v); // Enable update interrupt
        });

        // Set the auto-reload register to a full period.
        // This defines the timer to be a 16 bit timer.
        self.tim.tim_arr.arr().write_bits(0xFFFF);

        // Re-initialize the counter and generate an update of the registers.
        self.tim.tim_egr.ug().set_bit();

        // Start the counter.
        self.tim.tim_cr1.cen().set_bit();
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

unsafe impl<Tim: GeneralTimMap, T: Tick> Sync for UptimeTimerDrv<Tim, T> {}

impl<Tim: GeneralTimMap, T: Tick + Sync, A> UptimeTimer<T, A>
    for UptimeTimerDrv<Tim, T>
{
    const MAX: u32 = 0xFFFF;

    fn counter(&self) -> u32 {
        self.0.tim_cnt.cnt().read_bits() as u32
    }

    fn is_pending_overflow(&self) -> bool {
        self.0.tim_sr.uif().read_bit()
    }

    fn clear_pending_overflow(&self) {
        // rc_w0: Clear flag by writing a 0, 1 has no effect.
        let mut val = unsafe { Tim::STimSr::val_from(u32::MAX) };
        self.0.tim_sr.uif().clear(&mut val);
        self.0.tim_sr.store_val(val);
    }
}

impl<Tim: GeneralTimMap, T: Tick + Sync, A> AlarmCounter<T, A> for AlarmCounterDrv<Tim, T> {
    fn value(&self) -> u32 {
        self.0.tim_cnt.cnt().read_bits() as u32
    }
}

impl<
        Tim: GeneralTimMap + TimCr1Dir + TimCr1Cms,
        Int: IntToken,
        Ch: TimCh<Tim> + Send + 'static,
        T: Tick + Sync + 'static,
        A: 'static,
    > AlarmTimer<T, A> for AlarmTimerDrv<Tim, Int, Ch, T>
{
    type Stop = Self;
    const MAX: u32 = 0xFFFF;

    fn next(&mut self, compare: u32, soon: bool) -> AlarmTimerNext<'_, Self::Stop> {
        let tim_sr = self.tim.tim_sr;
        let tim_dier = self.tim.tim_dier;
        let tim_ch_ccr = unsafe { Ch::CTimCcr::take() };
        let timeout_future = Box::pin(self.tim_int.add_future(fib::new_fn(move || {
            if Ch::is_pending(tim_sr) {
                Ch::clear_pending(tim_sr);
                Ch::disable_interrupt(tim_dier);
                fib::Complete(())
            } else {
                fib::Yielded(())
            }
        })));

        let compare = u16::try_from(compare).unwrap();
        Ch::set_compare(tim_ch_ccr, compare);

        let already_passed = if soon {
            // Sample counter after interrupt is setup.
            let counter = self.tim.tim_cnt.cnt().read_bits() as u16;

            // Let's see if counter is later than compare in which case the time has already elapsed
            counter.wrapping_sub(compare) > 0x8000
        } else {
            false
        };

        if already_passed {
            // The counter has already passed the comfigured compare value - skip interrupt
            drop(timeout_future);
            Ch::clear_pending(tim_sr);

            AlarmTimerNext::new(self, Box::pin(future::ready(())))
        } else {
            Ch::enable_interrupt(tim_dier);

            AlarmTimerNext::new(self, Box::pin(timeout_future))
        }
    }
}

impl<Tim: GeneralTimMap + TimCr1Dir + TimCr1Cms, Int: IntToken, Ch: TimCh<Tim> + Send, T: Tick>
    AlarmTimerStop for AlarmTimerDrv<Tim, Int, Ch, T>
{
    fn stop(&mut self) {
        // Disable capture/compare interrupt.
        Ch::disable_interrupt(self.tim.tim_dier);
    }
}
