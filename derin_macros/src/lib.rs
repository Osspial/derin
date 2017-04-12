#![feature(conservative_impl_trait)]

// Quote recurses a lot.
#![recursion_limit="128"]

extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;

use syn::*;
use quote::Tokens;
use std::iter;

#[proc_macro_derive(Parent, attributes(derin))]
pub fn derive_parent(input_tokens: TokenStream) -> TokenStream {
    let input = input_tokens.to_string();
    let item = syn::parse_derive_input(&input).expect("Attempted derive on non-item");

    let output = impl_parent(&item).parse().unwrap();
    output
}

fn impl_parent(&DeriveInput{ref ident, ref attrs, ref generics, ref body, ..}: &DeriveInput) -> Tokens {
    // Process attributes on the item being derived
    let mut child_action_ty = None;
    derin_attribute_iter(attrs, |attr| {
        match *attr {
            MetaItem::NameValue(ref ident, Lit::Str(ref string, _)) if ident == "child_action" =>
                if child_action_ty.is_none() {
                    child_action_ty = Some(syn::parse_type(&string).expect("Bad type in child action"));
                } else {
                    panic!("Repeated child_action attribute: {}", quote!(#attr).to_string())
                },
            _ => panic!("Bad Derin attribute: {}", quote!(#attr).to_string())
        }
    });

    let child_action_ty = child_action_ty.expect("Missing #[derin(child_action = \"...\")] attribute");

    // Process attributes on the fields in the item being derived
    let mut layout_ident = None;
    let mut widget_fields = Vec::new();
    match *body {
        Body::Struct(ref variant_data) =>
            for (index, field) in variant_data.fields().iter().enumerate() {
                let mut is_layout_field = false;
                derin_attribute_iter(&field.attrs, |attr| {
                    match *attr {
                        MetaItem::Word(ref attr_name)
                            if attr_name == "layout" =>
                            if layout_ident.is_none() {
                                layout_ident = Some(field.ident.clone().unwrap_or(Ident::new(index)));
                                is_layout_field = true;
                            } else {
                                panic!("Repeated #[derin(layout)] attribute: {}", quote!(#attr).to_string())
                            },
                        _ => panic!("Bad Derin attribute: {}", quote!(#attr).to_string())
                    }
                });

                if !is_layout_field {
                    widget_fields.push(field);
                }
            },
        _ => unimplemented!()
    }
    let layout_ident = layout_ident.expect("No field with #[derin(layout)] attribute");

    let widget_info_iters = || (
        widget_fields.iter().enumerate().map(|(i, field)| field.ident.clone().unwrap_or(Ident::new(i))),
        widget_fields.iter().enumerate().map(|(i, field)| (i as u32, field))
                                        .map(|(i, field)| match field.ident {
                                            Some(ref ident) => quote!(_derive_derin::ui::ChildId::Str(stringify!(#ident))),
                                            None            => quote!(_derive_derin::ui::ChildId::Num(#i))
                                        })
    );

    let parent_mut = parent_mut(ident, generics, body, &child_action_ty, &widget_info_iters, &widget_fields, &layout_ident);
    let parent = parent(ident, generics, body, &widget_info_iters, &widget_fields, &layout_ident);

    let dummy_const = Ident::new(format!("_IMPL_PARENT_FOR_{}", ident));

    quote!{
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        const #dummy_const: () = {
            extern crate derin as _derive_derin;

            #parent_mut
            #parent
        };
    }
}

fn parent_mut<F, I, C>(
    ident: &Ident,
    generics: &Generics,
    body: &Body,
    child_action_ty: &Ty,
    widget_info_iters: &F,
    widget_fields: &[&Field],
    layout_ident: &Ident
) -> Tokens
        where F: Fn() -> (I, C),
              I: Iterator<Item = Ident>,
              C: Iterator<Item = Tokens>
{
    let mut_generics_raw = expand_generics(generics, body, widget_fields, |ty_string| format!("_derive_derin::ui::NodeProcessorGridMut<{}>", ty_string));
    let (mut_impl_generics, _, mut_where_clause) = mut_generics_raw.split_for_impl();
    let (_, ty_generics, _) = generics.split_for_impl();
    let layout_ident_iter = iter::repeat(&layout_ident);
    let (widget_idents, widget_child_ids) = widget_info_iters();

    quote!{
        impl #mut_impl_generics ParentMut<NPI> for #ident #ty_generics #mut_where_clause {
            type ChildAction = #child_action_ty;

            fn children_mut(&mut self, npi: NPI) -> Result<(), NPI::Error> {
                let mut np = npi.init_grid(
                    self.#layout_ident.grid_size(),
                    self.#layout_ident.col_hints(),
                    self.#layout_ident.row_hints()
                );

                #({
                    let child_id = #widget_child_ids;
                    if let Some(hints) = self.#layout_ident_iter.get_hints(child_id) {
                        np.add_child_mut(child_id, hints, &mut self.#widget_idents)?;
                    }
                })*
                Ok(())
            }
        }
    }
}

fn parent<F, I, C>(
    ident: &Ident,
    generics: &Generics,
    body: &Body,
    widget_info_iters: &F,
    widget_fields: &[&Field],
    layout_ident: &Ident
) -> Tokens
        where F: Fn() -> (I, C),
              I: Iterator<Item = Ident>,
              C: Iterator<Item = Tokens>
{
    let mut_generics_raw = expand_generics(generics, body, widget_fields, |ty_string| format!("_derive_derin::ui::NodeProcessorGrid<{}>", ty_string));
    let (mut_impl_generics, _, mut_where_clause) = mut_generics_raw.split_for_impl();
    let (_, ty_generics, _) = generics.split_for_impl();
    let layout_ident_iter = iter::repeat(&layout_ident);
    let (widget_idents, widget_child_ids) = widget_info_iters();

    quote!{
        impl #mut_impl_generics Parent<NPI> for #ident #ty_generics #mut_where_clause {
            fn children(&self, npi: NPI) -> Result<(), NPI::Error> {
                let mut np = npi.init_grid(
                    self.#layout_ident.grid_size(),
                    self.#layout_ident.col_hints(),
                    self.#layout_ident.row_hints()
                );

                #({
                    let child_id = #widget_child_ids;
                    if let Some(hints) = self.#layout_ident_iter.get_hints(child_id) {
                        np.add_child(child_id, hints, &self.#widget_idents)?;
                    }
                })*
                Ok(())
            }
        }
    }
}

fn derin_attribute_iter<F>(attrs: &[Attribute], mut for_each: F)
        where F: FnMut(&MetaItem)
{
    for attr in attrs.iter().filter(|attr| attr.name() == "derin") {
        if let MetaItem::List(_, ref meta_list) = attr.value {
            for inner_attr in meta_list.iter() {
                if let NestedMetaItem::MetaItem(ref inner_meta) = *inner_attr {
                    for_each(inner_meta)
                } else {
                    panic!("Invalid derin attribute: {}", quote!(#attr).to_string())
                }
            }
        } else {
            panic!("Invalid derin attribute: {}", quote!(#attr).to_string())
        }
    }
}

fn expand_generics<F>(generics: &Generics, body: &Body, widget_fields: &[&Field], mut trait_fn: F) -> Generics
        where F: FnMut(&str) -> String
{
    let mut generics = generics.clone();
    generics.ty_params.insert(0, TyParam {
        attrs: Vec::new(),
        ident: Ident::new("NPI"),
        bounds: Vec::new(),
        default: None
    });

    let init_bound = WhereBoundPredicate {
        bound_lifetimes: Vec::new(),
        bounded_ty: syn::parse_type("NPI").unwrap(),
        bounds: vec![TyParamBound::Trait(
            PolyTraitRef {
                bound_lifetimes: Vec::new(),
                trait_ref: syn::parse_path("_derive_derin::ui::NodeProcessorInit").unwrap()
            },
            TraitBoundModifier::None
        )]
    };
    generics.where_clause.predicates.push(WherePredicate::BoundPredicate(init_bound));

    let npi_gridproc_ty = syn::parse_type("NPI::GridProcessor").unwrap();
    for ty in field_types(body, widget_fields) {
        let ty_string = quote!(#ty).to_string();
        let member_bound = WhereBoundPredicate {
            bound_lifetimes: Vec::new(),
            bounded_ty: npi_gridproc_ty.clone(),
            bounds: vec![TyParamBound::Trait(
                PolyTraitRef{
                    bound_lifetimes: Vec::new(),
                    trait_ref: syn::parse_path(&trait_fn(&ty_string)).unwrap()
                },
                TraitBoundModifier::None
            )]
        };
        generics.where_clause.predicates.push(WherePredicate::BoundPredicate(member_bound));
    }

    let never_bound = WhereBoundPredicate {
        bound_lifetimes: Vec::new(),
        bounded_ty: npi_gridproc_ty.clone(),
        bounds: vec![TyParamBound::Trait(
            PolyTraitRef{
                bound_lifetimes: Vec::new(),
                trait_ref: syn::parse_path(&trait_fn("!")).unwrap()
            },
            TraitBoundModifier::None
        )]
    };
    generics.where_clause.predicates.push(WherePredicate::BoundPredicate(never_bound));


    generics
}

fn field_types(body: &Body, widget_fields: &[&Field]) -> Vec<Ty> {
    let mut ty_vec = Vec::new();
    match *body {
        Body::Struct(ref variant_data) => {
            for field in variant_data.fields() {
                if !ty_vec.contains(&field.ty) && widget_fields.contains(&field) {
                    ty_vec.push(field.ty.clone());
                }
            }
        },
        Body::Enum(ref variants) => {
            for variant_data in variants.iter().map(|variant| &variant.data) {
                for field in variant_data.fields() {
                    if !ty_vec.contains(&field.ty) {
                        ty_vec.push(field.ty.clone());
                    }
                }
            }
        }
    }
    ty_vec
}
