#[cfg(feature = "stm32")]
pub mod stm32;

#[cfg(feature = "systick-dint")]
mod systick_dint;
#[cfg(feature = "systick-dint")]
pub mod systick {
    pub use crate::drv::systick_dint::*;
}