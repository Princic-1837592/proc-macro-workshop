use std::marker::PhantomData;

trait TotalSizeIsMultipleOfEightBits {}

trait RenameSizeType {
    type CheckType;
}

#[allow(private_bounds)]
pub struct TotalSize<T>(PhantomData<T>)
where
    T: RenameSizeType,
    <T as RenameSizeType>::CheckType: TotalSizeIsMultipleOfEightBits;

macro_rules! impl_total_size_for {
    ($(($n:expr, $name:ident)),*) => {
        $(
            pub enum $name {}

            impl RenameSizeType for [(); $n] {
                type CheckType = $name;
            }
        )*
    }
}

impl_total_size_for!(
    (0, ZeroMod8),
    (1, OneMod8),
    (2, TwoMod8),
    (3, ThreeMod8),
    (4, FourMod8),
    (5, FiveMod8),
    (6, SixMod8),
    (7, SevenMod8)
);

impl TotalSizeIsMultipleOfEightBits for ZeroMod8 {}
