use crate::Tick;

/// The counter backing Uptime.
/// The timer counter must be monotonically increasing in the interval 0 <= value() <= MAX.
pub trait UptimeCounter<T: Tick, A>: Send + Sync {
    /// The maximum counter value.
    const MAX: u32;

    /// The timer period.
    const PERIOD: u64 = Self::MAX as u64 + 1;

    /// Get the current counter value of the timer.
    fn value(&self) -> u32;
}

/// The overflow interrupt control backing Uptime.
pub trait UptimeOverflow<A>: Send + Sync {
    /// Enable timer overflow interrupt.
    fn overflow_int_enable(&self);

    /// Get whether the timer has overflowed.
    fn is_pending_overflow(&self) -> bool;

    /// Clear the flag indicating that the timer has overflowed.
    fn clear_pending_overflow(&self);
}
