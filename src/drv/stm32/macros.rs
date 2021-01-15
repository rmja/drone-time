#[macro_export]
macro_rules! new_drv {
    ($trait_name:ident<$tim:ident>.$fn_name:ident($tim_periph:ident) -> $drv:ident<$tim_ch:ident>) => {
        impl<Int: drone_cortexm::thr::IntToken, T: 'static + crate::Tick> $trait_name<$tim, Int, T>
            for $drv<$tim, Int, $tim_ch, T>
        {
            fn $fn_name(tim: $tim_periph<$tim>, tim_int: Int, tick: T) -> Self {
                Self::new(tim, tim_int, tick)
            }
        }
    };
}
