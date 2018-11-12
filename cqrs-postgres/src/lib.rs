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
)]

extern crate cqrs_core;
extern crate fallible_iterator;
#[macro_use]
extern crate log;
#[macro_use]
extern crate postgres;
extern crate serde;
extern crate serde_json;

mod error;
mod store;
mod util;

pub use store::PostgresStore;
pub use error::{LoadError, PersistError};