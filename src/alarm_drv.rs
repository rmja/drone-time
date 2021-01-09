// use core::{future::Future, marker::PhantomData, pin::Pin, task::{Context, Poll}};

// use crate::{AlarmTimer, alarm::*};

// pub struct AlarmDrv<Timer: AlarmTimer<A>, A> {
//     timer: Timer,
//     interval: u64,
//     remaining: u64,
//     compare: u32,
//     adapter: PhantomData<A>,
// }

// struct SharedState {
//     remaining: u64,
//     compare: u32,
// }

// impl<Timer: AlarmTimer<A>, A: Send> AlarmDrv<Timer, A> {
//     fn advance(&mut self, base: u32) {
//         // The maximum delay is half the counters increment.
//         // This ensures that we can hit the actual fire time directly when the last timeout is setup.

//         self.compare = if self.remaining < Timer::overflow_increment() {
//             self.remaining as u32
//         } else {
//             (Timer::overflow_increment() / 2) as u32
//         };

//         let compare = base + self.compare;
//         let handle = self.timer.next(compare);
//         handle.root_wait();
//     }

//     fn timer_fire(&mut self, capture: u32) {
//         let mut invoke_count = 0;
//         let compare = self.compare as u64;

//         if compare < self.remaining {
//             // The alarm fired before the final time.
//             self.remaining -= compare;
//         }
//         else if compare == self.remaining {
//             if self.interval > 0 {
//                 // Multi-shot, periodic timer
//             }
//             else {
//                 // Single-shot timer
//                 invoke_count = 1;
//                 self.remaining = 0;
//             }
//         }

//         if self.remaining > 0 {
//             // Setup next timeout if this was an intermediate timeout,
//             // or if the timer is periodic.

//             self.advance(capture);
//         }

//         for i in 0..invoke_count {
//             // TODO: Invoke callback
//         }
//     }
// }

// impl<Timer: AlarmTimer<A>, A: Send> Alarm for AlarmDrv<Timer, A> {
//     type Stop = Self;

//     fn sleep(&mut self, duration: u64) -> TimerSleep<'_, Self::Stop> {
//         let now = self.timer.counter();

//         self.interval = 0;
//         self.remaining = duration;
//         self.advance(now);

//         todo!();
//         // TimerSleep::new(self, future)
//     }
// }

// impl<'a, T: TimerStop> Future for TimerSleep<'a, T> {
//     type Output = ();

//     #[inline]
//     fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
//         todo!();
//     }
// }

// impl<Timer: AlarmTimer<A>, A: Send> TimerStop for AlarmDrv<Timer, A> {
//     fn stop(&mut self) {
//     }
// }
