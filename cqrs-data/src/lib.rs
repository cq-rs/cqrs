#! This is documented.

extern crate cqrs;

#[cfg(test)] #[macro_use] extern crate static_assertions;

mod aggregate_source;
mod aggregate_store;

pub mod events;
pub mod snapshots;

mod types;
mod trivial;

pub use types::{Since};