use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Item};

#[proc_macro_attribute]
pub fn bitfield(_: TokenStream, input: TokenStream) -> TokenStream {
    expand(parse_macro_input!(input as Item)).into()
}

fn expand(item: Item) -> proc_macro2::TokenStream {
    match item {
        Item::Struct(syn::ItemStruct {
            attrs,
            vis,
            ident,
            generics,
            fields: syn::Fields::Named(fields),
            ..
        }) => {
            let (impl_generics, _ty_generics, where_clause) = generics.split_for_impl();
            let field_types = fields.named.clone().into_iter().map(|f| f.ty);
            let sums = quote!((#(<#field_types as ::bitfield::Specifier>::BITS)+*));
            quote!(
                #(#attrs)*
                #[repr(C)]
                #vis struct #ident #impl_generics #where_clause {
                    data: [u8; #sums / 8 + if #sums % 8 == 0 { 0 } else { 1 }],
                }
            )
        }
        _ => unimplemented!(),
    }
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
