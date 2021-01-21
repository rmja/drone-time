use crate::Tick;

/// The timer backing Uptime.
/// The timer must be monotonically increasing in the interval 0 <= counter() <= MAX.
pub trait UptimeTimer<T: Tick, A>
where
    Self: Sync,
{
    /// The maximum counter value.
    const MAX: u32;

    /// The timer period.
    const PERIOD: u64 = Self::MAX as u64 + 1;

    /// Get the current counter value of the timer.
    fn counter(&self) -> u32;

    /// Get whether the timer has overflowed.
    fn is_pending_overflow(&self) -> bool;

    /// Clear the flag indicating that the timer has overflowed.
    fn clear_pending_overflow(&self);
}
