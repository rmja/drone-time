const CYCLES_PER_ITERATION: u32 = 3;

pub(crate) fn burn(mut cycles: u32) {
    cycles /= CYCLES_PER_ITERATION;
    unsafe {
        asm!(
            "loop:",
            "subs {0}, {0}, #1",
            "bne loop",
            in(reg) cycles
        );
    }
}