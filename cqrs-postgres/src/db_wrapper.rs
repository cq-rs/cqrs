use crate::util::Sequence;
use cqrs_core::{reactor::Reaction, CqrsError, EventNumber, RawEvent, Since};
use num_traits::ToPrimitive;
use postgres::{Row, Socket, tls::{MakeTlsConnect, TlsConnect}, types::ToSql};
use r2d2::{ManageConnection, Pool, PooledConnection};
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
    fn load_since(&mut self, reaction_name: &str) -> Result<Since, Self::Error>;
    fn save_since(&mut self, reaction_name: &str, event_id: EventNumber) -> Result<(), Self::Error>;
    fn read_all_events(
        &mut self,
        query: &str,
        since: Since,
        params: &[Box<dyn ToSql + Sync>],
    ) -> Result<Vec<RawEvent>, Self::Error>;
}

pub struct NewConn(Box<dyn ManageConnection<Connection = postgres::Client, Error = postgres::Error>>);

impl NewConn {
    pub fn new<T>(mgr: PostgresConnectionManager<T>) -> NewConn
    where
        T: MakeTlsConnect<Socket> + Clone + 'static + Sync + Send,
        T::TlsConnect: Send,
        T::Stream: Send,
        <T::TlsConnect as TlsConnect<Socket>>::Future: Send {
        NewConn(Box::new(mgr))
    }
}

impl ManageConnection for NewConn {
    type Connection = postgres::Client;

    type Error = postgres::Error;

    fn connect(&self) -> Result<Self::Connection, Self::Error> {
        self.0.connect()
    }

    fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        self.0.is_valid(conn)
    }

    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        self.0.has_broken(conn)
    }
}

impl<'conn> DbPool<'conn> for Pool<NewConn> {
    type Connection = PooledConnection<NewConn>;
    type Error = r2d2::Error;

    fn get(&self) -> Result<Self::Connection, Self::Error> {
        (*self).get()
    }
}

impl<'conn> DbConnection<'conn> for PooledConnection<NewConn> {
    type Error = postgres::Error;

    fn load_since(&mut self, reaction_name: &str) -> Result<Since, Self::Error> {
        let stmt = self.prepare(
            "SELECT event_id \
             FROM reactions \
             WHERE reaction_name = $1 \
             LIMIT 1",
        )?;

        let rows = self.query(&stmt,&[&reaction_name])?;

        match rows.get(0){
            Some(row) => {
                let col: i64 = row.get(0);
                match EventNumber::new(col.unsigned_abs()) {
                    Some(num) => Ok(Since::Event(num)),
                    None => Ok(Since::BeginningOfStream),
                }
            },
            None => Ok(Since::BeginningOfStream),
        }

        
    }

    fn save_since(&mut self, reaction_name: &str, event_id: EventNumber) -> Result<(), Self::Error> {
        let stmt = self.prepare(
            "INSERT INTO reactions (reaction_name, event_id) \
             VALUES ($1, $2) \
             ON CONFLICT (reaction_name) \
             DO UPDATE SET event_id = EXCLUDED.event_id",
        )?;

        self.query(&stmt, &[
            &reaction_name,
            &event_id
                .get()
                .to_i64()
                .expect("Not expecting event_id > several billions"),
        ])?;

        Ok(())
    }

    fn read_all_events(
        &mut self,
        query: &str,
        since: Since,
        params: &[Box<dyn ToSql + Sync>],
    ) -> Result<Vec<RawEvent>, Self::Error> {
        let last_sequence = match since {
            Since::BeginningOfStream => 0,
            Since::Event(x) => x.get(),
        } as i64;

        fn handle_row(row: &mut Row) -> RawEvent {
            let event_id: Sequence = row.get(0);
            let aggregate_type = row.get(1);
            let entity_id = row.get(2);
            let sequence: Sequence = row.get(3);
            let event_type = row.get(4);
            let payload: Vec<u8> = row.get(5);
            RawEvent {
                event_id: event_id.0,
                aggregate_type,
                entity_id,
                sequence: sequence.0,
                event_type,
                payload: payload.to_owned(),
            }
        }

        let events: Vec<RawEvent>;
        // {
            let mut rows = {
                let local_params: Vec<_> = ::std::iter::once::<&(dyn ToSql + Sync)>(&last_sequence)
                    .chain(params.iter().map(|p| &**p))
                    .collect();
                let stmt = self.prepare(query)?;
                self.query(&stmt, &local_params[..])?
            };

            events = rows.iter_mut().map(handle_row).collect();
        // }

        Ok(events)
    }
}
