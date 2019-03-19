//! Types for reacting to raw event data in PostgreSQL event store.
use crate::db_wrapper::{DbConnection, DbPool, ReactorError};
use cqrs_core::{
    reactor::{AggregatePredicate, EventTypesPredicate, Reaction, ReactionPredicate},
    CqrsError, RawEvent,
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
    type Error = String;

    fn reaction_name() -> &'static str {
        "Null"
    }

    fn react(&mut self, _event: RawEvent) -> Result<(), Self::Error> {
        Ok(())
    }

    fn predicate(&self) -> ReactionPredicate {
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

    pub fn start_reaction<R: Reaction>(
        &self,
        mut reaction: R,
    ) -> Result<usize, ReactorError<R, impl CqrsError, impl CqrsError>> {
        let mut event_count = usize::default();

        while self.run.load(Ordering::Relaxed) {
            let conn = self.pool.get().map_err(ReactorError::pool)?;
            let since = conn
                .load_since(R::reaction_name())
                .map_err(ReactorError::postgres)?;
            let mut params: Vec<Box<dyn ToSql>> = Vec::default();
            let query_with_args =
                self.generate_query_with_args::<R>(reaction.predicate(), &mut params, 100);

            let raw_events = conn
                .read_all_events(&query_with_args, since, params.as_slice())
                .map_err(ReactorError::postgres)?;

            for event in raw_events {
                let event_id = event.event_id;
                reaction.react(event).map_err(ReactorError::react)?;

                conn.save_since(R::reaction_name(), event_id)
                    .map_err(ReactorError::postgres)?;

                event_count += 1;
            }

            drop(conn);

            ::std::thread::sleep(R::interval());
        }

        Ok(event_count)
    }

    fn generate_query_with_args<R: Reaction>(
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
                            .expect("Formatting integers into a string never fails");

                            params.push(Box::new(predicate.aggregate_type));
                            params.push(Box::new(event_types));
                            param_count += 2;
                        }
                        EventTypesPredicate::AllEventTypes => {
                            write!(query, " OR (aggregate_type = ${})", param_count + 1)
                                .expect("Formatting integers into a string never fails");

                            params.push(Box::new(predicate.aggregate_type));
                            param_count += 1;
                        }
                    }
                }

                write!(query, ") ORDER BY event_id ASC LIMIT ${}", param_count + 1)
                    .expect("Formatting integers into a string never fails");

                params.push(max_count);
                query
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        db_wrapper::{DbConnection, DbPool, ReactorError},
        reactor::{self, NullReaction, PostgresReactor},
    };
    use cqrs_core::{
        reactor::{
            AggregatePredicate, EventTypesPredicate, Reaction, ReactionPredicate,
            SpecificAggregatePredicate,
        },
        CqrsError, EventNumber, RawEvent, Since,
    };
    use lazy_static::lazy_static;
    use parking_lot::Mutex;
    use postgres::{error, types::ToSql, Connection};
    use r2d2_postgres::{r2d2::Pool, PostgresConnectionManager, TlsMode};
    use std::{
        io::{self, Error},
        sync::Arc,
        thread,
        time::Duration,
    };

    lazy_static! {
        static ref PREDICATE: Mutex<ReactionPredicate> = Mutex::new(ReactionPredicate::default());
        static ref RAW_EVENT: RawEvent = RawEvent {
            event_id: EventNumber::new(1).unwrap(),
            aggregate_type: String::from(""),
            entity_id: String::from(""),
            sequence: EventNumber::new(1).unwrap(),
            event_type: String::from(""),
            payload: Vec::from("{}"),
        };
        static ref RAW_EVENTS: Vec<RawEvent> = vec![RAW_EVENT.clone(), RAW_EVENT.clone(),];
    }

    #[derive(Debug)]
    pub struct MockPool {
        get_result: Result<MockConnection, String>,
    }

    impl<'conn> DbPool<'conn> for MockPool {
        type Connection = MockConnection;
        type Error = String;

        fn get(&self) -> Result<Self::Connection, Self::Error> {
            self.get_result.clone()
        }
    }

    #[derive(Debug, Clone)]
    pub struct LoadSince {
        expected_reaction_name: String,
        result: Result<Since, String>,
    }

    impl Default for LoadSince {
        fn default() -> Self {
            LoadSince {
                expected_reaction_name: String::from("Mock"),
                result: Ok(Since::BeginningOfStream),
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct SaveSince {
        expected_reaction_name: String,
        expected_event_id: EventNumber,
        result: Result<(), String>,
    }

    impl Default for SaveSince {
        fn default() -> Self {
            SaveSince {
                expected_reaction_name: String::from("Mock"),
                expected_event_id: EventNumber::MIN_VALUE,
                result: Ok(()),
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct ReadAllEvents {
        expected_query: Option<String>,
        expected_since: Since,
        expected_params: Option<String>,
        result: Result<Vec<RawEvent>, String>,
    }

    impl Default for ReadAllEvents {
        fn default() -> Self {
            ReadAllEvents {
                expected_query: None,
                expected_since: Since::BeginningOfStream,
                expected_params: None,
                result: Ok(vec![]),
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct MockConnection {
        load_since_data: LoadSince,
        save_since_data: SaveSince,
        read_all_events_data: ReadAllEvents,
    }

    impl Default for MockConnection {
        fn default() -> Self {
            MockConnection {
                load_since_data: LoadSince::default(),
                save_since_data: SaveSince::default(),
                read_all_events_data: ReadAllEvents::default(),
            }
        }
    }

    impl<'conn> DbConnection<'conn> for MockConnection {
        type Error = String;

        fn load_since(&self, reaction_name: &str) -> Result<Since, Self::Error> {
            assert_eq!(reaction_name, self.load_since_data.expected_reaction_name);
            self.load_since_data.result.clone()
        }

        fn save_since(
            &self,
            reaction_name: &str,
            event_id: EventNumber,
        ) -> Result<(), Self::Error> {
            assert_eq!(reaction_name, self.save_since_data.expected_reaction_name);
            assert_eq!(event_id, self.save_since_data.expected_event_id);
            self.save_since_data.result.clone()
        }

        fn read_all_events(
            &self,
            query: &str,
            since: Since,
            params: &[Box<dyn ToSql>],
        ) -> Result<Vec<RawEvent>, Self::Error> {
            assert_eq!(since, self.read_all_events_data.expected_since);

            if let Some(expected_query) = &self.read_all_events_data.expected_query {
                assert_eq!(query, expected_query);
            }

            if let Some(expected_params) = &self.read_all_events_data.expected_params {
                assert_eq!(&format!("{:?}", params), expected_params);
            }

            self.read_all_events_data.result.clone()
        }
    }

    #[derive(Clone, Debug)]
    pub struct MockReaction {
        events: Vec<RawEvent>,
        predicate: ReactionPredicate,
        react_result: Result<(), String>,
    }

    impl MockReaction {
        fn event_count(&self) -> usize {
            self.events.len()
        }
    }

    impl Default for MockReaction {
        fn default() -> Self {
            MockReaction {
                predicate: ReactionPredicate::default(),
                events: vec![],
                react_result: Ok(()),
            }
        }
    }

    impl Reaction for MockReaction {
        type Error = String;

        fn reaction_name() -> &'static str {
            "Mock"
        }

        fn react(&mut self, event: RawEvent) -> Result<(), Self::Error> {
            self.events.push(event);
            self.react_result.clone()
        }

        fn predicate(&self) -> ReactionPredicate {
            self.predicate
        }

        fn interval() -> Duration {
            Duration::from_millis(100)
        }
    }

    #[test]
    fn can_read_all_aggregates_and_all_events() {
        let pool = ok_pool(
            String::from(
                "SELECT event_id, aggregate_type, entity_id, sequence, event_type, payload \
                 FROM events \
                 WHERE event_id > $1 \
                 ORDER BY event_id ASC \
                 LIMIT $2",
            ),
            String::from("[100]"),
        );

        let reaction = MockReaction::default();

        assert_eq!(2, test_reaction(pool, reaction).unwrap());
    }

    #[test]
    fn can_read_specific_aggregates_and_all_events() {
        let pool = ok_pool(
            String::from(
                "SELECT event_id, aggregate_type, entity_id, sequence, event_type, payload \
                 FROM events \
                 WHERE event_id > $1 \
                 AND (FALSE OR (aggregate_type = $2)) \
                 ORDER BY event_id ASC \
                 LIMIT $3",
            ),
            String::from("[\"material_location_availability\", 100]"),
        );

        let reaction = MockReaction {
            predicate: ReactionPredicate {
                aggregate_predicate: AggregatePredicate::SpecificAggregates(&[
                    SpecificAggregatePredicate {
                        aggregate_type: "material_location_availability",
                        event_types: EventTypesPredicate::AllEventTypes,
                    },
                ]),
            },
            ..MockReaction::default()
        };

        assert_eq!(2, test_reaction(pool, reaction).unwrap());
    }

    #[test]
    fn can_read_all_aggregates_and_specific_events() {
        let pool = ok_pool(
            String::from(
                "SELECT event_id, aggregate_type, entity_id, sequence, event_type, payload \
                 FROM events \
                 WHERE event_id > $1 \
                 AND event_type = ANY ($2) \
                 ORDER BY event_id ASC \
                 LIMIT $3",
            ),
            String::from("[[\"sources_updated\"], 100]"),
        );

        let reaction = MockReaction {
            predicate: ReactionPredicate {
                aggregate_predicate: AggregatePredicate::AllAggregates(
                    EventTypesPredicate::SpecificEventTypes(&["sources_updated"]),
                ),
            },
            ..MockReaction::default()
        };

        assert_eq!(2, test_reaction(pool, reaction).unwrap());
    }

    #[test]
    fn can_read_specific_aggregates_and_specific_events() {
        let pool = ok_pool(
            String::from(
                "SELECT event_id, aggregate_type, entity_id, sequence, event_type, payload \
                 FROM events \
                 WHERE event_id > $1 \
                 AND (FALSE OR (aggregate_type = $2 AND event_type = ANY ($3))) \
                 ORDER BY event_id ASC \
                 LIMIT $4",
            ),
            String::from("[\"material_location_availability\", [\"sources_updated\", \"end_of_life_updated\"], 100]"),
        );

        let reaction = MockReaction {
            predicate: ReactionPredicate {
                aggregate_predicate: AggregatePredicate::SpecificAggregates(&[
                    SpecificAggregatePredicate {
                        aggregate_type: "material_location_availability",
                        event_types: EventTypesPredicate::SpecificEventTypes(&[
                            "sources_updated",
                            "end_of_life_updated",
                        ]),
                    },
                ]),
            },
            ..MockReaction::default()
        };

        assert_eq!(2, test_reaction(pool, reaction).unwrap());
    }

    #[test]
    fn get_connection_error() {
        let error_message = "connection pool error";
        let test_error = Err(String::from(error_message));

        let pool = MockPool {
            get_result: test_error,
        };

        let reaction = MockReaction {
            ..MockReaction::default()
        };

        let result = test_reaction(pool, reaction);

        assert!(result.is_err());
        assert_eq!(
            "Pool error during reaction: connection pool error",
            result.unwrap_err().to_string()
        );
    }

    #[test]
    fn load_since_error() {
        let error_message = "load_since error";
        let test_error = Err(String::from(error_message));

        let connection = MockConnection {
            load_since_data: LoadSince {
                result: test_error,
                ..LoadSince::default()
            },
            ..MockConnection::default()
        };

        let pool = MockPool {
            get_result: Ok(connection),
        };

        let reaction = MockReaction {
            ..MockReaction::default()
        };

        let result = test_reaction(pool, reaction);

        assert!(result.is_err());
        assert_eq!(
            "Postgres error during reaction: load_since error",
            result.err().unwrap().to_string()
        );
    }

    #[test]
    fn read_all_events_error() {
        let error_message = "read_all_events error";
        let test_error = Err(String::from(error_message));

        let connection = MockConnection {
            read_all_events_data: ReadAllEvents {
                result: test_error,
                ..ReadAllEvents::default()
            },
            ..MockConnection::default()
        };

        let pool = MockPool {
            get_result: Ok(connection),
        };

        let reaction = MockReaction {
            ..MockReaction::default()
        };

        let result = test_reaction(pool, reaction);

        assert!(result.is_err());
        assert_eq!(
            "Postgres error during reaction: read_all_events error",
            result.err().unwrap().to_string()
        );
    }

    #[test]
    fn react_error() {
        let error_message = "react error";
        let test_error = Err(String::from(error_message));

        let connection = MockConnection {
            read_all_events_data: ReadAllEvents {
                result: Ok(RAW_EVENTS.to_vec()),
                ..ReadAllEvents::default()
            },
            ..MockConnection::default()
        };

        let pool = MockPool {
            get_result: Ok(connection),
        };

        let reaction = MockReaction {
            react_result: test_error,
            ..MockReaction::default()
        };

        let result = test_reaction(pool, reaction);

        assert!(result.is_err());
        assert_eq!(
            "React error during reaction: react error",
            result.err().unwrap().to_string()
        );
    }

    #[test]
    fn save_since_error() {
        let error_message = "save_since error";
        let test_error = Err(String::from(error_message));

        let connection = MockConnection {
            read_all_events_data: ReadAllEvents {
                result: Ok(RAW_EVENTS.to_vec()),
                ..ReadAllEvents::default()
            },
            save_since_data: SaveSince {
                result: test_error,
                ..SaveSince::default()
            },
            ..MockConnection::default()
        };

        let pool = MockPool {
            get_result: Ok(connection),
        };

        let reaction = MockReaction {
            ..MockReaction::default()
        };

        let result = test_reaction(pool, reaction);

        assert!(result.is_err());
        assert_eq!(
            "Postgres error during reaction: save_since error",
            result.err().unwrap().to_string()
        );
    }

    fn ok_pool(expected_query: String, expected_params: String) -> MockPool {
        let connection = MockConnection {
            read_all_events_data: ReadAllEvents {
                expected_query: Some(expected_query),
                expected_params: Some(expected_params),
                result: Ok(RAW_EVENTS.to_vec()),
                ..ReadAllEvents::default()
            },
            ..MockConnection::default()
        };

        MockPool {
            get_result: Ok(connection),
        }
    }

    fn test_reaction(
        pool: MockPool,
        reaction: MockReaction,
    ) -> Result<usize, ReactorError<MockReaction, impl CqrsError, impl CqrsError>> {
        let local_reactor = Arc::new(PostgresReactor::new(pool));
        let thread_reactor = Arc::clone(&local_reactor);

        let handle = thread::spawn(move || thread_reactor.start_reaction(reaction));

        ::std::thread::sleep(Duration::from_millis(50));
        local_reactor.stop_reaction();

        handle.join().unwrap()
    }
}
