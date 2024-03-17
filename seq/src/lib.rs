use proc_macro2::TokenStream;
use quote::quote;
use syn::Error;

use crate::parser::{
    maybe_next_punct, next_braces, next_end, next_ident, next_keyword, next_punct, next_value,
};

mod parser;

#[proc_macro]
pub fn seq(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    internal(input.into())
        .unwrap_or_else(|error| error.into_compile_error())
        .into()
}

fn internal(input: TokenStream) -> syn::Result<TokenStream> {
    let mut input_iter = input.into_iter();
    let var = next_ident(&mut input_iter)?;
    next_keyword(&mut input_iter, "in")?;
    let (start, start_span) = next_value(&mut input_iter)?;
    next_punct(&mut input_iter, '.')?;
    next_punct(&mut input_iter, '.')?;
    let inclusive = maybe_next_punct(&mut input_iter, '=')?;
    let (end, end_span) = next_value(&mut input_iter)?;
    let body = next_braces(&mut input_iter)?;
    next_end(&mut input_iter)?;
    if (inclusive && start > end) || (!inclusive && start >= end) {
        return Err(Error::new(
            start_span.join(end_span).unwrap(),
            "Empty range",
        ));
    }
    Ok(quote!())
}
