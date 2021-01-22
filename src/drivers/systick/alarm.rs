use crate::{AlarmCounter, AlarmTimer, AlarmTimerMode, Tick, drivers::cortexm::dwt_burn_cycles};
use async_trait::async_trait;
use drone_core::{fib, thr::prelude::*};
use drone_cortexm::{map::{periph::sys_tick::SysTickPeriph}, reg::prelude::*};

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

impl<T: Tick> AlarmCounter<T, Adapter> for SysTickCounterDrv {
    fn value(&self) -> u32 {
        0 // The counter is not running
    }

    fn burn_cycles(&self, cycles: u32) {
        dwt_burn_cycles(cycles);
    }
}

#[async_trait]
impl<Int: ThrToken, T: Tick + 'static> AlarmTimer<T, Adapter> for SysTickTimerDrv<Int> {
    const MAX: u32 = 0xFFFFFF;
    const MODE: AlarmTimerMode = AlarmTimerMode::OneShotOnly;

    async fn delay(&mut self, duration: u32) {
        let stk_ctrl = self.0.stk_ctrl;
        let timeout_future = Box::pin(self.1.add_future(fib::new_fn(move || {
            if stk_ctrl.countflag.read_bit() {
                stk_ctrl.modify(|r| r.clear_tickint().clear_enable());

                fib::Complete(())
            } else {
                fib::Yielded(())
            }
        })));

        self.0.stk_load.store(|r| r.write_reload(duration));
        self.0.stk_ctrl.modify(|r| r.set_tickint().set_enable());

        timeout_future.await;
    }
}
