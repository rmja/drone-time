use async_trait::async_trait;

/// An alarm is backed by a timer and provides infinite timeout capabilites.
#[async_trait]
pub trait Alarm: Send {
    /// Returns a future that resolves when `duration` time is elapsed.
    async fn sleep(&mut self, duration: u64);
}
