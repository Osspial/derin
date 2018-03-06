#![feature(conservative_impl_trait)]

// Quote recurses a lot.
#![recursion_limit="256"]

extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;

use syn::*;
use quote::Tokens;

#[proc_macro_derive(WidgetContainer, attributes(derin))]
pub fn derive_widget_container(input_tokens: TokenStream) -> TokenStream {
    let input = input_tokens.to_string();
    let item = syn::parse_derive_input(&input).expect("Attempted derive on non-item");

    let output = impl_widget_container(&item).parse().unwrap();
    output
}

fn impl_widget_container(derive_input: &DeriveInput) -> Tokens {
    let DeriveInput{
        ref ident,
        ref attrs,
        ref body,
        ref generics,
        ..
    } = *derive_input;

    // Process attributes on the item being derived
    let mut action_ty_opt = None;
    derin_attribute_iter(attrs, |attr| {
        match *attr {
            MetaItem::NameValue(ref ident, Lit::Str(ref string, _)) if ident == "action" =>
                if action_ty_opt.is_none() {
                    action_ty_opt = Some(syn::parse_type(&string).expect("Bad type in child action"));
                } else {
                    panic!("Repeated action attribute: {}", quote!(#attr).to_string())
                },
            _ => panic!("Bad Derin attribute: {}", quote!(#attr).to_string())
        }
    });

    let action_ty = action_ty_opt.expect("Missing #[derin(action = \"...\")] attribute");

    // Process attributes on the fields in the item being derived
    let mut widget_fields = Vec::new();
    match *body {
        Body::Struct(ref variant_data) =>
            for field in variant_data.fields().iter() {
                let mut widget_field = Some(WidgetField::Widget(field));
                derin_attribute_iter(&field.attrs, |attr| {
                    match *attr {
                        MetaItem::NameValue(ref attr_name, Lit::Str(ref collection_inner, _))
                            if attr_name == "collection" =>
                            if let Some(ref mut widget_field_ref) = widget_field {
                                match *widget_field_ref {
                                    WidgetField::Widget(_) => *widget_field_ref = WidgetField::Collection(field, syn::parse_type(collection_inner).expect("Malformed collection type")),
                                    WidgetField::Collection(_, _) => panic!("Repeated #[derin(collection)] attribute")
                                }
                            } else {
                                panic!("layout and collection field on same attribute")
                            },
                        _ => panic!("Bad Derin attribute: {}", quote!(#attr).to_string())
                    }
                });

                if let Some(widget_field) = widget_field {
                    widget_fields.push(widget_field);
                }
            },
        _ => unimplemented!()
    }

    // let parent_mut = parent_mut(derive_input, &action_ty, &widget_fields, &layout_ident);
    // let parent = parent(derive_input, &widget_fields, &layout_ident);

    let dummy_const = Ident::new(format!("_IMPL_PARENT_FOR_{}", ident));

    let generics_expanded = expand_generics(generics, &action_ty, &widget_fields);
    let (impl_generics, _, where_clause) = generics_expanded.split_for_impl();
    let (_, ty_generics, _) = generics.split_for_impl();

    let call_child_iter = CallChildIter {
        fields: widget_fields.iter().cloned(),
        field_num: 0,
        is_mut: false
    };

    let call_child_mut_iter = CallChildIter {
        fields: widget_fields.iter().cloned(),
        field_num: 0,
        is_mut: true
    };

    let num_children_iter = widget_fields.iter().cloned().enumerate().map(|(field_num, widget_field)| {
        let widget_ident = widget_field.ident().clone().unwrap_or(Ident::new(field_num));
        match widget_field {
            WidgetField::Widget(_) => quote!(+ 1),
            WidgetField::Collection(_, _) => quote!(+ (&self.#widget_ident).into_iter().count())
        }
    });

    quote!{
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        const #dummy_const: () = {
            extern crate derin as _derive_derin;

            impl #impl_generics _derive_derin::container::WidgetContainer<__F> for #ident #ty_generics #where_clause {
                type Action = #action_ty;

                #[inline]
                fn num_children(&self) -> usize {
                    0 #(#num_children_iter)*
                }

                #[allow(unused_assignments)]
                fn children<'a, __G, __R>(&'a self, mut for_each_child: __G) -> Option<__R>
                    where __G: FnMut(_derive_derin::widgets::custom::WidgetSummary<&'a _derive_derin::widgets::custom::Widget<Self::Action, __F>>) -> _derive_derin::LoopFlow<__R>,
                          Self::Action: 'a,
                          __F: 'a
                {
                    let mut index = 0;
                    #(#call_child_iter)*
                    None
                }

                #[allow(unused_assignments)]
                fn children_mut<'a, __G, __R>(&'a mut self, mut for_each_child: __G) -> Option<__R>
                    where __G: FnMut(_derive_derin::widgets::custom::WidgetSummary<&'a mut _derive_derin::widgets::custom::Widget<Self::Action, __F>>) -> _derive_derin::LoopFlow<__R>,
                          Self::Action: 'a,
                          __F: 'a
                {
                    let mut index = 0;
                    #(#call_child_mut_iter)*
                    None
                }
            }
        };
    }
}

struct CallChildIter<'a, W>
        where W: Iterator<Item = WidgetField<'a>>
{
    fields: W,
    field_num: u32,
    is_mut: bool
}

impl<'a, W> Iterator for CallChildIter<'a, W>
        where W: Iterator<Item = WidgetField<'a>>
{
    type Item = Tokens;

    fn next(&mut self) -> Option<Tokens> {
        if let Some(widget_field) = self.fields.next() {
            let widget_ident = widget_field.ident().clone().unwrap_or(Ident::new(self.field_num as usize));
            let widget_expr = match self.is_mut {
                true => quote!(&mut self.#widget_ident),
                false => quote!(&self.#widget_ident)
            };

            let output: Tokens;

            match widget_field {
                WidgetField::Widget(field) => {
                    let child_id = match field.ident {
                        Some(_) => quote!(_derive_derin::widgets::custom::WidgetIdent::Str(stringify!(#widget_ident))),
                        None => quote!(_derive_derin::widgets::custom::WidgetIdent::Num(#widget_ident))
                    };

                    output = quote!{{
                        let flow = for_each_child(_derive_derin::widgets::custom::WidgetSummary {
                            ident: #child_id,
                            rect: <_ as _derive_derin::widgets::custom::Widget<Self::Action, __F>>::rect(&self.#widget_ident),
                            size_bounds: <_ as _derive_derin::widgets::custom::Widget<Self::Action, __F>>::size_bounds(&self.#widget_ident),
                            update_tag: <_ as _derive_derin::widgets::custom::Widget<Self::Action, __F>>::update_tag(&self.#widget_ident).clone(),
                            widget: #widget_expr,
                            index
                        });
                        if let _derive_derin::LoopFlow::Break(b) = flow {
                            return Some(b);
                        }
                        index += 1;
                    }};
                },
                WidgetField::Collection(field, _) => {
                    let child_id = match field.ident {
                        Some(_) => quote!(_derive_derin::widgets::custom::WidgetIdent::StrCollection(stringify!(#widget_ident), child_index as u32)),
                        None => quote!(_derive_derin::widgets::custom::WidgetIdent::NumCollection(#widget_ident, child_index as u32))
                    };

                    output = quote!{{
                        for (child_index, child) in (#widget_expr).into_iter().enumerate() {
                            let flow = for_each_child(_derive_derin::widgets::custom::WidgetSummary {
                                ident: #child_id,
                                rect: <_ as _derive_derin::widgets::custom::Widget<Self::Action, __F>>::rect(child),
                                size_bounds: <_ as _derive_derin::widgets::custom::Widget<Self::Action, __F>>::size_bounds(child),
                                update_tag: <_ as _derive_derin::widgets::custom::Widget<Self::Action, __F>>::update_tag(child).clone(),
                                widget: child,
                                index
                            });
                            if let _derive_derin::LoopFlow::Break(b) = flow {
                                return Some(b);
                            }
                            index += 1;
                        }
                    }}
                }
            }

            self.field_num += 1;
            Some(output)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
enum WidgetField<'a> {
    Widget(&'a Field),
    Collection(&'a Field, Ty)
}

impl<'a> WidgetField<'a> {
    fn ident(&self) -> &'a Option<Ident> {
        match *self {
            WidgetField::Widget(field) |
            WidgetField::Collection(field, _) => &field.ident
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

fn expand_generics(generics: &Generics, action_ty: &Ty, widget_fields: &[WidgetField]) -> Generics {
    let mut generics = generics.clone();
    generics.ty_params.insert(0, TyParam {
        attrs: Vec::new(),
        ident: Ident::new("__F"),
        bounds: Vec::new(),
        default: None
    });

    let init_bound = WhereBoundPredicate {
        bound_lifetimes: Vec::new(),
        bounded_ty: syn::parse_type("__F").unwrap(),
        bounds: vec![TyParamBound::Trait(
            PolyTraitRef {
                bound_lifetimes: Vec::new(),
                trait_ref: syn::parse_path("_derive_derin::gl_render::RenderFrame").unwrap()
            },
            TraitBoundModifier::None
        )]
    };
    generics.where_clause.predicates.push(WherePredicate::BoundPredicate(init_bound));

    for ty in field_types(widget_fields.iter()) {
        let member_bound = WhereBoundPredicate {
            bound_lifetimes: Vec::new(),
            bounded_ty: ty,
            bounds: vec![TyParamBound::Trait(
                PolyTraitRef{
                    bound_lifetimes: Vec::new(),
                    trait_ref: syn::parse_path(&quote!(_derive_derin::widgets::custom::Widget<#action_ty, __F>).to_string()).unwrap(),
                },
                TraitBoundModifier::None
            )]
        };
        generics.where_clause.predicates.push(WherePredicate::BoundPredicate(member_bound));
    }


    generics
}

fn field_types<'a, I: 'a + Iterator<Item = &'a WidgetField<'a>>>(widget_fields: I) -> impl 'a + Iterator<Item=Ty> {
    widget_fields.map(|widget_field|
        match *widget_field {
            WidgetField::Widget(ref widget_field) => widget_field.ty.clone(),
            WidgetField::Collection(_, ref collection_ty) => collection_ty.clone()
        }
    )
}
