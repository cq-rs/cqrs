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

#![deny(
    missing_copy_implementations,
    nonstandard_style,
    rust_2018_idioms,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_must_use
)]
#![warn(
    missing_debug_implementations,
    missing_docs,
    unused_import_braces,
    unused_labels,
    unused_qualifications,
    unused_results
)]
//#![warn(unreachable_pub)]

mod event_processing;
pub mod lifecycle;

use async_trait::async_trait;

#[doc(inline)]
pub use cqrs_codegen::*;
#[doc(inline)]
pub use cqrs_core::*;

#[doc(inline)]
pub use self::{
    event_processing::{
        EventHandler, EventHandlersRegistrar, EventProcessingConfiguration,
        EventProcessingConfigurationBuilder, RegisteredEvent,
    },
    lifecycle::BorrowableAsContext,
};

#[async_trait(?Send)]
pub trait CommandGateway<Cmd: Command, Mt> {
    type Err;
    type Ok;

    async fn send(&self, cmd: Cmd, meta: Mt) -> Result<Self::Ok, Self::Err>;
}

#[async_trait(?Send)]
pub trait CommandBus<Cmd: Command> {
    type Err;
    type Ok;

    async fn dispatch(&self, cmd: Cmd) -> Result<Self::Ok, Self::Err>
    where
        Cmd: 'async_trait;
}

pub trait DomainEvent: Event {}

pub trait AggregateEvent: Event {
    type Aggregate: Aggregate;

    fn event_types() -> &'static [EventType];
}

pub trait Query {}

#[async_trait(?Send)]
pub trait QueryGateway<Qr: Query> {
    type Err;
    type Ok;

    async fn query(&self, query: Qr) -> Result<Self::Ok, Self::Err>
    where
        Qr: 'async_trait;
}

#[async_trait(?Send)]
pub trait QueryHandler<Qr: Query> {
    type Context: ?Sized;
    type Err;
    type Ok;

    async fn handle(&self, query: Qr, ctx: &Self::Context) -> Result<Self::Ok, Self::Err>
    where
        Qr: 'async_trait;
}
