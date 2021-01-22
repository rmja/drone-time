use drone_core::reg::prelude::*;
use drone_cortexm::map::{
    periph::sys_tick::SysTickPeriph,
    reg::{scb, stk},
};

#[allow(dead_code)]
pub struct SysTickDiverged {
    pub(crate) scb_icsr_pendstclr: scb::icsr::Pendstclr<Crt>,
    pub(crate) scb_icsr_pendstset: scb::icsr::Pendstset<Srt>,
    pub(crate) stk_ctrl: stk::Ctrl<Crt>,
    pub(crate) stk_load: stk::Load<Srt>,
    pub(crate) stk_val: stk::Val<Srt>,
}

impl SysTickDiverged {
    pub(crate) fn new(systick: SysTickPeriph) -> Self {
        Self {
            scb_icsr_pendstclr: systick.scb_icsr_pendstclr.into_copy(),
            scb_icsr_pendstset: systick.scb_icsr_pendstset,
            stk_ctrl: systick.stk_ctrl.into_copy(),
            stk_load: systick.stk_load,
            stk_val: systick.stk_val,
        }
    }
}
