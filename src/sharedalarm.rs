use core::{cmp::min, marker::PhantomData, ops::Sub, pin::Pin};

use core::future::Future;

use crate::{Alarm, Tick, TimeSpan, Uptime};

pub struct SharedAlarm<A: Alarm<T>, T: Tick> {
    alarm: A,
    running: Option<Pin<Box<dyn Future<Output = ()>>>>,
    subscriptions: Vec<Subscription>,
    tick: PhantomData<T>,
}

pub struct Subscription {
    base: u32,
    remaining: u64,
}

impl Future for Subscription {
    type Output = ();

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        todo!()
    }
}

impl<A: Alarm<T>, T: Tick> SharedAlarm<A, T> {
    async fn sleep(&mut self, duration: TimeSpan<T>) {
        self.sleep_from(self.alarm.counter(), duration).await;
    }

    fn sleep_from(&mut self, base: u32, duration: TimeSpan<T>) -> Subscription {
        let asd = match &self.running {
            None => {
                // let future = self.alarm.sleep_from(base, duration);
                // self.running = Some(future);
            }
            Some(running) => {
                // min(timeout, running.timeout)
                todo!()
            }
        };

        // if self.running.i
        // self.alarm.sleep_from(base, duration)

        Subscription {
            base,
            remaining: duration.0,
        }
    }

    // Returns a future that resolves when `duration` time is elapsed.
    // pub fn timeout(&mut self, timeout: TimeSpan<T>) -> &Subscription {
    //     let now = self.uptime.now();
    //     let sub = Subscription {
    //         base: now.0,
    //         interval: 0,
    //         remaining: timeout.0,
    //     };

    //     self.subscriptions.push(sub);
    //     self.adjust(now);

    //     self.subscriptions.last().unwrap()
    // }

    // fn adjust(&mut self, now: TimeSpan<T>) {
    //     let passed = match self.running_alarm.take() {
    //         Some(mut running) => {
    //             running.stop.stop();
    //             now.0 - running.started
    //         },
    //         None => 0,
    //     };

    //     let mut next = Option::None;

    //     // Decrease the remaining time on all subscriptions.
    //     for sub in self.subscriptions.iter_mut() {
    //         // Subscriptions with remaining == 0 are already scheduled to be fired.
    //         // Only look at timers which are not yet scheduled.
    //         if sub.remaining > 0 {
    //             // Should the alarm be fired within the duration that was just passed?
    //             if sub.remaining <= passed {
    //                 // Set remaining to 0 indicating that the subscriber callback should be fired.

    //                 // TODO: Fire
    //             }
    //             else {
    //                 // Decrement the remaining time for the subscription.
    //                 sub.remaining -= passed;

    //                 next = match next {
    //                     None => Some(sub),
    //                     Some(current) => {
    //                         if current.remaining < sub.remaining {
    //                             Some(current)
    //                         }
    //                         else {
    //                             Some(sub)
    //                         }
    //                     }
    //                 };
    //             }
    //         }
    //     }

    //     if let Some(next) = next {
    //         // Setup the alarm for the earliest task timer, or at the maximum delay
    //     }
    // }
}

#[cfg(test)]
pub mod tests {
    use crate::{adapters::alarm::fakes::FakeAlarmTimer, AlarmDrv, AlarmTimer};

    use super::*;
    use futures_await_test::async_test;

    #[async_test]
    async fn asd() {
        let timer = FakeAlarmTimer {
            counter: 4,
            running: false,
            compares: Vec::new(),
        };
        let mut alarm = AlarmDrv::new(timer);
        let mux = SharedAlarm {
            alarm,
            running: None,
            subscriptions: vec![],
        };
    }
}
