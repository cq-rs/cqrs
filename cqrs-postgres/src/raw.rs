//! Types for interacting with raw event data in PostgreSQL event store.

use std::sync::Arc;
use std::sync::Mutex;

use crate::NewConn;
use crate::{error::LoadError, util::Sequence};
use cqrs_core::{BorrowedRawEvent, RawEvent, Since};
use postgres::Client;
use postgres::fallible_iterator::FallibleIterator;
use r2d2::PooledConnection;

/// A connection to a PostgreSQL storage backend that is not specific to any aggregate.
#[derive(Clone)]
pub struct RawPostgresStore {
    pub conn: Arc<Mutex<PooledConnection<NewConn>>>
}

impl RawPostgresStore {
    /// Reads all events from the event stream, starting with events after `since`,
    pub fn read_all_events(
        &self,
        since: Since,
        max_count: u64,
    ) -> Result<Vec<RawEvent>, postgres::Error> {
        let last_sequence = match since {
            Since::BeginningOfStream => 0,
            Since::Event(x) => x.get(),
        } as i64;

        let mut conn = self.conn.lock().unwrap();
        let mut trans = conn.build_transaction().read_only(true).start()?;

        let handle_row = |row: postgres::Row| {
            let event_id: Sequence = row.get(0);
            let aggregate_type = row.get(1);
            let entity_id = row.get(2);
            let sequence: Sequence = row.get(3);
            let event_type = row.get(4);
            let payload = row.get(5);
            log::trace!(
                "entity {}/{}: loaded event; sequence: {}, type: {}",
                aggregate_type,
                entity_id,
                sequence.0,
                event_type,
            );
            RawEvent {
                event_id: event_id.0,
                aggregate_type,
                entity_id,
                sequence: sequence.0,
                event_type,
                payload: payload,
            }
        };

        let events: Vec<RawEvent>;
        {
            let stmt = trans.prepare(
                "SELECT event_id, aggregate_type, entity_id, sequence, event_type, payload \
                 FROM events \
                 WHERE event_id > $1 \
                 ORDER BY event_id ASC \
                 LIMIT $2",
            )?;

            let portal = trans.bind(&stmt, &[
                &last_sequence,
                &(max_count.min(i64::max_value() as u64) as i64),
            ])?;

            let rows = trans.query_portal_raw(&portal, 0)?;
            
            events = rows
                .iterator()
                .map(|row_result| row_result.map(handle_row))
                .collect::<Result<_, postgres::Error>>()?;
        }

        trans.commit()?;

        log::trace!("read {} events", events.len(),);

        Ok(events)
    }

    /// Reads all events from the event stream, starting with events after `since`,
    pub fn read_all_events_with<E: cqrs_core::CqrsError>(
        &self,
        since: Since,
        max_count: u64,
        mut f: impl for<'row> FnMut(BorrowedRawEvent<'row>) -> Result<(), E>,
    ) -> Result<(), LoadError<E>> {
        let last_sequence = match since {
            Since::BeginningOfStream => 0,
            Since::Event(x) => x.get(),
        } as i64;

        let mut conn = self.conn.lock().unwrap();
        let mut trans = conn.build_transaction().read_only(true).start()?;

        let mut handle_row = |row: postgres::Row| -> Result<(), LoadError<E>> {
            let event_id: Sequence = row.get(0);
            let aggregate_type = std::str::from_utf8(row.get(1)).unwrap();
            let entity_id = std::str::from_utf8(row.get(2)).unwrap();
            let sequence: Sequence = row.get(3);
            let event_type = std::str::from_utf8(row.get(4)).unwrap();
            let payload = row.get(5);
            log::trace!(
                "entity {}/{}: loaded event; sequence: {}, type: {}",
                aggregate_type,
                entity_id,
                sequence.0,
                event_type,
            );
            f(BorrowedRawEvent {
                event_id: event_id.0,
                aggregate_type,
                entity_id,
                sequence: sequence.0,
                event_type,
                payload,
            })
            .map_err(LoadError::DeserializationError)
        };

        let events: Vec<()>;
        {
            let stmt = trans.prepare(
                "SELECT event_id, aggregate_type, entity_id, sequence, event_type, payload \
                 FROM events \
                 WHERE event_id > $1 \
                 ORDER BY event_id ASC \
                 LIMIT $2",
            )?;

            let portal = trans.bind(&stmt, &[
                &last_sequence,
                &(max_count.min(i64::max_value() as u64) as i64),
            ])?;

            let rows = trans.query_portal_raw(&portal, 0)?;

            events = rows
                .iterator()
                .map(|row_result| {
                    row_result
                        .map_err(LoadError::from)
                        .and_then(|row| handle_row(row))
                })
                .collect::<Result<_, LoadError<E>>>()?;
        }

        trans.commit()?;

        log::trace!("read {} events", events.len(),);

        Ok(())
    }
}
