use drone_cortexm::reg::prelude::*;
use drone_stm32_map::periph::tim::{
    general::{
        traits::*, GeneralTimMap, GeneralTimPeriph, TimCcr2, TimCcr3, TimCcr4,
        TimDierCc2Ie, TimDierCc3Ie, TimDierCc4Ie,
        TimSrCc2If, TimSrCc3If, TimSrCc4If,
    },
};

use super::gen::GeneralTimDiverged;

pub struct TimCh1<Tim: GeneralTimMap> {
    tim_sr: Tim::CTimSr,
    tim_dier: Tim::CTimDier,
    tim_ccr1: Tim::CTimCcr1,
}

pub struct TimCh2<Tim: GeneralTimMap + TimCcr2> {
    tim_sr: Tim::CTimSr,
    tim_dier: Tim::CTimDier,
    tim_ccr2: Tim::CTimCcr2,
}

pub struct TimCh3<Tim: GeneralTimMap + TimCcr3> {
    tim_sr: Tim::CTimSr,
    tim_dier: Tim::CTimDier,
    tim_ccr3: Tim::CTimCcr3,
}

pub struct TimCh4<Tim: GeneralTimMap + TimCcr4> {
    tim_sr: Tim::CTimSr,
    tim_dier: Tim::CTimDier,
    tim_ccr4: Tim::CTimCcr4,
}

pub trait TimCh<Tim: GeneralTimMap> where Self : Sized, Self::STimCcr: Send {
    type STimCcr;

    fn new_diverged(tim: GeneralTimPeriph<Tim>) -> GeneralTimDiverged<Tim, Self>;
    /// Set compare register for the channel.
    fn set_compare(tim_ccr: &Self::STimCcr,  value: u16);
    /// Enable the channel interrupt.
    fn enable_interrupt(tim_dier: Tim::CTimDier);
    /// Disable the channel interrupt.
    fn disable_interrupt(tim_dier: Tim::CTimDier);
    /// Get the compare interrupt flag.
    fn is_pending(tim_sr: Tim::CTimSr) -> bool;
    /// Clear the compare interrupt flag.
    fn clear_pending(tim_sr: Tim::CTimSr);
}

impl<Tim: GeneralTimMap> TimCh<Tim> for TimCh1<Tim> {
    type STimCcr = Tim::STimCcr1;

    fn new_diverged(tim: GeneralTimPeriph<Tim>) -> GeneralTimDiverged<Tim, Self> {
        GeneralTimDiverged {
            tim_cr1: tim.tim_cr1,
            tim_dier: tim.tim_dier.into_copy(),
            tim_sr: tim.tim_sr.into_copy(),
            tim_arr: tim.tim_arr,
            tim_egr: tim.tim_egr,
            tim_cnt: tim.tim_cnt,
            tim_ccr: tim.tim_ccr1,
        }
    }

    fn set_compare(tim_ccr: &Self::STimCcr, value: u16) {
        tim_ccr.store_bits(value as u32);
    }

    fn enable_interrupt(tim_dier: Tim::CTimDier) {
        tim_dier.modify_reg(|r, v| r.cc1ie().set(v));
    }

    fn disable_interrupt(tim_dier: Tim::CTimDier) {
        tim_dier.modify_reg(|r, v| r.cc1ie().clear(v));
    }

    fn is_pending(tim_sr: Tim::CTimSr) -> bool {
        tim_sr.cc1if().read_bit()
    }

    fn clear_pending(tim_sr: Tim::CTimSr) {
        // rc_w0: Clear flag by writing a 0, 1 has no effect.
        let mut val = unsafe { Tim::CTimSr::val_from(u32::MAX) };
        tim_sr.cc1if().clear(&mut val);
        tim_sr.store_val(val);
    }
}

impl<Tim: GeneralTimMap + TimCcr2 + TimDierCc2Ie + TimSrCc2If> TimCh<Tim> for TimCh2<Tim> {
    type STimCcr = Tim::STimCcr2;

    fn new_diverged(tim: GeneralTimPeriph<Tim>) -> GeneralTimDiverged<Tim, Self> {
        GeneralTimDiverged {
            tim_cr1: tim.tim_cr1,
            tim_dier: tim.tim_dier.into_copy(),
            tim_sr: tim.tim_sr.into_copy(),
            tim_arr: tim.tim_arr,
            tim_egr: tim.tim_egr,
            tim_cnt: tim.tim_cnt,
            tim_ccr: tim.tim_ccr2,
        }
    }

    fn set_compare(tim_ccr: &Self::STimCcr, value: u16) {
        tim_ccr.store_bits(value as u32);
    }

    fn enable_interrupt(tim_dier: Tim::CTimDier) {
        tim_dier.modify_reg(|r, v| r.cc2ie().set(v));
    }

    fn disable_interrupt(tim_dier: Tim::CTimDier) {
        tim_dier.modify_reg(|r, v| r.cc2ie().clear(v));
    }

    fn is_pending(tim_sr: Tim::CTimSr) -> bool {
        tim_sr.cc2if().read_bit()
    }

    fn clear_pending(tim_sr: Tim::CTimSr) {
        // rc_w0: Clear flag by writing a 0, 1 has no effect.
        let mut val = unsafe { Tim::CTimSr::val_from(u32::MAX) };
        tim_sr.cc2if().clear(&mut val);
        tim_sr.store_val(val);
    }
}

impl<Tim: GeneralTimMap + TimCcr3 + TimDierCc3Ie + TimSrCc3If> TimCh<Tim> for TimCh3<Tim> {
    type STimCcr = Tim::STimCcr3;

    fn new_diverged(tim: GeneralTimPeriph<Tim>) -> GeneralTimDiverged<Tim, Self> {
        GeneralTimDiverged {
            tim_cr1: tim.tim_cr1,
            tim_dier: tim.tim_dier.into_copy(),
            tim_sr: tim.tim_sr.into_copy(),
            tim_arr: tim.tim_arr,
            tim_egr: tim.tim_egr,
            tim_cnt: tim.tim_cnt,
            tim_ccr: tim.tim_ccr3,
        }
    }

    fn set_compare(tim_ccr: &Self::STimCcr, value: u16) {
        tim_ccr.store_bits(value as u32);
    }

    fn enable_interrupt(tim_dier: Tim::CTimDier) {
        tim_dier.modify_reg(|r, v| r.cc3ie().set(v));
    }

    fn disable_interrupt(tim_dier: Tim::CTimDier) {
        tim_dier.modify_reg(|r, v| r.cc3ie().clear(v));
    }

    fn is_pending(tim_sr: Tim::CTimSr) -> bool {
        tim_sr.cc3if().read_bit()
    }

    fn clear_pending(tim_sr: Tim::CTimSr) {
        // rc_w0: Clear flag by writing a 0, 1 has no effect.
        let mut val = unsafe { Tim::CTimSr::val_from(u32::MAX) };
        tim_sr.cc3if().clear(&mut val);
        tim_sr.store_val(val);
    }
}

impl<Tim: GeneralTimMap + TimCcr4 + TimDierCc4Ie + TimSrCc4If> TimCh<Tim> for TimCh4<Tim> {
    type STimCcr = Tim::STimCcr4;

    fn new_diverged(tim: GeneralTimPeriph<Tim>) -> GeneralTimDiverged<Tim, Self> {
        GeneralTimDiverged {
            tim_cr1: tim.tim_cr1,
            tim_dier: tim.tim_dier.into_copy(),
            tim_sr: tim.tim_sr.into_copy(),
            tim_arr: tim.tim_arr,
            tim_egr: tim.tim_egr,
            tim_cnt: tim.tim_cnt,
            tim_ccr: tim.tim_ccr4,
        }
    }

    fn set_compare(tim_ccr: &Self::STimCcr, value: u16) {
        tim_ccr.store_bits(value as u32);
    }

    fn enable_interrupt(tim_dier: Tim::CTimDier) {
        tim_dier.modify_reg(|r, v| r.cc4ie().set(v));
    }

    fn disable_interrupt(tim_dier: Tim::CTimDier) {
        tim_dier.modify_reg(|r, v| r.cc4ie().clear(v));
    }

    fn is_pending(tim_sr: Tim::CTimSr) -> bool {
        tim_sr.cc4if().read_bit()
    }

    fn clear_pending(tim_sr: Tim::CTimSr) {
        // rc_w0: Clear flag by writing a 0, 1 has no effect.
        let mut val = unsafe { Tim::CTimSr::val_from(u32::MAX) };
        tim_sr.cc4if().clear(&mut val);
        tim_sr.store_val(val);
    }
}
