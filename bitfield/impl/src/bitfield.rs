use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{Fields, ImplGenerics, Item, Type, TypeGenerics, WhereClause};

pub fn bitfield(item: Item) -> TokenStream {
    match item {
        Item::Struct(syn::ItemStruct {
            attrs,
            vis,
            ident,
            generics,
            fields: Fields::Named(fields),
            ..
        }) => {
            let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
            let field_names: Vec<_> = fields
                .named
                .clone()
                .into_iter()
                .map(|f| f.ident.unwrap())
                .collect();
            let field_types: Vec<_> = fields.named.clone().into_iter().map(|f| f.ty).collect();
            let total_bits = quote!((0 + #(<#field_types as ::bitfield::Specifier>::BITS)+*));
            let total_bytes = quote!(#total_bits / 8 + if #total_bits % 8 == 0 { 0 } else { 1 });
            let get_set = get_set(
                &field_names,
                &field_types,
                &ident,
                &impl_generics,
                &ty_generics,
                where_clause,
                &total_bytes,
            );
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
            )
        }
        _ => unimplemented!(),
    }
}

fn get_set(
    field_names: &[Ident],
    field_types: &[Type],
    ident: &Ident,
    impl_generics: &ImplGenerics,
    ty_generics: &TypeGenerics,
    where_clause: Option<&WhereClause>,
    total_bytes: &TokenStream,
) -> TokenStream {
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
