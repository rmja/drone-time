use core::{pin::Pin, task::{Context, Poll}};

use crate::{AlarmCounter, AlarmTimer, AlarmTimerMode, Tick};
use async_trait::async_trait;
use drone_core::{fib, thr::prelude::*};
use drone_cortexm::{map::{periph::sys_tick::SysTickPeriph, reg::stk}, reg::prelude::*, processor::spin};
use futures::Future;

use super::{diverged::SysTickDiverged, Adapter};

/// A cortex SysTick alarm driver.
pub struct SysTickAlarmDrv<Int: ThrToken> {
    /// The timer counter.
    pub counter: SysTickCounterDrv,
    /// The timer control.
    pub timer: SysTickTimerDrv<Int>,
}

pub struct SysTickCounterDrv;
pub struct SysTickTimerDrv<Int: ThrToken>(SysTickDiverged, Int);

impl<Int: ThrToken> SysTickAlarmDrv<Int> {
    pub fn new(systick: SysTickPeriph, systick_int: Int) -> Self {
        Self {
            counter: SysTickCounterDrv,
            timer: SysTickTimerDrv(SysTickDiverged::new(systick), systick_int),
        }
    }
}

impl<Int: ThrToken> SysTickTimerDrv<Int> {
    fn delay_impl(&mut self, duration: u32) -> DelayFuture {
        let stk_ctrl = self.0.stk_ctrl;
        let future = Box::pin(self.1.add_future(fib::new_fn(move || {
            if stk_ctrl.countflag.read_bit() {
                stk_ctrl.modify(|r| r.clear_tickint().clear_enable());

                fib::Complete(())
            } else {
                fib::Yielded(())
            }
        })));

        self.0.stk_load.store(|r| r.write_reload(duration));
        self.0.stk_ctrl.modify(|r| r.set_tickint().set_enable());

        DelayFuture {
            stk_ctrl,
            future,
        }
    }
}

struct DelayFuture<'a> {
    stk_ctrl: stk::Ctrl<Crt>,
    future: Pin<Box<dyn Future<Output = ()> + Send + 'a>>,
}

impl Future for DelayFuture<'_> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        self.future.as_mut().poll(cx)
    }
}

impl Drop for DelayFuture<'_> {
    fn drop(&mut self) {
        self.stk_ctrl.modify(|r| r.clear_tickint().clear_enable());
    }
}

impl<T: Tick> AlarmCounter<T, Adapter> for SysTickCounterDrv {
    fn value(&self) -> u32 {
        0 // The counter is not running
    }

    #[inline]
    fn spin(&self, cycles: u32) {
        spin(cycles);
    }
}

#[async_trait]
impl<Int: ThrToken, T: Tick> AlarmTimer<T, Adapter> for SysTickTimerDrv<Int> {
    const MAX: u32 = 0xFFFFFF;
    const MODE: AlarmTimerMode = AlarmTimerMode::OneShotOnly;

    async fn delay(&mut self, duration: u32) {
        self.delay_impl(duration).await;
    }
}
