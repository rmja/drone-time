#[macro_export]
macro_rules! new_drv {
    ($trait_name:ident<$tim:ident>.$fn_name:ident($tim_periph:ident) -> $drv:ident<$tim_ch:ident>) => {
        impl<Int: drone_cortexm::thr::IntToken> $trait_name<$tim, Int> for $drv<$tim, Int, $tim_ch> {
            fn $fn_name(tim: $tim_periph<$tim>, tim_int: Int) -> Self {
                Self::new(tim, tim_int)
            }
        }
    };
}