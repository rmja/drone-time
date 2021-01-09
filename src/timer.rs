use crate::{Tick, TimeSpan, Uptime};

/// Error returned from [`Timer::interval`] on overflow.
#[derive(Debug)]
pub struct TimerOverflow;



pub struct Timer<'a, U: Uptime<T>, T: Tick> {
    uptime: &'a U,
    subscriptions: Vec<Subscription>,
}

struct Subscription {
    base: u64,
    interval: u64,
    remaining: u64,
}

impl<U: Uptime<T>, T: Tick> Timer<'_, U, T> {

    /// Returns a future that resolves when `duration` time is elapsed.
    pub fn timeout(&mut self, timeout: TimeSpan<T>) -> Subscription {
        let now = self.uptime.now();
        let sub = Subscription {
            base: now,
            interval: 0,
            remaining: timeout.0,
        };

        self.subscriptions.push(sub);
        self.adjust(now);

        sub
    }

    fn adjust(&self, now: TimeSpan<T>) {
        
    }
}
