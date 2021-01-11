//! The root task.

use crate::{adapters::*, consts, thr, thr::ThrsInit, Regs};
use drone_core::log;
use drone_cortexm::{periph_sys_tick, reg::prelude::*, swo, thr::prelude::*};
use drone_stm32_map::periph::tim::periph_tim2;
use drone_stm32f4_hal::{rcc::{
    periph_flash, periph_pwr, periph_rcc, traits::*, Flash, Pwr, Rcc, RccSetup,
}, tim::{GeneralTimCfg, config::*, prelude::*}};
use drone_time::{DateTime, TimeSpan, Uptime, UptimeDrv, Watch, drv::stm32::*, drv::systick::SysTickDrv};

/// The root task handler.
#[inline(never)]
pub fn handler(reg: Regs, thr_init: ThrsInit) {
    let thr = thr::init(thr_init);

    thr.hard_fault.add_once(|| panic!("Hard Fault"));

    println!("Hello, world!");

    // Enable interrupts.
    thr.rcc.enable_int();
    thr.tim_2.enable_int();

    // Initialize clocks.
    let rcc = Rcc::init(RccSetup::new(periph_rcc!(reg), thr.rcc));
    let pwr = Pwr::init(periph_pwr!(reg));
    let flash = Flash::init(periph_flash!(reg));

    let hseclk = rcc.stabilize(consts::HSECLK).root_wait();
    let pll = rcc
        .select(consts::PLLSRC_HSECLK, hseclk)
        .stabilize(consts::PLL)
        .root_wait();
    let hclk = rcc.configure(consts::HCLK);
    let pclk1 = rcc.configure(consts::PCLK1);
    let pclk2 = rcc.configure(consts::PCLK2);
    pwr.enable_overdrive();
    flash.set_latency(consts::HCLK.get_wait_states(VoltageRange::HighVoltage));
    swo::flush();
    swo::update_prescaler(consts::HCLK.f() / log::baud_rate!() - 1);
    rcc.select(consts::SYSCLK_PLL, pll.p());

    println!("Hello, world!");

    // let uptime = UptimeDrv::start(
    //     SysTickDrv::new(periph_sys_tick!(reg)),
    //     thr.sys_tick,
    //     SysTickUptimeTick,
    // );

    let setup = GeneralTimSetup::new(periph_tim2!(reg), pclk1, TimFreq::Nominal(consts::TIM2_FREQ));
    let tim2 = GeneralTimCfg::with_enabled_clock(setup);
    let tim2 = GeneralTimDrv::new_ch1(tim2.release(), thr.tim_2);
    let uptime = UptimeDrv::start(
        tim2,
        thr.tim_2,
        Tim2UptimeTick,
    );

    let mut watch = Watch::new(&*uptime);

    watch.set(DateTime::new(2021, 1.into(), 1, 0, 0, 0), uptime.now());

    let mut last = TimeSpan::ZERO;
    let mut last_seconds = u32::MAX;
    loop {
        let now = uptime.now();
        assert!(now >= last);

        let now_seconds = now.total_seconds();
        if now_seconds != last_seconds {
            println!("{:?}: {:?}", now, watch.at(now).unwrap().parts());
        }

        last = now;
        last_seconds = now_seconds;
    }
}
