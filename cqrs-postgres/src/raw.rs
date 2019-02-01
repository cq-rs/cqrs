//! Types for interacting with raw event data in PostgreSQL event store.

use crate::{error::LoadError, util::Sequence};
use cqrs_core::{EventNumber, Since};
use fallible_iterator::FallibleIterator;
use postgres::Connection;

/// An owned, raw view of event data.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct RawEvent {
    /// The event id.
    pub event_id: EventNumber,
    /// The aggregate type.
    pub aggregate_type: String,
    /// The entity id.
    pub entity_id: String,
    /// The sequence number of this event in the entity's event stream.
    pub sequence: EventNumber,
    /// The event type.
    pub event_type: String,
    /// The raw event payload.
    pub payload: Vec<u8>,
}

/// An owned, raw view of event data.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct BorrowedRawEvent<'row> {
    /// The event id.
    pub event_id: EventNumber,
    /// The aggregate type.
    pub aggregate_type: &'row str,
    /// The entity id.
    pub entity_id: &'row str,
    /// The sequence number of this event in the entity's event stream.
    pub sequence: EventNumber,
    /// The event type.
    pub event_type: &'row str,
    /// The raw event payload.
    pub payload: &'row [u8],
}

/// A connection to a PostgreSQL storage backend that is not specific to any aggregate.
#[derive(Clone, Copy, Debug)]
pub struct RawPostgresStore<'conn> {
    conn: &'conn Connection,
}

impl<'conn> RawPostgresStore<'conn> {
    /// Reads all events from the event stream, starting with events after `since`,
    pub fn read_all_events(
        self,
        since: Since,
        max_count: u64,
    ) -> Result<Vec<RawEvent>, postgres::Error> {
        let last_sequence = match since {
            Since::BeginningOfStream => 0,
            Since::Event(x) => x.get(),
        } as i64;

        let trans = self
            .conn
            .transaction_with(postgres::transaction::Config::default().read_only(true))?;

        let handle_row = |row: postgres::rows::Row| {
            let event_id: Sequence = row.get(0);
            let aggregate_type = row.get(1);
            let entity_id = row.get(2);
            let sequence: Sequence = row.get(3);
            let event_type = row.get(4);
            let payload = row.get_bytes(5).unwrap();
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
                payload: payload.to_owned(),
            }
        };

        let events: Vec<RawEvent>;
        {
            let stmt = trans.prepare_cached(
                "SELECT event_id, aggregate_type, entity_id, sequence, event_type, payload \
                 FROM events \
                 WHERE event_id > $1 \
                 ORDER BY event_id ASC \
                 LIMIT $2",
            )?;
            let rows = stmt.lazy_query(
                &trans,
                &[
                    &last_sequence,
                    &(max_count.min(i64::max_value() as u64) as i64),
                ],
                100,
            )?;

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
        self,
        since: Since,
        max_count: u64,
        mut f: impl for<'row> FnMut(BorrowedRawEvent<'row>) -> Result<(), E>,
    ) -> Result<(), LoadError<E>> {
        let last_sequence = match since {
            Since::BeginningOfStream => 0,
            Since::Event(x) => x.get(),
        } as i64;

        let trans = self
            .conn
            .transaction_with(postgres::transaction::Config::default().read_only(true))?;

        let mut handle_row = |row: postgres::rows::Row| -> Result<(), LoadError<E>> {
            let event_id: Sequence = row.get(0);
            let aggregate_type = std::str::from_utf8(row.get_bytes(1).unwrap()).unwrap();
            let entity_id = std::str::from_utf8(row.get_bytes(2).unwrap()).unwrap();
            let sequence: Sequence = row.get(3);
            let event_type = std::str::from_utf8(row.get_bytes(4).unwrap()).unwrap();
            let payload = row.get_bytes(5).unwrap();
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
            let stmt = trans.prepare_cached(
                "SELECT event_id, aggregate_type, entity_id, sequence, event_type, payload \
                 FROM events \
                 WHERE event_id > $1 \
                 ORDER BY event_id ASC \
                 LIMIT $2",
            )?;
            let rows = stmt.lazy_query(
                &trans,
                &[
                    &last_sequence,
                    &(max_count.min(i64::max_value() as u64) as i64),
                ],
                100,
            )?;

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
