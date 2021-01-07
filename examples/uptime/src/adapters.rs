use crate::consts;
use drone_time::UptimeTick;

pub struct SysTickUptimeTick;
impl UptimeTick for SysTickUptimeTick {
    fn freq() -> u32 { consts::SYSTICKCLK.f() }
}

pub struct Tim2UptimeTick;
impl UptimeTick for Tim2UptimeTick {
    fn freq() -> u32 { consts::TIM2_FREQ }
}
