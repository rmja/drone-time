#![feature(asm)]
#![feature(const_fn)]
#![feature(const_panic)]
#![feature(never_type)]
#![feature(prelude_import)]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod adapters;
mod alarm;
mod datetime;
pub mod drivers;
mod timeout;
mod timespan;
mod uptime;
mod uptime_drv;
mod watch;

pub use self::{
    adapters::alarm::{AlarmCounter, AlarmTimer, AlarmTimerMode},
    adapters::tick::Tick,
    adapters::uptime::{UptimeCounter, UptimeOverflow},
    alarm::AlarmDrv,
    prelude::*,
    timeout::Timeout,
    uptime_drv::UptimeDrv,
    watch::Watch,
};

pub mod prelude {
    pub use super::{
        alarm::Alarm,
        datetime::{DateTime, Month},
        timespan::TimeSpan,
        uptime::Uptime,
    };
}

#[prelude_import]
#[allow(unused_imports)]
use drone_core::prelude::*;
