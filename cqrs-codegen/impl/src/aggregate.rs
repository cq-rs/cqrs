//! Codegen for [`cqrs::Aggregate`].

use proc_macro2::TokenStream;
use quote::quote;
use syn::{spanned::Spanned as _, Error, Result};

use crate::util;

/// Name of the derived trait.
const TRAIT_NAME: &str = "Aggregate";

/// Name of the attribute, used by [`cqrs::Aggregate`].
const ATTR_NAME: &str = "aggregate";

/// Implements [`crate::aggregate_derive`] macro expansion.
pub fn derive(input: syn::DeriveInput) -> Result<TokenStream> {
    util::derive(input, TRAIT_NAME, derive_struct, derive_enum)
}

/// Implements [`crate::aggregate_derive`] macro expansion for structs.
fn derive_struct(input: syn::DeriveInput) -> Result<TokenStream> {
    let meta = util::get_nested_meta(&input.attrs, ATTR_NAME)?;

    let const_val = parse_aggregate_type(&meta)?;
    let const_doc = format!("Type name of [`{}`] aggregate", input.ident);
    let additional = quote! {
        #[doc = #const_doc]
        pub const AGGREGATE_TYPE: ::cqrs::AggregateType = #const_val;
    };

    let data = match &input.data {
        syn::Data::Struct(data) => data,
        _ => unreachable!(),
    };

    let (id_type, id_field) = get_id_field(&data.fields)?;

    let body = quote! {
        type Id = #id_type;

        #[inline(always)]
        fn aggregate_type(&self) -> ::cqrs::AggregateType {
            Self::AGGREGATE_TYPE
        }

        #[inline(always)]
        fn id(&self) -> &Self::Id {
            &self.#id_field
        }
    };

    util::render_struct(&input, quote!(::cqrs::Aggregate), body, Some(additional))
}

/// Reports error if [`crate::aggregate_derive`] macro applied to enums.
fn derive_enum(input: syn::DeriveInput) -> Result<TokenStream> {
    match input.data {
        syn::Data::Enum(data) => Err(Error::new(
            data.enum_token.span(),
            format!("Structs are not supported for deriving {}", TRAIT_NAME),
        )),
        _ => unreachable!(),
    }
}

/// Parses type of [`cqrs::Aggregate`] from `#[aggregate(...)]` attribute.
fn parse_aggregate_type(meta: &util::Meta) -> Result<String> {
    let lit: &syn::LitStr = util::parse_lit(meta, "type", &["type"], ATTR_NAME, "= \"...\"")?;

    Ok(lit.value())
}

/// Infers or finds via `#[aggregate(id)]` attribute an `id` field
/// of this aggregate.
fn get_id_field(fields: &syn::Fields) -> Result<(&syn::Type, TokenStream)> {
    let mut id = None;

    for (index, field) in fields.iter().enumerate() {
        let meta = util::find_nested_meta(&field.attrs, ATTR_NAME)?;

        let meta = match meta {
            Some(meta) => meta,
            None => continue,
        };

        if util::parse_flag(&meta, "id", &["id"], ATTR_NAME)? {
            let span = field.span();
            if id.replace((index, field)).is_some() {
                return Err(Error::new(
                    span,
                    "Multiple fields marked with '#[aggregate(id)]' attribute; \
                     only single '#[aggregate(id)]' attribute allowed \
                     per struct",
                ));
            }
        }
    }

    let is_named = match fields {
        syn::Fields::Named(_) => true,
        _ => false,
    };

    if id.is_none() && is_named {
        id = fields.iter().enumerate().find(|(_, f)| match &f.ident {
            Some(ident) => ident == "id",
            None => false,
        });
    }

    id.map(|(index, field)| {
        let ty = &field.ty;
        let field = if is_named {
            // Named fields always have ident, so unwrapping is OK here.
            let ident = &field.ident.as_ref().unwrap();
            quote!(#ident)
        } else {
            let index = syn::Index::from(index);
            quote!(#index)
        };
        (ty, field)
    })
    .ok_or_else(|| Error::new(fields.span(), "No 'id' field found for an aggregate"))
}

#[cfg(test)]
mod spec {
    use super::*;

    #[test]
    fn derives_struct_impl() {
        let input = syn::parse_quote! {
            #[aggregate(type = "aggregate")]
            struct Aggregate {
                id: AggregateId,
                field: i32,
            }
        };

        let output = quote! {
            #[automatically_derived]
            impl Aggregate {
                #[doc = "Type name of [`Aggregate`] aggregate"]
                pub const AGGREGATE_TYPE: ::cqrs::AggregateType = "aggregate";
            }

            #[automatically_derived]
            impl ::cqrs::Aggregate for Aggregate {
                type Id = AggregateId;

                #[inline(always)]
                fn aggregate_type(&self) -> ::cqrs::AggregateType {
                    Self::AGGREGATE_TYPE
                }

                #[inline(always)]
                fn id(&self) -> &Self::Id {
                    &self.id
                }
            }
        };

        assert_eq!(derive(input).unwrap().to_string(), output.to_string());
    }
}
