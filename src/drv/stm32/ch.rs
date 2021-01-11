use drone_cortexm::reg::prelude::*;
use drone_stm32_map::periph::tim::{
    general::{
        traits::*, GeneralTimMap, GeneralTimPeriph, TimCcr2, TimCcr3, TimCcr4,
        TimDierCc2Ie, TimDierCc3Ie, TimDierCc4Ie,
        TimSrCc2If, TimSrCc3If, TimSrCc4If,
    },
};

pub trait TimCh<Tim: GeneralTimMap> {
    /// Set compare register for the channel.
    fn set_compare(tim: &GeneralTimPeriph<Tim>, ccr: u16);
    /// Enable the channel interrupt.
    fn enable_interrupt(tim: &GeneralTimPeriph<Tim>);
    /// Disable the channel interrupt.
    fn disable_interrupt(tim: &GeneralTimPeriph<Tim>);
    /// Get the compare interrupt flag.
    fn is_pending(tim: &GeneralTimPeriph<Tim>) -> bool;
    /// Clear the compare interrupt flag.
    fn clear_pending(tim: &GeneralTimPeriph<Tim>);
}

pub struct TimCh1;
pub struct TimCh2;
pub struct TimCh3;
pub struct TimCh4;

impl<Tim: GeneralTimMap> TimCh<Tim> for TimCh1 {
    fn set_compare(tim: &GeneralTimPeriph<Tim>, ccr: u16) {
        tim.tim_ccr1.store_bits(ccr as u32);
    }

    fn enable_interrupt(tim: &GeneralTimPeriph<Tim>) {
        tim.tim_dier.modify_reg(|r, v| r.cc1ie().set(v));
    }

    fn disable_interrupt(tim: &GeneralTimPeriph<Tim>) {
        tim.tim_dier.modify_reg(|r, v| r.cc1ie().clear(v));
    }

    fn is_pending(tim: &GeneralTimPeriph<Tim>) -> bool {
        tim.tim_sr.cc1if().read_bit()
    }

    fn clear_pending(tim: &GeneralTimPeriph<Tim>) {
        // rc_w0: Clear flag by writing a 0, 1 has no effect.
        let mut val = unsafe { Tim::STimSr::val_from(u32::MAX) };
        tim.tim_sr.cc1if().clear(&mut val);
        tim.tim_sr.store_val(val);
    }
}

impl<Tim: GeneralTimMap + TimCcr2 + TimDierCc2Ie + TimSrCc2If> TimCh<Tim> for TimCh2 {
    fn set_compare(tim: &GeneralTimPeriph<Tim>, ccr: u16) {
        tim.tim_ccr2.store_bits(ccr as u32);
    }

    fn enable_interrupt(tim: &GeneralTimPeriph<Tim>) {
        tim.tim_dier.modify_reg(|r, v| r.cc2ie().set(v));
    }

    fn disable_interrupt(tim: &GeneralTimPeriph<Tim>) {
        tim.tim_dier.modify_reg(|r, v| r.cc2ie().clear(v));
    }

    fn is_pending(tim: &GeneralTimPeriph<Tim>) -> bool {
        tim.tim_sr.cc2if().read_bit()
    }

    fn clear_pending(tim: &GeneralTimPeriph<Tim>) {
        // rc_w0: Clear flag by writing a 0, 1 has no effect.
        let mut val = unsafe { Tim::STimSr::val_from(u32::MAX) };
        tim.tim_sr.cc2if().clear(&mut val);
        tim.tim_sr.store_val(val);
    }
}

impl<Tim: GeneralTimMap + TimCcr3 + TimDierCc3Ie + TimSrCc3If> TimCh<Tim> for TimCh3 {
    fn set_compare(tim: &GeneralTimPeriph<Tim>, ccr: u16) {
        tim.tim_ccr3.store_bits(ccr as u32);
    }

    fn enable_interrupt(tim: &GeneralTimPeriph<Tim>) {
        tim.tim_dier.modify_reg(|r, v| r.cc3ie().set(v));
    }

    fn disable_interrupt(tim: &GeneralTimPeriph<Tim>) {
        tim.tim_dier.modify_reg(|r, v| r.cc3ie().clear(v));
    }

    fn is_pending(tim: &GeneralTimPeriph<Tim>) -> bool {
        tim.tim_sr.cc3if().read_bit()
    }

    fn clear_pending(tim: &GeneralTimPeriph<Tim>) {
        // rc_w0: Clear flag by writing a 0, 1 has no effect.
        let mut val = unsafe { Tim::STimSr::val_from(u32::MAX) };
        tim.tim_sr.cc3if().clear(&mut val);
        tim.tim_sr.store_val(val);
    }
}

impl<Tim: GeneralTimMap + TimCcr4 + TimDierCc4Ie + TimSrCc4If> TimCh<Tim> for TimCh4 {
    fn set_compare(tim: &GeneralTimPeriph<Tim>, ccr: u16) {
        tim.tim_ccr4.store_bits(ccr as u32);
    }

    fn enable_interrupt(tim: &GeneralTimPeriph<Tim>) {
        tim.tim_dier.modify_reg(|r, v| r.cc4ie().set(v));
    }

    fn disable_interrupt(tim: &GeneralTimPeriph<Tim>) {
        tim.tim_dier.modify_reg(|r, v| r.cc4ie().clear(v));
    }

    fn is_pending(tim: &GeneralTimPeriph<Tim>) -> bool {
        tim.tim_sr.cc4if().read_bit()
    }

    fn clear_pending(tim: &GeneralTimPeriph<Tim>) {
        // rc_w0: Clear flag by writing a 0, 1 has no effect.
        let mut val = unsafe { Tim::STimSr::val_from(u32::MAX) };
        tim.tim_sr.cc4if().clear(&mut val);
        tim.tim_sr.store_val(val);
    }
}
