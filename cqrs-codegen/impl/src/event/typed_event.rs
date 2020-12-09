//! Codegen for [`cqrs::TypedEvent`].

use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{spanned::Spanned as _, Error, Result, WhereClause};
use synstructure::Structure;

use crate::util;

/// Name of the derived trait.
const TRAIT_NAME: &str = "TypedEvent";

/// Implements `cqrs::TypedEvent` part of [`crate::event_derive`] macro
/// expansion.
pub fn derive(input: syn::DeriveInput) -> Result<TokenStream> {
    util::derive(input, TRAIT_NAME, derive_struct, derive_enum)
}

/// Implements `cqrs::TypedEvent` part of [`crate::event_derive`] macro
/// expansion for structs.
fn derive_struct(input: syn::DeriveInput) -> Result<TokenStream> {
    let body = quote! {
        type EventTypes = std::iter::Once<::cqrs::EventType>;

        #[inline(always)]
        fn event_types() -> Self::EventTypes {
            std::iter::once(Self::EVENT_TYPE)
        }
    };

    util::render_struct(&input, quote!(::cqrs::TypedEvent), body, None)
}

/// Implements `cqrs::TypedEvent` part of [`crate::event_derive`] macro
/// expansion for enums via [`synstructure`].
fn derive_enum(input: syn::DeriveInput) -> Result<TokenStream> {
    let structure = Structure::try_new(&input)?;
    util::assert_all_enum_variants_have_single_field(&structure, TRAIT_NAME)?;

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

    let mut types = Vec::new();
    for field in iter {
        let mut path = match &field.ty {
            syn::Type::Path(path) => path.path.clone(),
            _ => {
                return Err(Error::new(
                    field.span(),
                    "TypedEvent can only be derived for enums \
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

        types.push(quote!(#path));
    }

    let const_len = types.len();
    let const_doc = format!("Type names of [`{}`] events.", input.ident);

    let type_name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let assoc_type = generate_assoc_type(&types);
    let fn_body = generate_fn_body(&types);

    Ok(quote! {
        #[automatically_derived]
        impl#impl_generics ::cqrs::TypedEvent for #type_name#ty_generics #where_clause {
            type EventTypes = #assoc_type;

            #[inline(always)]
            fn event_types() -> Self::EventTypes {
                #fn_body
            }
        }
    })
}

fn generate_assoc_type(types: &[TokenStream]) -> TokenStream {
    match types {
        [] => TokenStream::new(),
        [first] => quote!(<#first as ::cqrs::TypedEvent>::EventTypes),
        [prev @ .., last] => {
            let prev = generate_assoc_type(prev);
            quote!(std::iter::Chain<#prev, #last>)
        }
    }
}

fn generate_fn_body(types: &[TokenStream]) -> TokenStream {
    if types.is_empty() {
        return TokenStream::new();
    }

    let mut s = types.first().unwrap().clone();
    for ty in types.iter().skip(1) {
        s.extend(quote!(.chain(#ty::event_types())));
    }
    s
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
                MyEvent(MyEvent),
                HisEvent(HisEvent),
                HerEvent(HerEvent),
            }
        };

        let output = quote! {
            #[automatically_derived]
            impl ::cqrs::TypedEvent for Event {
                type EventTypes = std::iter::Chain<
                    std::iter::Chain<
                        <MyEvent as ::cqrs::TypedEvent>::EventTypes,
                        <HisEvent as ::cqrs::TypedEvent>::EventTypes
                    >,
                    <HerEvent as ::cqrs::TypedEvent>::EventTypes
                >;

                #[inline(always)]
                fn event_types() -> Self::EventTypes {
                    MyEvent::event_types()
                        .chain(HisEvent::event_types())
                        .chain(HerEvent::event_types())
                }
            }
        };

        assert_eq!(derive(input).unwrap().to_string(), output.to_string())
    }
}
