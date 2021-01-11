#[cfg(feature = "systick")]
pub mod systick;

#[cfg(feature = "stm32")]
pub mod stm32;

#[cfg(feature = "systick-experimental")]
mod systick_experimental;
#[cfg(feature = "systick-experimental")]
pub mod systick {
    pub use crate::drv::systick_experimental::*;
}
