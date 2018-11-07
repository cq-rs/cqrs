//! This is documented.

extern crate cqrs_core;
extern crate hashbrown;
extern crate parking_lot;
extern crate void;

pub mod memory;
pub mod trivial;

mod entity;

pub use entity::Entity;
pub use cqrs_core::*;