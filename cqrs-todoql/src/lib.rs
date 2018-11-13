#![warn(
    unused_import_braces,
    unused_imports,
    unused_qualifications,
)]

#![deny(
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_must_use,
)]

extern crate cqrs;
extern crate cqrs_redis;
extern crate cqrs_todo_core;

#[macro_use] extern crate juniper;
extern crate juniper_iron;
#[macro_use] extern crate juniper_codegen;
extern crate chrono;
extern crate base64;
extern crate fnv;
extern crate iron;
extern crate mount;
extern crate hashids;
extern crate parking_lot;
extern crate redis;
extern crate r2d2_redis;
extern crate r2d2;
extern crate serde;
extern crate serde_json;
extern crate void;

mod graphql;
mod store;

use r2d2_redis::RedisConnectionManager;

type EventStore = store::MemoryOrNullEventStore;
type SnapshotStore = store::MemoryOrNullSnapshotStore;

#[derive(Debug)]
pub enum BackendChoice {
    Memory,
    Null,
    Redis(String)
}

pub fn start_todo_server(event_backend: BackendChoice, snapshot_backend: BackendChoice) -> iron::Listening {
    let es =
        match event_backend {
            BackendChoice::Null =>
                store::MemoryOrNullEventStore::new_null_store(),
            BackendChoice::Memory =>
                store::MemoryOrNullEventStore::new_memory_store(),
            BackendChoice::Redis(conn_str) => {
                let pool = r2d2::Pool::new(RedisConnectionManager::new(conn_str.as_ref()).unwrap()).unwrap();
                let config = cqrs_redis::Config::new("todoql");
                store::MemoryOrNullEventStore::new_redis_store(config, pool)
            }
        };

    let ss =
        match snapshot_backend {
            BackendChoice::Null =>
                store::MemoryOrNullSnapshotStore::new_null_store(),
            BackendChoice::Memory =>
                store::MemoryOrNullSnapshotStore::new_memory_store(),
            BackendChoice::Redis(conn_str) => {
                let pool = r2d2::Pool::new(r2d2_redis::RedisConnectionManager::new(conn_str.as_ref()).unwrap()).unwrap();
                let config = cqrs_redis::Config::new("todoql");
                store::MemoryOrNullSnapshotStore::new_redis_store(config, pool)
            }
        };

    let hashid =
        if let Ok(hashid) = hashids::HashIds::new_with_salt_and_min_length("cqrs".to_string(), 10) {
            hashid
        } else {
            panic!("Failed to generate hashid")
        };

    let id_provider = IdProvider(hashid, Default::default());

    let id = id_provider.new_id();

    helper::prefill(&id, &es, &ss);

    let stream_index = vec![id];

    let context = graphql::InnerContext::new(
        stream_index,
        es,
        ss,
        id_provider,
    );

    let chain = graphql::endpoint::create_chain(context);

    iron::Iron::new(chain).http("0.0.0.0:2777").unwrap()
}

pub struct IdProvider(hashids::HashIds,::std::sync::atomic::AtomicUsize);

impl IdProvider {
    fn new_id(&self) -> String {
        let next = self.1.fetch_add(1, ::std::sync::atomic::Ordering::SeqCst);
        let duration = ::std::time::SystemTime::now().duration_since(::std::time::UNIX_EPOCH).unwrap();
        self.0.encode(&vec![duration.as_secs() as i64, next as i64])
    }
}

mod helper {
    use chrono::{Duration,Utc,TimeZone};
    use cqrs::{Version, VersionedAggregateView};
    use cqrs::{EventSink, SnapshotSink};
    use cqrs_todo_core::{Event, TodoAggregate, TodoData, TodoStatus, domain};

    use super::{EventStore, SnapshotStore};

    pub fn prefill(id: &str, es: &EventStore, ss: &SnapshotStore) {
        let epoch = Utc.ymd(1970, 1, 1).and_hms(0, 0, 0);
        let reminder_time = epoch + Duration::seconds(10000);
        let mut events = Vec::new();
        events.push(Event::Completed);
        events.push(Event::Created(domain::Description::new("Hello!").unwrap()));
        events.push(Event::ReminderUpdated(Some(domain::Reminder::new(reminder_time, epoch).unwrap())));
        events.push(Event::TextUpdated(domain::Description::new("New text").unwrap()));
        events.push(Event::Created(domain::Description::new("Ignored!").unwrap()));
        events.push(Event::ReminderUpdated(None));

        es.append_events(id, &events, None).unwrap();
        ss.persist_snapshot(id, VersionedAggregateView {
            version: Version::new(1),
            payload: &TodoAggregate::Created(TodoData {
                description: domain::Description::new("Hello!").unwrap(),
                reminder: None,
                status: TodoStatus::NotCompleted,
            })
        }).unwrap();
    }
}
