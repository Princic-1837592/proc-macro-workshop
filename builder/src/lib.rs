use deluxe::{parse_attributes, ParseAttributes};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::{
    spanned::Spanned, Data, DataStruct, DeriveInput, Field, Fields, FieldsNamed, GenericArgument,
    PathSegment, Type, TypePath,
};

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input: DeriveInput = syn::parse(input).unwrap();
    let ident = &derive_input.ident;
    let builder_ident = format_ident!("{ident}Builder");
    let builder_struct = builder_struct(&derive_input.data, ident, builder_ident.clone());
    let defaults = defaults(&derive_input.data);
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

fn builder_struct(data: &Data, ident: &Ident, builder_ident: Ident) -> TokenStream {
    match data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => {
            let option_wrapped = option_wrapped(fields);
            let none_checks = none_checks(fields);
            let unwraps = unwraps(fields);
            let setters = setters(fields);
            quote!(
                struct #builder_ident {
                    #(#option_wrapped),*
                }

                impl #builder_ident {
                    fn build(&mut self) -> std::result::Result<#ident, std::boxed::Box<dyn std::error::Error>> {
                        #(#none_checks)*
                        std::result::Result::Ok(
                            #ident {
                                #(#unwraps,)*
                            }
                        )
                    }

                    #(#setters)*
                }
            )
        }
        _ => {
            unimplemented!()
        }
    }
}

fn defaults(data: &Data) -> TokenStream {
    match data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => {
            let defaults = fields.named.iter().map(|f| {
                let name = &f.ident;
                quote_spanned! {
                    f.span() => #name: std::option::Option::None
                }
            });
            quote!(#(#defaults),*)
        }
        _ => {
            unimplemented!()
        }
    }
}

#[derive(ParseAttributes, Debug)]
#[deluxe(attributes(builder))]
struct BuilderAttributes {
    optional: Option<bool>,
}

fn is_optional(field: &Field) -> bool {
    field.attrs.iter().any(|attr| {
        let default: Result<BuilderAttributes, _> = parse_attributes(attr);
        matches!(
            default,
            Ok(BuilderAttributes {
                optional: Some(true),
                ..
            })
        )
    })
}

fn get_inner(field: &Field) -> Option<TokenStream> {
    if is_optional(field) {
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

fn unwraps(fields: &FieldsNamed) -> Vec<TokenStream> {
    fields
        .named
        .iter()
        .map(|f| {
            let name = &f.ident;
            if is_optional(f) {
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

fn none_checks(fields: &FieldsNamed) -> Vec<TokenStream> {
    fields
        .named
        .iter()
        .filter_map(|f| {
            if is_optional(f) {
                None
            } else {
                let name = f.clone().ident.unwrap();
                let err = format!("{} was not set", name);
                Some(quote_spanned!(
                    f.span() => if self.#name.is_none() {
                        return std::result::Result::Err(#err.to_owned().into());
                    }
                ))
            }
        })
        .collect()
}

fn option_wrapped(fields: &FieldsNamed) -> Vec<TokenStream> {
    fields
        .named
        .iter()
        .map(|f| {
            let name = &f.ident;
            let ty = &f.ty;
            if is_optional(f) {
                quote_spanned! {
                    f.span() => #name: #ty
                }
            } else {
                quote_spanned! {
                    f.span() => #name: std::option::Option<#ty>
                }
            }
        })
        .collect()
}

fn setters(fields: &FieldsNamed) -> Vec<TokenStream> {
    fields
        .named
        .iter()
        .map(|f| {
            let name = &f.ident;
            let ty = &f.ty;
            let actual_ty = if let Some(inner) = get_inner(f) {
                quote!(#inner)
            } else {
                quote!(#ty)
            };
            quote_spanned! {
                f.span() => fn #name(&mut self, #name: #actual_ty) -> &mut Self {
                    self.#name = std::option::Option::Some(#name);
                    self
                }
            }
        })
        .collect()
}
