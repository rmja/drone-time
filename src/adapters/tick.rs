pub trait Tick: Send + Sync {
    /// The tick frequency, i.e. the number of ticks per second.
    const FREQ: u32;
}
