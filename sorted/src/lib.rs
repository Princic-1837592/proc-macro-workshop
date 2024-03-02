use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned, Error, Item};

#[proc_macro_attribute]
pub fn sorted(
    _args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item = parse_macro_input!(input as Item);
    match delegate_sorted(item) {
        Ok(result) => quote!(#result),
        Err(error) => error.to_compile_error(),
    }
    .into()
}

fn delegate_sorted(input: Item) -> syn::Result<TokenStream> {
    if let Item::Enum(item_enum) = input {
        let names: Vec<_> = item_enum.variants.iter().map(|v| &v.ident).collect();
        let mut sorted = names.clone();
        sorted.sort_by_key(|v| v.to_string());
        for (original, sorted) in names.iter().zip(&sorted) {
            if original != sorted {
                return Err(Error::new(
                    sorted.span(),
                    format!("{} should sort before {}", sorted, original),
                ));
            }
        }
        Ok(quote!(#item_enum))
    } else {
        Err(Error::new(
            Span::call_site(),
            "expected enum or match expression",
        ))
    }
}
