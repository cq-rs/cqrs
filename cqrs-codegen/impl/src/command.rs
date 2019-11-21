//! Codegen for [`cqrs::Command`].

use proc_macro2::TokenStream;
use quote::quote;
use syn::{spanned::Spanned as _, Error, Result};

use crate::util;

/// Name of the derived trait.
const TRAIT_NAME: &str = "Command";

/// Name of the attribute, used by [`cqrs::Command`].
const ATTR_NAME: &str = "command";

/// Names of the `#[command(...)]` attribute's arguments, used on struct fields
/// by [`cqrs::Command`].
const VALID_ARGS: &[&str] = &["id", "version"];

/// Implements [`crate::command_derive`] macro expansion.
pub fn derive(input: syn::DeriveInput) -> Result<TokenStream> {
    util::derive(input, TRAIT_NAME, derive_struct, derive_enum)
}

/// Implements [`crate::command_derive`] macro expansion for structs.
fn derive_struct(input: syn::DeriveInput) -> Result<TokenStream> {
    let meta = util::get_nested_meta(&input.attrs, ATTR_NAME)?;

    let aggregate = parse_command_aggregate(&meta)?;
    let aggregate: syn::Path = syn::parse_str(&aggregate)?;

    let data = match &input.data {
        syn::Data::Struct(data) => data,
        _ => unreachable!(),
    };

    let id = find_field_with_flag(&data.fields, "id")?.map(|id| {
        quote! {
            #[inline(always)]
            fn aggregate_id(&self) -> Option<&<Self::Aggregate as ::cqrs::Aggregate>::Id> {
                Some(&self.#id)
            }
        }
    });

    let ver = find_field_with_flag(&data.fields, "version")?.map(|ver| {
        quote! {
            #[inline(always)]
            fn expected_version(&self) -> Option<::cqrs::Version> {
                Some(self.#ver)
            }
        }
    });

    let body = quote! {
        type Aggregate = #aggregate;

        #id

        #ver
    };

    util::render_struct(&input, quote!(::cqrs::Command), body, None)
}

/// Reports error if [`crate::command_derive`] macro applied to enums.
fn derive_enum(input: syn::DeriveInput) -> Result<TokenStream> {
    match input.data {
        syn::Data::Enum(data) => Err(Error::new(
            data.enum_token.span(),
            format!("Structs are not supported for deriving {}", TRAIT_NAME),
        )),
        _ => unreachable!(),
    }
}

/// Parses aggregate of [`cqrs::Command`] from `#[command(...)]` attribute.
fn parse_command_aggregate(meta: &util::Meta) -> Result<String> {
    let lit: &syn::LitStr =
        util::parse_lit(meta, "aggregate", &["aggregate"], ATTR_NAME, "= \"...\"")?;

    Ok(lit.value())
}

/// Finds field marked with `flag` argument inside [`ATTR_NAME`] attribute.
fn find_field_with_flag(fields: &syn::Fields, flag: &str) -> Result<Option<TokenStream>> {
    util::find_field_with_flag(fields, ATTR_NAME, flag, VALID_ARGS)
        .map(|opt| opt.map(|(idx, fld)| util::render_field_ident(idx, fld)))
}

#[cfg(test)]
mod spec {
    use super::*;

    #[test]
    fn derives_struct_impl() {
        let input = syn::parse_quote! {
            #[command(aggregate = "Aggregate")]
            struct Command {
                #[command(id)]
                id: AggregateId,
                #[command(version)]
                version: i32,
            }
        };

        let output = quote! {
            #[automatically_derived]
            impl ::cqrs::Command for Command {
                type Aggregate = Aggregate;

                #[inline(always)]
                fn aggregate_id(&self) -> Option<&<Self::Aggregate as ::cqrs::Aggregate>::Id> {
                    Some(&self.id)
                }

                #[inline(always)]
                fn expected_version(&self) -> Option<::cqrs::Version> {
                    Some(self.version)
                }
            }
        };

        assert_eq!(derive(input).unwrap().to_string(), output.to_string());
    }
}
