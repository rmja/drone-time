mod gen_ch;
#[macro_use]
mod macros;
mod gen;
mod mappings;

pub use self::{
    gen::{GeneralTimDrv, NewGeneralCh1, NewGeneralCh2, NewGeneralCh3, NewGeneralCh4},
    gen_ch::{TimCh1, TimCh2, TimCh3, TimCh4},
};

pub mod prelude {
    pub use super::gen::{NewGeneralCh1, NewGeneralCh2, NewGeneralCh3, NewGeneralCh4};
}
