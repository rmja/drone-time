use core::{marker::PhantomData, ops::Sub};

use crate::{Tick, TimeSpan, Uptime, adapters::muxtimer::{Timer, TimerStop}};

/// Error returned from [`Timer::interval`] on overflow.
#[derive(Debug)]
pub struct TimerOverflow;

pub struct MuxTimer<'a, U: Uptime<T>, T: Tick, BackingTimer: Timer<A>, A> {
    uptime: &'a U,
    subscriptions: Vec<Subscription>,
    tick: PhantomData<T>,
    running_timer: Option<RunningTimer<BackingTimer::Stop>>,
}

struct RunningTimer<Stop: TimerStop> {
    started: u64,
    stop: Stop,
}

pub struct Subscription {
    base: u64,
    interval: u64,
    remaining: u64,
}

impl<U: Uptime<T>, T: Tick, BackingTimer: Timer<A>, A> MuxTimer<'_, U, T, BackingTimer, A> {
    /// Returns a future that resolves when `duration` time is elapsed.
    pub fn timeout(&mut self, timeout: TimeSpan<T>) -> &Subscription {
        let now = self.uptime.now();
        let sub = Subscription {
            base: now.0,
            interval: 0,
            remaining: timeout.0,
        };

        self.subscriptions.push(sub);
        self.adjust(now);

        self.subscriptions.last().unwrap()
    }

    fn adjust(&mut self, now: TimeSpan<T>) {
        let passed = match self.running_timer.take() {
            Some(mut running) => {
                running.stop.stop();
                now.0 - running.started
            },
            None => 0,
        };

        let mut next = Option::None;

        // Decrease the remaining time on all subscriptions.
        for sub in self.subscriptions.iter_mut() {
            // Subscriptions with remaining == 0 are already scheduled to be fired.
            // Only look at timers which are not yet scheduled.
            if sub.remaining > 0 {
                // Should the alarm be fired within the duration that was just passed?
                if sub.remaining <= passed {
                    // Set remaining to 0 indicating that the subscriber callback should be fired.

                    // TODO: Fire
                }
                else {
                    // Decrement the remaining time for the subscription.
                    sub.remaining -= passed;

                    next = match next {
                        None => Some(sub),
                        Some(current) => {
                            if current.remaining < sub.remaining {
                                Some(current)
                            }
                            else {
                                Some(sub)
                            }
                        }
                    };
                }
            }
        }

        if let Some(next) = next {
            // Setup the alarm for the earliest task timer, or at the maximum delay
        }
    }
}
