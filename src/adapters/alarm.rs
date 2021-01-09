use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

pub trait AlarmTimer<A>: Send {
    /// AlarmTimer stop handler.
    type Stop: AlarmTimerStop;

    /// Get the current counter value of the timer.
    fn counter(&self) -> u32;

    /// Get the maximum counter value.
    fn counter_max() -> u32;

    /// Get the timer period.
    fn overflow_increment() -> u64 {
        Self::counter_max() as u64 + 1
    }

    /// Returns a future that resolves when the timer counter is equal to `compare`.
    /// Note that compare is not a duration but an absolute timestamp.
    fn next(&mut self, compare: u32) -> AlarmTimerNext<'_, Self::Stop>;
}

/// AlarmTimer stop handler.
pub trait AlarmTimerStop: Send {
    /// Stop the timer.
    fn stop(&mut self);
}

pub struct AlarmTimerNext<'a, T: AlarmTimerStop> {
    stop: &'a mut T,
    future: Pin<Box<dyn Future<Output = ()> + Send + 'a>>,
}

impl<'a, T: AlarmTimerStop> AlarmTimerNext<'a, T> {
    /// Creates a new [`AlarmTimerNext`].
    pub fn new(stop: &'a mut T, future: Pin<Box<dyn Future<Output = ()> + Send + 'a>>) -> Self {
        Self { stop, future }
    }
}

impl<'a, T: AlarmTimerStop> Future for AlarmTimerNext<'a, T> {
    type Output = ();

    #[inline]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        self.future.as_mut().poll(cx)
    }
}

impl<'a, T: AlarmTimerStop> Drop for AlarmTimerNext<'a, T> {
    #[inline]
    fn drop(&mut self) {
        self.stop.stop();
    }
}
