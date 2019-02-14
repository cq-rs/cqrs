//! Types for reacting to raw event data in PostgreSQL event store.
use crate::util::Sequence;
use cqrs_core::{
    reactor::{AggregatePredicate, EventTypesPredicate, Reaction, ReactionPredicate},
    EventNumber, RawEvent, Since,
};
use num_traits::ToPrimitive;
use postgres::{rows::Rows, types::ToSql, Connection};
use std::{
    fmt::Write,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct NullReaction;

impl Reaction for NullReaction {
    type Error = void::Void;

    fn reaction_name() -> &'static str {
        "Null"
    }

    fn react(_event: RawEvent) -> Result<(), Self::Error> {
        Ok(())
    }

    fn predicate() -> ReactionPredicate {
        ReactionPredicate::default()
    }

    fn interval() -> Duration {
        Duration::from_secs(1)
    }
}

#[derive(Debug)]
pub struct PostgresReactor {
    pool: r2d2_postgres::r2d2::Pool<r2d2_postgres::PostgresConnectionManager>,
    run: AtomicBool,
}

impl PostgresReactor {
    pub fn new(pool: r2d2_postgres::r2d2::Pool<r2d2_postgres::PostgresConnectionManager>) -> Self {
        PostgresReactor {
            pool,
            run: AtomicBool::new(true),
        }
    }

    pub fn stop_reaction(&self) {
        self.run.store(false, Ordering::Relaxed);
    }

    pub fn start_reaction<R: Reaction>(&self, _reaction: R) {
        let conn = self.pool.get().unwrap();
        let mut since = self.load_since(&conn, R::reaction_name()).unwrap();
        let mut params: Vec<Box<dyn ToSql>> = Vec::default();
        let query_with_args = self.generate_query_with_args(R::predicate(), &mut params, 100);

        while self.run.load(Ordering::Relaxed) {
            let raw_events = {
                match self.read_all_events(&conn, &query_with_args, since, params.as_slice()) {
                    Ok(events) => events,
                    Err(error) => {
                        panic!(error);
                    }
                }
            };

            for event in raw_events {
                let event_id = event.event_id;
                R::react(event).unwrap();
                since = self
                    .save_since(&conn, R::reaction_name(), event_id)
                    .unwrap();
            }

            ::std::thread::sleep(R::interval());
        }
    }

    fn load_since(&self, conn: &Connection, reaction_name: &str) -> Result<Since, postgres::Error> {
        let trans =
            conn.transaction_with(postgres::transaction::Config::default().read_only(true))?;

        let rows: Rows = {
            let stmt = trans.prepare_cached(
                "SELECT event_id \
                 FROM reactions \
                 WHERE reaction_name = $1 \
                 LIMIT 1",
            )?;
            stmt.query(&[&reaction_name])?
        };

        for row in rows.iter() {
            let event_id: Sequence = row.get(0);
            return Ok(Since::Event(event_id.0));
        }

        Ok(Since::BeginningOfStream)
    }

    fn save_since(
        &self,
        conn: &Connection,
        reaction_name: &str,
        event_id: EventNumber,
    ) -> Result<Since, postgres::Error> {
        let trans =
            conn.transaction_with(postgres::transaction::Config::default().read_only(false))?;

        let stmt = trans.prepare_cached(
            "INSERT INTO reactions (reaction_name, event_id) \
             VALUES ($1, $2) \
             ON CONFLICT (reaction_name) \
             DO UPDATE SET event_id = EXCLUDED.event_id",
        )?;

        // TODO: assert 1 row updated
        let rows_updated = stmt
            .execute(&[
                &reaction_name,
                &event_id
                    .get()
                    .to_i64()
                    .expect("Not expecting event_id > several billions"),
            ])
            .unwrap();

        trans.commit()?;
        Ok(Since::Event(event_id))
    }

    /// Reads all events from the event stream, starting with events after `since`,
    fn read_all_events(
        &self,
        conn: &Connection,
        query: &str,
        since: Since,
        params: &[Box<dyn ToSql>],
    ) -> Result<Vec<RawEvent>, postgres::Error> {
        let last_sequence = match since {
            Since::BeginningOfStream => 0,
            Since::Event(x) => x.get(),
        } as i64;

        let trans =
            conn.transaction_with(postgres::transaction::Config::default().read_only(true))?;

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
            let rows: Rows = {
                let stmt = trans.prepare_cached(query)?;
                let local_params: Vec<_> = ::std::iter::once::<&dyn ToSql>(&last_sequence)
                    .chain(params.iter().map(|p| &**p))
                    .collect();
                stmt.query(&local_params)?
            };

            events = rows.iter().map(handle_row).collect();
        }

        trans.commit()?;
        Ok(events)
    }

    fn generate_query_with_args(
        &self,
        predicate: ReactionPredicate,
        params: &mut Vec<Box<dyn ToSql>>,
        max_count: u64,
    ) -> String {
        let max_count = Box::new(max_count.min(i64::max_value() as u64) as i64);

        match predicate.aggregate_predicate {
            AggregatePredicate::AllAggregates(EventTypesPredicate::AllEventTypes) => {
                params.push(max_count);

                String::from(
                    "SELECT event_id, aggregate_type, entity_id, sequence, event_type, payload \
                     FROM events \
                     WHERE event_id > $1 \
                     ORDER BY event_id ASC \
                     LIMIT $2",
                )
            }
            AggregatePredicate::AllAggregates(EventTypesPredicate::SpecificEventTypes(
                event_types,
            )) => {
                params.push(Box::new(event_types));
                params.push(max_count);

                String::from(
                    "SELECT event_id, aggregate_type, entity_id, sequence, event_type, payload \
                     FROM events \
                     WHERE event_id > $1 \
                     AND event_type = ANY ($2) \
                     ORDER BY event_id ASC \
                     LIMIT $3",
                )
            }
            AggregatePredicate::SpecificAggregates(aggregate_predicates) => {
                let mut query = String::from(
                    "SELECT event_id, aggregate_type, entity_id, sequence, event_type, payload \
                     FROM events \
                     WHERE event_id > $1 AND (FALSE",
                );

                let mut param_count = 1;

                for predicate in aggregate_predicates {
                    match &predicate.event_types {
                        EventTypesPredicate::SpecificEventTypes(event_types) => {
                            write!(
                                query,
                                " OR (aggregate_type = ${} AND event_type = ANY (${}))",
                                param_count + 1,
                                param_count + 2
                            )
                            .unwrap();
                            params.push(Box::new(predicate.aggregate_type));
                            params.push(Box::new(event_types));
                            param_count += 2;
                        }
                        EventTypesPredicate::AllEventTypes => {
                            write!(query, " OR (aggregate_type = ${})", param_count + 1).unwrap();
                            params.push(Box::new(predicate.aggregate_type));
                            param_count += 1;
                        }
                    }
                }

                write!(query, ") ORDER BY event_id ASC LIMIT ${}", param_count + 1).unwrap();
                params.push(max_count);
                query
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::reactor::{NullReaction, PostgresReactor};
    use cqrs_core::{
        reactor::{
            AggregatePredicate, EventTypesPredicate, Reaction, ReactionPredicate,
            SpecificAggregatePredicate,
        },
        EventNumber, RawEvent, Since,
    };
    use lazy_static::lazy_static;
    use parking_lot::Mutex;
    use postgres::Connection;
    use r2d2_postgres::{r2d2::Pool, PostgresConnectionManager, TlsMode};
    use std::{sync::Arc, thread, time::Duration};

    lazy_static! {
        static ref EVENTS: Mutex<Vec<RawEvent>> = Mutex::new(vec![]);
        static ref PREDICATE: Mutex<ReactionPredicate> = Mutex::new(ReactionPredicate::default());
        static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
    }

    macro_rules! isolated_test {
        (fn $name:ident() $body:block) => {
            #[test]
            fn $name() {
                let _guard = TEST_MUTEX.lock();
                $body
            }
        };
    }

    #[derive(Debug, Default, Eq, PartialEq, Hash)]
    pub struct MockReaction;

    impl Reaction for MockReaction {
        type Error = void::Void;

        fn reaction_name() -> &'static str {
            "Mock"
        }

        fn react(event: RawEvent) -> Result<(), Self::Error> {
            EVENTS.lock().push(event);
            Ok(())
        }

        fn predicate() -> ReactionPredicate {
            *PREDICATE.lock()
        }

        fn interval() -> Duration {
            Duration::from_millis(100)
        }
    }

    isolated_test! {
        fn can_read_all_aggregates_and_all_events() {
            *PREDICATE.lock() = ReactionPredicate::default();

            perform_test();
            assert_eq!(16, EVENTS.lock().len());
        }
    }

    isolated_test! {
        fn can_read_specific_aggregates_and_all_events() {
            *PREDICATE.lock() = ReactionPredicate {
                aggregate_predicate: AggregatePredicate::SpecificAggregates(&[
                    SpecificAggregatePredicate {
                        aggregate_type: "material_location_availability",
                        event_types: EventTypesPredicate::AllEventTypes,
                    },
                ]),
            };

            perform_test();
            assert_eq!(16, EVENTS.lock().len());
        }
    }

    isolated_test! {
        fn can_read_all_aggregates_and_specific_events() {
            *PREDICATE.lock() = ReactionPredicate {
                aggregate_predicate: AggregatePredicate::AllAggregates(
                    EventTypesPredicate::SpecificEventTypes(&["sources_updated"]),
                ),
            };

            perform_test();
            assert_eq!(8, EVENTS.lock().len());
        }
    }

    isolated_test! {
        fn can_read_specific_aggregates_and_specific_events() {
            *PREDICATE.lock() = ReactionPredicate {
                aggregate_predicate: AggregatePredicate::SpecificAggregates(&[
                    SpecificAggregatePredicate {
                        aggregate_type: "material_location_availability",
                        event_types: EventTypesPredicate::SpecificEventTypes(&[
                            "sources_updated",
                            "end_of_life_updated"
                        ]),
                    },
                ]),
            };

            perform_test();
            assert_eq!(10, EVENTS.lock().len());
        }
    }

    fn perform_test() {
        EVENTS.lock().clear();

        let manager = PostgresConnectionManager::new(
            "postgresql://postgres:test@localhost:5432/es",
            TlsMode::None,
        );

        let pool = Pool::new(manager.unwrap()).unwrap();

        delete_mock_reactor_row(pool.clone()).unwrap();

        let local_reactor = Arc::new(PostgresReactor::new(pool));
        let thread_reactor = Arc::clone(&local_reactor);

        let handle = Some(thread::spawn(move || {
            thread_reactor.start_reaction(MockReaction);
        }));

        if let Some(h) = handle {
            ::std::thread::sleep(Duration::from_millis(150));
            local_reactor.stop_reaction();

            match h.join() {
                Ok(_) => {}
                Err(error) => {
                    eprintln!("join error: {:?}", error);
                }
            }
        }
    }

    fn delete_mock_reactor_row(
        pool: r2d2_postgres::r2d2::Pool<r2d2_postgres::PostgresConnectionManager>,
    ) -> Result<(), postgres::Error> {
        let conn = pool.get().unwrap();
        let trans =
            conn.transaction_with(postgres::transaction::Config::default().read_only(false))?;
        let stmt = trans.prepare_cached("DELETE FROM reactions WHERE reaction_name = 'Mock'")?;
        let rows_deleted = stmt.execute(&[]).unwrap();
        trans.commit()?;
        Ok(())
    }
}
