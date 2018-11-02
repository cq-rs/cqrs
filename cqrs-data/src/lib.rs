#! This is documented.

extern crate cqrs;
extern crate hashbrown;
extern crate parking_lot;
extern crate void;

#[cfg(test)] #[macro_use] extern crate static_assertions;

//mod aggregate_source;
//mod aggregate_store;

pub mod event;
pub mod state;
pub mod memory;
pub mod trivial;

mod entity;
mod types;

pub use entity::Entity;
pub use types::Since;