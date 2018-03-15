#! This is documented.

extern crate cqrs;

#[cfg(test)] #[macro_use] extern crate static_assertions;

//mod aggregate_source;
//mod aggregate_store;

pub mod event;
pub mod state;

mod types;
mod trivial;

pub use types::{Expectation, Since};