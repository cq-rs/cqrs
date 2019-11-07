//! Codegen for [`cqrs::RegisteredEvent`].

use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;
use synstructure::Structure;

use crate::util;

/// Name of the derived trait.
const TRAIT_NAME: &str = "RegisteredEvent";

/// Implements [`crate::derive_registered_event`] macro expansion.
pub fn derive(input: syn::DeriveInput) -> Result<TokenStream> {
    util::derive(input, TRAIT_NAME, derive_struct, derive_enum)
}

/// Implements [`crate::derive_registered_event`] macro expansion for structs.
fn derive_struct(input: syn::DeriveInput) -> Result<proc_macro2::TokenStream> {
    let body = quote! {
        #[inline(always)]
        fn type_id(&self) -> ::core::any::TypeId {
            ::core::any::TypeId::of::<Self>()
        }
    };

    super::render_struct(&input, quote!(::cqrs::RegisteredEvent), body, None)
}

/// Implements [`crate::derive_registered_event`] macro expansion for enums
/// via [`synstructure`].
fn derive_enum(input: syn::DeriveInput) -> Result<proc_macro2::TokenStream> {
    util::assert_attr_does_not_exist(&input.attrs, super::ATTR_NAME)?;

    let mut structure = Structure::try_new(&input)?;

    super::render_enum_proxy_method_calls(
        &mut structure,
        TRAIT_NAME,
        quote!(::cqrs::RegisteredEvent),
        quote!(type_id),
        quote!(::core::any::TypeId),
    )
}

#[cfg(test)]
mod spec {
    use super::*;

    #[test]
    fn derives_struct_impl() {
        let input = syn::parse_quote! {
            struct Event;
        };

        let output = quote! {
            #[automatically_derived]
            impl ::cqrs::RegisteredEvent for Event {
                #[inline(always)]
                fn type_id (&self) -> ::core::any::TypeId {
                    ::core::any::TypeId::of::<Self>()
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
            const _DERIVE_cqrs_RegisteredEvent_FOR_Event: () = {
                #[automatically_derived]
                impl ::cqrs::RegisteredEvent for Event {
                    fn type_id(&self) -> ::core::any::TypeId {
                        match *self {
                            Event::Event1(ref ev,) => {{ ev.type_id() }}
                            Event::Event2{other_event: ref other_event,} => {{ other_event.type_id() }}
                        }
                    }
                }
            };
        };

        assert_eq!(derive(input).unwrap().to_string(), output.to_string())
    }
}
