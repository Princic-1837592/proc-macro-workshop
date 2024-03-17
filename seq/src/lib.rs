use std::iter;

use proc_macro2::{Group, Ident, Literal, TokenStream, TokenTree};
use quote::format_ident;
use syn::{spanned::Spanned, Error};

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
    let mut result = TokenStream::new();
    for val in start..=if inclusive { end } else { end - 1 } {
        result.extend(replace_value(&var, val, body.clone()));
    }
    Ok(result)
}

fn replace_value(var: &Ident, val: u64, body: TokenStream) -> TokenStream {
    let mut tokens = Vec::from_iter(body);
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            TokenTree::Ident(ident) if ident == var => {
                let mut literal = Literal::u64_unsuffixed(val);
                literal.set_span(ident.span());
                tokens[i] = TokenTree::Literal(literal);
                i += 1;
                continue;
            }
            _ => {}
        }
        if i + 2 < tokens.len() {
            match &tokens[i..=i + 2] {
                [TokenTree::Ident(prefix), TokenTree::Punct(punct), TokenTree::Ident(ident)]
                    if ident == var && punct.as_char() == '~' =>
                {
                    let mut ident = format_ident!("{}{}", prefix, val);
                    ident.set_span(prefix.span());
                    tokens.splice(i..i + 3, iter::once(TokenTree::Ident(ident)));
                    i += 3;
                    continue;
                }
                _ => {}
            }
        }
        if let TokenTree::Group(group) = &mut tokens[i] {
            let original_span = group.span();
            let content = replace_value(var, val, group.stream());
            *group = Group::new(group.delimiter(), content);
            group.set_span(original_span);
        }
        i += 1;
    }
    TokenStream::from_iter(tokens)
}
