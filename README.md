# cqrs

`cqrs` is an event-driven framework for writing software that uses events as
the "source of truth" and implements command–query responsibility separation (CQRS).

The framework is built around a few key concepts:

* _Events_: The things that happened in the system
* _Aggregates_: Projections of events that calculate a view of the current 
    state of the system
* _Commands_: Intentions which, when executed against an aggregate, may produce
    zero or more events, or which may be prohibited by the current state of
    an aggregate
* _Reactions_: Processes that execute an action when certain events occur
     in the system

The framework is written to be applicable to a generic backend, with an
implementation provided for a PostgreSQL backend.

For an example of how to construct a domain which includes aggregates, events,
and commands, look at the `cqrs-todo-core` crate, which is a simple to-do list
implementation.

The source repository also contains a binary in the `cqrs-todoql-psql` directory 
which demonstrates the use of the `todo` domain in concert with the PostgreSQL
backend and a GraphQL frontend using the [`juniper`][juniper] crate.

  [juniper]: https://crates.io/crates/juniper

Minimum supported version of the Rust compiler is currently 1.32.

## Development

To build all crates in this repository:

    cargo build

To test all crates and documentation:

    cargo test

To compile documentation for the crates in the repository (remove `--no-deps`
to also include documentation for dependencies; add `--open` to automatically
open the documentation in a browser):

    cargo doc --no-deps

This crate aims to support the `wasm32-unknown-unknown` target for the `cqrs`,
`cqrs-core`, and `cqrs-todo-core` crates. To build against this target, execute:

    cargo build --target=wasm32-unknown-unknown -p cqrs-core -p cqrs -p cqrs-todo-core

## License

`cqrs` is openly-licensed under the [Apache-2.0][] license.

  [Apache-2.0]: https://opensource.org/licenses/Apache-2.0