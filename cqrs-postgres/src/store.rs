use crate::{
    error::{LoadError, PersistError},
    util::{BorrowedJson, Json, RawJsonPersist, RawJsonRead, Sequence},
};
use cqrs_core::{
    Aggregate, AggregateEvent, AggregateId, DeserializableEvent, EventNumber, EventSink,
    EventSource, NeverSnapshot, Precondition, SerializableEvent, Since, SnapshotRecommendation,
    SnapshotSink, SnapshotSource, SnapshotStrategy, Version, VersionedAggregate, VersionedEvent,
};
use fallible_iterator::FallibleIterator;
use postgres::Connection;
use serde::{de::DeserializeOwned, Serialize};
use std::{fmt, marker::PhantomData};

/// A PostgreSQL storage backend.
#[derive(Clone)]
pub struct PostgresStore<'conn, A, E, M, S = NeverSnapshot>
where
    A: Aggregate,
    E: AggregateEvent<A>,
    S: SnapshotStrategy,
{
    conn: &'conn Connection,
    snapshot_strategy: S,
    _phantom: PhantomData<&'conn (A, E, M)>,
}

impl<'conn, A, E, M, S> fmt::Debug for PostgresStore<'conn, A, E, M, S>
where
    A: Aggregate,
    E: AggregateEvent<A>,
    S: SnapshotStrategy + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("PostgresStore")
            .field("conn", &*self.conn)
            .field("strategy", &self.snapshot_strategy)
            .field("phantom", &self._phantom)
            .finish()
    }
}

impl<'conn, A, E, M, S> PostgresStore<'conn, A, E, M, S>
where
    A: Aggregate,
    E: AggregateEvent<A>,
    S: SnapshotStrategy + Default,
{
    const DB_VERSION: u32 = 1;

    /// Constructs a transient store based on a provided PostgreSQL connection using the default snapshot strategy.
    pub fn new(conn: &'conn Connection) -> Self {
        PostgresStore {
            conn,
            snapshot_strategy: S::default(),
            _phantom: PhantomData,
        }
    }

    /// Constructs a transient store based on a provided PostgreSQL connection and snapshot strategy.
    pub fn with_snapshot_strategy(conn: &'conn Connection, snapshot_strategy: S) -> Self {
        PostgresStore {
            conn,
            snapshot_strategy,
            _phantom: PhantomData,
        }
    }

    /// Creates the base set of tables required to support the CQRS system.
    pub fn create_tables(&self) -> Result<(), postgres::Error> {
        self.conn
            .batch_execute(include_str!("migrations/00_create_migrations.sql"))?;

        let current_version: i32 = self
            .conn
            .query("SELECT MAX(version) from migrations", &[])?
            .iter()
            .next()
            .and_then(|r| r.get(0))
            .unwrap_or_default();

        if current_version < 1 {
            self.conn
                .batch_execute(include_str!("migrations/01_create_tables.sql"))?;
        }

        Ok(())
    }

    /// Checks to see if the database is the latest version as seen by the current executable..
    pub fn is_latest(&self) -> Result<bool, postgres::Error> {
        let current_version: i32 = self
            .conn
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
            .query("SELECT MAX(version) from migrations", &[])?
            .iter()
            .next()
            .and_then(|r| r.get(0))
            .unwrap_or_default();

        Ok(Self::DB_VERSION >= current_version as u32)
    }

    /// Gets the total number of entities of this type in the store.
    pub fn get_entity_count(&self) -> Result<u64, postgres::Error> {
        let stmt = self.conn.prepare_cached(
            "SELECT COUNT(DISTINCT entity_id) \
             FROM events \
             WHERE aggregate_type = $1",
        )?;
        let rows = stmt.query(&[&A::aggregate_type()])?;
        Ok(rows
            .iter()
            .next()
            .map(|r| r.get::<_, i64>(0) as u64)
            .unwrap_or_default())
    }

    /// Loads a page of entity IDs.
    pub fn get_entity_ids(&self, offset: u32, limit: u32) -> Result<Vec<String>, postgres::Error> {
        let stmt = self.conn.prepare_cached(
            "SELECT entity_id \
             FROM events \
             WHERE aggregate_type = $1 \
             GROUP BY entity_id \
             ORDER BY MIN(event_id) ASC \
             OFFSET $2 LIMIT $3",
        )?;
        let rows = stmt.query(&[
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
        let stmt = self.conn.prepare_cached(
            "SELECT COUNT(DISTINCT entity_id) \
             FROM events \
             WHERE aggregate_type = $1 AND entity_id LIKE $2",
        )?;
        let rows = stmt.query(&[&A::aggregate_type(), &pattern])?;
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
        let stmt = self.conn.prepare_cached(
            "SELECT entity_id \
             FROM events \
             WHERE aggregate_type = $1 AND entity_id LIKE $2 \
             GROUP BY entity_id \
             ORDER BY MIN(event_id) ASC \
             OFFSET $3 LIMIT $4",
        )?;
        let rows = stmt.query(&[
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
        let stmt = self.conn.prepare_cached(
            "SELECT COUNT(DISTINCT entity_id) \
             FROM events \
             WHERE aggregate_type = $1 AND entity_id SIMILAR TO $2",
        )?;
        let rows = stmt.query(&[&A::aggregate_type(), &regex])?;
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
        let stmt = self.conn.prepare_cached(
            "SELECT entity_id \
             FROM events \
             WHERE aggregate_type = $1 AND entity_id SIMILAR TO $2 \
             GROUP BY entity_id \
             ORDER BY MIN(event_id) ASC \
             OFFSET $3 LIMIT $4",
        )?;
        let rows = stmt.query(&[
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
        let stmt = self.conn.prepare_cached(
            "SELECT COUNT(DISTINCT entity_id) \
             FROM events \
             WHERE aggregate_type = $1 AND entity_id ~ $2",
        )?;
        let rows = stmt.query(&[&A::aggregate_type(), &regex])?;
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
        let stmt = self.conn.prepare_cached(
            "SELECT entity_id \
             FROM events \
             WHERE aggregate_type = $1 AND entity_id ~ $2 \
             GROUP BY entity_id \
             ORDER BY MIN(event_id) ASC \
             OFFSET $3 LIMIT $4",
        )?;
        let rows = stmt.query(&[
            &A::aggregate_type(),
            &regex,
            &(i64::from(offset)),
            &(i64::from(limit)),
        ])?;
        Ok(rows.iter().map(|r| r.get(0)).collect())
    }
}

impl<'conn, A, E, M, S> EventSink<A, E, M> for PostgresStore<'conn, A, E, M, S>
where
    A: Aggregate,
    E: AggregateEvent<A> + SerializableEvent + fmt::Debug,
    M: Serialize + fmt::Debug,
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
        I: AggregateId<Aggregate = A>,
    {
        let trans = self.conn.transaction()?;

        let check_stmt = trans.prepare_cached(
            "SELECT MAX(sequence) FROM events WHERE aggregate_type = $1 AND entity_id = $2",
        )?;

        let result = check_stmt.query(&[&A::aggregate_type(), &id.as_ref()])?;
        let current_version = result.iter().next().and_then(|r| {
            let max_sequence: Option<Sequence> = r.get(0);
            max_sequence.map(|x| Version::from(x.0))
        });

        log::trace!(
            "entity {}: current version: {:?}",
            id.as_ref(),
            current_version
        );

        if events.is_empty() {
            return Ok(current_version.unwrap_or_default().next_event());
        }

        if let Some(precondition) = precondition {
            precondition.verify(current_version)?;
        }

        log::trace!("entity {}: precondition satisfied", id.as_ref());

        let first_sequence = current_version.unwrap_or_default().next_event();
        let mut next_sequence = Version::Number(first_sequence);
        let mut buffer = Vec::with_capacity(128);

        let stmt = trans.prepare_cached(
            "INSERT INTO events (aggregate_type, entity_id, sequence, event_type, payload, metadata, timestamp) \
            VALUES ($1, $2, $3, $4, $5, $6, CURRENT_TIMESTAMP)",
        )?;
        for event in events {
            buffer.clear();
            event
                .serialize_event_to_buffer(&mut buffer)
                .map_err(PersistError::SerializationError)?;
            let modified_count = stmt.execute(&[
                &A::aggregate_type(),
                &id.as_ref(),
                &(next_sequence.get() as i64),
                &event.event_type(),
                &RawJsonPersist(&buffer),
                &BorrowedJson(&metadata),
            ])?;
            debug_assert!(modified_count > 0);
            log::trace!(
                "entity {}: inserted event; sequence: {}",
                id.as_ref(),
                next_sequence
            );
            next_sequence.incr();
        }

        trans.commit()?;

        Ok(first_sequence)
    }
}

impl<'conn, A, E, M, S> EventSource<A, E> for PostgresStore<'conn, A, E, M, S>
where
    A: Aggregate,
    E: AggregateEvent<A> + DeserializableEvent,
    S: SnapshotStrategy,
{
    type Error = LoadError<<E as DeserializableEvent>::Error>;
    type Events = Vec<Result<VersionedEvent<E>, Self::Error>>;

    fn read_events<I>(
        &self,
        id: &I,
        since: Since,
        max_count: Option<u64>,
    ) -> Result<Option<Self::Events>, Self::Error>
    where
        I: AggregateId<Aggregate = A>,
    {
        let last_sequence = match since {
            cqrs_core::Since::BeginningOfStream => 0,
            cqrs_core::Since::Event(x) => x.get(),
        } as i64;

        let events;
        let trans = self
            .conn
            .transaction_with(postgres::transaction::Config::default().read_only(true))?;

        let handle_row = |row: postgres::rows::Row| {
            let event_type: String = row.get("event_type");
            let sequence: Sequence = row.get("sequence");
            let raw: RawJsonRead = row.get("payload");
            let event = E::deserialize_event_from_buffer(&raw.0, &event_type)
                .map_err(LoadError::DeserializationError)?
                .ok_or_else(|| LoadError::UnknownEventType(event_type.clone()))?;
            log::trace!(
                "entity {}: loaded event; sequence: {}, type: {}",
                id.as_ref(),
                sequence.0,
                event_type
            );
            Ok(VersionedEvent {
                sequence: sequence.0,
                event,
            })
        };

        let stmt;
        {
            let mut rows;
            if let Some(max_count) = max_count {
                stmt = trans.prepare_cached(
                    "SELECT sequence, event_type, payload \
                     FROM events \
                     WHERE aggregate_type = $1 AND entity_id = $2 AND sequence > $3 \
                     ORDER BY sequence ASC \
                     LIMIT $4",
                )?;
                rows = stmt.lazy_query(
                    &trans,
                    &[
                        &A::aggregate_type(),
                        &id.as_ref(),
                        &last_sequence,
                        &(max_count.min(i64::max_value() as u64) as i64),
                    ],
                    100,
                )?;
            } else {
                stmt = trans.prepare_cached(
                    "SELECT sequence, event_type, payload \
                     FROM events \
                     WHERE aggregate_type = $1 AND entity_id = $2 AND sequence > $3 \
                     ORDER BY sequence ASC",
                )?;
                rows = stmt.lazy_query(
                    &trans,
                    &[&A::aggregate_type(), &id.as_ref(), &last_sequence],
                    100,
                )?;
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

        log::trace!("entity {}: read {} events", id.as_ref(), events.len());

        Ok(Some(events))
    }
}

impl<'conn, A, E, M, S> SnapshotSink<A> for PostgresStore<'conn, A, E, M, S>
where
    A: Aggregate + Serialize + fmt::Debug,
    E: AggregateEvent<A>,
    S: SnapshotStrategy,
{
    type Error = PersistError<serde_json::Error>;

    fn persist_snapshot<I>(
        &self,
        id: &I,
        aggregate: &A,
        version: Version,
        last_snapshot_version: Version,
    ) -> Result<Version, Self::Error>
    where
        I: AggregateId<Aggregate = A>,
    {
        if version <= last_snapshot_version
            || self
                .snapshot_strategy
                .snapshot_recommendation(version, last_snapshot_version)
                == SnapshotRecommendation::DoNotSnapshot
        {
            return Ok(last_snapshot_version);
        }

        let stmt = self.conn.prepare_cached(
            "INSERT INTO snapshots (aggregate_type, entity_id, sequence, payload) \
             VALUES ($1, $2, $3, $4)",
        )?;
        let _modified_count = stmt.execute(&[
            &A::aggregate_type(),
            &id.as_ref(),
            &(version.get() as i64),
            &Json(aggregate),
        ])?;

        // Clean up strategy for snapshots?
        //        let stmt = self.conn.prepare_cached("DELETE FROM snapshots WHERE aggregate_type = $1 AND entity_id = $2 AND sequence < $3")?;
        //        let _modified_count = stmt.execute(&[&A::aggregate_type(), &id.as_ref(), &(version.get() as i64)])?;

        log::trace!("entity {}: persisted snapshot", id.as_ref());
        Ok(version)
    }
}

impl<'conn, A, E, M, S> SnapshotSource<A> for PostgresStore<'conn, A, E, M, S>
where
    A: Aggregate + DeserializeOwned,
    E: AggregateEvent<A>,
    S: SnapshotStrategy,
{
    type Error = postgres::Error;

    fn get_snapshot<I>(&self, id: &I) -> Result<Option<VersionedAggregate<A>>, Self::Error>
    where
        I: AggregateId<Aggregate = A>,
    {
        let stmt = self.conn.prepare_cached(
            "SELECT sequence, payload \
             FROM snapshots \
             WHERE aggregate_type = $1 AND entity_id = $2 \
             ORDER BY sequence DESC \
             LIMIT 1",
        )?;
        let rows = stmt.query(&[&A::aggregate_type(), &id.as_ref()])?;
        if let Some(row) = rows.iter().next() {
            let sequence: Sequence = row.get("sequence");
            let raw: Json<A> = row.get("payload");
            log::trace!("entity {}: loaded snapshot", id.as_ref());
            Ok(Some(VersionedAggregate {
                version: Version::from(sequence.0),
                payload: raw.0,
            }))
        } else {
            log::trace!("entity {}: no snapshot found", id.as_ref());
            Ok(None)
        }
    }
}
