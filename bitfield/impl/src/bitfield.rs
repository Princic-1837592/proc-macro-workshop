use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::{
    spanned::Spanned, Expr, ExprLit, Field, Fields, FieldsNamed, Generics, Item, Lit, Meta,
    MetaNameValue, Type,
};

pub fn bitfield(item: Item) -> TokenStream {
    match item {
        Item::Struct(syn::ItemStruct {
            attrs,
            vis,
            ident,
            generics,
            fields: Fields::Named(FieldsNamed { named: fields, .. }),
            ..
        }) => {
            let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
            let fields: Vec<_> = fields.into_iter().collect();
            let field_names: Vec<_> = fields
                .clone()
                .into_iter()
                .map(|f| f.ident.unwrap())
                .collect();
            let field_types: Vec<_> = fields.clone().into_iter().map(|f| f.ty).collect();
            let total_bits = quote!((0 + #(<#field_types as ::bitfield::Specifier>::BITS)+*));
            let total_bytes = quote!(#total_bits / 8 + if #total_bits % 8 == 0 { 0 } else { 1 });
            let get_set = get_set(&field_names, &field_types, &ident, &generics, &total_bytes);
            let bits_checks = bits_checks(fields);
            quote!(
                #(#attrs)*
                #[repr(C)]
                #vis struct #ident #impl_generics #where_clause {
                    data: [u8; #total_bytes],
                }

                impl #impl_generics #ident #ty_generics #where_clause {
                    fn new() -> Self {
                        let _: ::bitfield::checks::TotalSize::<[(); #total_bits % 8]>;
                        Self {
                            data: [0; #total_bytes]
                        }
                    }
                }

                #get_set

                #bits_checks
            )
        }
        _ => unimplemented!(),
    }
}

fn get_set(
    field_names: &[Ident],
    field_types: &[Type],
    ident: &Ident,
    generics: &Generics,
    total_bytes: &TokenStream,
) -> TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let gets = field_names.iter().map(|n| format_ident!("get_{}", n));
    let sets = field_names.iter().map(|n| format_ident!("set_{}", n));
    let mut offsets = vec![quote!(0)];
    for field_type in field_types.iter().take(field_types.len() - 1) {
        let prev = offsets.last().unwrap();
        offsets.push(quote!(#prev + <#field_type as ::bitfield::Specifier>::BITS));
    }
    quote!(
        impl #impl_generics #ident #ty_generics #where_clause {
            #(
                pub fn #gets(&self) -> <#field_types as ::bitfield::Specifier>::T {
                    <#field_types as ::bitfield::Specifier>::get::<{#offsets}, {#total_bytes}>(&self.data)
                }

                pub fn #sets(&mut self, value: <#field_types as ::bitfield::Specifier>::T) {
                    <#field_types as ::bitfield::Specifier>::set::<{#offsets}, {#total_bytes}>(&mut self.data, value);
                }
            )*
        }
    )
}

fn bits_checks(fields: Vec<Field>) -> TokenStream {
    let checks = fields
        .into_iter()
        .filter_map(|f| {
            let bits = f.attrs.iter().find_map(|a| match &a.meta {
                Meta::NameValue(
                    meta @ MetaNameValue {
                        value:
                            Expr::Lit(ExprLit {
                                lit: Lit::Int(..), ..
                            }),
                        ..
                    },
                ) if meta.path.is_ident("bits") => Some(meta.value.clone()),
                _ => None,
            });
            bits.map(|bits| (f.clone(), bits))
        })
        .map(|(f, bits)| {
            let f_type = f.ty;
            quote_spanned!(
                bits.span() => let _: [(); #bits] = [(); <#f_type as ::bitfield::Specifier>::BITS];
            )
        });
    quote!(
        const _: () = {
            #(#checks)*
        };
    )
}
