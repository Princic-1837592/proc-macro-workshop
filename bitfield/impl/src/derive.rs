use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::{spanned::Spanned, Error, Fields, ItemEnum};

use crate::to_u_type;

pub fn derive(
    ItemEnum {
        ident,
        generics,
        variants,
        ..
    }: ItemEnum,
) -> syn::Result<TokenStream> {
    if let Some(invalid) = variants.iter().find(|v| !matches!(v.fields, Fields::Unit)) {
        return Err(Error::new(
            invalid.fields.span(),
            "Variants with fields are not supported",
        ));
    }
    if variants.len().count_ones() != 1 || variants.len() == 1 {
        return Err(Error::new(
            Span::call_site(),
            "The number of variants must be a power of 2 and greater than 1",
        ));
    }
    let bits = variants.len().ilog2() as usize;
    let b_type = format_ident!("B{}", bits);
    let u_type = to_u_type(bits);
    let variants: Vec<_> = variants.into_iter().map(|v| v.ident).collect();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let discriminant_checks = discriminant_checks(&ident, &variants);
    Ok(quote!(

        #discriminant_checks

        impl #impl_generics ::bitfield::Specifier for #ident #ty_generics #where_clause {
            const BITS: usize = #bits;
            type T = Self;

            fn get<const OFFSET: usize, const SIZE: usize>(bytes: &[u8]) -> <Self as Specifier>::T {
                fn from_integer(num: #u_type) -> #ident {
                    use #ident::*;
                    [#((#variants as #u_type, #variants)),*].into_iter().find_map(|(u, e)| if u == num { Some(e) } else { None }).unwrap()
                }

                from_integer(<::bitfield::#b_type as ::bitfield::Specifier>::get::<OFFSET, SIZE>(bytes))
            }

            fn set<const OFFSET: usize, const SIZE: usize>(bytes: &mut [u8], value: <Self as Specifier>::T) {
                <::bitfield::#b_type as ::bitfield::Specifier>::set::<OFFSET, SIZE>(bytes, value as #u_type);
            }
        }
    ))
}

fn discriminant_checks(ident: &Ident, variants: &[Ident]) -> TokenStream {
    let n_variants = variants.len();
    let checks = variants.iter().map(|v| {
        quote_spanned!(
            v.span() => let _: ::bitfield::checks::Discriminant::<[(); ((#v as usize) < #n_variants) as usize]>;
        )
    });
    quote!(
        const _: () = {
            use #ident::*;
            #(#checks;)*
        };
    )
}
