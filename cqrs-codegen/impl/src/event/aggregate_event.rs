//! Codegen for [`cqrs::AggregateEvent`].

use proc_macro2::TokenStream;
use quote::quote;
use syn::{spanned::Spanned as _, Error, Result};

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

    let type_name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        #[automatically_derived]
        impl#impl_generics ::cqrs::AggregateEvent for #type_name#ty_generics #where_clause {
            type Aggregate = #aggregate;
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
            enum Event {}
        };

        let output = quote! {
            #[automatically_derived]
            impl ::cqrs::AggregateEvent for Event {
                type Aggregate = Aggregate;
            }
        };

        assert_eq!(derive(input).unwrap().to_string(), output.to_string())
    }
}
