//! Codegen for [`cqrs::EventSourced`].

use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{punctuated::Punctuated, spanned::Spanned as _, Error, Result};
use synstructure::Structure;

use crate::util;

/// Name of the derived trait.
const TRAIT_NAME: &str = "EventSourced";

/// Name of the attribute, used by [`cqrs::EventSourced`].
const ATTR_NAME: &str = "event_sourced";

/// Implements [`crate::event_sourced_derive`] macro expansion.
pub fn derive(input: syn::DeriveInput) -> Result<TokenStream> {
    util::derive(input, TRAIT_NAME, derive_struct, derive_enum)
}

/// Reports error if [`crate::event_sourced_derive`] macro applied to structs.
fn derive_struct(input: syn::DeriveInput) -> Result<TokenStream> {
    match input.data {
        syn::Data::Struct(data) => Err(Error::new(
            data.struct_token.span(),
            format!("Structs are not supported for deriving {}", TRAIT_NAME),
        )),
        _ => unreachable!(),
    }
}

/// Implements [`crate::event_sourced_derive`] macro expansion for enums
/// via [`synstructure`].
fn derive_enum(input: syn::DeriveInput) -> Result<TokenStream> {
    let meta = util::get_nested_meta(&input.attrs, ATTR_NAME)?;

    let aggregate = parse_event_sourced_aggregate(&meta)?;
    let aggregate: syn::Path = syn::parse_str(&aggregate)?;

    let mut structure = Structure::try_new(&input)?;
    util::assert_all_enum_variants_have_single_field(&structure, TRAIT_NAME)?;

    structure.binding_name(|field, _| {
        field.ident.as_ref().map_or_else(
            || syn::Ident::new("ev", proc_macro2::Span::call_site()),
            |ident| ident.clone(),
        )
    });

    let body = structure.each(|info| {
        let ev = &info.binding;
        quote!(self.apply(#ev))
    });

    let type_name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let type_params: HashSet<_> = input
        .generics
        .params
        .iter()
        .filter_map(|p| match p {
            syn::GenericParam::Type(tp) => Some(&tp.ident),
            _ => None,
        })
        .collect();

    let iter = structure
        .variants()
        .iter()
        .map(|v| v.ast().fields.iter())
        .flatten();

    let mut where_clause = where_clause.cloned();

    for field in iter {
        if let Some(ty) = util::get_type_if_type_param_used_in_type(&type_params, &field.ty) {
            let predicates = &mut where_clause
                .get_or_insert_with(|| syn::WhereClause {
                    where_token: <syn::Token![where]>::default(),
                    predicates: Punctuated::new(),
                })
                .predicates;

            predicates.push(syn::parse2(quote!(#aggregate: ::cqrs::EventSourced<#ty>))?);
            predicates.push(syn::parse2(quote!(#ty: ::cqrs::Event))?);
        }
    }

    Ok(quote! {
        #[automatically_derived]
        impl#impl_generics ::cqrs::EventSourced<#type_name#ty_generics> for #aggregate #where_clause {
            fn apply(&mut self, ev: &#type_name#ty_generics) {
                match *ev {
                    #body
                }
            }
        }
    })
}

/// Parses aggregate to be [`cqrs::EventSourced`] from `#[event_sourced(...)]`
/// attribute.
fn parse_event_sourced_aggregate(meta: &util::Meta) -> Result<String> {
    let lit: &syn::LitStr =
        util::parse_lit(meta, "aggregate", &["aggregate"], ATTR_NAME, "= \"...\"")?;
    Ok(lit.value())
}

#[cfg(test)]
mod spec {
    use super::*;

    #[test]
    fn derives_enum_impl() {
        let input = syn::parse_quote! {
            #[event_sourced(aggregate = "Aggregate")]
            enum Event {
                Event1(Event1),
                Event2 {
                    other_event: Event2,
                },
            }
        };

        let output = quote! {
            #[automatically_derived]
            impl ::cqrs::EventSourced<Event> for Aggregate {
                fn apply(&mut self, ev: &Event) {
                    match *ev {
                        Event::Event1(ref ev,) => {{ self.apply(ev) }}
                        Event::Event2{other_event: ref other_event,} => {{ self.apply(other_event) }}
                    }
                }
            }
        };

        assert_eq!(derive(input).unwrap().to_string(), output.to_string())
    }
}
