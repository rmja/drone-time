use crate::{Tick, TimeSpan};

pub trait Uptime<T: Tick>: Sync {
    /// Sample the uptime counter, returning the non-wrapping time since the uptime was started.
    fn now(&self) -> TimeSpan<T>;
}
