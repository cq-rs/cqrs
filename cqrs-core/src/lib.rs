//! # cqrs-core
//!
//! `cqrs-core` defines the core types for the CQRS aggregate system

#![warn(
    unused_import_braces,
    unused_imports,
    unused_qualifications,
    missing_docs,
)]

#![deny(
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_must_use,
)]

#[cfg(test)] #[macro_use] extern crate static_assertions;
#[cfg(test)] extern crate void;

mod aggregate;
mod store;
mod types;

#[doc(inline)]
pub use aggregate::{Aggregate, AggregateCommand, AggregateEvent, Event, SerializableEvent, DeserializableEvent};
#[doc(inline)]
pub use store::{EventSource, EventSink, SnapshotSource, SnapshotSink};
#[doc(inline)]
pub use types::{CqrsError, EventNumber, Version, Precondition, VersionedEvent, Since, VersionedAggregate, VersionedAggregateView};
