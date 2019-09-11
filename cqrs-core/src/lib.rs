//! Core types for the [CQRS]/[ES] aggregate system.
//!
//! [CQRS]: https://martinfowler.com/bliki/CQRS.html
//! [ES]: https://martinfowler.com/eaaDev/EventSourcing.html

#![deny(
    missing_debug_implementations,
    nonstandard_style,
    rust_2018_idioms,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_must_use
)]
#![warn(
    missing_docs,
    missing_copy_implementations,
    unused_import_braces,
    unused_labels,
    unused_lifetimes,
    unused_qualifications,
    unused_results
)]
//#![warn(unreachable_pub)]

mod aggregate;
mod command;

mod event;
mod into;

#[doc(inline)]
pub use self::{aggregate::*, command::*, event::*, into::*};
