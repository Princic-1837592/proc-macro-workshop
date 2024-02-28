use std::iter::Map;

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::{
    punctuated::Iter, spanned::Spanned, Data, DataStruct, DeriveInput, Field, Fields, FieldsNamed,
};

#[proc_macro_derive(Builder)]
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

fn unwraps(fields: &FieldsNamed) -> Map<Iter<Field>, fn(&Field) -> TokenStream> {
    fields.named.iter().map(|f| {
        let name = &f.ident;
        quote_spanned! {
            f.span() => #name: self.#name.clone().unwrap()
        }
    })
}

fn none_checks(fields: &FieldsNamed) -> Map<Iter<Field>, fn(&Field) -> TokenStream> {
    fields.named.iter().map(|f| {
        let name = f.clone().ident.unwrap();
        let err = format!("{} was not set", name);
        quote_spanned!(
            f.span() => if self.#name.is_none() {
                return std::result::Result::Err(#err.to_owned().into());
            }
        )
    })
}

fn option_wrapped(fields: &FieldsNamed) -> Map<Iter<Field>, fn(&Field) -> TokenStream> {
    fields.named.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;
        quote_spanned! {
            f.span() => #name: std::option::Option<#ty>
        }
    })
}

fn setters(fields: &FieldsNamed) -> Map<Iter<Field>, fn(&Field) -> TokenStream> {
    fields.named.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;
        quote_spanned! {
            f.span() => fn #name(&mut self, #name: #ty) -> &mut Self {
                self.#name = std::option::Option::Some(#name);
                self
            }
        }
    })
}
