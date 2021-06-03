use alloc::sync::Arc;

use crate::{DateTime, Tick, TimeSpan, Uptime};

struct Adjust<T: Tick> {
    datetime: DateTime,
    upstamp: TimeSpan<T>,
}

#[derive(Debug)]
pub struct NotSetError;

pub struct Watch<U: Uptime<T>, T: Tick> {
    uptime: Arc<U>,
    adjust: Option<Adjust<T>>,
}

impl<U: Uptime<T>, T: Tick> Watch<U, T> {
    pub fn new(uptime: Arc<U>) -> Self {
        Self {
            uptime,
            adjust: None,
        }
    }

    pub fn set(&mut self, datetime: DateTime, upstamp: TimeSpan<T>) {
        self.adjust = Some(Adjust { datetime, upstamp });
    }

    pub fn now(&self) -> Result<DateTime, NotSetError> {
        self.at(self.uptime.now())
    }

    pub fn at(&self, upstamp: TimeSpan<T>) -> Result<DateTime, NotSetError> {
        if let Some(adjust) = &self.adjust {
            if upstamp > adjust.upstamp {
                // upstamp was sampled after the time was last adjusted.
                Ok(adjust.datetime + (upstamp - adjust.upstamp))
            } else {
                // upstamp was sampled before the time was last adjusted.
                Ok(adjust.datetime - (adjust.upstamp - upstamp))
            }
        } else {
            Err(NotSetError)
        }
    }
}

#[cfg(test)]
pub mod tests {
    use drone_core::{
        fib,
        thr,
        token::Token,
    };

    use crate::{Month, UptimeCounter, UptimeDrv, UptimeOverflow};

    use super::*;

    struct Adapter;

    struct TestAlarm;

    struct TestTick;
    impl Tick for TestTick {
        const FREQ: u32 = 32768;
    }

    impl UptimeCounter<TestTick, Adapter> for TestAlarm {
        fn value(&self) -> u32 {
            0
        }
    }

    impl UptimeOverflow<Adapter> for TestAlarm {
        const MAX: u32 = 0xFFFF;

        fn overflow_int_enable(&self) {}

        fn is_pending_overflow(&self) -> bool {
            false
        }

        fn clear_pending_overflow(&self) {
            unreachable!();
        }
    }

    thr::pool! {
        thread => Thr {};
        local => ThrLocal {};
        index => Thrs;
        threads => { thr0 };
    }

    #[test]
    fn set() {
        let counter = TestAlarm;
        let overflow = TestAlarm;
        let thread = unsafe { Thr0::take() };
        let uptime = UptimeDrv::new(counter, overflow, thread, TestTick);
        let mut watch = Watch::new(uptime);

        let now = watch.now();
        assert!(now.is_err());

        let set_datetime = DateTime::new(2021, Month::January, 8, 10, 39, 27);
        let set_upstamp = TimeSpan::from_secs(100);
        watch.set(set_datetime, set_upstamp);

        assert_eq!(
            set_datetime - TimeSpan::<TestTick>::from_secs(1),
            watch
                .at(set_upstamp - TimeSpan::<TestTick>::from_secs(1))
                .unwrap()
        );
        assert_eq!(set_datetime, watch.at(set_upstamp).unwrap());
        assert_eq!(
            set_datetime + TimeSpan::<TestTick>::from_secs(1),
            watch
                .at(set_upstamp + TimeSpan::<TestTick>::from_secs(1))
                .unwrap()
        );
    }
}
