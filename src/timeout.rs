use crate::{Tick, TimeSpan};
use core::marker::PhantomData;

pub struct Timeout<T: Tick>(PhantomData<T>);

impl<T: Tick> Timeout<T> {
    pub const INFINITE: TimeSpan<T> = TimeSpan::from_ticks(-1);

    pub const fn is_infinite(timespan: &TimeSpan<T>) -> bool {
        timespan.0 == -1
    }
}