//! This is documented.

extern crate cqrs_core;
extern crate hashbrown;
extern crate parking_lot;
extern crate void;

pub mod memory;
pub mod trivial;

mod entity;

mod testing;

pub use entity::{Entity, EntitySink, EntitySource, EntityStore, CompositeEntitySink, CompositeEntitySource, CompositeEntityStore};
pub use cqrs_core::*;