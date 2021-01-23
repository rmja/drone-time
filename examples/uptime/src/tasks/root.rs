//! The root task.

use crate::{consts, thr, thr::ThrsInit, Regs};
use drone_core::log;
use drone_cortexm::{periph_sys_tick, swo, thr::prelude::*};
use drone_stm32_map::periph::{
    tim::periph_tim2,
    gpio::{periph_gpio_d_head, periph_gpio_d13},
};
use drone_stm32f4_hal::{
    rcc::{periph_flash, periph_pwr, periph_rcc, traits::*, Flash, Pwr, Rcc, RccSetup},
    tim::{prelude::*, GeneralTimCfg, GeneralTimSetup},
    gpio::{GpioHead, prelude::*}
};
use drone_time::{Alarm, AlarmDrv, DateTime, TimeSpan, Uptime, UptimeDrv, Watch, drivers::SysTickUptimeDrv};
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

    let gpio_a = GpioHead::with_enabled_clock(periph_gpio_d_head!(reg));
    let dbg_pin = gpio_a.pin(periph_gpio_d13!(reg))
        .into_output()
        .into_pushpull();

    // let systick = SysTickUptimeDrv::new(periph_sys_tick!(reg));
    // let uptime = UptimeDrv::new(
    //     systick.counter,
    //     systick.overflow,
    //     thr.sys_tick,
    //     consts::SysTickTick,
    // );

    let setup = GeneralTimSetup::new(
        periph_tim2!(reg),
        thr.tim_2,
        pclk1,
        TimFreq::Nominal(consts::TIM2_FREQ),
    );
    let tim2 = GeneralTimCfg::with_enabled_clock(setup).ch1(|ch| ch.into_output_compare());

    tim2.start();

    let uptime = UptimeDrv::new(tim2.counter.clone(), tim2.overflow, thr.tim_2, consts::Tim2Tick);
    let alarm = AlarmDrv::new(tim2.counter, tim2.ch1, consts::Tim2Tick);

    assert_eq!(180_000_000, consts::SYSCLK.f());
    // 1000000 cycles should take 5555us@180MHz, measured in release build to 5.555ms-5.557ms.
    for _i in 0..10 {
        dbg_pin.set();
        alarm.burn_cycles(1000000);
        dbg_pin.clear();
        alarm.burn_cycles(1000000);
        dbg_pin.set();
        alarm.burn_cycles(1000000);
        dbg_pin.clear();
        alarm.burn_cycles(1000000);
        dbg_pin.set();
        alarm.burn_cycles(1000000);
        dbg_pin.clear();
        alarm.burn_cycles(1000000);
    }

    let mut watch = Watch::new(uptime.clone());
    watch.set(DateTime::new(2021, 1.into(), 1, 0, 0, 0), uptime.now());

    let f1 = alarm.sleep(TimeSpan::from_secs(6)).then(|_| {
        println!("{:?}", uptime.now());
        println!("6 seconds passed");
        future::ready(())
    });
    let f2 = alarm.sleep(TimeSpan::from_secs(4)).then(|_| {
        println!("{:?}", uptime.now());
        println!("4 seconds passed");
        future::ready(())
    });
    let f3 = alarm.sleep(TimeSpan::from_secs(8)).then(|_| {
        println!("{:?}", uptime.now());
        println!("8 seconds passed");
        future::ready(())
    });

    future::join3(f1, f2, f3).root_wait();

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
