use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{ImplGenerics, Item, parse_macro_input, Type, TypeGenerics, WhereClause};

#[proc_macro_attribute]
pub fn bitfield(
    _: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    expand(parse_macro_input!(input as Item)).into()
}

fn expand(item: Item) -> TokenStream {
    match item {
        Item::Struct(syn::ItemStruct {
            attrs,
            vis,
            ident,
            generics,
            fields: syn::Fields::Named(fields),
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
            let total_bits = quote!((#(<#field_types as ::bitfield::Specifier>::BITS)+*));
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
        const _: () ={
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
        };
    )
}

#[proc_macro]
pub fn generate_specifier(_: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let bits = 1..=64_usize;
    let names: Vec<_> = bits.clone().map(|i| format_ident!("B{}", i)).collect();
    let u_types: Vec<_> = bits.clone().map(to_u_type).collect();
    let shifts = u_types
        .clone()
        .into_iter()
        .zip(bits.clone())
        .map(|(u_type, bits)| quote!(
            ((::std::mem::size_of::<#u_type>() * 8 - #bits % (::std::mem::size_of::<#u_type>() * 8)) % (::std::mem::size_of::<#u_type>() * 8))
    ));
    quote!(
        #(
            pub enum #names {}

            impl Specifier for #names {
                const BITS: usize = #bits;
                type T = #u_types;

                fn get<const OFFSET: usize, const SIZE: usize>(bytes: &[u8]) -> <Self as Specifier>::T {
                    <<Self as Specifier>::T as PrivateSpecifier>::get::<OFFSET, SIZE>(bytes) >> #shifts
                }

                fn set<const OFFSET: usize, const SIZE: usize>(bytes: &mut [u8], value: <Self as Specifier>::T) {
                    <<Self as Specifier>::T as PrivateSpecifier>::set::<{<Self as Specifier>::BITS}, OFFSET, SIZE>(bytes, value << #shifts);
                }
            }
        )*
    )
    .into()
}

fn to_u_type(bits: usize) -> Ident {
    let bits: u8 = match bits {
        0..=8 => 8,
        9..=16 => 16,
        17..=32 => 32,
        33..=64 => 64,
        _ => unimplemented!(),
    };
    format_ident!("u{}", bits)
}

#[proc_macro]
pub fn generate_private_specifier(_: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let bits: [usize; 4] = [8, 16, 32, 64];
    let types: Vec<_> = bits
        .clone()
        .iter()
        .map(|i| format_ident!("u{}", i))
        .collect();
    quote!(
        #(
            impl PrivateSpecifier for #types {
                const BITS: usize = #bits;
                type T = Self;

                fn get<const OFFSET: usize, const SIZE: usize>(bytes: &[u8]) -> <Self as PrivateSpecifier>::T {
                    // OFFSET / 8 ==> index of the first byte
                    // OFFSET / 8 + 1 ==> index of the second byte
                    if OFFSET % 8 == 0 {
                        // if aligned, just take the bytes from the first one
                        let len = ::std::mem::size_of::<Self>().min(SIZE - OFFSET / 8);
                        let mut slice = [0; ::std::mem::size_of::<Self>()];
                        slice[..len].copy_from_slice(&bytes[OFFSET / 8..OFFSET / 8 + len]);
                        Self::from_be_bytes(slice)
                    } else {
                        // if not aligned, take the |$u_type| bytes starting from the second one,
                        // shift them to the right and add the first byte on the left
                        let len = ::std::mem::size_of::<Self>().min(SIZE - (OFFSET / 8 + 1));
                        let mut slice = [0; ::std::mem::size_of::<Self>()];
                        slice[..len].copy_from_slice(&bytes[OFFSET / 8 + 1..OFFSET / 8 + 1 + len]);
                        let right = Self::from_be_bytes(slice);
                        ((bytes[OFFSET / 8] as Self)
                            << (::std::mem::size_of::<Self>() * 8 - 8 + OFFSET % 8))
                            | (right >> (8 - OFFSET % 8))
                    }
                }
                
                fn set<const ACTUAL_BITS: usize, const OFFSET: usize, const SIZE: usize>(
                    bytes: &mut [u8],
                    value: <Self as PrivateSpecifier>::T,
                ) {
                    if OFFSET % 8 == 0 {
                        let len = 8.min(SIZE - (OFFSET / 8));
                        let old = <Self as PrivateSpecifier>::get::<OFFSET, SIZE>(bytes);
                        let mask = if ACTUAL_BITS == <Self as PrivateSpecifier>::BITS {
                            0
                        } else {
                            Self::MAX >> ACTUAL_BITS
                        };
                        let new = value | (old & mask);
                        bytes[OFFSET / 8..OFFSET / 8 + len].copy_from_slice(&new.to_be_bytes()[..len]);
                    } else {
                        let start_right = OFFSET / 8 + 1;
                        let len = ::std::mem::size_of::<Self>().min(SIZE - start_right);
                        let old =
                            Self::from_be_bytes(bytes[start_right..start_right + len].try_into().unwrap());
                        let actual_bits = ACTUAL_BITS - (8 - OFFSET % 8);
                        let mask = if actual_bits == <Self as PrivateSpecifier>::BITS {
                            0
                        } else {
                            Self::MAX >> actual_bits
                        };
                        let new = (value << (8 - OFFSET % 8)) | (old & mask);
                        bytes[start_right..start_right + len].copy_from_slice(&new.to_be_bytes()[..len]);
                        let left = (value >> (::std::mem::size_of::<Self>() * 8 - 8 + OFFSET % 8)) as u8;
                        let old_left = bytes[OFFSET / 8];
                        let mask = u8::MAX << (8 - OFFSET % 8);
                        bytes[OFFSET / 8] = (old_left & mask) | left
                    }
                }
            }
        )*
    )
        .into()
}
