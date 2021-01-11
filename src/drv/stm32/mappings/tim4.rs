use crate::{drv::stm32::{TimCh1, TimCh2, TimCh3, TimCh4, gen::{NewGeneralCh1, NewGeneralCh2, NewGeneralCh3, NewGeneralCh4}}, new_drv};
use drone_stm32_map::periph::tim::general::{GeneralTimPeriph, Tim4};

new_drv!(NewGeneralCh1<Tim4>.new_ch1(GeneralTimPeriph) -> TimCh1);
new_drv!(NewGeneralCh2<Tim4>.new_ch2(GeneralTimPeriph) -> TimCh2);
new_drv!(NewGeneralCh3<Tim4>.new_ch3(GeneralTimPeriph) -> TimCh3);
new_drv!(NewGeneralCh4<Tim4>.new_ch4(GeneralTimPeriph) -> TimCh4);