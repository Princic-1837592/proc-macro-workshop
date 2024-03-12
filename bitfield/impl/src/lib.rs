use proc_macro::TokenStream as TokenStream1;
use proc_macro2::Ident;
use quote::format_ident;
use syn::{parse_macro_input, spanned::Spanned, Item, ItemEnum};

mod bitfield;
mod derive;
mod gen;

#[proc_macro_attribute]
pub fn bitfield(_: TokenStream1, input: TokenStream1) -> TokenStream1 {
    bitfield::bitfield(parse_macro_input!(input as Item)).into()
}

#[proc_macro]
pub fn generate_specifier(_: TokenStream1) -> TokenStream1 {
    gen::generate_specifier().into()
}

#[proc_macro]
pub fn generate_private_specifier(_: TokenStream1) -> TokenStream1 {
    gen::generate_private_specifier().into()
}

#[proc_macro_derive(BitfieldSpecifier)]
pub fn derive(input: TokenStream1) -> TokenStream1 {
    derive::derive(parse_macro_input!(input as ItemEnum))
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

fn to_u_type(bits: usize) -> Ident {
    let bits: u8 = match bits {
        0..=8 => 8,
        9..=16 => 16,
        17..=32 => 32,
        33..=64 => 64,
        _ => unimplemented!(),
    };
    format_ident!("u{}", bits)
}
