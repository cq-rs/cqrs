extern crate cqrs_eventstore;
extern crate cqrs;
extern crate cqrs_data;
extern crate hyper;
extern crate serde;
#[macro_use] extern crate serde_derive;

use cqrs_data::events::Source;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
struct Data {
    pub winter: String,
    pub is_bool: bool,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
struct Metadata {
    pub who: String,
    pub when: usize,
}

fn main() {
    let client = hyper::Client::new();
    let conn = cqrs_eventstore::http::EventStoreConnection::new(
        client,
        hyper::Url::parse("http://127.0.0.1:2113/").unwrap(),
        "admin".to_string(),
        "changeit".to_string(),
    );

    let es = cqrs_eventstore::EventStore::<Data, Metadata>::new(&conn);

    let event_iter = es.read_events("test-1", cqrs_data::Since::BeginningOfStream);

    for e in event_iter {
        println!("{:#?}", e);
    }
}