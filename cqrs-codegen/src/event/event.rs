//! Codegen for [`cqrs::Event`].

use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;
use synstructure::Structure;

use crate::util;

/// Name of the derived trait.
const TRAIT_NAME: &str = "Event";

/// Implements [`crate::derive_event`] macro expansion.
pub fn derive(input: syn::DeriveInput) -> Result<TokenStream> {
    util::derive(input, TRAIT_NAME, derive_struct, derive_enum)
}

/// Implements [`crate::derive_event`] macro expansion for structs.
fn derive_struct(input: syn::DeriveInput) -> Result<TokenStream> {
    let meta = util::get_nested_meta(&input.attrs, super::ATTR_NAME)?;

    let const_val = parse_event_type_from_nested_meta(&meta)?;
    let const_doc = format!("Type name of [`{}`] event", input.ident);
    let additional = quote! {
        #[doc = #const_doc]
        pub const EVENT_TYPE: ::cqrs::EventType = #const_val;
    };

    let body = quote! {
        #[inline(always)]
        fn event_type(&self) -> ::cqrs::EventType {
            Self::EVENT_TYPE
        }
    };

    super::render_struct(&input, quote!(::cqrs::Event), body, Some(additional))
}

/// Implements [`crate::derive_event`] macro expansion for enums
/// via [`synstructure`].
fn derive_enum(input: syn::DeriveInput) -> Result<TokenStream> {
    util::assert_attr_does_not_exist(&input.attrs, super::ATTR_NAME)?;

    let mut structure = Structure::try_new(&input)?;

    super::render_enum_proxy_method_calls(
        &mut structure,
        TRAIT_NAME,
        quote!(::cqrs::Event),
        quote!(event_type),
        quote!(::cqrs::EventType),
    )
}

/// Parses type of [`cqrs::Event`] from `#[event(...)]` attribute.
fn parse_event_type_from_nested_meta(meta: &util::Meta) -> Result<String> {
    let lit: &syn::LitStr = super::parse_attr_from_nested_meta(meta, "type", "type = \"...\"")?;
    Ok(lit.value())
}

#[cfg(test)]
mod spec {
    use super::*;

    #[test]
    fn derives_struct_impl() {
        let input = syn::parse_quote! {
            #[event(type = "event")]
            struct Event;
        };

        let output = quote! {
            #[automatically_derived]
            impl Event {
                #[doc = "Type name of [`Event`] event"]
                pub const EVENT_TYPE: ::cqrs::EventType = "event";
            }

            #[automatically_derived]
            impl ::cqrs::Event for Event {
                #[inline(always)]
                fn event_type(&self) -> ::cqrs::EventType {
                    Self::EVENT_TYPE
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
            const _DERIVE_cqrs_Event_FOR_Event: () = {
                #[automatically_derived]
                impl ::cqrs::Event for Event {
                    fn event_type(&self) -> ::cqrs::EventType {
                        match *self {
                            Event::Event1(ref ev,) => {{ ev.event_type() }}
                            Event::Event2{other_event: ref other_event,} => {{ other_event.event_type() }}
                        }
                    }
                }
            };
        };

        assert_eq!(derive(input).unwrap().to_string(), output.to_string())
    }
}
