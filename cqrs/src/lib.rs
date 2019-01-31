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
//! * _Reactions_: Processes that execute an action when a certain events are occur
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

#![warn(unused_import_braces, unused_imports, unused_qualifications)]
#![deny(
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_must_use,
    missing_docs
)]

pub mod memory;
pub mod trivial;

mod entity;

#[cfg(test)]
mod testing;

#[doc(inline)]
pub use crate::entity::{
    CompositeEntitySink, CompositeEntitySource, CompositeEntityStore, Entity, EntitySink,
    EntitySource, EntityStore,
};
#[doc(inline)]
pub use cqrs_core::*;
