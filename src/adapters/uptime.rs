pub trait UptimeTimer<A>: Sync {
    fn start(&self);

    /// Get the current counter value of the timer.
    /// The value must be monotonically increasing in the interval 0 <= counter <= counter_max().
    fn counter(&self) -> u32;

    /// Get the maximum counter value.
    fn counter_max() -> u32;

    /// Get the timer period.
    fn overflow_increment() -> u64 {
        Self::counter_max() as u64 + 1
    }

    fn is_pending_overflow(&self) -> bool;
    fn clear_pending_overflow(&self);
}
