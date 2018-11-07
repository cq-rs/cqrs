///! The core types for a CQRS aggregate system

#[cfg(test)] #[macro_use] extern crate static_assertions;

mod event;
mod snapshot;

mod aggregate;
mod types;

pub use aggregate::Aggregate;
pub use event::{EventSource, EventSink};
pub use snapshot::{SnapshotSource, SnapshotSink};
pub use types::{EventNumber, Version, Precondition, SequencedEvent, Since, StateSnapshot};

