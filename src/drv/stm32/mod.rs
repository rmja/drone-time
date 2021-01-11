mod ch;
mod mappings;
#[macro_use]
mod gen;

pub use self::{
    ch::{TimCh1, TimCh2, TimCh3, TimCh4},
    gen::{
        GeneralTimDrv,
        NewGeneralCh1,
        NewGeneralCh2,
        NewGeneralCh3,
        NewGeneralCh4,
    },
};

pub mod prelude {
    pub use super::gen::{
        NewGeneralCh1,
        NewGeneralCh2,
        NewGeneralCh3,
        NewGeneralCh4,
    };
}