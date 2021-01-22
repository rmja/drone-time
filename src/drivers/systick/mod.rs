mod alarm;
mod diverged;
mod uptime;

pub struct Adapter;

pub use self::{alarm::SysTickAlarmDrv, uptime::SysTickUptimeDrv};