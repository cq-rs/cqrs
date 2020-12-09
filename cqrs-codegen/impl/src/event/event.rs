//! Codegen for [`cqrs::Event`].

use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;
use synstructure::Structure;

use crate::{event::typed_event, util};

/// Name of the derived trait.
const TRAIT_NAME: &str = "Event";

/// Implements [`crate::event_derive`] macro expansion.
pub fn derive(input: syn::DeriveInput) -> Result<TokenStream> {
    let mut s = util::derive(input.clone(), TRAIT_NAME, derive_struct, derive_enum)?;
    s.extend(typed_event::derive(input)?);
    Ok(s)
}

/// Implements [`crate::event_derive`] macro expansion for structs.
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

    util::render_struct(&input, quote!(::cqrs::Event), body, Some(additional))
}

/// Implements [`crate::event_derive`] macro expansion for enums
/// via [`synstructure`].
fn derive_enum(input: syn::DeriveInput) -> Result<TokenStream> {
    util::assert_valid_attr_args_used(&input.attrs, super::ATTR_NAME, super::VALID_ENUM_ARGS)?;

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
    let lit: &syn::LitStr = util::parse_lit(
        meta,
        "type",
        super::VALID_STRUCT_ARGS,
        super::ATTR_NAME,
        "= \"...\"",
    )?;
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
            #[automatically_derived]
            impl ::cqrs::TypedEvent for Event {
                type EventTypes = std::iter::Once<::cqrs::EventType>;

                #[inline(always)]
                fn event_types() -> Self::EventTypes {
                    std::iter::once(Self::EVENT_TYPE)
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
            #[automatically_derived]
            impl ::cqrs::TypedEvent for Event {
                type EventTypes = std::iter::Chain<
                    <Event1 as ::cqrs::TypedEvent>::EventTypes,
                    <Event2 as ::cqrs::TypedEvent>::EventTypes
                >;

                #[inline(always)]
                fn event_types() -> Self::EventTypes {
                    Event1::event_types()
                        .chain(Event2::event_types())
                }
            }
        };

        assert_eq!(derive(input).unwrap().to_string(), output.to_string())
    }
}
