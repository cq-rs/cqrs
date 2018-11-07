extern crate cqrs;
extern crate cqrs_data;
extern crate fallible_iterator;
extern crate postgres;
extern crate serde;
extern crate serde_json;

use std::fmt;
use std::marker::PhantomData;
use cqrs_data::{EventSource, EventSink, SnapshotSource, SnapshotSink, Since};
use fallible_iterator::FallibleIterator;
use postgres::Connection;

pub struct PostgresStore<'conn, A>
where A: cqrs::Aggregate,
{
    conn: &'conn Connection,
    _phantom: PhantomData<A>,
}

impl<'conn, A> PostgresStore<'conn, A>
where A: cqrs::Aggregate,
{
    pub fn new(conn: &'conn Connection) -> Self {
        PostgresStore {
            conn,
            _phantom: PhantomData,
        }
    }
}

#[derive(Debug)]
pub enum StoreError {
    Postgres(postgres::Error),
    Serde(serde_json::Error),
    PreconditionFailed(cqrs::Precondition),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            StoreError::Postgres(ref e) => write!(f, "postgres error: {}", e),
            StoreError::Serde(ref e) => write!(f, "serde error: {}", e),
            StoreError::PreconditionFailed(ref e) => write!(f, "precondition error: {}", e),
        }
    }
}

impl From<postgres::Error> for StoreError {
    fn from(err: postgres::Error) -> Self {
        StoreError::Postgres(err)
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(err: serde_json::Error) -> Self {
        StoreError::Serde(err)
    }
}

impl From<cqrs::Precondition> for StoreError {
    fn from(precondition: cqrs::Precondition) -> Self {
        StoreError::PreconditionFailed(precondition)
    }
}

// CREATE TABLE events (
//   event_id bigserial NOT NULL PRIMARY KEY,
//   entity_type text NOT NULL,
//   entity_id text NOT NULL,
//   sequence bigint CHECK (sequence > 0) NOT NULL,
//   event_payload jsonb NOT NULL,
//   timestamp timestamp with time zone DEFAULT (CURRENT_TIMESTAMP),
//   UNIQUE (entity_type, entity_id, sequence)
// );

// CREATE TABLE snapshots (
//   snapshot_id bigserial NOT NULL PRIMARY KEY,
//   entity_type text NOT NULL,
//   entity_id text NOT NULL,
//   sequence bigint CHECK (sequence >= 0) NOT NULL,
//   snapshot_payload jsonb NOT NULL,
//   UNIQUE (entity_type, entity_id, sequence)
// );

impl<'conn, A> EventSink<A> for PostgresStore<'conn, A>
where
    A: cqrs::Aggregate,
    A::Event: serde::Serialize,
{
    type Error = StoreError;

    fn append_events<Id: AsRef<str> + Into<String>>(&self, id: Id, events: &[A::Event], precondition: Option<cqrs::Precondition>) -> Result<cqrs::EventNumber, Self::Error> {
        let trans = self.conn.transaction()?;

        let check_stmt = trans.prepare_cached("SELECT MAX(sequence) FROM events WHERE entity_type = $1 AND entity_id = $2")?;

        let result = check_stmt.query(&[&A::entity_type(), &id.as_ref()])?;
        let current_version = result.iter().next().and_then(|r| {
            let max_sequence: Option<i64> = r.get(0);
            max_sequence.map(|x| {
                cqrs::Version::new(x as u64)
            })
        });

        println!("Current version: {:?}", current_version);

        if let Some(precondition) = precondition {
            precondition.verify(current_version)?;
        }

        let first_sequence = current_version.unwrap_or_default().incr();
        let mut next_sequence = first_sequence;

        let stmt = trans.prepare_cached("INSERT INTO events (entity_type, entity_id, sequence, event_payload, timestamp) VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP)")?;
        for event in events {
            let value = serde_json::to_value(event)?;
            let _modified_count = stmt.execute(&[&A::entity_type(), &id.as_ref(), &(next_sequence.get() as i64), &value])?;
            println!("Inserted event: {:?}", next_sequence);
            next_sequence = next_sequence.incr();
        }

        trans.commit()?;

        Ok(first_sequence.event_number().expect("valid event number"))
    }
}

impl<'conn, A> EventSource<A> for PostgresStore<'conn, A>
where
    A: cqrs::Aggregate,
    A::Event: serde::de::DeserializeOwned,
{
    type Events = Vec<Result<cqrs::SequencedEvent<A::Event>, StoreError>>;
    type Error = StoreError;

    fn read_events<Id: AsRef<str> + Into<String>>(&self, id: Id, since: Since) -> Result<Option<Self::Events>, Self::Error> {
        let last_sequence = match since {
            cqrs_data::Since::BeginningOfStream => 0,
            cqrs_data::Since::Event(x) => x.get(),
        } as i64;

        let events;
        let trans = self.conn.transaction_with(postgres::transaction::Config::default().read_only(true))?;
        let stmt = trans.prepare_cached("SELECT sequence, event_payload FROM events WHERE entity_type = $1 AND entity_id = $2 AND sequence > $3 ORDER BY sequence ASC")?;
        {
            let mut rows = stmt.lazy_query(&trans, &[&A::entity_type(), &id.as_ref(), &last_sequence], 100)?;
            let (lower, upper) = rows.size_hint();
            let cap = upper.unwrap_or(lower);
            let mut inner_events = Vec::with_capacity(cap);

            let handle_row = |row: postgres::rows::Row| {
                let sequence: i64 = row.get("sequence");
                let event_payload: serde_json::Value = row.get("event_payload");
                let event: A::Event = serde_json::from_value(event_payload)?;
                Ok(cqrs::SequencedEvent {
                    sequence: cqrs::EventNumber::new(sequence as u64).expect("Sequence number should be non-zero"),
                    event,
                })
            };

            while let Some(row) = rows.next()? {
                inner_events.push(handle_row(row));
            }
            events = inner_events;
        }

        trans.commit()?;

        Ok(Some(events))
    }
}

impl<'conn, A> SnapshotSink<A> for PostgresStore<'conn, A>
where
    A: cqrs::Aggregate + serde::Serialize,
{
    type Error = StoreError;

    fn persist_snapshot<Id: AsRef<str> + Into<String>>(&self, id: Id, snapshot: cqrs::StateSnapshot<A>) -> Result<(), Self::Error> {
        let stmt = self.conn.prepare_cached("INSERT INTO snapshots (entity_type, entity_id, sequence, snapshot_payload) VALUES ($1, $2, $3, $4)")?;
        let value = serde_json::to_value(snapshot.snapshot)?;
        let _modified_count = stmt.execute(&[&A::entity_type(), &id.as_ref(), &(snapshot.version.get() as i64), &value])?;
        Ok(())
    }
}

impl<'conn, A> SnapshotSource<A> for PostgresStore<'conn, A>
where
    A: cqrs::Aggregate + serde::de::DeserializeOwned,
{
    type Error = StoreError;

    fn get_snapshot<Id: AsRef<str> + Into<String>>(&self, id: Id) -> Result<Option<cqrs::StateSnapshot<A>>, Self::Error> {
        let stmt = self.conn.prepare_cached("SELECT sequence, snapshot_payload FROM snapshots WHERE entity_type = $1 AND entity_id = $2 ORDER BY sequence DESC LIMIT 1")?;
        let rows = stmt.query(&[&A::entity_type(), &id.as_ref()])?;
        if let Some(row) = rows.iter().next() {
            let sequence: i64 = row.get("sequence");
            let payload: serde_json::Value = row.get("snapshot_payload");
            let snapshot: A = serde_json::from_value(payload)?;
            Ok(Some(cqrs::StateSnapshot {
                version: cqrs::Version::new(sequence as u64),
                snapshot,
            }))
        } else {
            Ok(None)
        }
    }
}