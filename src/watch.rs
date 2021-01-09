use crate::{DateTime, Tick, TimeSpan, Uptime};

struct Adjust<T: Tick> {
    datetime: DateTime,
    upstamp: TimeSpan<T>,
}

#[derive(Debug)]
pub struct NotSetError;

pub struct Watch<'a, U: Uptime<T>, T: Tick> {
    uptime: &'a U,
    adjust: Option<Adjust<T>>,
}

impl<'a, U: Uptime<T>, T: Tick> Watch<'a, U, T> {
    pub fn new(uptime: &'a U) -> Self {
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
        thr::{PreemptedCell, ThrToken, Thread, ThreadLocal},
        token::Token,
    };

    use crate::Month;

    use super::*;

    struct TestTick;
    struct TestTimer;

    #[derive(Clone, Copy)]
    struct TestToken;
    struct TestThreadLocal;
    struct TestThread {
        fibers: fib::Chain,
        local: TestThreadLocal,
    }
    const TEST_THREAD: TestThread = TestThread {
        fibers: fib::Chain::new(),
        local: TestThreadLocal,
    };

    impl Tick for TestTick {
        fn freq() -> u32 {
            32768
        }
    }

    impl UptimeTimer<TestTick> for TestTimer {
        fn start(&self) {}

        fn counter(&self) -> u32 {
            0
        }

        fn counter_max() -> u32 {
            0xFFFF
        }

        fn is_pending_overflow(&self) -> bool {
            false
        }

        fn clear_pending_overflow(&self) {
            unreachable!();
        }
    }

    unsafe impl Token for TestToken {
        unsafe fn take() -> Self {
            todo!()
        }
    }

    unsafe impl ThrToken for TestToken {
        type Thr = TestThread;

        const THR_IDX: usize = 0;
    }

    impl Thread for TestThread {
        type Local = TestThreadLocal;

        fn first() -> *const Self {
            &TEST_THREAD
        }

        fn fib_chain(&self) -> &fib::Chain {
            &self.fibers
        }

        unsafe fn local(&self) -> &Self::Local {
            &self.local
        }
    }

    impl ThreadLocal for TestThreadLocal {
        fn preempted(&self) -> &PreemptedCell {
            todo!()
        }
    }

    #[test]
    fn set() {
        let timer = TestTimer;
        let thread = TestToken;
        let uptime = Uptime::start(timer, thread, TestTick);
        let mut watch = Watch::new(&uptime);

        let now = watch.now();
        assert!(now.is_err());

        let set_datetime = DateTime::new(2021, Month::January, 8, 10, 39, 27);
        let set_upstamp = TimeSpan::from_seconds(100);
        watch.set(set_datetime, set_upstamp);

        assert_eq!(
            set_datetime - TimeSpan::<TestTick>::from_seconds(1),
            watch
                .at(set_upstamp - TimeSpan::<TestTick>::from_seconds(1))
                .unwrap()
        );
        assert_eq!(set_datetime, watch.at(set_upstamp).unwrap());
        assert_eq!(
            set_datetime + TimeSpan::<TestTick>::from_seconds(1),
            watch
                .at(set_upstamp + TimeSpan::<TestTick>::from_seconds(1))
                .unwrap()
        );
    }
}
