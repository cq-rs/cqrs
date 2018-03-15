///! The core types for a CQRS aggregate system

#[cfg(test)] #[macro_use] extern crate static_assertions;

pub mod error;

mod aggregate;
mod projection;
mod types;

pub use aggregate::Aggregate;
pub use aggregate::hydrated::HydratedAggregate;
pub use projection::Projection;
pub use types::{EventNumber, Version, Precondition, SequencedEvent, StateSnapshot};

