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
    unused_must_use
)]
#![warn(missing_docs)]

mod aggregate;
mod future;
pub mod reactor;
mod store;
mod types;

#[doc(inline)]
pub use crate::aggregate::{
    Aggregate, AggregateCommand, AggregateEvent, Command, CommandError, CommandHandler,
    DeserializableEvent, Event, EventSourced, Events, IntoEvents, ProducedEvent, ProducedEvents,
    SerializableEvent, VersionedEvent,
};
#[doc(inline)]
pub use crate::future::*;
#[doc(inline)]
pub use crate::store::{
    AlwaysSnapshot, EventSink, EventSource, NeverSnapshot, SnapshotSink, SnapshotSource,
    SnapshotStrategy,
};
#[doc(inline)]
pub use crate::types::{
    BorrowedRawEvent, CqrsError, EventNumber, NumberedEvent, NumberedEventWithMeta, Precondition,
    RawEvent, Since, SnapshotRecommendation, Version, VersionedAggregate,
};
