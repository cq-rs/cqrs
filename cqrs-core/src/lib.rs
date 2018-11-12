///! The core types for a CQRS aggregate system

#[cfg(test)] #[macro_use] extern crate static_assertions;
#[cfg(test)] extern crate void;

mod aggregate;
mod store;
mod types;

pub use aggregate::{Aggregate, PersistableAggregate, SerializableEvent};
pub use store::{EventSource, EventSink, SnapshotSource, SnapshotSink};
pub use types::{CqrsError, EventNumber, Version, Precondition, RawEvent, VersionedEvent, Since, VersionedAggregate, VersionedAggregateView, EventDeserializeError};

#[cfg(test)]
pub use aggregate::testing;
