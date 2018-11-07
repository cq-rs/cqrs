extern crate cqrs;
extern crate cqrs_data;
extern crate cqrs_postgres;
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
extern crate r2d2_postgres;
extern crate r2d2;
extern crate serde;
extern crate serde_json;
extern crate void;

mod graphql;

use r2d2_postgres::PostgresConnectionManager;

pub fn start_todo_server(conn_str: &str) -> iron::Listening {
    let pool = r2d2::Pool::new(PostgresConnectionManager::new(conn_str, r2d2_postgres::TlsMode::None).unwrap()).unwrap();

    let hashid =
        if let Ok(hashid) = hashids::HashIds::new_with_salt_and_min_length("cqrs".to_string(), 10) {
            hashid
        } else {
            panic!("Failed to generate hashid")
        };

    let id_provider = IdProvider(hashid, Default::default());

    let id = id_provider.new_id();

    helper::prefill(&id, pool.get().unwrap());

    let stream_index = vec![id];

    let context = graphql::InnerContext::new(
        stream_index,
        pool,
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
    use cqrs::{Version, StateSnapshot};
    use cqrs_data::{event::Store as EO, state::Store as SO};
    use cqrs_todo_core::{Event, TodoAggregate, TodoData, TodoStatus, domain};
    use cqrs_postgres::PostgresStore;
    use r2d2_postgres::postgres::Connection;

    pub fn prefill(id: &str, conn: impl ::std::ops::Deref<Target=Connection>) {
        let epoch = Utc.ymd(1970, 1, 1).and_hms(0, 0, 0);
        let reminder_time = epoch + Duration::seconds(10000);
        let mut events = Vec::new();
        events.push(Event::Completed);
        events.push(Event::Created(domain::Description::new("Hello!").unwrap()));
        events.push(Event::ReminderUpdated(Some(domain::Reminder::new(reminder_time, epoch).unwrap())));
        events.push(Event::TextUpdated(domain::Description::new("New text").unwrap()));
        events.push(Event::Created(domain::Description::new("Ignored!").unwrap()));
        events.push(Event::ReminderUpdated(None));

        let store = PostgresStore::new(&*conn, "todo");

        store.append_events(id, &events, None).unwrap();
        store.persist_snapshot(id, StateSnapshot {
            version: Version::new(1),
            snapshot: TodoAggregate::Created(TodoData {
                description: domain::Description::new("Hello!").unwrap(),
                reminder: None,
                status: TodoStatus::NotCompleted,
            })
        }).unwrap();
    }
}
