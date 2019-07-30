//! # cqrs
//!
//! `cqrs` is an event-driven framework for writing software that uses events as
//! the "source of truth" and implements command–query responsibility separation (CQRS).
//!
//! The framework is built around a few key concepts:
//!
//! * _Events_: The things that happened in the system
//! * _Aggregates_: Projections of events that calculate a view of the current
//!     state of the system
//! * _Commands_: Intentions which, when executed against an aggregate, may produce
//!     zero or more events, or which may be prohibited by the current state of
//!     an aggregate
//! * _Reactions_: Processes that execute an action when certain events occur
//!      in the system
//!
//! The framework is written to be applicable to a generic backend, with an
//! implementation provided for a PostgreSQL backend.
//!
//! For an example of how to construct a domain which includes aggregates, events,
//! and commands, look at the `cqrs-todo-core` crate, which is a simple to-do list
//! implementation.
//!
//! The source repository also contains a binary in the `cqrs-todoql-psql` directory
//! which demonstrates the use of the `todo` domain in concert with the PostgreSQL
//! backend and a GraphQL frontend using the [`juniper`][juniper] crate.
//!
//!   [juniper]: https://crates.io/crates/juniper

#![feature(async_await)]
#![deny(
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    nonstandard_style,
    rust_2018_idioms,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_must_use
)]
#![warn(
    unused_import_braces,
    unused_labels,
    unused_qualifications,
    unused_results
)]
//#![warn(unreachable_pub)]

mod system;

use async_trait::async_trait;

#[doc(inline)]
pub use cqrs_core::*;

#[doc(inline)]
pub use self::system::*;

/// TODO
#[async_trait]
pub trait CommandGateway<C: Command, M> {
    type Err;
    type Ok;

    /// TODO
    async fn command(&self, cmd: C, meta: M) -> Result<Self::Ok, Self::Err>;
}
