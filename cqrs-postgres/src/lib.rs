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

#[cfg(test)] extern crate cqrs;
#[cfg(test)] extern crate cqrs_todo_core;
#[cfg(test)] extern crate static_assertions;

mod error;
mod store;
mod util;

pub use store::PostgresStore;
pub use error::{LoadError, PersistError};

#[cfg(test)]
mod tests {
    use super::*;
    use cqrs_todo_core::TodoAggregate;
    use static_assertions::assert_impl;

    #[test]
    fn postgres_store_is_an_entity_store() {
        assert_impl!(PostgresStore<TodoAggregate>, cqrs::EntityStore<TodoAggregate>);
    }

    #[test]
    fn postgres_store_is_an_entity_source() {
        assert_impl!(PostgresStore<TodoAggregate>, cqrs::EntitySource<TodoAggregate>);
    }

    #[test]
    fn postgres_store_is_an_entity_sink() {
        assert_impl!(PostgresStore<TodoAggregate>, cqrs::EntitySink<TodoAggregate>);
    }
}