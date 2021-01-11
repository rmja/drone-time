use crate::consts;
use drone_time::Tick;

pub struct SysTickUptimeTick;
impl Tick for SysTickUptimeTick {
    fn freq() -> u32 {
        consts::SYSTICKCLK.f()
    }
}

pub struct Tim2UptimeTick;
impl Tick for Tim2UptimeTick {
    fn freq() -> u32 {
        consts::TIM2_FREQ
    }
}
