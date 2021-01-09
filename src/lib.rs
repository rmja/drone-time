#![feature(asm)]
#![feature(never_type)]
#![feature(prelude_import)]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod adapters;
mod datetime;
pub mod drv;
mod muxtimer;
mod timespan;
mod uptime;
mod watch;

pub use self::{
    adapters::tick::Tick,
    adapters::uptime::UptimeTimer,
    datetime::{DateTime, Month},
    timespan::TimeSpan,
    uptime::{Uptime, UptimeDrv},
    watch::Watch,
};

#[prelude_import]
#[allow(unused_imports)]
use drone_core::prelude::*;
