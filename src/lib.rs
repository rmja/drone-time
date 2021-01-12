#![feature(asm)]
#![feature(never_type)]
#![feature(prelude_import)]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod adapters;
mod alarm;
mod alarm_drv;
mod datetime;
pub mod drv;
mod muxtimer;
mod timespan;
mod uptime;
mod watch;

pub use self::{
    adapters::alarm::{AlarmTimer, AlarmTimerNext, AlarmTimerStop},
    adapters::tick::Tick,
    adapters::uptime::UptimeAlarm,
    alarm::Alarm,
    alarm_drv::AlarmDrv,
    datetime::{DateTime, Month},
    timespan::TimeSpan,
    uptime::{Uptime, UptimeDrv},
    watch::Watch,
};

#[prelude_import]
#[allow(unused_imports)]
use drone_core::prelude::*;
