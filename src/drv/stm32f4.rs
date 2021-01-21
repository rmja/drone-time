use crate::{AlarmCounter, AlarmTimer, Tick, UptimeCounter, UptimeOverflow};
use async_trait::async_trait;
use drone_cortexm::thr::IntToken;
use drone_stm32_map::periph::tim::general::GeneralTimMap;
use drone_stm32f4_hal::tim::{
    DirCountUp, GeneralTimCh, GeneralTimChDrv, GeneralTimCntDrv, GeneralTimOvfDrv,
    OutputCompareMode, TimerCompareCh, TimerCounter, TimerOverflow,
};

pub struct Adapter;

impl<Tim: GeneralTimMap, T: Tick> UptimeCounter<T, Adapter> for GeneralTimCntDrv<Tim, DirCountUp> {
    const MAX: u32 = 0xFFFF;

    fn value(&self) -> u32 {
        TimerCounter::value(self)
    }
}

impl<Tim: GeneralTimMap, Int: IntToken> UptimeOverflow<Adapter> for GeneralTimOvfDrv<Tim, Int> {
    fn overflow_int_enable(&self) {
        self.int_enable();
    }

    fn is_pending_overflow(&self) -> bool {
        self.is_pending()
    }

    fn clear_pending_overflow(&self) {
        self.clear_pending();
    }
}

impl<Tim: GeneralTimMap, T: Tick> AlarmCounter<T, Adapter> for GeneralTimCntDrv<Tim, DirCountUp> {
    fn value(&self) -> u32 {
        TimerCounter::value(self)
    }
}

#[async_trait]
impl<Tim: GeneralTimMap, Int: IntToken, Ch: GeneralTimCh<Tim> + 'static, T: Tick + 'static>
    AlarmTimer<T, Adapter> for GeneralTimChDrv<Tim, Int, Ch, OutputCompareMode>
{
    const MAX: u32 = 0xFFFF;

    async fn next(&mut self, compare: u32, soon: bool) {
        assert!(compare <= 0xFFFF);

        TimerCompareCh::next(self, compare, soon).await;
    }
}
