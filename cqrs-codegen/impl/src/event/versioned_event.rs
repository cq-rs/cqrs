//! Codegen for [`cqrs::VersionedEvent`].

use std::num::NonZeroU8;

use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;
use synstructure::Structure;

use crate::util;

/// Name of the derived trait.
const TRAIT_NAME: &str = "VersionedEvent";

/// Implements [`crate::versioned_event_derive`] macro expansion.
pub fn derive(input: syn::DeriveInput) -> Result<TokenStream> {
    util::derive(input, TRAIT_NAME, derive_struct, derive_enum)
}

/// Implements [`crate::versioned_event_derive`] macro expansion for structs.
fn derive_struct(input: syn::DeriveInput) -> Result<TokenStream> {
    let meta = util::get_nested_meta(&input.attrs, super::ATTR_NAME)?;

    let const_val = parse_event_version_from_nested_meta(&meta)?;
    let const_doc = format!("Version of [`{}`] event", input.ident);
    let additional = quote! {
        #[doc = #const_doc]
        #[allow(unsafe_code)]
        pub const EVENT_VERSION: ::cqrs::EventVersion =
            unsafe { ::cqrs::EventVersion::new_unchecked(#const_val) };
    };

    let body = quote! {
        #[inline(always)]
        fn event_version(&self) -> &'static ::cqrs::EventVersion {
            &Self::EVENT_VERSION
        }
    };

    util::render_struct(
        &input,
        quote!(::cqrs::VersionedEvent),
        body,
        Some(additional),
    )
}

/// Implements [`crate::versioned_event_derive`] macro expansion for enums
/// via [`synstructure`].
fn derive_enum(input: syn::DeriveInput) -> Result<TokenStream> {
    util::assert_valid_attr_args_used(&input.attrs, super::ATTR_NAME, super::VALID_ENUM_ARGS)?;

    let mut structure = Structure::try_new(&input)?;

    super::render_enum_proxy_method_calls(
        &mut structure,
        TRAIT_NAME,
        quote!(::cqrs::VersionedEvent),
        quote!(event_version),
        quote!(&'static ::cqrs::EventVersion),
    )
}

/// Parses version of [`cqrs::Event`] from `#[event(...)]` attribute.
fn parse_event_version_from_nested_meta(meta: &util::Meta) -> Result<u8> {
    let lit: &syn::LitInt = util::parse_lit(
        meta,
        "version",
        super::VALID_STRUCT_ARGS,
        super::ATTR_NAME,
        "= <non-zero unsigned integer>",
    )?;
    Ok(lit.base10_parse::<NonZeroU8>()?.get())
}

#[cfg(test)]
mod spec {
    use super::*;

    #[test]
    fn derives_struct_impl() {
        let input = syn::parse_quote! {
            #[event(version = 1)]
            struct Event;
        };

        let output = quote! {
            #[automatically_derived]
            impl Event {
                #[doc = "Version of [`Event`] event"]
                #[allow(unsafe_code)]
                pub const EVENT_VERSION: ::cqrs::EventVersion =
                    unsafe { ::cqrs::EventVersion::new_unchecked(1u8) };
            }

            #[automatically_derived]
            impl ::cqrs::VersionedEvent for Event {
                #[inline(always)]
                fn event_version(&self) -> &'static ::cqrs::EventVersion {
                    &Self::EVENT_VERSION
                }
            }
        };

        assert_eq!(derive(input).unwrap().to_string(), output.to_string())
    }

    #[test]
    fn derives_enum_impl() {
        let input = syn::parse_quote! {
            enum Event {
                Event1(Event1),
                Event2 {
                    other_event: Event2,
                },
            }
        };

        let output = quote! {
            #[allow(non_upper_case_globals)]
            const _DERIVE_cqrs_VersionedEvent_FOR_Event: () = {
                #[automatically_derived]
                impl ::cqrs::VersionedEvent for Event {
                    fn event_version(&self) -> &'static ::cqrs::EventVersion {
                        match *self {
                            Event::Event1(ref ev,) => {{ ev.event_version() }}
                            Event::Event2{other_event: ref other_event,} => {{ other_event.event_version() }}
                        }
                    }
                }
            };
        };

        assert_eq!(derive(input).unwrap().to_string(), output.to_string())
    }
}
