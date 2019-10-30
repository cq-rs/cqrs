extern crate proc_macro;

mod event;
mod util;

use proc_macro::TokenStream;
use syn::parse::Result;

/// Derives [`cqrs::Event`] implementation for structs and enums.
///
/// # Structs
///
/// When deriving [`cqrs::Event`] for struct, the struct is treated as
/// a single distinct event.
///
/// Specifying `#[event(type = "...")]` attribute is __mandatory__ (and only
/// single such attribute allowed per struct).
///
/// # Enums
///
/// When deriving [`cqrs::Event`] for enum, the enum is treated as a sum-type
/// representing a set of possible events.
///
/// In practice this means, that [`cqrs::Event`] can only be derived for a enum
/// when all variants of such enum have exactly one field (variant can be both
/// a tuple-variant or a struct-variant) and the field have to implement
/// [`cqrs::Event`] itself.
///
/// Generated implementation of [`cqrs::Event::event_type`] would match on all
/// variants and proxy calls to each variant's field.
///
/// # Examples
/// ```
/// # use cqrs_codegen::Event;
/// #
/// #[derive(Event)]
/// #[event(type = "user.created")]
/// struct UserCreated;
///
/// #[derive(Event)]
/// #[event(type = "user.removed")]
/// struct UserRemoved;
///
/// #[derive(Event)]
/// enum UserEvents {
///     UserCreated(UserCreated),
///     UserRemoved(UserRemoved),
/// }
/// ```
#[proc_macro_derive(Event, attributes(event))]
pub fn derive_event(input: TokenStream) -> TokenStream {
    expand(input, event::derive)
}

type MacroImpl = fn(syn::DeriveInput) -> Result<proc_macro2::TokenStream>;

/// Expands given input [`TokenStream`] with a given macro implementation.
fn expand(input: TokenStream, macro_impl: MacroImpl) -> TokenStream {
    match syn::parse(input).and_then(|i| macro_impl(i)) {
        Ok(res) => res.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
