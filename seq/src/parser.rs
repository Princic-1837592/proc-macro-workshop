use std::fmt::Display;

use proc_macro2::{token_stream::IntoIter, Delimiter, Ident, Span, TokenStream, TokenTree};
use syn::Error;

fn next(iter: &mut IntoIter) -> syn::Result<TokenTree> {
    iter.next()
        .ok_or_else(|| Error::new(Span::call_site(), "unexpected end of input".to_owned()))
}

fn error(span: TokenTree, message: impl Display) -> Error {
    Error::new(span.span(), message)
}

pub(crate) fn next_ident(iter: &mut IntoIter) -> syn::Result<Ident> {
    match next(iter)? {
        TokenTree::Ident(ident) => Ok(ident),
        other => Err(error(other, "expected ident")),
    }
}

pub(crate) fn next_keyword(iter: &mut IntoIter, keyword: &str) -> syn::Result<()> {
    let token = next(iter)?;
    if let TokenTree::Ident(ident) = &token {
        if *ident == keyword {
            return Ok(());
        }
    }
    Err(error(token, format!("expected `{}`", keyword)))
}

pub(crate) fn next_value(iter: &mut IntoIter) -> syn::Result<(u64, Span)> {
    let token = next(iter)?;
    if let TokenTree::Literal(lit) = &token {
        if let Ok(value) = lit.to_string().parse::<u64>() {
            return Ok((value, lit.span()));
        }
    }
    Err(Error::new(token.span(), "expected numeric literal"))
}

pub(crate) fn maybe_next_punct(iter: &mut IntoIter, ch: char) -> syn::Result<bool> {
    let present = match iter.clone().next() {
        Some(TokenTree::Punct(_)) => {
            next_punct(iter, ch)?;
            true
        }
        _ => false,
    };
    Ok(present)
}

pub(crate) fn next_punct(iter: &mut IntoIter, char: char) -> syn::Result<()> {
    let token = next(iter)?;
    if let TokenTree::Punct(punct) = &token {
        if punct.as_char() == char {
            return Ok(());
        }
    }
    Err(error(token, format!("expected `{}`", char)))
}

pub(crate) fn next_braces(iter: &mut IntoIter) -> syn::Result<TokenStream> {
    let token = next(iter)?;
    if let TokenTree::Group(group) = &token {
        if group.delimiter() == Delimiter::Brace {
            return Ok(group.stream());
        }
    }
    Err(error(token, "expected curly braces"))
}

pub(crate) fn next_end(iter: &mut IntoIter) -> syn::Result<()> {
    match iter.next() {
        Some(token) => Err(error(token, "unexpected token")),
        None => Ok(()),
    }
}
