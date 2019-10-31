//! Codegen for [`cqrs::Event`].

use quote::quote;
use syn::{
    parse::{Error, Result},
    punctuated::Punctuated,
    spanned::Spanned as _,
};
use synstructure::Structure;

use crate::util;

/// Implements [`crate::derive_event`] macro expansion.
pub(crate) fn derive(input: syn::DeriveInput) -> Result<proc_macro2::TokenStream> {
    match input.data {
        syn::Data::Struct(_) => derive_struct(input),
        syn::Data::Enum(_) => derive_enum(input),
        syn::Data::Union(data) => Err(Error::new(
            data.union_token.span(),
            "Unions are not supported for deriving Event",
        )),
    }
}

/// Implements [`crate::derive_event`] macro expansion for structs.
fn derive_struct(input: syn::DeriveInput) -> Result<proc_macro2::TokenStream> {
    let syn::DeriveInput {
        attrs,
        ident,
        generics,
        ..
    } = input;

    let meta = util::get_nested_meta(&attrs, "event")?.ok_or_else(|| {
        Error::new(
            ident.span(),
            "Expected struct to have #[event(...)] attribute",
        )
    })?;

    let event_type = parse_event_type_from_nested_meta(meta)?;

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let const_doc = format!("Type name of [`{}`] event", ident);

    Ok(quote! {
        #[automatically_derived]
        impl#impl_generics #ident#ty_generics #where_clause {
            #[doc = #const_doc]
            pub const EVENT_TYPE: ::cqrs::EventType = #event_type;
        }

        #[automatically_derived]
        impl#impl_generics ::cqrs::Event for #ident#ty_generics #where_clause {
            #[inline(always)]
            fn event_type(&self) -> ::cqrs::EventType {
                Self::EVENT_TYPE
            }
        }
    })
}

/// Implements [`crate::derive_event`] macro expansion for enums.
fn derive_enum(input: syn::DeriveInput) -> Result<proc_macro2::TokenStream> {
    derive_enum_impl(Structure::try_new(&input)?)
}

/// Implements [`crate::derive_event`] macro expansion for enums
/// via [`synstructure`].
fn derive_enum_impl(mut structure: Structure) -> Result<proc_macro2::TokenStream> {
    if util::get_nested_meta(&structure.ast().attrs, "event")?.is_some() {
        return Err(Error::new(
            structure.ast().span(),
            "#[event(...)] attribute is not allowed for enums",
        ));
    }

    for variant in structure.variants() {
        let ast = variant.ast();
        if ast.fields.len() != 1 {
            return Err(Error::new(
                ast.ident.span(),
                "Event can only be derived for enums with variants that have \
                 exactly one field",
            ));
        }
    }

    structure.add_bounds(synstructure::AddBounds::Fields);

    structure.binding_name(|field, _| {
        field.ident.as_ref().map_or_else(
            || syn::Ident::new("event", proc_macro2::Span::call_site()),
            |ident| ident.clone(),
        )
    });

    let body = structure.each(|binding_info| {
        let ident = &binding_info.binding;
        quote!(#ident.event_type())
    });

    Ok(structure.gen_impl(quote! {
        #[automatically_derived]
        gen impl ::cqrs::Event for @Self {
            fn event_type(&self) -> ::cqrs::EventType {
                match *self {
                    #body
                }
            }
        }
    }))
}

/// Parses type of [`cqrs::Event`] from `#[event(...)]` attribute.
fn parse_event_type_from_nested_meta(
    meta: Punctuated<syn::NestedMeta, syn::Token![,]>,
) -> Result<String> {
    const WRONG_FORMAT: &str = "Wrong attribute format; expected #[event(type = \"...\")]";

    let mut event_type = None;

    for m in meta {
        let m = match m {
            syn::NestedMeta::Meta(m) => m,
            _ => return Err(Error::new(m.span(), WRONG_FORMAT)),
        };
        let m = match m {
            syn::Meta::NameValue(m) => m,
            _ => return Err(Error::new(m.span(), WRONG_FORMAT)),
        };
        if !m.path.is_ident("type") {
            return Err(Error::new(m.span(), WRONG_FORMAT));
        }
        let lit = match m.lit {
            syn::Lit::Str(lit) => lit,
            _ => return Err(Error::new(m.lit.span(), WRONG_FORMAT)),
        };
        if event_type.replace(lit.value()).is_some() {
            return Err(Error::new(
                lit.span(),
                "Only one #[event(type = \"...\")] attribute is allowed",
            ));
        }
    }

    event_type.ok_or_else(|| {
        Error::new(
            proc_macro2::Span::call_site(),
            "Expected to have #[event(type = \"...\")] attribute",
        )
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn derives_struct_impl() {
        let input = syn::parse_quote! {
            #[event(type = "event")]
            struct Event;
        };

        let output = derive_struct(input).unwrap();

        let expected_output = quote! {
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

        assert_eq!(output.to_string(), expected_output.to_string());
    }

    #[test]
    fn derives_enum_impl() {
        synstructure::test_derive! {
            derive_enum_impl {
                enum Event {
                    Event1(Event1),
                    Event2 {
                        other_event: Event2,
                    },
                }
            }
            expands to {
                #[allow(non_upper_case_globals)]
                const _DERIVE_cqrs_Event_FOR_Event: () = {
                    #[automatically_derived]
                    impl ::cqrs::Event for Event {
                        fn event_type(&self) -> ::cqrs::EventType {
                            match *self {
                                Event::Event1(ref event,) => {{ event.event_type() }}
                                Event::Event2{other_event: ref other_event,} => {{ other_event.event_type() }}
                            }
                        }
                    }
                };
            }
            no_build
        }
    }
}
