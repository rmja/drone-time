pub struct CountUp;
pub struct CountDown;

pub trait JiffiesClock {
    fn freq() -> u32;
}

pub trait JiffiesTimer<Direction, A> {
    /// Get the current counter value of the timer.
    /// The value must be in the interval 0 <= counter <= max().
    fn counter(&self) -> u32;

    /// Get the maximum counter value.
    fn counter_max() -> u32;

    /// Try and clear pending overflow flag in an atomic operation; return true if flag actually cleared.
    fn try_clear_pending_overflow(&self) -> bool;
}