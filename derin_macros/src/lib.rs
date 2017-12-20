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

#[proc_macro_derive(NodeContainer, attributes(derin))]
pub fn derive_node_container(input_tokens: TokenStream) -> TokenStream {
    let input = input_tokens.to_string();
    let item = syn::parse_derive_input(&input).expect("Attempted derive on non-item");

    let output = impl_node_container(&item).parse().unwrap();
    output
}

fn impl_node_container(derive_input: &DeriveInput) -> Tokens {
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
                        MetaItem::Word(ref attr_name)
                            if attr_name == "collection" =>
                            if let Some(ref mut widget_field_ref) = widget_field {
                                match *widget_field_ref {
                                    WidgetField::Widget(_) => *widget_field_ref = WidgetField::Collection(field),
                                    WidgetField::Collection(_) => panic!("Repeated #[derin(collection)] attribute")
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

    quote!{
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        const #dummy_const: () = {
            extern crate derin as _derive_derin;

            impl #impl_generics _derive_derin::NodeContainer<__F> for #ident #ty_generics #where_clause {
                type Action = #action_ty;

                fn children<'a, __G, __R>(&'a self, mut for_each_child: __G) -> Option<__R>
                    where __G: FnMut(_derive_derin::core::tree::NodeSummary<&'a _derive_derin::core::tree::Node<Self::Action, __F>>) -> _derive_derin::core::LoopFlow<__R>,
                          Self::Action: 'a,
                          __F: 'a
                {
                    #(#call_child_iter)*
                    None
                }

                fn children_mut<'a, __G, __R>(&'a mut self, mut for_each_child: __G) -> Option<__R>
                    where __G: FnMut(_derive_derin::core::tree::NodeSummary<&'a mut _derive_derin::core::tree::Node<Self::Action, __F>>) -> _derive_derin::core::LoopFlow<__R>,
                          Self::Action: 'a,
                          __F: 'a
                {
                    #(#call_child_mut_iter)*
                    None
                }
            }
        };
    }

    // quote!{
    //     #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    //     const #dummy_const: () = {
    //         extern crate derin as _derive_derin;

    //         #parent_mut
    //         #parent
    //     };
    // }
}

// fn parent_mut(derive_input: &DeriveInput, action_ty: &Ty, widget_fields: &[WidgetField], layout_ident: &Ident) -> Tokens {
//     let &DeriveInput{
//         ref ident,
//         ref generics,
//         ..
//     } = derive_input;

//     let generics_raw = expand_generics(generics, widget_fields, |ty_string| format!("_derive_derin::ui::NodeProcessorGridMut<{}>", ty_string));
//     let (impl_generics_mut, _, where_clause_mut) = generics_raw.split_for_impl();
//     let (_, ty_generics, _) = generics.split_for_impl();
//     let add_child_mut_iter = AddChildIter {
//         layout_ident: layout_ident.clone(),
//         fields: widget_fields.iter().cloned(),
//         field_num: 0,
//         is_mut: true
//     };

//     quote!{
//         impl #impl_generics_mut ParentMut<NPI> for #ident #ty_generics #where_clause_mut {
//             type ChildAction = #action_ty;

//             fn children_mut(&mut self, npi: NPI) -> Result<(), NPI::Error> {
//                 let mut np = npi.init_grid(
//                     self.#layout_ident.grid_size(),
//                     self.#layout_ident.col_hints(),
//                     self.#layout_ident.row_hints()
//                 );

//                 #(#add_child_mut_iter)*
//                 Ok(())
//             }
//         }
//     }
// }

// fn parent(derive_input: &DeriveInput, widget_fields: &[WidgetField], layout_ident: &Ident) -> Tokens {
//     let &DeriveInput{
//         ref ident,
//         ref generics,
//         ..
//     } = derive_input;

//     let generics_raw = expand_generics(generics, widget_fields, |ty_string| format!("_derive_derin::ui::NodeProcessorGrid<{}>", ty_string));
//     let (impl_generics, _, where_clause) = generics_raw.split_for_impl();
//     let (_, ty_generics, _) = generics.split_for_impl();
//     let add_child_iter = AddChildIter {
//         layout_ident: layout_ident.clone(),
//         fields: widget_fields.iter().cloned(),
//         field_num: 0,
//         is_mut: false
//     };

//     quote!{
//         impl #impl_generics Parent<NPI> for #ident #ty_generics #where_clause {

//             fn children(&self, npi: NPI) -> Result<(), NPI::Error> {
//                 let mut np = npi.init_grid(
//                     self.#layout_ident.grid_size(),
//                     self.#layout_ident.col_hints(),
//                     self.#layout_ident.row_hints()
//                 );

//                 #(#add_child_iter)*
//                 Ok(())
//             }
//         }
//     }
// }

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
                        Some(_) => quote!(_derive_derin::core::tree::NodeIdent::Str(stringify!(#widget_ident))),
                        None => quote!(_derive_derin::core::tree::NodeIdent::Num(#widget_ident))
                    };

                    output = quote!{{
                        let flow = for_each_child(_derive_derin::core::tree::NodeSummary {
                            ident: #child_id,
                            rect: <_ as _derive_derin::core::tree::Node<Self::Action, __F>>::bounds(&self.#widget_ident),
                            update_tag: <_ as _derive_derin::core::tree::Node<Self::Action, __F>>::update_tag(&self.#widget_ident).clone(),
                            node: #widget_expr
                        });
                        if let _derive_derin::core::LoopFlow::Break(b) = flow {
                            return Some(b);
                        }
                    }};
                },
                WidgetField::Collection(field) => {
                    let child_id = match field.ident {
                        Some(_) => quote!(_derive_derin::core::tree::NodeIdent::StrCollection(stringify!(#widget_ident), child_index as u32)),
                        None => quote!(_derive_derin::core::tree::NodeIdent::NumCollection(#widget_ident, child_index as u32))
                    };

                    output = quote!{{
                        for (child_index, child) in (#widget_expr).into_iter().enumerate() {
                            let flow = for_each_child(NodeSummary {
                                ident: #child_id,
                                rect: <_ as Node<Self::Action, __F>>::bounds(child),
                                update_tag: <_ as Node<Self::Action, __F>>::update_tag(child).clone(),
                                node: child
                            });
                            if let _derive_derin::core::LoopFlow::Break(b) = flow {
                                return Some(b);
                            }
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

#[derive(Debug, Clone, Copy)]
enum WidgetField<'a> {
    Widget(&'a Field),
    Collection(&'a Field)
}

impl<'a> WidgetField<'a> {
    fn ident(self) -> &'a Option<Ident> {
        match self {
            WidgetField::Widget(field) |
            WidgetField::Collection(field) => &field.ident
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
                trait_ref: syn::parse_path("_derive_derin::core::render::RenderFrame").unwrap()
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
                    trait_ref: syn::parse_path(&quote!(_derive_derin::core::tree::Node<#action_ty, __F>).to_string()).unwrap(),
                },
                TraitBoundModifier::None
            )]
        };
        generics.where_clause.predicates.push(WherePredicate::BoundPredicate(member_bound));
    }

    // let never_bound = WhereBoundPredicate {
    //     bound_lifetimes: Vec::new(),
    //     bounded_ty: npi_gridproc_ty.clone(),
    //     bounds: vec![TyParamBound::Trait(
    //         PolyTraitRef{
    //             bound_lifetimes: Vec::new(),
    //             trait_ref: syn::parse_path(&trait_fn("!")).unwrap()
    //         },
    //         TraitBoundModifier::None
    //     )]
    // };
    // generics.where_clause.predicates.push(WherePredicate::BoundPredicate(never_bound));


    generics
}

fn field_types<'a, I: 'a + Iterator<Item = &'a WidgetField<'a>>>(widget_fields: I) -> impl 'a + Iterator<Item=Ty> {
    widget_fields.map(|widget_field|
        match *widget_field {
            WidgetField::Widget(ref widget_field) => widget_field.ty.clone(),
            WidgetField::Collection(&Field{ref ty, ..}) =>
                syn::parse_type(&format!("{}", quote!(<#ty as IntoIterator>::Item))).unwrap()
        }
    )
}
