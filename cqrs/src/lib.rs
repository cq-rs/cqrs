///! The core types for a CQRS aggregate system

#[cfg(test)] #[macro_use] extern crate static_assertions;

pub mod error;

mod aggregate;
mod types;

pub use aggregate::Aggregate;
pub use types::{EventNumber, Version, Precondition, SequencedEvent, StateSnapshot};

