use core::{future::Future, pin::Pin};

pub trait MuxAlarm<A>: Send {
    /// MuxAlarm stop handler.
    type Stop: TimerStop;

    /// Returns a future that resolves when `duration` time is elapsed.
    fn sleep(&mut self, duration: u32) -> TimerSleep<'_, Self::Stop>;
}

/// MuxAlarm stop handler.
pub trait TimerStop: Send {
    /// Stops the timer.
    fn stop(&mut self);
}
/// Future created from [`MuxAlarm::sleep`].
pub struct TimerSleep<'a, T: TimerStop> {
    stop: &'a mut T,
    future: Pin<Box<dyn Future<Output = ()> + Send + 'a>>,
}