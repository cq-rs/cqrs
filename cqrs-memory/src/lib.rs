#[cfg(test)]
extern crate fnv;
extern crate cqrs;
extern crate cqrs_data;
extern crate void;

pub mod event_store;
pub mod state_store;

mod event_stream;

pub use event_store::MemoryEventStore;
pub use state_store::MemoryStateStore;
