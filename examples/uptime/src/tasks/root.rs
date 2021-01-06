//! The root task.

use crate::{adapters::UptimeClock, consts, thr, thr::ThrsInit, Regs};
use drone_core::{fib, log};
use drone_cortexm::{periph_sys_tick, reg::prelude::*, swo, thr::prelude::*};
use drone_stm32f4_hal::rcc::{
    periph_flash, periph_pwr, periph_rcc, traits::*, Flash, Pwr, Rcc, RccSetup,
};
use drone_time::{SysTickDrv, TimeSpan, Uptime};

/// The root task handler.
#[inline(never)]
pub fn handler(reg: Regs, thr_init: ThrsInit) {
    let thr = thr::init(thr_init);

    thr.hard_fault.add_once(|| panic!("Hard Fault"));

    println!("Hello, world!");

    // Enable interrupts.
    thr.rcc.enable_int();

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

    let uptime = Uptime::start(
        SysTickDrv::init(periph_sys_tick!(reg)),
        thr.sys_tick,
        UptimeClock,
    );

    let mut last = TimeSpan::ZERO;
    let mut last_seconds = u32::MAX;
    loop {
        let now = uptime.now();
        assert!(now >= last);

        let now_seconds = now.total_seconds();
        if now_seconds != last_seconds {
            println!("{} ({}): {:?}", now_seconds, now.total_milliseconds(), now);
        }

        last = now;
        last_seconds = now_seconds;
    }

    // Enter a sleep state on ISR exit.
    reg.scb_scr.sleeponexit.set_bit();
}
