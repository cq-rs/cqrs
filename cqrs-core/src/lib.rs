//! The core types for a CQRS aggregate system

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
)]

#[cfg(test)] #[macro_use] extern crate static_assertions;
#[cfg(test)] extern crate void;

mod aggregate;
mod store;
mod types;

pub use aggregate::{Aggregate, PersistableAggregate, SerializableEvent};
pub use store::{EventSource, EventSink, SnapshotSource, SnapshotSink};
pub use types::{CqrsError, EventNumber, Version, Precondition, VersionedEvent, Since, VersionedAggregate, VersionedAggregateView, EventDeserializeError};

#[cfg(test)]
pub use aggregate::testing;
