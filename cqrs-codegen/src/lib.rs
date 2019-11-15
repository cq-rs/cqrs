extern crate proc_macro;

use proc_macro::TokenStream;

#[cfg(all(not(feature = "watt"), feature = "no-watt"))]
/// Imports proc macro from implementation crate as is.
macro_rules! import {
    ($input:expr, $fn:ident) => {
        cqrs_codegen_impl::expand(syn::parse($input), cqrs_codegen_impl::$fn)
    };
}

#[cfg(all(feature = "watt", not(feature = "no-watt")))]
/// Imports proc macro from implementation crate via WASM ABI.
macro_rules! import {
    ($input:expr, $fn:ident) => {
        wasm::MACRO.proc_macro(stringify!($fn), $input)
    };
}

#[cfg(all(feature = "watt", not(feature = "no-watt")))]
mod wasm {
    /// Generated WASM of implementation crate.
    static WASM: &[u8] = include_bytes!("codegen.wasm");

    /// Callable interface of the generated [`WASM`].
    pub static MACRO: watt::WasmMacro = watt::WasmMacro::new(WASM);
}

/// Derives [`cqrs::AggregateEvent`] implementation for enums.
///
/// The enum is treated as a sum-type representing a set of possible events.
///
/// Specifying `#[event(aggregate = "...")]` attribute is __mandatory__
/// (and only single such attribute allowed per enum). The attribute is
/// treated as a type of the aggregate with which event is associated.
///
/// In practice this means, that [`cqrs::AggregateEvent`] can only be derived
/// for an enum when all variants of such enum have exactly one field
/// (variant can be either a tuple-variant or a struct-variant).
///
/// Each field is expected to have a defined associated constant `EVENT_TYPE`.
/// [`cqrs::Event`] derive macro generates such constant automatically.
///
/// Note, that generic enums deriving [`cqrs::AggregateEvent`] cannot have
/// variants with field of type-parameter type (e.g., `T`) as it's impossible
/// for type-parameter to have associated constants. However, fields of
/// generic types that dependent on type-parameters (e.g., `Event<T>`) are fine.
///
/// # Examples
/// ```
/// # use cqrs_codegen::{AggregateEvent, Event};
/// #
/// # #[derive(Default)]
/// # struct User(i32);
/// #
/// # impl cqrs::Aggregate for User {
/// #   type Id = i32;
/// #   fn aggregate_type(&self) -> &'static str { "user" }
/// #   fn id(&self) -> &Self::Id { &self.0 }
/// # }
/// #
/// #[derive(Event)]
/// #[event(type = "user.created")]
/// struct UserCreated;
///
/// #[derive(Event)]
/// #[event(type = "user.removed")]
/// struct UserRemoved;
///
/// #[derive(AggregateEvent, Event)]
/// #[event(aggregate = "User")]
/// enum UserEvents {
///     UserCreated(UserCreated),
///     UserRemoved(UserRemoved),
/// }
/// ```
#[proc_macro_derive(AggregateEvent, attributes(event))]
pub fn aggregate_event_derive(input: TokenStream) -> TokenStream {
    import!(input, aggregate_event_derive)
}

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
/// In practice this means, that [`cqrs::Event`] can only be derived for an enum
/// when all variants of such enum have exactly one field (variant can be either
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
pub fn event_derive(input: TokenStream) -> TokenStream {
    import!(input, event_derive)
}

/// Derives [`cqrs::RegisteredEvent`] implementation for structs and enums.
///
/// # Structs
///
/// When deriving [`cqrs::RegisteredEvent`] for struct, the struct is treated as
/// a single distinct event.
///
/// # Enums
///
/// When deriving [`cqrs::RegisteredEvent`] for enum, the enum is treated as
/// a sum-type representing a set of possible events.
///
/// In practice this means, that [`cqrs::RegisteredEvent`] can only be derived
/// for an enum when all variants of such enum have exactly one field (variant
/// can be either a tuple-variant or a struct-variant).
///
/// Generated implementation of [`cqrs::RegisteredEvent::type_id`] would
/// match on all variants and return [`cqrs::RegisteredEvent::type_id`] of each
/// variant's field (__with a proxy call__, so fields do have to implement
/// [`cqrs::RegisteredEvent`] themself).
///
/// # Examples
/// ```
/// # use cqrs_codegen::{Event, RegisteredEvent};
/// #
/// #[derive(Event, RegisteredEvent)]
/// #[event(type = "user.created")]
/// struct UserCreated;
///
/// #[derive(Event, RegisteredEvent)]
/// #[event(type = "user.removed")]
/// struct UserRemoved;
///
/// #[derive(Event, RegisteredEvent)]
/// enum UserEvents {
///     UserCreated(UserCreated),
///     UserRemoved(UserRemoved),
/// }
/// ```
#[proc_macro_derive(RegisteredEvent)]
pub fn registered_event_derive(input: TokenStream) -> TokenStream {
    import!(input, registered_event_derive)
}

/// Derives [`cqrs::VersionedEvent`] implementation for structs and enums.
///
/// # Structs
///
/// When deriving [`cqrs::VersionedEvent`] for struct, the struct is treated as
/// a single distinct event.
///
/// Specifying `#[event(version = <non-zero unsigned integer>)]` attribute is
/// __mandatory__ (and only single such attribute allowed per struct).
///
/// # Enums
///
/// When deriving [`cqrs::VersionedEvent`] for enum, the enum is treated as
/// a sum-type representing a set of possible events.
///
/// In practice this means, that [`cqrs::VersionedEvent`] can only be derived
/// for an enum when all variants of such enum have exactly one field (variant
/// can be either a tuple-variant or a struct-variant) and the field have to
/// implement [`cqrs::VersionedEvent`] itself.
///
/// Generated implementation of [`cqrs::VersionedEvent::event_version`] would
/// match on all variants and proxy calls to each variant's field.
///
/// # Examples
/// ```
/// # use cqrs_codegen::{Event, VersionedEvent};
/// #
/// #[derive(Event, VersionedEvent)]
/// #[event(type = "user.created", version = 1)]
/// struct UserCreated;
///
/// #[derive(Event, VersionedEvent)]
/// #[event(type = "user.removed", version = 2)]
/// struct UserRemoved;
///
/// #[derive(Event, VersionedEvent)]
/// enum UserEvents {
///     UserCreated(UserCreated),
///     UserRemoved(UserRemoved),
/// }
/// ```
#[proc_macro_derive(VersionedEvent, attributes(event))]
pub fn versioned_event_derive(input: TokenStream) -> TokenStream {
    import!(input, versioned_event_derive)
}
