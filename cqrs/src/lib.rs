//! # cqrs
//!
//! `cqrs` defines the types for interacting with entities in the CQRS system

#![warn(unused_import_braces, unused_imports, unused_qualifications)]
#![deny(
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_must_use,
    missing_docs
)]

pub mod memory;
pub mod trivial;

mod entity;

#[cfg(test)]
mod testing;

#[doc(inline)]
pub use crate::entity::{
    CompositeEntitySink, CompositeEntitySource, CompositeEntityStore, Entity, EntitySink,
    EntitySource, EntityStore,
};
#[doc(inline)]
pub use cqrs_core::*;
