use drone_core::{reg::prelude::*, token::Token};
use drone_cortexm::map::reg::dwt;

pub(crate) fn dwt_burn_cycles(cycles: u32) {
    let cyccnt = unsafe { dwt::Cyccnt::<Urt>::take() };
    let entry = cyccnt.load_bits() as u32;
    let mut now = cyccnt.load_bits() as u32;
    while now.wrapping_sub(entry) < cycles {
        now = cyccnt.load_bits() as u32;
    }
}