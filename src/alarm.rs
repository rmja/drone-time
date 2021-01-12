use async_trait::async_trait;

/// An alarm is backed by a timer and provides infinite timeout capabilites.
#[async_trait]
pub trait Alarm: Send {
    /// Get the alarm counter.
    fn counter(&self) -> u32;

    /// Returns a future that resolves when `duration` time is elapsed.
    async fn sleep(&mut self, duration: u64) {
        let base = self.counter();
        self.sleep_from(base, duration).await
    }

    /// Returns a future that resolves when `duration` time is elapsed relative to `base`,
    /// where base is sampled from the underlying timer counter.
    async fn sleep_from(&mut self, base: u32, duration: u64);
}
