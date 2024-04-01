use std::marker::PhantomData;

trait TotalSizeIsMultipleOfEightBits {}

trait TotalSizeRenameType {
    type CheckType;
}

#[allow(private_bounds)]
pub struct TotalSize<T>(PhantomData<T>)
where
    T: TotalSizeRenameType,
    <T as TotalSizeRenameType>::CheckType: TotalSizeIsMultipleOfEightBits;

macro_rules! impl_total_size_for {
    ($(($n:expr, $name:ident $(, $vis:tt)?)),*) => {
        $(
            $($vis)? struct $name;

            impl TotalSizeRenameType for [(); $n] {
                type CheckType = $name;
            }
        )*
    };
}

impl_total_size_for!(
    (0, ZeroMod8, pub),
    (1, OneMod8),
    (2, TwoMod8),
    (3, ThreeMod8),
    (4, FourMod8),
    (5, FiveMod8),
    (6, SixMod8),
    (7, SevenMod8)
);

impl TotalSizeIsMultipleOfEightBits for ZeroMod8 {}

trait DiscriminantInRange {}

trait DiscriminantInRangeRenameType {
    type CheckType;
}

#[allow(private_bounds)]
pub struct Discriminant<T>(PhantomData<T>)
where
    T: DiscriminantInRangeRenameType,
    <T as DiscriminantInRangeRenameType>::CheckType: DiscriminantInRange;

struct False;

impl DiscriminantInRangeRenameType for [(); false as usize] {
    type CheckType = False;
}

pub struct True;

impl DiscriminantInRangeRenameType for [(); true as usize] {
    type CheckType = True;
}

impl DiscriminantInRange for True {}
