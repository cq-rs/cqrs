use crate::{NewConn, error::{LoadError, PersistError}, util::{BorrowedJson, RawJsonPersist, Sequence}};
use cqrs_core::{
    Aggregate, AggregateEvent, AggregateId, Before, DeserializableEvent, EventNumber, EventSink,
    EventSource, NeverSnapshot, Precondition, SerializableEvent, Since, SnapshotRecommendation,
    SnapshotSink, SnapshotSource, SnapshotStrategy, Version, VersionedAggregate, VersionedEvent,
    VersionedEventWithMetadata,
};
use num_traits::FromPrimitive;
use postgres::{Client, fallible_iterator::FallibleIterator};
use r2d2::PooledConnection;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use std::{fmt, marker::PhantomData, sync::{Arc, Mutex}};

/// A PostgreSQL storage backend.
#[derive(Clone)]
pub struct PostgresStore<A, E, M, S = NeverSnapshot>
where
    A: Aggregate,
    E: AggregateEvent<A>,
    S: SnapshotStrategy,
{
    conn: Arc<Mutex<PooledConnection<NewConn>>>,
    snapshot_strategy: S,
    _phantom: PhantomData<(A, E, M)>,
}

impl<A, E, M, S> fmt::Debug for PostgresStore<A, E, M, S>
where
    A: Aggregate,
    E: AggregateEvent<A>,
    S: SnapshotStrategy + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("PostgresStore")
            .field("strategy", &self.snapshot_strategy)
            .field("phantom", &self._phantom)
            .finish()
    }
}

impl<A, E, M, S> PostgresStore<A, E, M, S>
where
    A: Aggregate,
    E: AggregateEvent<A>,
    S: SnapshotStrategy + Default,
{
    const DB_VERSION: u32 = 1;

    /// Constructs a transient store based on a provided PostgreSQL connection using the default snapshot strategy.
    pub fn new(conn: PooledConnection<NewConn>) -> Self {
        PostgresStore {
            conn: Arc::new(Mutex::new(conn)),
            snapshot_strategy: S::default(),
            _phantom: PhantomData,
        }
    }

    /// Constructs a transient store based on a provided PostgreSQL connection and snapshot strategy.
    pub fn with_snapshot_strategy(conn: PooledConnection<NewConn>, snapshot_strategy: S) -> Self {
        PostgresStore {
            conn: Arc::new(Mutex::new(conn)),
            snapshot_strategy,
            _phantom: PhantomData,
        }
    }

    /// Creates the base set of tables required to support the CQRS system.
    pub fn create_tables(&self) -> Result<(), postgres::Error> {
        let mut conn = self.conn.lock().unwrap();
        
        conn.batch_execute(include_str!("migrations/00_create_migrations.sql"))?;

        let current_version: i32 = conn
            .query("SELECT MAX(version) from migrations", &[])?
            .iter()
            .next()
            .and_then(|r| r.get(0))
            .unwrap_or_default();

        if current_version < 1 {
            conn.batch_execute(include_str!("migrations/01_create_tables.sql"))?;
        }

        Ok(())
    }

    /// Checks to see if the database is the latest version as seen by the current executable..
    pub fn is_latest(&self) -> Result<bool, postgres::Error> {
        let current_version: i32 = self
            .conn
            .lock()
            .unwrap()
            .query("SELECT MAX(version) from migrations", &[])?
            .iter()
            .next()
            .and_then(|r| r.get(0))
            .unwrap_or_default();

        Ok(Self::DB_VERSION == current_version as u32)
    }

    /// Checks to see if the database is compatible with the current executable.
    pub fn is_compatible(&self) -> Result<bool, postgres::Error> {
        let current_version: i32 = self
            .conn
            .lock()
            .unwrap()
            .query("SELECT MAX(version) from migrations", &[])?
            .iter()
            .next()
            .and_then(|r| r.get(0))
            .unwrap_or_default();

        Ok(Self::DB_VERSION >= current_version as u32)
    }

    /// Gets the total number of entities of this type in the store.
    pub fn get_entity_count(&self) -> Result<u64, postgres::Error> {
        let mut conn = self.conn.lock().unwrap();

        let stmt = conn.prepare(
            "SELECT COUNT(DISTINCT entity_id) \
             FROM events \
             WHERE aggregate_type = $1",
        )?;
        let rows = conn.query(&stmt, &[&A::aggregate_type()])?;
        Ok(rows
            .iter()
            .next()
            .map(|r| r.get::<_, i64>(0) as u64)
            .unwrap_or_default())
    }

    /// Loads a page of entity IDs.
    pub fn get_entity_ids(&self, offset: u32, limit: u32) -> Result<Vec<String>, postgres::Error> {
        let mut conn = self.conn.lock().unwrap();
        let stmt = conn.prepare(
            "SELECT DISTINCT entity_id \
             FROM events \
             WHERE aggregate_type = $1 \
             OFFSET $2 LIMIT $3",
        )?;
        let rows = conn.query(&stmt, &[
            &A::aggregate_type(),
            &(i64::from(offset)),
            &(i64::from(limit)),
        ])?;
        Ok(rows.iter().map(|r| r.get(0)).collect())
    }

    /// Gets the total number of entities of this type matching a particular PostgreSQL pattern in the store.
    ///
    /// PostgreSQL pattern matching rules:
    ///
    /// * `_` matches any single character.
    /// * `%` matches any number of characters.
    ///
    /// See the [PostgreSQL documentation on pattern matching](https://www.postgresql.org/docs/current/functions-matching.html#FUNCTIONS-LIKE)
    pub fn get_entity_count_matching_pattern(&self, pattern: &str) -> Result<u64, postgres::Error> {
        let mut conn = self.conn.lock().unwrap();

        let stmt = conn.prepare(
            "SELECT COUNT(DISTINCT entity_id) \
             FROM events \
             WHERE aggregate_type = $1 AND entity_id LIKE $2",
        )?;
        let rows = conn.query(&stmt, &[&A::aggregate_type(), &pattern])?;
        Ok(rows
            .iter()
            .next()
            .map(|r| r.get::<_, i64>(0) as u64)
            .unwrap_or_default())
    }

    /// Loads a page of entity IDs matching a particular PostgreSQL pattern.
    ///
    /// PostgreSQL pattern matching rules:
    ///
    /// * `_` matches any single character.
    /// * `%` matches any number of characters.
    ///
    /// See the [PostgreSQL documentation on pattern matching](https://www.postgresql.org/docs/current/functions-matching.html#FUNCTIONS-LIKE)
    pub fn get_entity_ids_matching_pattern(
        &self,
        pattern: &str,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<String>, postgres::Error> {
        let mut conn = self.conn.lock().unwrap();

        let stmt = conn.prepare(
            "SELECT DISTINCT entity_id \
             FROM events \
             WHERE aggregate_type = $1 AND entity_id LIKE $2 \
             OFFSET $3 LIMIT $4",
        )?;
        let rows = conn.query(&stmt, &[
            &A::aggregate_type(),
            &pattern,
            &(i64::from(offset)),
            &(i64::from(limit)),
        ])?;
        Ok(rows.iter().map(|r| r.get(0)).collect())
    }

    /// Gets the total number of entities of this type matching a particular PostgreSQL regular expression in the store.
    ///
    /// See the [PostgreSQL documentation on pattern matching](https://www.postgresql.org/docs/current/functions-matching.html#FUNCTIONS-SIMILARTO-REGEXP)
    pub fn get_entity_count_matching_sql_regex(&self, regex: &str) -> Result<u64, postgres::Error> {
        let mut conn = self.conn.lock().unwrap();

        let stmt = conn.prepare(
            "SELECT COUNT(DISTINCT entity_id) \
             FROM events \
             WHERE aggregate_type = $1 AND entity_id SIMILAR TO $2",
        )?;
        let rows = conn.query(&stmt, &[&A::aggregate_type(), &regex])?;
        Ok(rows
            .iter()
            .next()
            .map(|r| r.get::<_, i64>(0) as u64)
            .unwrap_or_default())
    }

    /// Loads a page of entity IDs matching a particular PostgreSQL regular expression.
    ///
    /// See the [PostgreSQL documentation on pattern matching](https://www.postgresql.org/docs/current/functions-matching.html#FUNCTIONS-SIMILARTO-REGEXP)
    pub fn get_entity_ids_matching_sql_regex(
        &self,
        regex: &str,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<String>, postgres::Error> {
        let mut conn = self.conn.lock().unwrap();

        let stmt = conn.prepare(
            "SELECT DISTINCT entity_id \
             FROM events \
             WHERE aggregate_type = $1 AND entity_id SIMILAR TO $2 \
             OFFSET $3 LIMIT $4",
        )?;
        let rows = conn.query(&stmt, &[
            &A::aggregate_type(),
            &regex,
            &(i64::from(offset)),
            &(i64::from(limit)),
        ])?;
        Ok(rows.iter().map(|r| r.get(0)).collect())
    }

    /// Gets the total number of entities of this type matching a particular POSIX regular expression in the store.
    ///
    /// See the [PostgreSQL documentation on pattern matching](https://www.postgresql.org/docs/current/functions-matching.html#FUNCTIONS-POSIX-REGEXP)
    pub fn get_entity_count_matching_posix_regex(
        &self,
        regex: &str,
    ) -> Result<u64, postgres::Error> {
        let mut conn = self.conn.lock().unwrap();

        let stmt = conn.prepare(
            "SELECT COUNT(DISTINCT entity_id) \
             FROM events \
             WHERE aggregate_type = $1 AND entity_id ~ $2",
        )?;
        let rows = conn.query(&stmt, &[&A::aggregate_type(), &regex])?;
        Ok(rows
            .iter()
            .next()
            .map(|r| r.get::<_, i64>(0) as u64)
            .unwrap_or_default())
    }

    /// Loads a page of entity IDs matching a particular POSIX regular expression.
    ///
    /// See the [PostgreSQL documentation on pattern matching](https://www.postgresql.org/docs/current/functions-matching.html#FUNCTIONS-POSIX-REGEXP)
    pub fn get_entity_ids_matching_posix_regex(
        &self,
        regex: &str,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<String>, postgres::Error> {
        let mut conn = self.conn.lock().unwrap();

        let stmt = conn.prepare(
            "SELECT DISTINCT entity_id \
             FROM events \
             WHERE aggregate_type = $1 AND entity_id ~ $2 \
             OFFSET $3 LIMIT $4",
        )?;
        let rows = conn.query(&stmt, &[
            &A::aggregate_type(),
            &regex,
            &(i64::from(offset)),
            &(i64::from(limit)),
        ])?;
        Ok(rows.iter().map(|r| r.get(0)).collect())
    }

    /// Reads events and associated metadata from the event source for a given identifier.
    ///
    /// Only loads events after the event number provided in `since` (See [Since]), and will only load a maximum of
    /// `max_count` events, if given. If not given, will read all remaining events.
    pub fn read_events_with_metadata<I>(
        &self,
        id: &I,
        since: Since,
        max_count: Option<u64>,
    ) -> Result<
        Option<Vec<Result<VersionedEventWithMetadata<E, M>, LoadError<E::Error>>>>,
        LoadError<E::Error>,
    >
    where
        I: AggregateId<A>,
        E: DeserializableEvent,
        M: for<'de> serde::Deserialize<'de>,
    {
        let last_sequence = match since {
            cqrs_core::Since::BeginningOfStream => 0,
            cqrs_core::Since::Event(x) => x.get(),
        } as i64;

        let events;
        let mut conn = self.conn.lock().unwrap();
        let mut trans = conn.build_transaction().read_only(true).start()?;

        let handle_row = |row: postgres::Row| {
            let sequence: Sequence = row.get(0);
            let event_type: String = row.get(1);
            let raw: Vec<u8> = row.get(2);
            let metadata: Value = row.get(3);
            let event = E::deserialize_event_from_buffer(&raw, &event_type)
                .map_err(LoadError::DeserializationError)?
                .ok_or_else(|| LoadError::UnknownEventType(event_type.clone()))?;
            log::trace!(
                "entity {}: loaded event; sequence: {}, type: {}",
                id.as_str(),
                sequence.0,
                event_type
            );
            Ok(VersionedEventWithMetadata {
                sequence: sequence.0,
                event,
                metadata: serde_json::from_value(metadata).unwrap(),
            })
        };

        let stmt;
        {
            let mut rows;
            if let Some(max_count) = max_count {
                stmt = trans.prepare(
                    "SELECT sequence, event_type, payload, metadata \
                     FROM events \
                     WHERE aggregate_type = $1 AND entity_id = $2 AND sequence > $3 \
                     ORDER BY sequence ASC \
                     LIMIT $4",
                )?;

                let portal = trans.bind(&stmt, &[
                    &A::aggregate_type(),
                    &id.as_str(),
                    &last_sequence,
                    &(max_count.min(i64::max_value() as u64) as i64),
                ])?;
    
                rows = trans.query_portal_raw(&portal, 0)?;
            } else {
                stmt = trans.prepare(
                    "SELECT sequence, event_type, payload, metadata \
                     FROM events \
                     WHERE aggregate_type = $1 AND entity_id = $2 AND sequence > $3 \
                     ORDER BY sequence ASC",
                )?;

                let portal = trans.bind(&stmt,  &[&A::aggregate_type(), &id.as_str(), &last_sequence])?;
    
                rows = trans.query_portal_raw(&portal, 0)?;
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

        log::trace!("entity {}: read {} events", id.as_str(), events.len());

        Ok(Some(events))
    }

    /// Reads events and associated metadata from the event source for a given identifier going
    /// backward in sequence.
    ///
    /// Only loads events before the event number provided in `before` (See [Before]), and will only load a maximum of
    /// `max_count` events, if given. If not given, will read all remaining events.
    pub fn read_events_reverse_with_metadata<I>(
        &self,
        id: &I,
        before: Before,
        max_count: Option<u64>,
    ) -> Result<
        Option<Vec<Result<VersionedEventWithMetadata<E, M>, LoadError<E::Error>>>>,
        LoadError<E::Error>,
    >
    where
        I: AggregateId<A>,
        E: DeserializableEvent,
        M: for<'de> serde::Deserialize<'de>,
    {
        let last_sequence = match before {
            Before::EndOfStream => std::i64::MAX,
            Before::Event(x) => x.get() as i64,
        };

        let events;
        let mut conn = self.conn.lock().unwrap();
        let mut trans = conn.build_transaction().read_only(true).start()?;

        let handle_row = |row: postgres::Row| {
            let sequence: Sequence = row.get(0);
            let event_type: String = row.get(1);
            let raw: Vec<u8> = row.get(2);
            let metadata: Value = row.get(3);
            let event = E::deserialize_event_from_buffer(&raw, &event_type)
                .map_err(LoadError::DeserializationError)?
                .ok_or_else(|| LoadError::UnknownEventType(event_type.clone()))?;
            log::trace!(
                "entity {}: loaded event; sequence: {}, type: {}",
                id.as_str(),
                sequence.0,
                event_type
            );
            Ok(VersionedEventWithMetadata {
                sequence: sequence.0,
                event,
                metadata: serde_json::from_value(metadata).unwrap(),
            })
        };

        let stmt;
        {
            let mut rows;
            if let Some(max_count) = max_count {
                stmt = trans.prepare(
                    "SELECT sequence, event_type, payload, metadata \
                     FROM events \
                     WHERE aggregate_type = $1 AND entity_id = $2 AND sequence < $3 \
                     ORDER BY sequence DESC \
                     LIMIT $4",
                )?;

                let portal = trans.bind(&stmt, &[
                    &A::aggregate_type(),
                    &id.as_str(),
                    &last_sequence,
                    &(max_count.min(i64::max_value() as u64) as i64),
                ])?;
    
                rows = trans.query_portal_raw(&portal, 0)?;
            } else {
                stmt = trans.prepare(
                    "SELECT sequence, event_type, payload, metadata \
                     FROM events \
                     WHERE aggregate_type = $1 AND entity_id = $2 AND sequence < $3 \
                     ORDER BY sequence DESC",
                )?;

                let portal = trans.bind(&stmt, &[&A::aggregate_type(), &id.as_str(), &last_sequence])?;
    
                rows = trans.query_portal_raw(&portal, 0)?;
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

        log::trace!("entity {}: read {} events", id.as_str(), events.len());

        Ok(Some(events))
    }
}

impl<A, E, M, S> EventSink<A, E, M> for PostgresStore<A, E, M, S>
where
    A: Aggregate,
    E: AggregateEvent<A> + SerializableEvent + fmt::Debug,
    M: Serialize + fmt::Debug + Sync,
    S: SnapshotStrategy,
{
    type Error = PersistError<<E as SerializableEvent>::Error>;

    fn append_events<I>(
        &self,
        id: &I,
        events: &[E],
        precondition: Option<Precondition>,
        metadata: M,
    ) -> Result<EventNumber, Self::Error>
    where
        I: AggregateId<A>,
    {
        let mut conn = self.conn.lock().unwrap();
        let mut trans = conn.transaction()?;

        let check_stmt = trans.prepare(
            "SELECT MAX(sequence) FROM events WHERE aggregate_type = $1 AND entity_id = $2",
        )?;

        let result = trans.query(&check_stmt, &[&A::aggregate_type(), &id.as_str()])?;

        let current_version = result.iter().next().and_then(|r| {
            let max_sequence: Option<Sequence> = r.get(0);
            max_sequence.map(|x| Version::from(x.0))
        });

        log::trace!(
            "entity {}: current version: {:?}",
            id.as_str(),
            current_version
        );

        if events.is_empty() {
            return Ok(current_version.unwrap_or_default().next_event());
        }

        if let Some(precondition) = precondition {
            precondition.verify(current_version)?;
        }

        log::trace!("entity {}: precondition satisfied", id.as_str());

        let first_sequence = current_version.unwrap_or_default().next_event();
        let mut next_sequence = Version::Number(first_sequence);
        let mut buffer = Vec::with_capacity(128);

        let stmt = trans.prepare(
            "INSERT INTO events (aggregate_type, entity_id, sequence, event_type, payload, metadata, timestamp) \
            VALUES ($1, $2, $3, $4, $5, $6, CURRENT_TIMESTAMP)",
        )?;
        for event in events {
            buffer.clear();
            event
                .serialize_event_to_buffer(&mut buffer)
                .map_err(PersistError::SerializationError)?;
            let modified_count = trans.execute(&stmt, &[
                &A::aggregate_type(),
                &id.as_str(),
                &(next_sequence.get() as i64),
                &event.event_type(),
                &RawJsonPersist(&buffer),
                &BorrowedJson(&metadata),
            ])?;
            debug_assert!(modified_count > 0);
            log::trace!(
                "entity {}: inserted event; sequence: {}",
                id.as_str(),
                next_sequence
            );
            next_sequence.incr();
        }

        trans.commit()?;

        Ok(first_sequence)
    }
}

impl<A, E, M, S> EventSource<A, E> for PostgresStore<A, E, M, S>
where
    A: Aggregate,
    E: AggregateEvent<A> + DeserializableEvent,
    S: SnapshotStrategy,
{
    type Error = LoadError<<E as DeserializableEvent>::Error>;
    type Events = Vec<VersionedEvent<E>>;

    fn read_events<I>(
        &self,
        id: &I,
        since: Since,
        max_count: Option<u64>,
    ) -> Result<Option<Self::Events>, Self::Error>
    where
        I: AggregateId<A>,
    {
        let last_sequence = match since {
            cqrs_core::Since::BeginningOfStream => 0,
            cqrs_core::Since::Event(x) => x.get(),
        } as i64;

        let mut conn = self.conn.lock().unwrap();
        let mut trans = conn.build_transaction().read_only(true).start()?;

        let handle_row = |row: postgres::Row| {
            let sequence: Sequence = row.get(0);
            let event_type: String = row.get(1);
            let raw: Vec<u8> = row.get(2);
            let event = E::deserialize_event_from_buffer(&raw, &event_type)
                .map_err(LoadError::DeserializationError)?
                .ok_or_else(|| LoadError::UnknownEventType(event_type.clone()))?;
            log::trace!(
                "entity {}: loaded event; sequence: {}, type: {}",
                id.as_str(),
                sequence.0,
                event_type
            );
            Ok(VersionedEvent {
                sequence: sequence.0,
                event,
            })
        };

        let events: Vec<VersionedEvent<E>> =
            if let Some(max_count) = max_count.and_then(i64::from_u64) {
                let stmt = trans.prepare(
                    "SELECT sequence, event_type, payload \
                     FROM events \
                     WHERE aggregate_type = $1 AND entity_id = $2 AND sequence > $3 \
                     ORDER BY sequence ASC \
                     LIMIT $4",
                )?;

                let portal = trans.bind(&stmt, &[
                    &A::aggregate_type(),
                    &id.as_str(),
                    &last_sequence,
                    &max_count,
                ])?;
    
                let rows = trans.query_portal_raw(&portal, 0)?;

                rows
                    .map_err(LoadError::Postgres)
                    .map(handle_row)
                    .collect()?
            } else {
                let stmt = trans.prepare(
                    "SELECT sequence, event_type, payload \
                     FROM events \
                     WHERE aggregate_type = $1 AND entity_id = $2 AND sequence > $3 \
                     ORDER BY sequence ASC",
                )?;

                let portal = trans.bind(&stmt, &[&A::aggregate_type(), &id.as_str(), &last_sequence])?;
    
                let rows = trans.query_portal_raw(&portal, 0)?;

                rows
                    .map_err(LoadError::Postgres)
                    .map(handle_row)
                    .collect()?
            };

        trans.commit()?;

        log::trace!("entity {}: read {} events", id.as_str(), events.len());

        Ok(Some(events))
    }
}

impl<A, E, M, S> SnapshotSink<A> for PostgresStore<A, E, M, S>
where
    A: Aggregate + Serialize + fmt::Debug + Sync,
    E: AggregateEvent<A>,
    S: SnapshotStrategy,
{
    type Error = PersistError<serde_json::Error>;

    fn persist_snapshot<I>(
        &self,
        id: &I,
        aggregate: &A,
        version: Version,
        last_snapshot_version: Option<Version>,
    ) -> Result<Version, Self::Error>
    where
        I: AggregateId<A>,
    {
        if version <= last_snapshot_version.unwrap_or_default()
            || self
                .snapshot_strategy
                .snapshot_recommendation(version, last_snapshot_version)
                == SnapshotRecommendation::DoNotSnapshot
        {
            return Ok(last_snapshot_version.unwrap_or_default());
        }

        let mut conn = self.conn.lock().unwrap();
        let stmt = conn.prepare(
            "INSERT INTO snapshots (aggregate_type, entity_id, sequence, payload) \
             VALUES ($1, $2, $3, $4)",
        )?;
        let _modified_count = conn.execute(&stmt, &[
            &A::aggregate_type(),
            &id.as_str(),
            &(version.get() as i64),
            &serde_json::to_value(aggregate).unwrap(),
        ])?;

        // Clean up strategy for snapshots?
        //        let stmt = conn.prepare("DELETE FROM snapshots WHERE aggregate_type = $1 AND entity_id = $2 AND sequence < $3")?;
        //        let _modified_count = stmt.execute(&[&A::aggregate_type(), &id.as_str(), &(version.get() as i64)])?;

        log::trace!("entity {}: persisted snapshot", id.as_str());
        Ok(version)
    }
}

impl<A, E, M, S> SnapshotSource<A> for PostgresStore<A, E, M, S>
where
    A: Aggregate + DeserializeOwned,
    E: AggregateEvent<A>,
    S: SnapshotStrategy,
{
    type Error = postgres::Error;

    fn get_snapshot<I>(&self, id: &I) -> Result<Option<VersionedAggregate<A>>, Self::Error>
    where
        I: AggregateId<A>,
    {
        let mut conn = self.conn.lock().unwrap();
        let stmt = conn.prepare(
            "SELECT sequence, payload \
             FROM snapshots \
             WHERE aggregate_type = $1 AND entity_id = $2 \
             ORDER BY sequence DESC \
             LIMIT 1",
        )?;
        let rows = conn.query(&stmt, &[&A::aggregate_type(), &id.as_str()])?;
        if let Some(row) = rows.iter().next() {
            let sequence: Sequence = row.get(0);
            let raw: Value = row.get(1);
            log::trace!("entity {}: loaded snapshot", id.as_str());
            Ok(Some(VersionedAggregate {
                version: Version::from(sequence.0),
                payload: serde_json::from_value(raw).unwrap(),
            }))
        } else {
            log::trace!("entity {}: no snapshot found", id.as_str());
            Ok(None)
        }
    }
}
