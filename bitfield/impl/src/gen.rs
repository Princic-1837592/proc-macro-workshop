use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::to_u_type;

pub fn generate_specifier() -> TokenStream {
    let bits = 1..=64_usize;
    let names: Vec<_> = bits.clone().map(|i| format_ident!("B{}", i)).collect();
    let u_types: Vec<_> = bits.clone().map(to_u_type).collect();
    let shifts = u_types
        .clone()
        .into_iter()
        .zip(bits.clone())
        .map(|(u_type, bits)| quote!(
            ((::std::mem::size_of::<#u_type>() * 8 - #bits % (::std::mem::size_of::<#u_type>() * 8)) % (::std::mem::size_of::<#u_type>() * 8))
    ));
    quote!(
        #(
            pub enum #names {}

            impl Specifier for #names {
                const BITS: usize = #bits;
                type T = #u_types;

                fn get<const OFFSET: usize, const SIZE: usize>(bytes: &[u8]) -> <Self as Specifier>::T {
                    <<Self as Specifier>::T as PrivateSpecifier>::get::<OFFSET, SIZE>(bytes) >> #shifts
                }

                fn set<const OFFSET: usize, const SIZE: usize>(bytes: &mut [u8], value: <Self as Specifier>::T) {
                    <<Self as Specifier>::T as PrivateSpecifier>::set::<{<Self as Specifier>::BITS}, OFFSET, SIZE>(bytes, value << #shifts);
                }
            }
        )*
    )
}
pub fn generate_private_specifier() -> TokenStream {
    let bits: [usize; 4] = [8, 16, 32, 64];
    let types: Vec<_> = bits
        .clone()
        .iter()
        .map(|i| format_ident!("u{}", i))
        .collect();
    quote!(
        #(
            impl PrivateSpecifier for #types {
                const BITS: usize = #bits;
                type T = Self;

                fn get<const OFFSET: usize, const SIZE: usize>(bytes: &[u8]) -> <Self as PrivateSpecifier>::T {
                    // OFFSET / 8 ==> index of the first byte
                    // OFFSET / 8 + 1 ==> index of the second byte
                    if OFFSET % 8 == 0 {
                        // if aligned, just take the bytes from the first one
                        let len = ::std::mem::size_of::<Self>().min(SIZE - OFFSET / 8);
                        let mut slice = [0; ::std::mem::size_of::<Self>()];
                        slice[..len].copy_from_slice(&bytes[OFFSET / 8..OFFSET / 8 + len]);
                        Self::from_be_bytes(slice)
                    } else {
                        // if not aligned, take the |$u_type| bytes starting from the second one,
                        // shift them to the right and add the first byte on the left
                        let len = ::std::mem::size_of::<Self>().min(SIZE - (OFFSET / 8 + 1));
                        let mut slice = [0; ::std::mem::size_of::<Self>()];
                        slice[..len].copy_from_slice(&bytes[OFFSET / 8 + 1..OFFSET / 8 + 1 + len]);
                        let right = Self::from_be_bytes(slice);
                        ((bytes[OFFSET / 8] as Self)
                            << (::std::mem::size_of::<Self>() * 8 - 8 + OFFSET % 8))
                            | (right >> (8 - OFFSET % 8))
                    }
                }

                fn set<const ACTUAL_BITS: usize, const OFFSET: usize, const SIZE: usize>(
                    bytes: &mut [u8],
                    value: <Self as PrivateSpecifier>::T,
                ) {
                    if OFFSET % 8 == 0 {
                        let len = ::std::mem::size_of::<Self>().min(SIZE - (OFFSET / 8));
                        let old = <Self as PrivateSpecifier>::get::<OFFSET, SIZE>(bytes);
                        let mask = if ACTUAL_BITS == <Self as PrivateSpecifier>::BITS {
                            0
                        } else {
                            Self::MAX >> ACTUAL_BITS
                        };
                        let new = value | (old & mask);
                        bytes[OFFSET / 8..OFFSET / 8 + len].copy_from_slice(&new.to_be_bytes()[..len]);
                    } else {
                        let start_right = OFFSET / 8 + 1;
                        if ACTUAL_BITS > 8 - OFFSET % 8 {
                            let len = ::std::mem::size_of::<Self>().min(SIZE - start_right);
                            let old =
                                Self::from_be_bytes(bytes[start_right..start_right + len].try_into().unwrap());
                            let actual_bits = ACTUAL_BITS - (8 - OFFSET % 8);
                            let mask = if actual_bits == <Self as PrivateSpecifier>::BITS {
                                0
                            } else {
                                Self::MAX >> actual_bits
                            };
                            let new = (value << (8 - OFFSET % 8)) | (old & mask);
                            bytes[start_right..start_right + len].copy_from_slice(&new.to_be_bytes()[..len]);
                        }
                        let left = (value >> (::std::mem::size_of::<Self>() * 8 - 8 + OFFSET % 8)) as u8;
                        let old_left = bytes[OFFSET / 8];
                        let mask = u8::MAX << (8 - OFFSET % 8);
                        bytes[OFFSET / 8] = (old_left & mask) | left
                    }
                }
            }
        )*
    )
}
