use crate::consts;
use drone_time::Tick;

pub struct SysTickUptimeTick;
impl Tick for SysTickUptimeTick {
    const FREQ: u32 = consts::SYSCLK.f();
}

pub struct Tim2UptimeTick;
impl Tick for Tim2UptimeTick {
    const FREQ: u32 = consts::TIM2_FREQ;
}
