use crate::{Tick, TimeSpan};

pub trait Uptime<T: Tick>: Sync {
    /// Sample the counter of the underlying timer.
    fn counter(&self) -> u32;

    /// Get the non-wrapping time since the uptime was started.
    fn now(&self) -> TimeSpan<T>;

    /// Get the upstamp value for a specific `counter` value.
    /// `counter` is assumed to be in the past, no longer than a period old.
    fn at(&self, counter: u32) -> TimeSpan<T>;
}
