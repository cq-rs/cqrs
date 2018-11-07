extern crate cqrs_todoql;
extern crate clap;

use clap::{App, Arg};

use cqrs_todoql::start_todo_server;

fn main() {
    let app = App::new("todo")
        .arg(Arg::with_name("conn-str")
            .long("connection-string")
            .short("c")
            .takes_value(true)
            .help("Backend PostgreSQL connection string")
            .value_name("postgresql://user:pass@localhost:5433/db")
        );

    let matches = app.get_matches();

    let listening = start_todo_server(matches.value_of("conn-str").unwrap());

    println!("Now listening at {}", listening.socket);
    println!("Press Ctrl+C to quit");
}
