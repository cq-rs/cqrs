#[cfg(test)]
extern crate fnv;
extern crate okazis;

pub mod event_store;
pub mod state_store;

mod event_stream;

pub use event_store::MemoryEventStore;
pub use state_store::MemoryStateStore;

#[derive(PartialEq, Debug, Clone, Copy, Hash)]
pub enum Never {}

