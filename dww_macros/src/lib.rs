extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;

use syn::{DeriveInput, Body, Ident, Variant, VariantData};
use quote::Tokens;
use std::slice::Iter as SliceIter;
use std::iter;


#[proc_macro_derive(UserMsg)]
pub fn user_msg(input_tokens: TokenStream) -> TokenStream {
    let input = input_tokens.to_string();
    let item = syn::parse_derive_input(&input).expect("Attempted derive on non-item");

    impl_user_msg(&item).parse().unwrap()
}

fn impl_user_msg(&DeriveInput{ref ident, ref generics, ref body, ..}: &DeriveInput) -> Tokens {
    if let Body::Enum(ref variants) = *body {

        let discriminant_match_branches = DiscriminantMatchIter {
            enum_ident: ident,
            variants: variants.iter(),
            discriminant: 0
        };

        let empty_match_branches = EmptyMatchIter {
            enum_ident: ident,
            variants: variants.iter(),
            discriminant: 0
        };

        let conversion_offset_branches = ConversionOffsetIter {
            enum_ident: ident,
            variants: variants.iter(),
            discriminant: 0
        };

        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        #[cfg(not(feature = "inside_dww"))]
        let crate_root = quote!(_dww);
        #[cfg(feature = "inside_dww")]
        let crate_root = quote!();

        #[cfg(not(feature = "inside_dww"))]
        let extern_crate = quote!(extern crate dww as _dww;);
        #[cfg(feature = "inside_dww")]
        let extern_crate = quote!();

        let dummy_const = Ident::new(format!("_IMPL_USERMSG_FOR_{}", ident));

        quote!{
            #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
            const #dummy_const: () = {
                #extern_crate
                #[automatically_derived]
                impl #impl_generics #crate_root::user_msg::UserMsg for #ident #ty_generics #where_clause {
                    fn discriminant(&self) -> u16 {
                        match *self {
                            #(#discriminant_match_branches),*
                        }
                    }

                    unsafe fn empty(discriminant: u16) -> Self {
                        use std::mem;
                        match discriminant {
                            #(#empty_match_branches,)*
                            _ => panic!("Invalid discriminant")
                        }
                    }

                    fn register_conversion<C>(discriminant: u16, converter: &mut C)
                            where C: #crate_root::user_msg::UserMsgConverter<Self>
                    {
                        match discriminant {
                            #(#conversion_offset_branches,)*
                            _ => panic!("Invalid discriminant")
                        }
                    }
                }
            };
        }
    } else {panic!("Deriving UserMsg is only supported for enums")}
}

struct DiscriminantMatchIter<'a> {
    enum_ident: &'a Ident,
    variants: SliceIter<'a, Variant>,
    discriminant: u16
}

impl<'a> Iterator for DiscriminantMatchIter<'a> {
    type Item = Tokens;

    fn next(&mut self) -> Option<Tokens> {
        if let Some(cur_variant) = self.variants.next() {
            let enum_ident = self.enum_ident;
            let var_ident = &cur_variant.ident;
            let discriminant = self.discriminant;
            self.discriminant += 1;

            Some(match cur_variant.data {
                VariantData::Unit      => quote!(#enum_ident::#var_ident => #discriminant),
                VariantData::Struct(_) => quote!(#enum_ident::#var_ident{..} => #discriminant),
                VariantData::Tuple(ref fields) => {
                    let ignore_tokens = iter::repeat(quote!(_)).take(fields.len());
                    quote!(#enum_ident::#var_ident(#(#ignore_tokens),*) => #discriminant)
                }
            })
        } else {None}
    }
}

struct EmptyMatchIter<'a> {
    enum_ident: &'a Ident,
    variants: SliceIter<'a, Variant>,
    discriminant: u16
}

impl<'a> Iterator for EmptyMatchIter<'a> {
    type Item = Tokens;

    fn next(&mut self) -> Option<Tokens> {
        if let Some(cur_variant) = self.variants.next() {
            let enum_ident = self.enum_ident;
            let var_ident = &cur_variant.ident;
            let discriminant = self.discriminant;
            self.discriminant += 1;

            Some(match cur_variant.data {
                VariantData::Unit => quote!(#discriminant => #enum_ident::#var_ident),
                VariantData::Struct(ref fields) => {
                    let field_idents = fields.iter().map(|f| f.ident.as_ref().unwrap());
                    quote!(#discriminant => #enum_ident::#var_ident{#(#field_idents: mem::zeroed()),*})
                },
                VariantData::Tuple(ref fields) => {
                    let zeroed_tokens = iter::repeat(quote!(mem::zeroed())).take(fields.len());
                    quote!(#discriminant => #enum_ident::#var_ident(#(#zeroed_tokens),*))
                }
            })
        } else {None}
    }
}

struct ConversionOffsetIter<'a> {
    enum_ident: &'a Ident,
    variants: SliceIter<'a, Variant>,
    discriminant: u16
}

impl<'a> Iterator for ConversionOffsetIter<'a> {
    type Item = Tokens;

    fn next(&mut self) -> Option<Tokens> {
        if let Some(cur_variant) = self.variants.next() {
            let enum_ident = self.enum_ident;
            let var_ident = &cur_variant.ident;
            let discriminant = self.discriminant;
            self.discriminant += 1;

            Some(match cur_variant.data {
                VariantData::Unit => quote!(#discriminant => ()),
                VariantData::Struct(ref fields) => {
                    let field_idents_0 = fields.iter().map(|f| f.ident.as_ref().unwrap());
                    let field_idents_1 = fields.iter().map(|f| f.ident.as_ref().unwrap());
                    let field_types = fields.iter().map(|f| &f.ty);
                    quote!{
                        #discriminant => unsafe {
                            let base = Self::empty(#discriminant);
                            if let &#enum_ident::#var_ident{#(ref #field_idents_0),*} = &base {
                                #(converter.push_param::<#field_types>(#field_idents_1 as *const _ as usize - &base as *const _ as usize);)*
                            }
                        }
                    }
                },
                VariantData::Tuple(ref fields) => {
                    let field_idents_0 = (0..fields.len()).map(|n| Ident::new("anon_".to_string() + &n.to_string()));
                    let field_idents_1 = (0..fields.len()).map(|n| Ident::new("anon_".to_string() + &n.to_string()));
                    let field_types = fields.iter().map(|f| &f.ty);
                    quote!{
                        #discriminant => unsafe {
                            let base = Self::empty(#discriminant);
                            if let &#enum_ident::#var_ident(#(ref #field_idents_0),*) = &base {
                                #(converter.push_param::<#field_types>(#field_idents_1 as *const _ as usize - &base as *const _ as usize);)*
                            }
                        }
                    }
                }
            })
        } else {None}
    }
}
