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