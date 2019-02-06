//! Types for reacting to raw event data in PostgreSQL event store.

use crate::{error::LoadError, util::Sequence};
use cqrs_core::{
    reactor::{AggregatePredicate, EventTypesPredicate, Reaction, ReactionPredicate, Reactor},
    EventNumber, RawEvent, Since,
};
use fallible_iterator::FallibleIterator;
use postgres::{rows::Rows, types::ToSql, Connection};
use std::time::Duration;
use std::fmt::Write;
use cqrs_core::reactor::SpecificAggregatePredicate;

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct NullReaction;

impl Reaction for NullReaction {
    type Error = void::Void;

    fn name() -> &'static str {
        "Null Reaction"
    }

    fn react(&mut self, event: RawEvent) -> Result<(), Self::Error> {
        dbg!(event);
        Ok(())
    }

    fn predicate() -> ReactionPredicate {
        // TODO: default()
        ReactionPredicate{ aggregate_predicate: (AggregatePredicate::SpecificAggregates(&[
            SpecificAggregatePredicate{
                aggregate_type: "Fred",
                event_types: EventTypesPredicate::SpecificEventTypes(&[
                    "Wilma",
                    "Barney",
                    "Betty",
                ])},
            SpecificAggregatePredicate{
                aggregate_type: "George",
                event_types: EventTypesPredicate::SpecificEventTypes(&[
                    "Jane",
                    "Judy",
                    "Elroy",
                ])},
        ])) }
    }
}

#[derive(Clone, Debug)]
pub struct PostgresReactor {
    pool: r2d2_postgres::r2d2::Pool<r2d2_postgres::PostgresConnectionManager>,
}

impl PostgresReactor {
    pub fn new(pool: r2d2_postgres::r2d2::Pool<r2d2_postgres::PostgresConnectionManager>) -> Self {
        PostgresReactor { pool }
    }

    pub fn start_reaction<R: Reaction>(&self, reaction: R) {
        let mut since = Since::BeginningOfStream;
        let mut reaction = reaction;

        loop {
            let conn = self.pool.get().unwrap();
            let raw_events = self
                .read_all_events(&conn, R::predicate(), since)
                .unwrap();

            for event in raw_events {
                let event_id = event.event_id;
                reaction.react(event).unwrap();
                since = Since::Event(event_id); // TODO: Persist
            }

            ::std::thread::sleep(std::time::Duration::from_secs(60));
        }
    }

    /// Reads all events from the event stream, starting with events after `since`,
    pub fn read_all_events(
        &self,
        conn: &Connection,
        predicate: ReactionPredicate,
        since: Since,
    ) -> Result<Vec<RawEvent>, postgres::Error> {
        let max_count = 1;

        let last_sequence = match since {
            Since::BeginningOfStream => 0,
            Since::Event(x) => x.get(),
        } as i64;

        let trans = conn.transaction_with(postgres::transaction::Config::default().read_only(true))?;

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
            let rows: Rows = match predicate.aggregate_predicate {
                AggregatePredicate::AllAggregates(EventTypesPredicate::AllEventTypes) => {
                    let stmt = trans.prepare_cached(
                        "SELECT event_id, aggregate_type, entity_id, sequence, event_type, payload \
                         FROM events \
                         WHERE event_id > $1 \
                         ORDER BY event_id ASC \
                         LIMIT $2",
                    )?;
                    stmt.query(
                        &[
                            &last_sequence,
                            &(max_count.min(i64::max_value() as u64) as i64),
                        ],
                    )?
                }
                AggregatePredicate::AllAggregates(EventTypesPredicate::SpecificEventTypes(
                    event_types,
                )) => {
                    let stmt = trans.prepare_cached(
                        "SELECT event_id, aggregate_type, entity_id, sequence, event_type, payload \
                         FROM events \
                         WHERE event_id > $1 \
                         AND event_type IN ($2) \
                         ORDER BY event_id ASC \
                         LIMIT $3",
                    )?;
                    stmt.query(
                        &[
                            &last_sequence,
                            &event_types,
                            &(max_count.min(i64::max_value() as u64) as i64),
                        ],
                    )?
                }
                AggregatePredicate::SpecificAggregates(aggregate_predicates) => {
                    let mut query = String::from("SELECT event_id, aggregate_type, entity_id, sequence, event_type, payload \
                         FROM events \
                         WHERE event_id > $1 AND (FALSE");

                    let mut params: Vec<&ToSql> = vec![&last_sequence];
                    let mut param_count = 1;

                    for predicate in aggregate_predicates {
                        match &predicate.event_types {
                            EventTypesPredicate::SpecificEventTypes(event_types) => {
                                write!(
                                    query,
                                    " OR (aggregate_type = ${} AND event_type IN (${}))",
                                    param_count + 1,
                                    param_count + 2
                                ).unwrap();
                                params.push(&predicate.aggregate_type);
                                params.push(event_types);
                                param_count += 2;
                            }
                            EventTypesPredicate::AllEventTypes => {
                                write!(query, " OR (aggregate_type = ${})", param_count + 1).unwrap();
                                params.push(&predicate.aggregate_type);
                                param_count += 1;
                            }
                        }
                    }

                    write!(query, ") ORDER BY event_id ASC LIMIT ${}", param_count + 1).unwrap();
                    params.push(&5);    // TODO: Proper limit

                    let stmt = trans.prepare_cached(dbg!(&query))?;
                    stmt.query(dbg!(&params))?
                }
            };

            events = rows
                .iter()
                .map(handle_row)
                .collect();
        }

        trans.commit()?;

        log::trace!("read {} events", events.len(),);

        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use crate::reactor::NullReaction;
    use crate::reactor::PostgresReactor;
    use r2d2_postgres::{PostgresConnectionManager, r2d2::Pool, TlsMode};

    #[test]
    fn can_read() {
        let reaction = NullReaction;

        let manager = PostgresConnectionManager::new(
            "postgresql://postgres:test@localhost:5432/es",
            TlsMode::None,
        );

        let pool = Pool::new(manager.unwrap());

        let reactor = PostgresReactor::new(pool.unwrap());

        reactor.start_reaction(reaction);
    }
}
