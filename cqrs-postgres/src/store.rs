use std::{fmt, marker::PhantomData};
use cqrs_core::{Aggregate, SerializableEvent, EventNumber, EventSource, EventSink, Precondition, VersionedEvent, Since, Version, EventDeserializeError, SnapshotSink, SnapshotSource, PersistableAggregate, VersionedAggregate, VersionedAggregateView};
use fallible_iterator::FallibleIterator;
use postgres::Connection;
use error::{LoadError, PersistError};
use util::RawJson;

pub struct PostgresStore<'conn, A>
where A: Aggregate,
{
    conn: &'conn Connection,
    _phantom: PhantomData<A>,
}

impl<'conn, A> fmt::Debug for PostgresStore<'conn, A>
where A: Aggregate,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("PostgresStore")
            .field("conn", &*self.conn)
            .finish()
    }
}

impl<'conn, A> PostgresStore<'conn, A>
    where A: Aggregate,
{
    pub fn new(conn: &'conn Connection) -> Self {
        PostgresStore {
            conn,
            _phantom: PhantomData,
        }
    }
}

// CREATE TABLE events (
//   event_id bigserial NOT NULL PRIMARY KEY,
//   entity_type text NOT NULL,
//   entity_id text NOT NULL,
//   sequence bigint CHECK (sequence > 0) NOT NULL,
//   event_type text NOT NULL,
//   payload jsonb NOT NULL,
//   timestamp timestamp with time zone DEFAULT (CURRENT_TIMESTAMP),
//   metadata jsonb,
//   UNIQUE (entity_type, entity_id, sequence)
// );

impl<'conn, A> EventSink<A> for PostgresStore<'conn, A>
where
    A: Aggregate,
    A::Event: SerializableEvent,
{
    type Error = PersistError;

    fn append_events(&self, id: &str, events: &[A::Event], precondition: Option<Precondition>) -> Result<EventNumber, Self::Error> {
        let trans = self.conn.transaction()?;

        let check_stmt = trans.prepare_cached("SELECT MAX(sequence) FROM events WHERE entity_type = $1 AND entity_id = $2")?;

        let result = check_stmt.query(&[&A::entity_type(), &id])?;
        let current_version = result.iter().next().and_then(|r| {
            let max_sequence: Option<i64> = r.get(0);
            max_sequence.map(|x| {
                Version::new(x as u64)
            })
        });

        trace!("entity {}: current version: {:?}", id, current_version);

        if events.len() == 0 {
            return Ok(current_version.unwrap_or_default().next_event())
        }

        if let Some(precondition) = precondition {
            precondition.verify(current_version)?;
        }

        trace!("entity {}: precondition satisfied", id);

        let first_sequence = current_version.unwrap_or_default().next_event();
        let mut next_sequence = Version::Number(first_sequence);

        let stmt = trans.prepare_cached("INSERT INTO events (entity_type, entity_id, sequence, event_type, payload, timestamp) VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP)")?;
        let mut payload_buffer = RawJson(Vec::new());
        for event in events {
            event.serialize_in_place(&mut payload_buffer.0);
            let _modified_count = stmt.execute(&[&A::entity_type(), &id, &(next_sequence.get() as i64), &event.event_type(), &payload_buffer])?;
            trace!("entity {}: inserted event; sequence: {}, payload: {}", id, next_sequence, payload_buffer);
            next_sequence = next_sequence.incr();
            payload_buffer.0.clear();
        }

        trans.commit()?;

        Ok(first_sequence)
    }
}

impl<'conn, A> EventSource<A> for PostgresStore<'conn, A>
where
    A: Aggregate,
    A::Event: SerializableEvent + 'static,
{
    type Events = Vec<Result<VersionedEvent<A::Event>, Self::Error>>;
    type Error = LoadError<EventDeserializeError<A::Event>>;

    fn read_events(&self, id: &str, since: Since, max_count: Option<u64>) -> Result<Option<Self::Events>, Self::Error> {
        let last_sequence = match since {
            cqrs_core::Since::BeginningOfStream => 0,
            cqrs_core::Since::Event(x) => x.get(),
        } as i64;

        let events;
        let trans = self.conn.transaction_with(postgres::transaction::Config::default().read_only(true))?;

        let handle_row = |row: postgres::rows::Row| {
            let event_type: String = row.get("event_type");
            let sequence: i64 = row.get("sequence");
            let raw: RawJson = row.get("payload");
            trace!("entity {}: loaded event; sequence: {}, type: {}, payload: {}", id, sequence, event_type, raw);
            Ok(cqrs_core::VersionedEvent {
                sequence: EventNumber::new(sequence as u64).expect("Sequence number should be non-zero"),
                event: A::Event::deserialize(&event_type, &raw.0).map_err(LoadError::Deserialize)?,
            })
        };

        let stmt;
        {
            let mut rows;
            if let Some(max_count) = max_count {
                stmt = trans.prepare_cached("SELECT sequence, event_type, payload FROM events WHERE entity_type = $1 AND entity_id = $2 AND sequence > $3 ORDER BY sequence ASC LIMIT $4")?;
                rows = stmt.lazy_query(&trans, &[&A::entity_type(), &id, &last_sequence, &(max_count.max(i64::max_value() as u64) as i64)], 100)?;
            } else {
                stmt = trans.prepare_cached("SELECT sequence, event_type, payload FROM events WHERE entity_type = $1 AND entity_id = $2 AND sequence > $3 ORDER BY sequence ASC")?;
                rows = stmt.lazy_query(&trans, &[&A::entity_type(), &id, &last_sequence], 100)?;
            }

            let (lower, upper) = rows.size_hint();
            let cap = upper.unwrap_or(lower);
            let mut inner_events = Vec::with_capacity(cap);

            while let Some(row) = rows.next()? {
                inner_events.push(handle_row(row));
            }
            events = inner_events;
        }

        trans.commit()?;

        trace!("entity {}: read {} events", id, events.len());

        Ok(Some(events))
    }
}

// CREATE TABLE snapshots (
//   snapshot_id bigserial NOT NULL PRIMARY KEY,
//   entity_type text NOT NULL,
//   entity_id text NOT NULL,
//   sequence bigint CHECK (sequence >= 0) NOT NULL,
//   payload jsonb NOT NULL,
//   UNIQUE (entity_type, entity_id, sequence)
// );

impl<'conn, A> SnapshotSink<A> for PostgresStore<'conn, A>
where
    A: PersistableAggregate,
{
    type Error = PersistError;

    fn persist_snapshot(&self, id: &str, aggregate: VersionedAggregateView<A>) -> Result<(), Self::Error> {
        let stmt = self.conn.prepare_cached("INSERT INTO snapshots (entity_type, entity_id, sequence, payload) VALUES ($1, $2, $3, $4)")?;
        let raw = RawJson(aggregate.payload.snapshot());
        let _modified_count = stmt.execute(&[&A::entity_type(), &id, &(aggregate.version.get() as i64), &raw])?;
        trace!("entity {}: persisted snapshot; payload: {}", id, raw);
        Ok(())
    }
}

impl<'conn, A> SnapshotSource<A> for PostgresStore<'conn, A>
where
    A: PersistableAggregate,
{
    type Error = LoadError<A::SnapshotError>;

    fn get_snapshot(&self, id: &str) -> Result<Option<VersionedAggregate<A>>, Self::Error> {
        let stmt = self.conn.prepare_cached("SELECT sequence, payload FROM snapshots WHERE entity_type = $1 AND entity_id = $2 ORDER BY sequence DESC LIMIT 1")?;
        let rows = stmt.query(&[&A::entity_type(), &id])?;
        if let Some(row) = rows.iter().next() {
            let sequence: i64 = row.get("sequence");
            let raw: RawJson = row.get("payload");
            trace!("entity {}: loaded snapshot; payload: {}", id, raw);
            Ok(Some(VersionedAggregate {
                version: Version::new(sequence as u64),
                payload: A::restore(&raw.0).map_err(LoadError::Deserialize)?,
            }))
        } else {
            trace!("entity {}: no snapshot found", id);
            Ok(None)
        }
    }
}