use proc_macro::TokenStream;
use quote::{format_ident, quote};

#[proc_macro_attribute]
pub fn bitfield(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;
    let _ = input;

    unimplemented!()
}

#[proc_macro]
pub fn generate_bs(_: TokenStream) -> TokenStream {
    let range = 1..=64_usize;
    let names = range.clone().map(|i| format_ident!("B{}", i));
    quote!(
        #(
            pub enum #names {}

            impl Specifier for #names {
                const BITS: usize = #range;
            }
        )*
    )
    .into()
}
