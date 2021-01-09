/// The alarm backing Uptime.
/// The timer must be monotonically increasing in the interval 0 <= counter() <= counter_max().
pub trait UptimeAlarm<A>: Sync {
    /// Start the timer.
    fn start(&self);

    /// Get the current counter value of the timer.
    fn counter(&self) -> u32;

    /// Get the maximum counter value.
    fn counter_max() -> u32;

    /// Get the timer period.
    fn overflow_increment() -> u64 {
        Self::counter_max() as u64 + 1
    }

    /// Get whether the timer has overflowed.
    fn is_pending_overflow(&self) -> bool;

    /// Clear the flag indicating that the timer has overflowed.
    fn clear_pending_overflow(&self);
}
