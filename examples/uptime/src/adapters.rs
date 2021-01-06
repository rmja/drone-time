use crate::consts::SYSTICKCLK;
use drone_time::UptimeTick;

pub struct SysTickUptimeTick;

impl UptimeTick for SysTickUptimeTick {
    fn freq() -> u32 {
        SYSTICKCLK.f()
    }
}
