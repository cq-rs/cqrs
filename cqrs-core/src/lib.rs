//! # cqrs-core
//!
//! `cqrs-core` defines the core types for the CQRS aggregate system

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

#[cfg(test)]
extern crate void;

mod aggregate;
mod store;
mod types;

#[doc(inline)]
pub use aggregate::{
    Aggregate, AggregateCommand, AggregateEvent, AggregateId, AggregateIdentifiedBy, ApplyTarget,
    CommandError, DeserializableEvent, Event, EventFor, Events, ExecuteTarget, ProducedEvent,
    ProducedEvents, SerializableEvent,
};
#[doc(inline)]
pub use store::{
    AlwaysSnapshot, EventSink, EventSource, NeverSnapshot, SnapshotSink, SnapshotSource,
    SnapshotStrategy,
};
#[doc(inline)]
pub use types::{
    CqrsError, EventNumber, Precondition, Since, SnapshotRecommendation, Version,
    VersionedAggregate, VersionedEvent,
};
