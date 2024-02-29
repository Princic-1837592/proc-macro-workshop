use deluxe::{parse_attributes, ParseAttributes};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::{
    spanned::Spanned, Data, DataStruct, DeriveInput, Field, Fields, GenericArgument, PathSegment,
    Type, TypePath,
};

#[derive(ParseAttributes, Clone, Debug, Default)]
#[deluxe(attributes(builder))]
struct BuilderAttributes {
    optional: Option<()>,
    each: Option<String>,
}

type MaybeAttr = Result<BuilderAttributes, syn::Error>;
type FieldAndAttr = (Field, MaybeAttr);
type FieldsAndAttrs = Vec<FieldAndAttr>;

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input: DeriveInput = syn::parse(input).unwrap();
    match derive_input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => {
            let fields_attrs: FieldsAndAttrs = fields
                .named
                .iter()
                .map(|f| (f.clone(), parse_attributes(&f.attrs)))
                .collect();
            if let Some((_, err)) = fields_attrs.iter().find(|(_, attr)| attr.is_err()) {
                return err.clone().unwrap_err().to_compile_error().into();
            }
            let ident = &derive_input.ident;
            let builder_ident = format_ident!("{ident}Builder");
            let builder_struct = builder_struct(&fields_attrs, ident, builder_ident.clone());
            let defaults = defaults(&fields_attrs);
            quote!(
                #builder_struct

                impl #ident {
                    fn builder() -> #builder_ident {
                        #builder_ident {
                            #defaults
                        }
                    }
                }
            )
            .into()
        }
        _ => unimplemented!(),
    }
}

fn builder_struct(fields: &FieldsAndAttrs, ident: &Ident, builder_ident: Ident) -> TokenStream {
    let option_wrapped = option_wrapped(fields);
    let none_checks = none_checks(fields);
    let unwraps = unwraps(fields);
    let setters = setters(fields);
    quote!(
        struct #builder_ident {
            #(#option_wrapped),*
        }

        impl #builder_ident {
            fn build(&mut self) -> ::std::result::Result<#ident, ::std::boxed::Box<dyn ::std::error::Error>> {
                #(#none_checks)*
                ::std::result::Result::Ok(
                    #ident {
                        #(#unwraps,)*
                    }
                )
            }

            #(#setters)*
        }
    )
}

fn is_optional(attr: &MaybeAttr) -> bool {
    matches!(
        attr,
        Ok(BuilderAttributes {
            optional: Some(_),
            ..
        })
    )
}

fn is_vec(attr: &MaybeAttr) -> bool {
    matches!(attr, Ok(BuilderAttributes { each: Some(_), .. }))
}

fn get_inner((field, attr): &FieldAndAttr) -> Option<TokenStream> {
    if is_optional(attr) || is_vec(attr) {
        match &field.ty {
            Type::Path(TypePath {
                path: syn::Path { segments, .. },
                ..
            }) => {
                if segments.len() != 1 {
                    return None;
                }
                let segment = segments.iter().next().unwrap();
                match segment {
                    PathSegment {
                        arguments:
                            syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                                args,
                                ..
                            }),
                        ..
                    } => {
                        if args.len() != 1 {
                            return None;
                        }
                        let arg = args.iter().next().unwrap();
                        match arg {
                            GenericArgument::Type(t) => Some(quote!(#t)),
                            _ => None,
                        }
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    } else {
        None
    }
}

fn defaults(fields_attrs: &FieldsAndAttrs) -> TokenStream {
    let defaults = fields_attrs.iter().map(|(f, attr)| {
        let name = &f.ident;
        if is_vec(attr) {
            quote_spanned! {
                f.span() => #name: ::std::vec::Vec::new()
            }
        } else {
            quote_spanned! {
                f.span() => #name: ::std::option::Option::None
            }
        }
    });
    quote!(#(#defaults),*)
}

fn unwraps(fields: &FieldsAndAttrs) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|(f, attr)| {
            let name = &f.ident;
            if is_optional(attr) || is_vec(attr) {
                quote_spanned! {
                    f.span() => #name: self.#name.clone()
                }
            } else {
                quote_spanned! {
                    f.span() => #name: self.#name.clone().unwrap()
                }
            }
        })
        .collect()
}

fn none_checks(fields: &FieldsAndAttrs) -> Vec<TokenStream> {
    fields
        .iter()
        .filter_map(|(f, attr)| {
            if is_optional(attr) || is_vec(attr) {
                None
            } else {
                let name = f.clone().ident.unwrap();
                let err = format!("{} was not set", name);
                Some(quote_spanned!(
                    f.span() => if self.#name.is_none() {
                        return ::std::result::Result::Err(#err.to_owned().into());
                    }
                ))
            }
        })
        .collect()
}

fn option_wrapped(fields: &FieldsAndAttrs) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|(f, attr)| {
            let name = &f.ident;
            let ty = &f.ty;
            if is_optional(attr) || is_vec(attr) {
                quote_spanned! {
                    f.span() => #name: #ty
                }
            } else {
                quote_spanned! {
                    f.span() => #name: ::std::option::Option<#ty>
                }
            }
        })
        .collect()
}

fn setters(fields: &FieldsAndAttrs) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|pair @ (f, attr)| {
            let name = &f.ident;
            let ty = &f.ty;
            if is_vec(attr) {
                let inner =
                    get_inner(pair).unwrap_or_else(|| panic!("Vector fields require type Vec<T>"));
                let method = format_ident!("{}", attr.clone().unwrap().each.unwrap());
                quote_spanned! {
                    f.span() => fn #method(&mut self, #name: #inner) -> &mut Self {
                        self.#name.push(#name);
                        self
                    }
                }
            } else {
                let actual_ty = if let Some(inner) = get_inner(pair) {
                    quote!(#inner)
                } else {
                    quote!(#ty)
                };
                quote_spanned! {
                    f.span() => fn #name(&mut self, #name: #actual_ty) -> &mut Self {
                        self.#name = ::std::option::Option::Some(#name);
                        self
                    }
                }
            }
        })
        .collect()
}
