//! The root task.

use crate::{adapters::*, consts, thr, thr::ThrsInit, Regs};
use drone_core::log;
use drone_cortexm::{periph_sys_tick, reg::prelude::*, swo, thr::prelude::*};
use drone_stm32_map::periph::tim::periph_tim2;
use drone_stm32f4_hal::{
    rcc::{periph_flash, periph_pwr, periph_rcc, traits::*, Flash, Pwr, Rcc, RccSetup},
    tim::{prelude::*, GeneralTimCfg, GeneralTimSetup},
};
use drone_time::{
    drv::stm32f4::*, drv::systick::SysTickDrv, Alarm, DateTime, TimeSpan, Uptime, UptimeDrv, Watch,
};
use futures::prelude::*;

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

    let setup = GeneralTimSetup::new(
        periph_tim2!(reg),
        thr.tim_2,
        pclk1,
        TimFreq::Nominal(consts::TIM2_FREQ),
    );
    let tim2 = GeneralTimCfg::with_enabled_clock(setup).ch1(|ch| ch.into_output_compare());

    tim2.start();

    let uptime = UptimeDrv::new(tim2.cnt.clone(), tim2.ovf, thr.tim_2, Tim2Tick);
    let mut alarm = Alarm::new(tim2.cnt, tim2.ch1, Tim2Tick);
    let mut watch = Watch::new(&*uptime);
    watch.set(DateTime::new(2021, 1.into(), 1, 0, 0, 0), uptime.now());

    // let f1 = alarm.sleep(TimeSpan::from_secs(6)).then(|_| {
    //     println!("{:?}", uptime.now());
    //     println!("6 seconds passed");
    //     future::ready(())
    // });
    // let f2 = alarm.sleep(TimeSpan::from_secs(4)).then(|_| {
    //     println!("{:?}", uptime.now());
    //     println!("4 seconds passed");
    //     future::ready(())
    // });
    // let f3 = alarm.sleep(TimeSpan::from_secs(8)).then(|_| {
    //     println!("{:?}", uptime.now());
    //     println!("8 seconds passed");
    //     future::ready(())
    // });

    // future::join3(f1, f2, f3).root_wait();

    let mut last = TimeSpan::ZERO;
    let mut last_seconds = i32::MAX;
    loop {
        let now = uptime.now();
        assert!(now >= last);

        let now_seconds = now.as_secs();
        if now_seconds != last_seconds {
            println!("{:?}: {:?}", now, watch.at(now).unwrap().parts());
        }

        last = now;
        last_seconds = now_seconds;
    }
}
