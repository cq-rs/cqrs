#![warn(unused_import_braces, unused_imports, unused_qualifications)]
#![deny(
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_must_use
)]

#[macro_use]
extern crate juniper;

mod graphql;

use cqrs_postgres::NewConn;
use r2d2_postgres::{PostgresConnectionManager, postgres::NoTls};

#[derive(Clone, Copy, Default, Debug, Hash, PartialEq, Eq)]
struct SnapshotEvery10;

impl cqrs::SnapshotStrategy for SnapshotEvery10 {
    fn snapshot_recommendation(
        &self,
        version: cqrs::Version,
        last_snapshot_version: Option<cqrs::Version>,
    ) -> cqrs::SnapshotRecommendation {
        if version - last_snapshot_version.unwrap_or_default() >= 10 {
            cqrs::SnapshotRecommendation::ShouldSnapshot
        } else {
            cqrs::SnapshotRecommendation::DoNotSnapshot
        }
    }
}

type TodoStore = cqrs_postgres::PostgresStore<
    cqrs_todo_core::TodoAggregate,
    cqrs_todo_core::TodoEvent,
    cqrs_todo_core::TodoMetadata,
    SnapshotEvery10,
>;

pub fn start_todo_server(conn_str: &str, prefill_qty: usize) -> iron::Listening {
    let pool = r2d2::Pool::new(
        NewConn::new(PostgresConnectionManager::new(conn_str.parse().unwrap(), NoTls)),
    )
    .unwrap();

    let hashid = if let Ok(hashid) =
        hashids::HashIds::new_with_salt_and_min_length("cqrs".to_string(), 10)
    {
        hashid
    } else {
        panic!("Failed to generate hashid")
    };

    let id_provider = IdProvider(hashid, Default::default());

    {

        for _ in 0..prefill_qty {
            let id = id_provider.new_id();

            helper::prefill(&id, pool.get().unwrap());
        }
    }

    let context = graphql::InnerContext::new(pool, id_provider);

    let chain = graphql::endpoint::create_chain(context);

    iron::Iron::new(chain).http("0.0.0.0:2777").unwrap()
}

pub struct IdProvider(hashids::HashIds, ::std::sync::atomic::AtomicUsize);

impl IdProvider {
    fn new_id(&self) -> cqrs_todo_core::TodoId {
        let next = self.1.fetch_add(1, ::std::sync::atomic::Ordering::SeqCst);
        let duration = ::std::time::SystemTime::now()
            .duration_since(::std::time::UNIX_EPOCH)
            .unwrap();
        cqrs_todo_core::TodoId(self.0.encode(&vec![duration.as_secs() as i64, next as i64]))
    }
}

mod helper {
    use chrono::{Duration, TimeZone, Utc};
    use cqrs::{AlwaysSnapshot, EventSink, SnapshotSink, Version};
    use cqrs_postgres::NewConn;
    use cqrs_todo_core::{
        domain, events, TodoAggregate, TodoData, TodoEvent, TodoId, TodoMetadata, TodoStatus,
    };
    use r2d2::PooledConnection;
    type TodoStore =
        cqrs_postgres::PostgresStore<TodoAggregate, TodoEvent, TodoMetadata, AlwaysSnapshot>;

    pub fn prefill(id: &TodoId, conn: PooledConnection<NewConn>) {
        let epoch = Utc.ymd(1970, 1, 1).and_hms(0, 0, 0);
        let reminder_time = epoch + Duration::seconds(10000);
        let mut events = Vec::new();
        events.push(TodoEvent::Completed(events::Completed {}));
        events.push(TodoEvent::Created(events::Created {
            initial_description: domain::Description::new("Hello!").unwrap(),
        }));
        events.push(TodoEvent::ReminderUpdated(events::ReminderUpdated {
            new_reminder: Some(domain::Reminder::new(reminder_time, epoch).unwrap()),
        }));
        events.push(TodoEvent::DescriptionUpdated(events::DescriptionUpdated {
            new_description: domain::Description::new("New text").unwrap(),
        }));
        events.push(TodoEvent::Created(events::Created {
            initial_description: domain::Description::new("Ignored!").unwrap(),
        }));
        events.push(TodoEvent::ReminderUpdated(events::ReminderUpdated {
            new_reminder: None,
        }));

        let store = TodoStore::new(conn);

        if let Err(err) = store.create_tables() {
            eprintln!("Error preparing tables: {:?}", err);
        }

        let metadata = TodoMetadata {
            initiated_by: String::from("prefill"),
        };

        store.append_events(id, &events, None, metadata).unwrap();

        let quick_aggregate = TodoAggregate::Created(TodoData {
            description: domain::Description::new("Hello!").unwrap(),
            reminder: None,
            status: TodoStatus::NotCompleted,
        });

        store
            .persist_snapshot(id, &quick_aggregate, Version::new(2), None)
            .unwrap();
    }
}
