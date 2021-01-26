use crate::{AlarmCounter, AlarmTimer, Tick, UptimeCounter, UptimeOverflow, drivers::cortexm::spin};
use async_trait::async_trait;
use drone_stm32_map::periph::tim::general::GeneralTimMap;
use drone_stm32f4_hal::{
    tim::{
        DirCountUp, GeneralTimCh, GeneralTimChDrv, GeneralTimCntDrv, GeneralTimOvfDrv,
        OutputCompareMode, TimerCompareCh, TimerCounter, TimerOverflow,
    },
    IntToken,
};

pub struct Adapter;

impl<Tim: GeneralTimMap, T: Tick> UptimeCounter<T, Adapter> for GeneralTimCntDrv<Tim, DirCountUp> {
    fn value(&self) -> u32 {
        TimerCounter::value(self)
    }
}

impl<Tim: GeneralTimMap, Int: IntToken> UptimeOverflow<Adapter> for GeneralTimOvfDrv<Tim, Int> {
    const MAX: u32 = 0xFFFF;

    fn overflow_int_enable(&self) {
        TimerOverflow::int_enable(self);
    }

    fn is_pending_overflow(&self) -> bool {
        TimerOverflow::is_pending(self)
    }

    fn clear_pending_overflow(&self) {
        TimerOverflow::clear_pending(self);
    }
}

impl<Tim: GeneralTimMap, T: Tick> AlarmCounter<T, Adapter> for GeneralTimCntDrv<Tim, DirCountUp> {
    fn value(&self) -> u32 {
        TimerCounter::value(self)
    }

    #[inline]
    fn burn_cycles(&self, cycles: u32) {
        spin(cycles);
    }
}

#[async_trait]
impl<Tim: GeneralTimMap, Int: IntToken, Ch: GeneralTimCh<Tim>, T: Tick>
    AlarmTimer<T, Adapter> for GeneralTimChDrv<Tim, Int, Ch, OutputCompareMode>
{
    const MAX: u32 = 0xFFFF;

    async fn next(&mut self, compare: u32, soon: bool) {
        assert!(compare <= 0xFFFF);

        TimerCompareCh::next(self, compare, soon).await;
    }
}
