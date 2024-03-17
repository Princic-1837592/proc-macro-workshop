use std::{iter, ops::RangeInclusive};

use proc_macro2::{Delimiter, Group, Ident, Literal, TokenStream, TokenTree};
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
    let range = start..=if inclusive { end } else { end - 1 };
    let mut found_group = false;
    let expanded = expand_groups(&var, &range, body.clone(), &mut found_group);
    if found_group {
        Ok(expanded)
    } else {
        Ok(TokenStream::from_iter(repeat(&var, range, body)))
    }
}

fn repeat(var: &Ident, range: RangeInclusive<u64>, body: TokenStream) -> Vec<TokenTree> {
    let mut result = Vec::new();
    for val in range {
        result.extend(replace_value(var, val, body.clone()));
    }
    result
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
                    tokens.splice(i..=i + 2, iter::once(TokenTree::Ident(ident)));
                    i += 1;
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

fn expand_groups(
    var: &Ident,
    range: &RangeInclusive<u64>,
    body: TokenStream,
    found: &mut bool,
) -> TokenStream {
    let mut tokens = Vec::from_iter(body);
    let mut i = 0;
    while i < tokens.len() {
        if i + 2 < tokens.len() {
            match &tokens[i..=i + 2] {
                [TokenTree::Punct(left), TokenTree::Group(group), TokenTree::Punct(right)]
                    if left.as_char() == '#'
                        && group.delimiter() == Delimiter::Parenthesis
                        && right.as_char() == '*' =>
                {
                    *found = true;
                    let repeated = repeat(var, range.clone(), group.stream());
                    let len = repeated.len();
                    tokens.splice(i..=i + 2, repeated);
                    i += len;
                    continue;
                }
                _ => {}
            }
        }
        if let TokenTree::Group(group) = &mut tokens[i] {
            let original_span = group.span();
            let content = expand_groups(var, range, group.stream(), found);
            *group = Group::new(group.delimiter(), content);
            group.set_span(original_span);
        }
        i += 1;
    }
    TokenStream::from_iter(tokens)
}
