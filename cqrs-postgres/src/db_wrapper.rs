use crate::util::Sequence;
use cqrs_core::{reactor::Reaction, CqrsError, EventNumber, RawEvent, Since};
use num_traits::ToPrimitive;
use postgres::{rows::Row, types::ToSql};
use r2d2::{Pool, PooledConnection};
use r2d2_postgres::PostgresConnectionManager;
use std::{error, fmt, sync::Arc};

#[derive(Debug)]
pub enum ReactorError<R, P = r2d2::Error, D = postgres::Error>
where
    R: Reaction,
{
    Pool(Arc<P>),
    Postgres(Arc<D>),
    React(R::Error),
}

impl<R, P, D> Clone for ReactorError<R, P, D>
where
    R: Reaction,
    R::Error: Clone,
{
    fn clone(&self) -> Self {
        match self {
            ReactorError::Pool(e) => ReactorError::Pool(e.clone()),
            ReactorError::Postgres(e) => ReactorError::Postgres(e.clone()),
            ReactorError::React(e) => ReactorError::React(e.clone()),
        }
    }
}

impl<R, P, D> ReactorError<R, P, D>
where
    R: Reaction,
{
    pub fn pool(err: P) -> Self {
        ReactorError::Pool(Arc::new(err))
    }

    pub fn postgres(err: D) -> Self {
        ReactorError::Postgres(Arc::new(err))
    }

    pub fn react(err: R::Error) -> Self {
        ReactorError::React(err)
    }
}

impl<R, P, D> fmt::Display for ReactorError<R, P, D>
where
    R: Reaction,
    R::Error: fmt::Display,
    P: fmt::Display,
    D: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ReactorError::Pool(ref err) => write!(f, "Pool error during reaction: {}", err),
            ReactorError::Postgres(ref err) => write!(f, "Postgres error during reaction: {}", err),
            ReactorError::React(ref err) => write!(f, "React error during reaction: {}", err),
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
    type Error = r2d2::Error;

    fn get(&self) -> Result<Self::Connection, Self::Error> {
        (*self).get()
    }
}

impl<'conn> DbConnection<'conn> for PooledConnection<PostgresConnectionManager> {
    type Error = postgres::Error;

    fn load_since(&self, reaction_name: &str) -> Result<Since, Self::Error> {
        let stmt = self.prepare_cached(
            "SELECT event_id \
             FROM reactions \
             WHERE reaction_name = $1 \
             LIMIT 1",
        )?;

        let rows = stmt.query(&[&reaction_name])?;

        if rows.len() > 0 {
            let event_id: Sequence = rows.get(0).get(0);
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
            let event_id: Sequence = row.get(0);
            let aggregate_type = row.get(1);
            let entity_id = row.get(2);
            let sequence: Sequence = row.get(3);
            let event_type = row.get(4);
            let payload = row.get_bytes(5).unwrap();
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
                stmt.query(&local_params)?
            };

            events = (&rows).into_iter().map(handle_row).collect();
        }

        Ok(events)
    }
}
