extern crate cqrs_todoql;
extern crate clap;

use clap::{App, Arg};

use cqrs_todoql::{BackendChoice, start_todo_server};

fn main() {
    let app = App::new("todo")
        .arg(Arg::with_name("null-event-store")
            .long("null-event-store")
            .takes_value(false)
            .help("Use null event store")
            .long_help("Operates with an event store that stores nothing and never returns any events."))
        .arg(Arg::with_name("redis-event-store")
            .long("redis-event-store")
            .help("Address for the redis event store (example: redis://127.0.0.1:6379)")
            .value_name("redis://HOST:PORT"))
        .arg(Arg::with_name("redis-snapshot-store")
            .long("redis-snapshot-store")
            .help("Address for the redis snapshot store")
            .value_name("redis://HOST:PORT"))
        .arg(Arg::with_name("null-snapshot-store")
            .long("null-snapshot-store")
            .takes_value(false)
            .help("Use null snapshot store")
            .long_help("Operates with a snapshot store that stores nothing and never returns a snapshot."));

    let matches = app.get_matches();

    let event_backend =
        if matches.is_present("null-event-store") {
            BackendChoice::Null
        } else if let Some(host_str) = matches.value_of("redis-event-store") {
            BackendChoice::Redis(host_str.to_string())
        } else {
            BackendChoice::Memory
        };

    let snapshot_backend =
        if matches.is_present("null-snapshot-store") {
            BackendChoice::Null
        } else if let Some(host_str) = matches.value_of("redis-snapshot-store") {
            BackendChoice::Redis(host_str.to_string())
        } else {
            BackendChoice::Memory
        };

    let listening = start_todo_server(event_backend, snapshot_backend);

    println!("Now listening at {}", listening.socket);
    println!("Press Ctrl+C to quit");
}
