use crate::{
    drv::stm32::{
        gen::{NewGeneralCh1, NewGeneralCh2, NewGeneralCh3, NewGeneralCh4},
        GeneralTimDrv, TimCh1, TimCh2, TimCh3, TimCh4,
    },
    new_drv,
};
use drone_stm32_map::periph::tim::general::{GeneralTimPeriph, Tim2};

new_drv!(NewGeneralCh1<Tim2>.new_ch1(GeneralTimPeriph) -> GeneralTimDrv<TimCh1>);
new_drv!(NewGeneralCh2<Tim2>.new_ch2(GeneralTimPeriph) -> GeneralTimDrv<TimCh2>);
new_drv!(NewGeneralCh3<Tim2>.new_ch3(GeneralTimPeriph) -> GeneralTimDrv<TimCh3>);
new_drv!(NewGeneralCh4<Tim2>.new_ch4(GeneralTimPeriph) -> GeneralTimDrv<TimCh4>);
