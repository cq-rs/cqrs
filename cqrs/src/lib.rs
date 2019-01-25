//! This is documented.

#![warn(
    unused_import_braces,
    unused_imports,
    unused_qualifications,
    missing_docs,
)]

#![deny(
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_must_use,
)]

extern crate cqrs_core;
extern crate hashbrown;
extern crate parking_lot;
#[cfg(feature = "proptest")]
extern crate proptest;
extern crate void;

pub mod memory;
pub mod trivial;

mod entity;

mod testing;

pub use entity::{Entity, EntitySink, EntitySource, EntityStore, CompositeEntitySink, CompositeEntitySource, CompositeEntityStore};
pub use cqrs_core::*;