use std::cmp::Ordering;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, spanned::Spanned, visit_mut, visit_mut::VisitMut, Error,
    ExprMatch, Item, ItemFn, Meta, Pat, Path,
};

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

#[proc_macro_attribute]
pub fn check(
    _args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut item = parse_macro_input!(input as ItemFn);
    MatchSorted.visit_item_fn_mut(&mut item);
    quote!(#item).into()
}

struct MatchSorted;

impl VisitMut for MatchSorted {
    fn visit_expr_match_mut(&mut self, match_expr: &mut ExprMatch) {
        if let Some(a) = match_expr.attrs.iter().position(|attr| match &attr.meta {
            Meta::Path(Path { segments, .. }) => {
                segments.len() == 1 && segments[0].ident == "sorted"
            }
            _ => false,
        }) {
            match_expr.attrs.remove(a);
            if let Some(a) = match_expr.arms.iter().position(|arm| {
                !matches!(
                    arm.pat,
                    Pat::Path(_)
                        | Pat::Struct(_)
                        | Pat::TupleStruct(_)
                        | Pat::Ident(_)
                        | Pat::Wild(_)
                )
            }) {
                let err = Error::new(match_expr.arms[a].pat.span(), "unsupported by #[sorted]")
                    .to_compile_error();
                match_expr.arms[a].body = Box::new(parse_quote!(#err));
                return;
            }
            let arms: Vec<_> = match_expr.arms.clone().into_iter().enumerate().collect();
            let mut sorted = arms.clone();
            macro_rules! cmp {
                ($l:ident, $r:ident) => {{
                    let l: Vec<_> = $l.path.segments.iter().map(|s| &s.ident).collect();
                    let r: Vec<_> = $r.path.segments.iter().map(|s| &s.ident).collect();
                    l.cmp(&r)
                }};
            }
            sorted.sort_by(|(_, l), (_, r)| match (&l.pat, &r.pat) {
                (Pat::Wild(_), _) => Ordering::Greater,
                (_, Pat::Wild(_)) => Ordering::Less,
                (Pat::Path(l), Pat::Path(r)) => cmp!(l, r),
                (Pat::Struct(l), Pat::Struct(r)) => cmp!(l, r),
                (Pat::TupleStruct(l), Pat::TupleStruct(r)) => cmp!(l, r),
                (Pat::Ident(ident), other) => vec![&ident.ident].cmp(&match other {
                    Pat::Path(item) => item.path.segments.iter().map(|s| &s.ident).collect(),
                    Pat::Struct(item) => item.path.segments.iter().map(|s| &s.ident).collect(),
                    Pat::TupleStruct(item) => item.path.segments.iter().map(|s| &s.ident).collect(),
                    Pat::Ident(ident) => vec![&ident.ident],
                    _ => unimplemented!("Unsupported arm type"),
                }),
                (_, _) => unimplemented!("Unsupported arm type"),
            });
            for ((o, original), (s, sorted)) in arms.iter().zip(&sorted) {
                if o != s {
                    let (original_pat, sorted_pat) = (&original.pat, &sorted.pat);
                    let err = Error::new(
                        extract_span(&match_expr.arms[*s].pat),
                        format!(
                            "{} should sort before {}",
                            path_to_string(sorted_pat),
                            path_to_string(original_pat),
                        ),
                    )
                    .to_compile_error();
                    match_expr.arms[*s].body = Box::new(parse_quote!(#err));
                    break;
                }
            }
        }
        visit_mut::visit_expr_match_mut(self, match_expr);
    }
}

fn extract_span(variant: &Pat) -> Span {
    match variant {
        Pat::Path(item) => item.path.span(),
        Pat::Struct(item) => item.path.span(),
        Pat::TupleStruct(item) => item.path.span(),
        Pat::Ident(item) => item.ident.span(),
        Pat::Wild(item) => item.span(),
        _ => unimplemented!(),
    }
}

fn path_to_string(variant: &Pat) -> String {
    let path = match variant {
        Pat::Path(item) => &item.path,
        Pat::Struct(item) => &item.path,
        Pat::TupleStruct(item) => &item.path,
        Pat::Ident(item) => return item.ident.to_string(),
        Pat::Wild(_) => return "_".to_string(),
        _ => unimplemented!(),
    };
    let mut result = String::new();
    if path.leading_colon.is_some() {
        result.push_str("::");
    }
    for (i, segment) in path.segments.iter().enumerate() {
        if i > 0 {
            result.push_str("::");
        }
        result.push_str(&segment.ident.to_string());
    }
    result
}
