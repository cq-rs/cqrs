extern crate clap;
extern crate cqrs_todoql_psql;
extern crate env_logger;

use clap::{App, Arg};

use cqrs_todoql_psql::start_todo_server;

fn main() {
    env_logger::init();

    let app = App::new("todo")
        .arg(
            Arg::with_name("conn-str")
                .long("connection-string")
                .short("c")
                .takes_value(true)
                .help("Backend PostgreSQL connection string")
                .value_name("postgresql://user:pass@localhost:5433/db"),
        )
        .arg(
            Arg::with_name("prefill")
                .long("prefill")
                .short("p")
                .takes_value(true)
                .help("Prefill quantity")
                .value_name("QTY"),
        );

    let matches = app.get_matches();

    let listening = start_todo_server(
        matches.value_of("conn-str").unwrap(),
        matches
            .value_of("prefill")
            .map(|s| s.parse().unwrap())
            .unwrap_or_default(),
    );

    println!("Now listening at {}", listening.socket);
    println!("Press Ctrl+C to quit");
}
