// Crates that have the "proc-macro" crate type are only allowed to export
// procedural macros. So we cannot have one crate that defines procedural macros
// alongside other types of public APIs like traits and structs.
//
// For this project we are going to need a #[bitfield] macro but also a trait
// and some structs. We solve this by defining the trait and structs in this
// crate, defining the attribute macro in a separate bitfield-impl crate, and
// then re-exporting the macro from this crate so that users only have one crate
// that they need to import.
//
// From the perspective of a user of this crate, they get all the necessary APIs
// (macro, trait, struct) through the one bitfield crate.
use bitfield_impl::generate_private_specifier;
pub use bitfield_impl::{bitfield, generate_specifier};

pub trait Specifier {
    const BITS: usize;
    type T;

    fn get<const OFFSET: usize, const SIZE: usize>(bytes: &[u8]) -> <Self as Specifier>::T;

    fn set<const OFFSET: usize, const SIZE: usize>(bytes: &mut [u8], value: <Self as Specifier>::T);
}

trait PrivateSpecifier {
    const BITS: usize;
    type T;

    fn get<const OFFSET: usize, const SIZE: usize>(bytes: &[u8]) -> <Self as PrivateSpecifier>::T;

    fn set<const ACTUAL_BITS: usize, const OFFSET: usize, const SIZE: usize>(
        bytes: &mut [u8],
        value: <Self as PrivateSpecifier>::T,
    );
}

generate_specifier!();

generate_private_specifier!();

/*impl<T> Specifier for T
where
    T: PrivateSpecifier + Sized,
{
    const BITS: usize = <T as PrivateSpecifier>::BITS;
    type T = <T as PrivateSpecifier>::T;

    fn get<const OFFSET: usize, const SIZE: usize>(bytes: &[u8]) -> <Self as Specifier>::T {
        <Self as PrivateSpecifier>::get::<OFFSET, SIZE>(bytes)
    }

    fn set<const OFFSET: usize, const SIZE: usize>(
        bytes: &mut [u8],
        value: <Self as Specifier>::T,
    ) {
        <Self as PrivateSpecifier>::set::<{ <Self as Specifier>::BITS }, OFFSET, SIZE>(
            bytes, value,
        )
    }
}*/
