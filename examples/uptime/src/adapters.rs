use crate::consts::SYSTICKCLK;
use drone_time::JiffiesClock;

pub struct UptimeClock;

impl JiffiesClock for UptimeClock {
    fn freq() -> u32 {
        SYSTICKCLK.f()
    }
}
