use deluxe::{parse_attributes, ParseAttributes};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, Data, DataStruct, DeriveInput, Field, Fields, GenericParam, Generics};

#[derive(ParseAttributes, Clone, Debug, Default)]
#[deluxe(attributes(debug))]
struct DebugFieldAttributes {
    format: Option<String>,
}

deluxe::define_with_collection!(
    mod mod_path_vec,
    deluxe::with::from_str,
    Vec<String>
);

#[derive(ParseAttributes, Clone, Debug, Default)]
#[deluxe(attributes(debug))]
struct DebugStructAttributes {
    #[deluxe(default = Vec::new())]
    #[deluxe(with = mod_path_vec)]
    unbound: Vec<String>,
    #[deluxe(default = Vec::new())]
    #[deluxe(with = mod_path_vec)]
    bound: Vec<String>,
}

type MaybeAttr = Result<DebugFieldAttributes, syn::Error>;
type FieldAndAttr = (Field, MaybeAttr);
type FieldsAndAttrs = Vec<FieldAndAttr>;

// #[allow(unused)]
#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input: DeriveInput = syn::parse(input).unwrap();
    match derive_input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => {
            let attrs: DebugStructAttributes = match parse_attributes(&derive_input.attrs) {
                Ok(attrs) => attrs,
                Err(err) => return err.to_compile_error().into(),
            };
            let fields_attrs: FieldsAndAttrs = fields
                .named
                .iter()
                .map(|f| (f.clone(), parse_attributes(&f.attrs)))
                .collect();
            if let Some((_, err)) = fields_attrs.iter().find(|(_, attr)| attr.is_err()) {
                return err.clone().unwrap_err().to_compile_error().into();
            }
            let ident = &derive_input.ident;
            let method_calls = method_calls(&fields_attrs);
            let (impl_generics, generics, where_clause) = generics(&derive_input.generics, &attrs);

            // let result = quote!(
            //     impl <#impl_generics> ::std::fmt::Debug for #ident <#generics> where #where_clause {
            //         fn fmt(&self, fmt: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            //             let mut debug_struct = fmt.debug_struct(stringify!(#ident));
            //             #(#method_calls)*
            //             debug_struct.finish()
            //         }
            //     }
            // );
            // println!("{}", result);

            quote!(
                impl <#impl_generics> ::std::fmt::Debug for #ident <#generics> where #where_clause {
                    fn fmt(&self, fmt: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                        let mut debug_struct = fmt.debug_struct(stringify!(#ident));
                        #(#method_calls)*
                        debug_struct.finish()
                    }
                }
            )
            .into()
        }
        _ => panic!("CustomDebug can only be applied to structs with named fields"),
    }
}

fn generics(
    generics: &Generics,
    attrs: &DebugStructAttributes,
) -> (TokenStream, TokenStream, TokenStream) {
    let params = &generics.params;
    let impl_generics = quote!(#params);
    let mut generic_types: Vec<_> = params
        .iter()
        .filter_map(|p| match p {
            GenericParam::Type(syn::TypeParam { ident: ty, .. })
                if !attrs.unbound.contains(&format!("{}", ty)) =>
            {
                Some(quote_spanned!(
                    ty.span() => #ty: ::std::fmt::Debug
                ))
            }
            _ => None,
        })
        .collect();
    generic_types.extend(attrs.bound.iter().map(|ty| {
        let ty: syn::TypePath = syn::parse_str(ty).unwrap();
        quote!(#ty: ::std::fmt::Debug)
    }));
    if let Some(where_clause) = &generics.where_clause {
        generic_types.extend(
            where_clause
                .predicates
                .iter()
                .map(|p| quote_spanned!(p.span() => #p)),
        );
    }
    let while_clause = quote!(#(#generic_types),*);
    (impl_generics.clone(), impl_generics, while_clause)
}

fn method_calls(fields: &FieldsAndAttrs) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|(f, attr)| {
            let name = &f.ident;
            let value = match attr {
                Ok(DebugFieldAttributes {
                    format: Some(format),
                    ..
                }) => quote!(&format_args!(#format, &self.#name)),
                _ => quote!(&self.#name),
            };
            quote_spanned!(
                f.span() => debug_struct.field(stringify!(#name), #value);
            )
        })
        .collect()
}
