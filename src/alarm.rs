use core::{future::Future, pin::Pin};

/// An alarm is backed by a timer and provides infinite timeout capabilites.
pub trait Alarm: Send {
    /// Timer stop handler.
    type Stop: TimerStop;

    /// Returns a future that resolves when `duration` time is elapsed.
    fn sleep(&mut self, duration: u64) -> TimerSleep<'_, Self::Stop>;
}

/// Future created from [`Timer::sleep`].
pub struct TimerSleep<'a, T: TimerStop> {
    stop: &'a mut T,
    future: Pin<Box<dyn Future<Output = ()> + Send + 'a>>,
}

impl<'a, T: TimerStop> TimerSleep<'a, T> {
    pub fn new(stop: &'a mut T, future: Pin<Box<dyn Future<Output = ()> + Send + 'a>>) -> Self {
        Self { stop, future }
    }
}

/// Timer stop handler.
pub trait TimerStop: Send {
    /// Stops the timer.
    fn stop(&mut self);
}

impl<'a, T: TimerStop> Drop for TimerSleep<'a, T> {
    #[inline]
    fn drop(&mut self) {
        self.stop.stop();
    }
}
