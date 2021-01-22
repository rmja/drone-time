pub trait Tick: Send + Sync {
    /// The timer tick frequency, i.e. the number of ticks per second.
    const FREQ: u32;

    /// The CPU frequency, used to burn clock cycles.
    const CPU_FREQ: u32 = 0;
}
