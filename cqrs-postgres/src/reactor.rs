//! Types for reacting to raw event data in PostgreSQL event store.
use crate::{
    db_wrapper::{DbConnection, DbPool},
    util::Sequence,
};
use cqrs_core::{
    reactor::{AggregatePredicate, EventTypesPredicate, Reaction, ReactionPredicate},
    EventNumber, RawEvent, Since,
};
use postgres::{rows::Rows, types::ToSql, Connection};
use r2d2::Pool;
use r2d2_postgres::PostgresConnectionManager;
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

    fn react(&mut self, _event: RawEvent) -> Result<(), Self::Error> {
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
pub struct PostgresReactor<P = Pool<PostgresConnectionManager>> {
    pool: P,
    run: AtomicBool,
}

impl<P> PostgresReactor<P>
where
    P: for<'conn> DbPool<'conn>,
{
    pub fn new(pool: P) -> Self {
        Self {
            pool,
            run: AtomicBool::new(true),
        }
    }

    pub fn stop_reaction(&self) {
        self.run.store(false, Ordering::Relaxed);
    }

    pub fn start_reaction<R: Reaction>(&self, mut reaction: R) {
        while self.run.load(Ordering::Relaxed) {
            let conn = self.pool.get().unwrap();
            let mut since = conn.load_since(R::reaction_name()).unwrap();
            let mut params: Vec<Box<dyn ToSql>> = Vec::default();
            let query_with_args = self.generate_query_with_args(R::predicate(), &mut params, 100);

            let raw_events = {
                match conn.read_all_events(&query_with_args, since, params.as_slice()) {
                    Ok(events) => events,
                    Err(error) => {
                        panic!(error);
                    }
                }
            };

            for event in raw_events {
                let event_id = event.event_id;
                reaction.react(event).unwrap();
                conn.save_since(R::reaction_name(), event_id).unwrap();
            }

            drop(conn);

            ::std::thread::sleep(R::interval());
        }
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
    use crate::{
        db_wrapper::{DbConnection, DbPool},
        reactor::{NullReaction, PostgresReactor},
    };
    use cqrs_core::{
        reactor::{
            AggregatePredicate, EventTypesPredicate, Reaction, ReactionPredicate,
            SpecificAggregatePredicate,
        },
        EventNumber, RawEvent, Since,
    };
    use lazy_static::lazy_static;
    use parking_lot::Mutex;
    use postgres::{types::ToSql, Connection};
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

    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
    pub struct MockPool;

    impl<'conn> DbPool<'conn> for MockPool {
        type Connection = MockConnection;
        type Error = String;

        fn get(&self) -> Result<Self::Connection, Self::Error> {
            Ok(MockConnection)
        }
    }

    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
    pub struct MockConnection;

    impl<'conn> DbConnection<'conn> for MockConnection {
        type Error = String;

        fn load_since(&self, reaction_name: &str) -> Result<Since, Self::Error> {
            Ok(Since::BeginningOfStream)
        }

        fn save_since(
            &self,
            reaction_name: &str,
            event_id: EventNumber,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        fn read_all_events(
            &self,
            query: &str,
            since: Since,
            params: &[Box<dyn ToSql>],
        ) -> Result<Vec<RawEvent>, Self::Error> {
            println!("{:?}", params);

            let raw_event = RawEvent {
                event_id: EventNumber::new(123).unwrap(),
                aggregate_type: "".to_string(),
                entity_id: "".to_string(),
                sequence: EventNumber::new(123).unwrap(),
                event_type: "".to_string(),
                payload: Vec::from("{}"),
            };

            let raw_events = vec![
                raw_event.clone(),
                raw_event.clone(),
                raw_event.clone(),
                raw_event.clone(),
                raw_event.clone(),
                raw_event.clone(),
                raw_event.clone(),
                raw_event.clone(),
            ];

            Ok(raw_events)
        }
    }

    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
    pub struct MockReaction;

    impl Reaction for MockReaction {
        type Error = void::Void;

        fn reaction_name() -> &'static str {
            "Mock"
        }

        fn react(&mut self, event: RawEvent) -> Result<(), Self::Error> {
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

        //        let pool = Pool::new(manager.unwrap()).unwrap();
        let pool = MockPool;

        let local_reactor = Arc::new(PostgresReactor::new(pool));
        let thread_reactor = Arc::clone(&local_reactor);

        let handle = Some(thread::spawn(move || {
            thread_reactor.start_reaction(MockReaction);
        }));

        if let Some(h) = handle {
            ::std::thread::sleep(Duration::from_millis(50));
            local_reactor.stop_reaction();

            match h.join() {
                Ok(_) => {}
                Err(error) => {
                    eprintln!("join error: {:?}", error);
                }
            }
        }
    }
}
