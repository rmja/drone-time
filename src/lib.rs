#![feature(never_type)]
#![feature(prelude_import)]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod adapters;
pub mod drv;
mod timespan;
mod uptime;

pub use self::{
    adapters::uptime::{UptimeTick, UptimeTimer},
    timespan::TimeSpan,
    uptime::Uptime,
};

#[prelude_import]
#[allow(unused_imports)]
use drone_core::prelude::*;
