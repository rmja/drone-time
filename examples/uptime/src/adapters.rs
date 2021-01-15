use crate::consts;
use drone_cortexm::thr::ThrExec;
use drone_time::Tick;

pub struct SysTickUptimeTick;
impl Tick for SysTickUptimeTick {
    const FREQ: u32 = consts::SYSCLK.f();
}

pub struct Tim2Tick;
impl Tick for Tim2Tick {
    const FREQ: u32 = consts::TIM2_FREQ;
}
