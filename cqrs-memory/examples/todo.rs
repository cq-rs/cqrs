extern crate cqrs;
extern crate cqrs_memory;
extern crate cqrs_todo_core;
extern crate base64;
extern crate fnv;
//#[macro_use] extern crate nickel;
extern crate chrono;
extern crate iron;
#[macro_use] extern crate juniper;
#[macro_use] extern crate juniper_codegen;
extern crate juniper_iron;
extern crate mount;
extern crate clap;

use cqrs::trivial::{NullEventStore, NullSnapshotStore};
use cqrs::{Precondition, Since, VersionedEvent, VersionedSnapshot, EventAppend, SnapshotPersist, Version};
use cqrs::domain::query::QueryableSnapshotAggregate;
use cqrs::domain::execute::ViewExecutor;
use cqrs::domain::persist::{PersistableSnapshotAggregate, AggregateCommand};
use cqrs::domain::ident::{AggregateIdProvider, UsizeIdProvider};
use cqrs::domain::{HydratedAggregate, AggregatePrecondition, AggregateVersion};
use cqrs::error::{AppendEventsError, Never};
use cqrs_memory::{MemoryEventStore, MemoryStateStore};

use std::sync::{Arc,RwLock};
use std::boxed::Box;

use cqrs_todo_core::{Event, TodoAggregate, TodoState, TodoData, TodoStatus, Command};
use cqrs_todo_core::domain;

use mount::Mount;
use juniper::{Value, FieldResult};
use juniper_iron::GraphQLHandler;
use chrono::prelude::*;
use chrono::Duration;

use iron::headers::ContentType;
use iron::mime::{Mime, TopLevel, SubLevel};

use clap::{App, Arg};

pub enum MemoryOrNullEventStore<E, I, H = fnv::FnvBuildHasher>
    where
        E: Clone,
        I: ::std::hash::Hash + Eq + Clone,
        H: ::std::hash::BuildHasher,
{
    Memory(MemoryEventStore<E, I, H>),
    Null(NullEventStore<E, I>),
}

impl<E, I, H> cqrs::EventSource for MemoryOrNullEventStore<E, I, H>
    where
        E: Clone,
        I: ::std::hash::Hash + Eq + Clone,
        H: ::std::hash::BuildHasher,
{
    type AggregateId = I;
    type Event = E;
    type Events = Vec<VersionedEvent<Self::Event>>;
    type Error = Never;

    fn read_events(&self, agg_id: &Self::AggregateId, since: Since) -> Result<Option<Self::Events>, Self::Error> {
        match *self {
            MemoryOrNullEventStore::Memory(ref mem) => mem.read_events(agg_id, since),
            MemoryOrNullEventStore::Null(ref nil) => nil.read_events(agg_id, since),
        }
    }
}

impl<E, I, H> cqrs::EventAppend for MemoryOrNullEventStore<E, I, H>
    where
        E: Clone,
        I: ::std::hash::Hash + Eq + Clone,
        H: ::std::hash::BuildHasher,
{
    type AggregateId = I;
    type Event = E;
    type Error = AppendEventsError<Never>;

    fn append_events(&self, agg_id: &Self::AggregateId, events: &[Self::Event], precondition: Option<Precondition>) -> Result<(), Self::Error> {
        match *self {
            MemoryOrNullEventStore::Memory(ref mem) => mem.append_events(agg_id, events, precondition),
            MemoryOrNullEventStore::Null(ref nil) => nil.append_events(agg_id, events, precondition),
        }
    }
}

pub enum MemoryOrNullSnapshotStore<S, I, H = fnv::FnvBuildHasher>
    where
        S: Clone,
        I: ::std::hash::Hash + Eq + Clone,
        H: ::std::hash::BuildHasher,
{
    Memory(MemoryStateStore<S, I, H>),
    Null(NullSnapshotStore<S, I>),
}

impl<S, I, H> cqrs::SnapshotSource for MemoryOrNullSnapshotStore<S, I, H>
    where
        S: Clone,
        I: ::std::hash::Hash + Eq + Clone,
        H: ::std::hash::BuildHasher,
{
    type AggregateId = I;
    type Snapshot = S;
    type Error = Never;

    fn get_snapshot(&self, agg_id: &Self::AggregateId) -> Result<Option<VersionedSnapshot<S>>, Self::Error> {
        match *self {
            MemoryOrNullSnapshotStore::Memory(ref mem) => mem.get_snapshot(agg_id),
            MemoryOrNullSnapshotStore::Null(ref nil) => nil.get_snapshot(agg_id),
        }
    }
}

impl<S, I, H> cqrs::SnapshotPersist for MemoryOrNullSnapshotStore<S, I, H>
    where
        S: Clone,
        I: ::std::hash::Hash + Eq + Clone,
        H: ::std::hash::BuildHasher,
{
    type AggregateId = I;
    type Snapshot = S;
    type Error = Never;

    fn persist_snapshot(&self, agg_id: &Self::AggregateId, snapshot: VersionedSnapshot<S>) -> Result<(), Self::Error> {
        match *self {
            MemoryOrNullSnapshotStore::Memory(ref mem) => mem.persist_snapshot(agg_id, snapshot),
            MemoryOrNullSnapshotStore::Null(ref nil) => nil.persist_snapshot(agg_id, snapshot),
        }
    }
}

type View =
    cqrs::domain::query::SnapshotAndEventsView<
        TodoAggregate,
        Arc<MemoryOrNullEventStore<
            Event,
            usize,
            fnv::FnvBuildHasher,
        >>,
        Arc<MemoryOrNullSnapshotStore<
            TodoState,
            usize,
            fnv::FnvBuildHasher,
        >>,
    >;

type Commander =
    cqrs::domain::persist::EventsAndSnapshotWithDecorator<
        TodoAggregate,
        cqrs::domain::execute::ViewExecutor<
            TodoAggregate,
            View,
        >,
        Arc<MemoryOrNullEventStore<
            Event,
            usize,
            fnv::FnvBuildHasher,
        >>,
        Arc<MemoryOrNullSnapshotStore<
            TodoState,
            usize,
            fnv::FnvBuildHasher,
        >>,
        cqrs::trivial::NopEventDecorator<Event>,
    >;

struct Context {
    stream_index: Arc<RwLock<Vec<usize>>>,
    query: Arc<View>,
    command: Arc<Commander>,
    id_provider: Arc<UsizeIdProvider>,
}

impl juniper::Context for Context {}

fn parse_id(id: juniper::ID) -> Result<usize,::std::num::ParseIntError> {
    Ok(id.parse()?)
}

struct Query;

graphql_object!(Query: Context |&self| {
    field apiVersion() -> &str {
        "1.0"
    }

    field allTodos(&executor, first: Option<juniper::ID>, after: Option<Cursor>) -> FieldResult<TodoPage> {
        let context = executor.context();

        let first_idx =
            if let Some(Cursor(idx)) = after {
                idx + 1
            } else if let Some(id) = first {
                parse_id(id)?
            } else { 0 };

        let reader = context.stream_index.read().unwrap();
        let len = reader.len();

        let mut items = Vec::default();
        let mut end_cursor = None;
        for agg_id in reader.iter().skip(first_idx).take(5) {
            let cursor = Cursor(*agg_id);
            items.push(TodoEdge {
                agg_id: *agg_id,
                cursor,
            });
            end_cursor = Some(cursor);
        }
        Ok(TodoPage {
            total_count: len,
            page_info: PageInfo {
                has_next_page: items.len() != 0,
                end_cursor,
            },
            edges: items,
        })
    }

    field todo(&executor, id: juniper::ID) -> FieldResult<Option<TodoQL>> {
        let context = executor.context();
        let int_id = parse_id(id)?;

        let rehydrate_result = context.query.rehydrate(&int_id)?;

        Ok(rehydrate_result.map(|agg| TodoQL(int_id, agg)))
    }
});

struct TodoQL(usize, HydratedAggregate<TodoAggregate>);

graphql_object!(TodoQL: Context |&self| {
    field id() -> FieldResult<juniper::ID> {
        Ok(self.0.to_string().into())
    }

    field description() -> FieldResult<&str> {
        Ok(self.1.inspect_aggregate().inspect_state().get_data()?.description.as_str())
    }

    field reminder() -> FieldResult<Option<DateTime<Utc>>> {
        Ok(self.1.inspect_aggregate().inspect_state().get_data()?.reminder.map(|r| r.get_time()))
    }

    field completed() -> FieldResult<bool> {
        Ok(self.1.inspect_aggregate().inspect_state().get_data()?.status == TodoStatus::Completed)
    }

    field version() -> FieldResult<String> {
        Ok(self.1.get_version().to_string())
    }
});

struct TodoPage {
    total_count: usize,
    edges: Vec<TodoEdge>,
    page_info: PageInfo,
}

graphql_object!(TodoPage: Context |&self| {
    field total_count() -> FieldResult<i32> {
        Ok(self.total_count as i32)
    }

    field edges() -> FieldResult<&[TodoEdge]> {
        Ok(&*self.edges)
    }

    field page_info() -> FieldResult<&PageInfo> {
        Ok(&self.page_info)
    }
});

struct TodoEdge {
    agg_id: usize,
    cursor: Cursor,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Cursor(usize);

impl ToString for Cursor {
    fn to_string(&self) -> String {
        base64::encode(&self.0.to_string())
    }
}

graphql_scalar!(Cursor {
    description: "An opaque identifier, represented as a location in an enumeration"

    resolve(&self) -> Value {
        Value::string(self.to_string())
    }

    from_input_value(v: &InputValue) -> Option<Cursor> {
        v.as_string_value()
            .and_then(|v| base64::decode(v).ok())
            .and_then(|v| String::from_utf8_lossy(&v).parse::<usize>().ok())
            .map(Cursor)
    }
});

graphql_object!(TodoEdge: Context |&self| {
    field node(&executor) -> FieldResult<Option<TodoQL>> {
        let rehydrate_result = executor.context().query.rehydrate(&self.agg_id)?;

        Ok(rehydrate_result.map(|agg| TodoQL(self.agg_id, agg)))
    }

    field cursor() -> FieldResult<Cursor> {
        Ok(self.cursor)
    }
});

#[derive(GraphQLObject)]
struct PageInfo {
    has_next_page: bool,
    end_cursor: Option<Cursor>,
}

struct Mutations;

graphql_object!(Mutations: Context |&self| {
    field todo(id: juniper::ID) -> FieldResult<TodoMutQL> {
        Ok(TodoMutQL(parse_id(id)?))
    }

    field new_todo(&executor, text: String, reminder_time: Option<DateTime<Utc>>) -> FieldResult<TodoQL> {
        let context = executor.context();

        let description = domain::Description::new(text)?;
        let reminder =
            if let Some(time) = reminder_time {
                Some(domain::Reminder::new(time, Utc::now())?)
            } else { None };


        let command = Command::Create(description, reminder);

        let new_id = context.id_provider.new_id();
        let agg = context.command
            .execute_and_persist_with_decorator(&new_id, command, Some(AggregatePrecondition::New), Default::default())?;

        context.stream_index.write().unwrap().push(new_id);

        Ok(TodoQL(new_id, agg))
    }

});

struct TodoMutQL(usize);

fn i32_as_aggregate_version(version_int: i32) -> AggregateVersion {
    if version_int < 0 {
        AggregateVersion::Initial
    } else {
        AggregateVersion::Version(Version::new(version_int as usize))
    }
}

fn expect_exists_or(expected_version: Option<i32>) -> AggregatePrecondition {
    expected_version
        .map(i32_as_aggregate_version)
        .map(AggregatePrecondition::ExpectedVersion)
        .unwrap_or(AggregatePrecondition::Exists)
}

graphql_object!(TodoMutQL: Context |&self| {
    field set_description(&executor, text: String, expected_version: Option<i32>) -> FieldResult<TodoQL> {
        let expectation = expect_exists_or(expected_version);

        let description = domain::Description::new(text)?;

        let command = Command::UpdateText(description);

        let agg = executor.context().command
            .execute_and_persist_with_decorator(&self.0, command, Some(expectation), Default::default())?;

        Ok(TodoQL(self.0, agg))
    }

    field set_reminder(&executor, time: DateTime<Utc>, expected_version: Option<i32>) -> FieldResult<TodoQL> {
        let expectation = expect_exists_or(expected_version);

        let reminder = domain::Reminder::new(time, Utc::now())?;

        let command = Command::SetReminder(reminder);

        let agg = executor.context().command
            .execute_and_persist_with_decorator(&self.0, command, Some(expectation), Default::default())?;

        Ok(TodoQL(self.0, agg))
    }

    field cancel_reminder(&executor, expected_version: Option<i32>) -> FieldResult<TodoQL> {
        let expectation = expect_exists_or(expected_version);

        let command = Command::CancelReminder;

        let agg = executor.context().command
            .execute_and_persist_with_decorator(&self.0, command, Some(expectation), Default::default())?;

        Ok(TodoQL(self.0, agg))
    }

    field toggle(&executor, expected_version: Option<i32>) -> FieldResult<TodoQL> {
        let expectation = expect_exists_or(expected_version);

        let command = Command::ToggleCompletion;

        let agg = executor.context().command
            .execute_and_persist_with_decorator(&self.0, command, Some(expectation), Default::default())?;

        Ok(TodoQL(self.0, agg))
    }

    field reset(&executor, expected_version: Option<i32>) -> FieldResult<TodoQL> {
        let expectation = expect_exists_or(expected_version);

        let command = Command::ResetCompleted;

        let agg = executor.context().command
            .execute_and_persist_with_decorator(&self.0, command, Some(expectation), Default::default())?;

        Ok(TodoQL(self.0, agg))
    }

    field complete(&executor, expected_version: Option<i32>) -> FieldResult<TodoQL> {
        let expectation = expect_exists_or(expected_version);

        let command = Command::MarkCompleted;

        let agg = executor.context().command
            .execute_and_persist_with_decorator(&self.0, command, Some(expectation), Default::default())?;

        Ok(TodoQL(self.0, agg))
    }
});

fn main() {
    let app = App::new("todo")
        .arg(Arg::with_name("null-event-store")
            .long("null-event-store")
            .takes_value(false)
            .help("Use null event store")
            .long_help("Operates with an event store that stores nothing and never returns any events."))
        .arg(Arg::with_name("null-snapshot-store")
            .long("null-snapshot-store")
            .takes_value(false)
            .help("Use null snapshot store")
            .long_help("Operates with a snapshot store that stores nothing and never returns a snapshot."));

    let matches = app.get_matches();

    let es: Arc<MemoryOrNullEventStore<Event, usize, fnv::FnvBuildHasher>> =
        if matches.is_present("null-event-store") {
            Arc::new(MemoryOrNullEventStore::Null(NullEventStore::<Event, usize>::default()))
        } else {
            Arc::new(MemoryOrNullEventStore::Memory(MemoryEventStore::<Event, usize, fnv::FnvBuildHasher>::default()))
        };

    let ss: Arc<MemoryOrNullSnapshotStore<TodoState, usize, fnv::FnvBuildHasher>> =
        if matches.is_present("null-snapshot-store") {
            Arc::new(MemoryOrNullSnapshotStore::Null(NullSnapshotStore::<TodoState, usize>::default()))
        } else {
            Arc::new(MemoryOrNullSnapshotStore::Memory(MemoryStateStore::<TodoState, usize, fnv::FnvBuildHasher>::default()))
        };
    let id_provider = Arc::new(UsizeIdProvider::default());

    let epoch = Utc.ymd(1970, 1, 1).and_hms(0, 0, 0);
    let reminder_time = epoch + Duration::seconds(10000);
    let mut events = Vec::new();
    events.push(Event::Completed);
    events.push(Event::Created(domain::Description::new("Hello!").unwrap()));
    events.push(Event::ReminderUpdated(Some(domain::Reminder::new(reminder_time, epoch).unwrap())));
    events.push(Event::TextUpdated(domain::Description::new("New text").unwrap()));
    events.push(Event::Created(domain::Description::new("Ignored!").unwrap()));
    events.push(Event::ReminderUpdated(None));

    let id = id_provider.new_id();

    es.append_events(&id, &events, None).unwrap();

    ss.persist_snapshot(&id, VersionedSnapshot {
        version: Version::from(1),
        snapshot: TodoState::Created(TodoData {
            description: domain::Description::new("Hello!").unwrap(),
            reminder: None,
            status: TodoStatus::NotCompleted,
        })
    }).unwrap();

    let view = TodoAggregate::snapshot_with_events_view(Arc::clone(&es), Arc::clone(&ss));

    let executor = ViewExecutor::new(TodoAggregate::snapshot_with_events_view(Arc::clone(&es), Arc::clone(&ss)));
    let command =
        TodoAggregate::persist_events_and_snapshot(executor, Arc::clone(&es), Arc::clone(&ss))
            .without_decorator();

    let stream_index = Arc::new(RwLock::new(vec![id]));
    let query = Arc::new(view);
    let command = Arc::new(command);

    let context_factory = move |_: &mut iron::Request| {
        Context {
            stream_index: Arc::clone(&stream_index),
            query: Arc::clone(&query),
            command: Arc::clone(&command),
            id_provider: Arc::clone(&id_provider),
        }
    };

    let mut mount = Mount::new();

    let graphql_endpoint = GraphQLHandler::new(
        context_factory,
        Query,
        Mutations,
    );

    mount.mount("/graphql", graphql_endpoint);
    mount.mount("/graphiql", |_req: &mut iron::Request| {
        let mut res = iron::Response::new();
        res.body = Some(Box::new(
        juniper::http::graphiql::graphiql_source("http://127.0.0.1:2777/graphql")));
        res.headers.set(ContentType(Mime(TopLevel::Text, SubLevel::Html, Vec::default())));
        Ok(res)
    });

    let chain = iron::Chain::new(mount);

    iron::Iron::new(chain).http("0.0.0.0:2777").unwrap();

    println!("---DONE---");
}

