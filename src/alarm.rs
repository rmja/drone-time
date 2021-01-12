use async_trait::async_trait;

use crate::{Tick, TimeSpan};

/// An alarm is backed by a timer and provides infinite timeout capabilites.
#[async_trait]
pub trait Alarm<T: Tick>: Send {
    const MAX: u32;
    const PERIOD: u64 = Self::MAX as u64 + 1;

    /// Get the alarm counter.
    fn counter(&self) -> u32;

    /// Returns a future that resolves when `duration` time is elapsed.
    async fn sleep(&mut self, duration: TimeSpan<T>)
    where
        T: 'async_trait,
    {
        let base = self.counter();
        self.sleep_from(base, duration).await
    }

    /// Returns a future that resolves when `duration` time is elapsed relative to `base`,
    /// where base is sampled from the underlying timer counter.
    async fn sleep_from(&mut self, base: u32, duration: TimeSpan<T>)
    where
        T: 'async_trait;
}
