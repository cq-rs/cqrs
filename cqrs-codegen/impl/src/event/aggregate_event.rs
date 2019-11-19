//! Codegen for [`cqrs::AggregateEvent`].

use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{spanned::Spanned as _, Error, Result};
use synstructure::Structure;

use crate::util;

/// Name of the derived trait.
const TRAIT_NAME: &str = "AggregateEvent";

/// Implements [`crate::aggregate_event_derive`] macro expansion.
pub fn derive(input: syn::DeriveInput) -> Result<TokenStream> {
    util::derive(input, TRAIT_NAME, derive_struct, derive_enum)
}

/// Reports error if [`crate::aggregate_event_derive`] macro applied to structs.
fn derive_struct(input: syn::DeriveInput) -> Result<TokenStream> {
    match input.data {
        syn::Data::Struct(data) => Err(Error::new(
            data.struct_token.span(),
            format!("Structs are not supported for deriving {}", TRAIT_NAME),
        )),
        _ => unreachable!(),
    }
}

/// Implements [`crate::aggregate_event_derive`] macro expansion for enums
/// via [`synstructure`].
fn derive_enum(input: syn::DeriveInput) -> Result<TokenStream> {
    let meta = util::get_nested_meta(&input.attrs, super::ATTR_NAME)?;

    let aggregate = parse_event_aggregate_from_nested_meta(&meta)?;
    let aggregate: syn::Path = syn::parse_str(&aggregate)?;

    let structure = Structure::try_new(&input)?;
    super::assert_all_enum_variants_have_single_field(&structure, TRAIT_NAME)?;

    let type_params: HashSet<_> = input
        .generics
        .params
        .iter()
        .filter_map(|generic_param| match generic_param {
            syn::GenericParam::Type(type_param) => Some(&type_param.ident),
            _ => None,
        })
        .collect();

    let iter = structure
        .variants()
        .iter()
        .map(|variant| variant.ast().fields.iter())
        .flatten();

    let mut event_types = Vec::new();
    for field in iter {
        let mut path = match &field.ty {
            syn::Type::Path(path) => path.path.clone(),
            _ => {
                return Err(Error::new(
                    field.span(),
                    "AggregateEvent can only be derived for enums \
                     with variants containing owned scalar data",
                ))
            }
        };

        // type-path cannot ever be empty, unless there is an error in syn
        let first_segment = path.segments.first().unwrap();

        if type_params.contains(&first_segment.ident) {
            return Err(Error::new(
                first_segment.ident.span(),
                "Type parameters are not allowed here, as they cannot have \
                 associated constants (but generic types dependent on generic \
                 type parameters, e.g., 'Event<T>', are fine)",
            ));
        }

        // type-path cannot ever be empty, unless there is an error in syn
        let last_segment = path.segments.last_mut().unwrap();

        if let syn::PathArguments::AngleBracketed(args) = &mut last_segment.arguments {
            args.colon2_token = Some(Default::default());
        }

        path.segments.push(syn::parse2(quote!(EVENT_TYPE))?);

        event_types.push(quote!(#path));
    }

    let const_len = event_types.len();
    let const_doc = format!("Type names of [`{}`] aggregate events.", input.ident);

    let type_name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        #[automatically_derived]
        impl#impl_generics #type_name#ty_generics #where_clause {
            #[doc = #const_doc]
            pub const EVENT_TYPES: [::cqrs::EventType; #const_len] = [#(#event_types),*];
        }

        #[automatically_derived]
        impl#impl_generics ::cqrs::AggregateEvent for #type_name#ty_generics #where_clause {
            type Aggregate = #aggregate;

            #[inline(always)]
            fn event_types() -> &'static [cqrs::EventType] {
                &Self::EVENT_TYPES
            }
        }
    })
}

/// Parses aggregate of [`cqrs::AggregateEvent`] from `#[event(...)]` attribute.
fn parse_event_aggregate_from_nested_meta(meta: &util::Meta) -> Result<String> {
    let lit: &syn::LitStr = util::parse_lit(
        meta,
        "aggregate",
        super::VALID_ENUM_ARGS,
        super::ATTR_NAME,
        "= \"...\"",
    )?;

    Ok(lit.value())
}

#[cfg(test)]
mod spec {
    use super::*;

    #[test]
    fn derives_enum_impl() {
        let input = syn::parse_quote! {
            #[event(aggregate = "Aggregate")]
            enum Event {
                MyEvent(MyEvent),
                HisEvent(HisEvent),
                HerEvent(HerEvent),
            }
        };

        let output = quote! {
            #[automatically_derived]
            impl Event {
                #[doc = "Type names of [`Event`] aggregate events."]
                pub const EVENT_TYPES: [::cqrs::EventType; 3usize] = [
                    MyEvent::EVENT_TYPE, HisEvent::EVENT_TYPE, HerEvent::EVENT_TYPE
                ];
            }

            #[automatically_derived]
            impl ::cqrs::AggregateEvent for Event {
                type Aggregate = Aggregate;

                #[inline(always)]
                fn event_types() -> &'static [cqrs::EventType] {
                    &Self::EVENT_TYPES
                }
            }
        };

        assert_eq!(derive(input).unwrap().to_string(), output.to_string())
    }
}
