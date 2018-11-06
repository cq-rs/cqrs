extern crate cqrs;
extern crate cqrs_data;
extern crate fallible_iterator;
extern crate postgres;
extern crate serde;
extern crate serde_json;

use fallible_iterator::FallibleIterator;
use postgres::Connection;

pub struct PostgresStore<'a, 'b> {
    conn: &'a Connection,
    entity_type: &'b str
}

impl<'a, 'b> PostgresStore<'a, 'b> {
    pub fn new(conn: &'a Connection, entity_type: &'b str) -> Self {
        PostgresStore {
            conn,
            entity_type,
        }
    }
}

#[derive(Debug)]
pub enum StoreError {
    Postgres(postgres::Error),
    Serde(serde_json::Error),
    PreconditionFailed(cqrs::Precondition),
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

impl<'a, 'b, E: serde::Serialize> cqrs_data::event::Store<E> for PostgresStore<'a, 'b> {
    type AggregateId = str;
    type Error = StoreError;

    fn append_events(&self, agg_id: &Self::AggregateId, events: &[E], precondition: Option<cqrs::Precondition>) -> Result<cqrs::EventNumber, Self::Error> {
        let trans = self.conn.transaction()?;

        let check_stmt = trans.prepare_cached("SELECT MAX(sequence) FROM Events WHERE entity_type = $1 AND entity_id = $2")?;

        let result = check_stmt.query(&[&self.entity_type, &agg_id])?;
        let current_verison = result.iter().next().and_then(|r| {
            let max_sequence: Option<i64> = r.get(0);
            max_sequence.map(|x| {
                cqrs::Version::new(x as u64)
            })
        });

        if let Some(precondition) = precondition {
            precondition.verify(current_verison)?;
        }

        let first_sequence = current_verison.unwrap_or_default().incr();
        let mut next_sequence = first_sequence;

        let stmt = trans.prepare_cached("INSERT INTO events (entity_type, entity_id, sequence, event_payload) VALUES ($1, $2, $3, $4)")?;
        for event in events {
            let value = serde_json::to_value(event)?;
            let _modified_count = stmt.execute(&[&self.entity_type, &agg_id, &(next_sequence.get() as i64), &value])?;
            next_sequence = next_sequence.incr();
        }

        Ok(first_sequence.event_number().expect("valid event number"))
    }
}

impl<'a, 'b, E: serde::de::DeserializeOwned> cqrs_data::event::Source<E> for PostgresStore<'a, 'b> {
    type AggregateId = str;
    type Events = Vec<Result<cqrs::SequencedEvent<E>, StoreError>>;
    type Error = StoreError;

    fn read_events(&self, agg_id: &Self::AggregateId, since: cqrs_data::Since) -> Result<Option<Self::Events>, Self::Error> {
        let last_sequence = match since {
            cqrs_data::Since::BeginningOfStream => 0,
            cqrs_data::Since::Event(x) => x.get(),
        } as i64;

        let trans = self.conn.transaction_with(postgres::transaction::Config::default().read_only(true))?;
        let stmt = trans.prepare_cached("SELECT sequence, event_payload FROM events WHERE entity_type = $1 AND entity_id = $2 AND sequence > $3 ORDER BY sequence ASC")?;
        let mut rows = stmt.lazy_query(&trans, &[&self.entity_type, &agg_id, &last_sequence], 100)?;
        let (lower, upper) = rows.size_hint();
        let cap = upper.unwrap_or(lower);
        let mut events = Vec::with_capacity(cap);

        let handle_row = |row: postgres::rows::Row| {
            let sequence: i64 = row.get("sequence");
            let event_payload: serde_json::Value = row.get("event_payload");
            let event: E = serde_json::from_value(event_payload)?;
            Ok(cqrs::SequencedEvent {
                sequence: cqrs::EventNumber::new(sequence as usize).expect("Sequence number should be non-zero"),
                event,
            })
        };

        while let Some(row) = rows.next()? {
            events.push(handle_row(row));
        }

        Ok(Some(events))
    }
}

impl<'a, 'b, S: serde::Serialize> cqrs_data::state::Store<S> for PostgresStore<'a, 'b> {
    type AggregateId = str;
    type Error = StoreError;

    fn persist_snapshot(&self, agg_id: &Self::AggregateId, snapshot: cqrs::StateSnapshot<S>) -> Result<(), Self::Error> {
        let stmt = self.conn.prepare_cached("INSERT INTO snapshots (entity_type, entity_id, sequence, snapshot_payload) VALUES ($1, $2, $3, $4)")?;
        let value = serde_json::to_value(snapshot.snapshot)?;
        let _modified_count = stmt.execute(&[&self.entity_type, &agg_id, &(snapshot.version.get() as i64), &value])?;
        Ok(())
    }
}

impl<'a, 'b, S: serde::de::DeserializeOwned> cqrs_data::state::Source<S> for PostgresStore<'a, 'b> {
    type AggregateId = str;
    type Error = StoreError;

    fn get_snapshot(&self, agg_id: &Self::AggregateId) -> Result<Option<cqrs::StateSnapshot<S>>, Self::Error> {
        let stmt = self.conn.prepare_cached("SELECT sequence, snapshot_payload FROM snapshots WHERE entity_type = $1 AND entity_id = $2 ORDER BY sequence DESC LIMIT 1")?;
        let rows = stmt.query(&[&self.entity_type, &agg_id])?;
        if let Some(row) = rows.iter().next() {
            let sequence: i64 = row.get("sequence");
            let payload: serde_json::Value = row.get("snapshot_payload");
            let snapshot: S = serde_json::from_value(payload)?;
            Ok(Some(cqrs::StateSnapshot {
                version: cqrs::Version::new(sequence as u64),
                snapshot,
            }))
        } else {
            Ok(None)
        }
    }
}