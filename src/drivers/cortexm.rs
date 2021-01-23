const CYCLES_PER_ITERATION: u32 = 3;

#[inline]
pub(crate) fn spin(mut cycles: u32) {
    cycles /= CYCLES_PER_ITERATION;
    unsafe {
        asm!(
            "0:",
            "subs {0}, {0}, #1",
            "bne 0b", // The 'b' suffix tells that the jump should be to the "previously defined" label "0".
            inout(reg) cycles,
            options(nomem, nostack)
        );
    }
    // We must use cycles after it is returned, rust will otherwise optimize the asm away.
    assert_eq!(0, cycles);
}