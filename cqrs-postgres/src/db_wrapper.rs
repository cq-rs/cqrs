use crate::util::Sequence;
use cqrs_core::{EventNumber, RawEvent, Since};
use num_traits::ToPrimitive;
use postgres::{rows::Row, types::ToSql};
use r2d2::{Pool, PooledConnection};
use r2d2_postgres::PostgresConnectionManager;
use std::{error, fmt, sync::Arc};
use cqrs_core::CqrsError;

#[derive(Debug, Clone)]
pub enum DbError {
    Pool(Arc<dyn CqrsError>),
    Postgres(Arc<postgres::Error>),
}

impl DbError {
    pub fn pool(err: impl CqrsError) -> Self {
        DbError::Pool(Arc::new(err))
    }
}

impl From<r2d2::Error> for DbError {
    fn from(err: r2d2::Error) -> Self {
        DbError::Pool(Arc::new(err))
    }
}

impl From<postgres::Error> for DbError {
    fn from(err: postgres::Error) -> Self {
        DbError::Postgres(Arc::new(err))
    }
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DbError::Pool(ref err) => write!(f, "Pool error: {}", err),
            DbError::Postgres(ref err) => write!(f, "Postgres error: {}", err),
        }
    }
}

pub trait DbPool<'conn> {
    type Connection: DbConnection<'conn> + 'conn;
    type Error: CqrsError;
    fn get(&self) -> Result<Self::Connection, Self::Error>;
}

pub trait DbConnection<'conn> {
    type Error: CqrsError;
    fn load_since(&self, reaction_name: &str) -> Result<Since, Self::Error>;
    fn save_since(&self, reaction_name: &str, event_id: EventNumber) -> Result<(), Self::Error>;
    fn read_all_events(
        &self,
        query: &str,
        since: Since,
        params: &[Box<dyn ToSql>],
    ) -> Result<Vec<RawEvent>, Self::Error>;
}

impl<'conn> DbPool<'conn> for Pool<PostgresConnectionManager> {
    type Connection = PooledConnection<PostgresConnectionManager>;
    type Error = DbError;

    fn get(&self) -> Result<Self::Connection, Self::Error> {
        match self.get() {
            Ok(c) => Ok(c),
            Err(err) => Err(DbError::from(err)),
        }
    }
}

impl<'conn> DbConnection<'conn> for PooledConnection<PostgresConnectionManager> {
    type Error = DbError;

    fn load_since(&self, reaction_name: &str) -> Result<Since, Self::Error> {
        let stmt = self.prepare_cached(
            "SELECT event_id \
             FROM reactions \
             WHERE reaction_name = $1 \
             LIMIT 1",
        )?;

        let rows = stmt.query(&[&reaction_name])?;

        for row in &rows {
            let event_id: Sequence = row.get("event_id");
            dbg!(event_id);
            return Ok(Since::Event(event_id.0));
        }

        Ok(Since::BeginningOfStream)
    }

    fn save_since(&self, reaction_name: &str, event_id: EventNumber) -> Result<(), Self::Error> {
        let stmt = self.prepare_cached(
            "INSERT INTO reactions (reaction_name, event_id) \
             VALUES ($1, $2) \
             ON CONFLICT (reaction_name) \
             DO UPDATE SET event_id = EXCLUDED.event_id",
        )?;

        stmt.execute(&[
            &reaction_name,
            &event_id
                .get()
                .to_i64()
                .expect("Not expecting event_id > several billions"),
        ])?;

        Ok(())
    }

    fn read_all_events(
        &self,
        query: &str,
        since: Since,
        params: &[Box<ToSql>],
    ) -> Result<Vec<RawEvent>, Self::Error> {
        let last_sequence = match since {
            Since::BeginningOfStream => 0,
            Since::Event(x) => x.get(),
        } as i64;

        fn handle_row(row: Row) -> RawEvent {
            // TODO: Names...
            let event_id: Sequence = row.get(0);
            let aggregate_type = row.get(1);
            let entity_id = row.get(2);
            let sequence: Sequence = row.get(3);
            let event_type = row.get(4);
            let payload = row.get_bytes(5).unwrap();
            eprintln!(
                "entity {}/{}: loaded event; sequence: {}, type: {}",
                aggregate_type, entity_id, sequence.0, event_type,
            );
            RawEvent {
                event_id: event_id.0,
                aggregate_type,
                entity_id,
                sequence: sequence.0,
                event_type,
                payload: payload.to_owned(),
            }
        };

        let events: Vec<RawEvent>;
        {
            let rows = {
                let local_params: Vec<_> = ::std::iter::once::<&dyn ToSql>(&last_sequence)
                    .chain(params.iter().map(|p| &**p))
                    .collect();
                let stmt = self.prepare_cached(query)?;
                dbg!(&query);
                dbg!(&local_params);
                stmt.query(&local_params)?
            };

            events = (&rows).into_iter().map(handle_row).collect();
            dbg!(&events);
        }

        Ok(events)
    }
}
