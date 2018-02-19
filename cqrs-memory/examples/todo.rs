extern crate cqrs;
extern crate cqrs_memory;
extern crate cqrs_todo_core;
extern crate fnv;
//#[macro_use] extern crate nickel;
extern crate iron;
#[macro_use] extern crate juniper;
extern crate juniper_iron;
extern crate mount;
extern crate clap;

use cqrs::trivial::{NullEventStore, NullSnapshotStore, NopEventDecorator};
use cqrs::{Precondition, Since, VersionedEvent, VersionedSnapshot, EventAppend, SnapshotPersist};
use cqrs::domain::query::QueryableSnapshotAggregate;
use cqrs::domain::execute::ViewExecutor;
use cqrs::domain::persist::{PersistableSnapshotAggregate, AggregateCommand};
use cqrs::domain::{HydratedAggregate, AggregatePrecondition};
use cqrs::error::{ExecuteError, ExecuteAndPersistError, AppendEventsError, Never};
use cqrs_memory::{MemoryEventStore, MemoryStateStore};

use std::borrow::Borrow;
use std::io::Read;
use std::error::Error as StdError;
use std::time::{Instant, Duration};
use std::sync::Arc;

use cqrs_todo_core::{Event, TodoAggregate, TodoState, TodoData, TodoStatus, Command};
use cqrs_todo_core::domain;

use mount::Mount;
use juniper::{FieldResult, Value};
use juniper::EmptyMutation;
use juniper_iron::GraphQLHandler;

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
    query: Arc<View>,
    command: Arc<Commander>,
}

impl juniper::Context for Context {}

struct Query;

graphql_object!(Query: Context |&self| {
    field apiVersion() -> &str {
        "1.0"
    }

    field todo(&executor, id: i32) -> FieldResult<TodoQL> {
        let context = executor.context();

        let value = context.query.rehydrate(&(id as usize))?;

        let hydro = value.ok_or("not found")?;

        Ok(TodoQL(hydro))
    }
});

struct TodoQL(HydratedAggregate<TodoAggregate>);

graphql_object!(TodoQL: Context |&self| {
    field description() -> FieldResult<&str> {
        Ok(self.0.inspect_aggregate().inspect_state().get_data()?.description.as_str())
    }

    field reminder() -> FieldResult<Option<ReminderQL>> {
        Ok(self.0.inspect_aggregate().inspect_state().get_data()?.reminder.map(|r| ReminderQL(r)))
    }

    field completed() -> FieldResult<bool> {
        Ok(self.0.inspect_aggregate().inspect_state().get_data()?.status == TodoStatus::Completed)
    }

    field version() -> FieldResult<String> {
        Ok(self.0.get_version().to_string())
    }
});

struct ReminderQL(cqrs_todo_core::domain::Reminder);

graphql_scalar!(ReminderQL {
    description: "A reminder"

    resolve(&self) -> Value {
        Value::string("a reminder")
    }

    from_input_value(v: &InputValue) -> Option<ReminderQL> {
        if v.as_string_value() == Some("now") {
            Some(ReminderQL(cqrs_todo_core::domain::Reminder::new(Instant::now(), Instant::now() + Duration::from_secs(100)).unwrap()))
        } else {
            None
        }
    }
});

struct Mutations;

graphql_object!(Mutations: Context |&self| {
    field todo(id: i32) -> FieldResult<TodoMutQL> {
        Ok(TodoMutQL(id as usize))
    }
});

struct TodoMutQL(usize);

graphql_object!(TodoMutQL: Context |&self| {
    field set_description(&executor, text: String) -> FieldResult<&str> {
        let description = domain::Description::new(text)?;

        let command = Command::UpdateText(description);

        executor.context().command.execute_and_persist_with_decorator(&self.0, command, Some(AggregatePrecondition::Exists), Default::default())?;

        Ok("Done.")
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

    let time_0 = Instant::now();
    let time_1 = time_0 + Duration::from_secs(10000);
    let mut events = Vec::new();
    events.push(Event::Completed);
    events.push(Event::Created(domain::Description::new("Hello!").unwrap()));
    events.push(Event::ReminderUpdated(Some(domain::Reminder::new(time_1, time_0).unwrap())));
    events.push(Event::TextUpdated(domain::Description::new("New text").unwrap()));
    events.push(Event::Created(domain::Description::new("Ignored!").unwrap()));
    events.push(Event::ReminderUpdated(None));

    es.append_events(&0, &events, None).unwrap();

    ss.persist_snapshot(&0, VersionedSnapshot {
        version: 1.into(),
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


    let query = Arc::new(view);
    let command = Arc::new(command);

    let context_factory = move |_: &mut iron::Request| {
        Context {
            query: Arc::clone(&query),
            command: Arc::clone(&command),
        }
    };

    let mut mount = Mount::new();

    let graphql_endpoint = GraphQLHandler::new(
        context_factory,
        Query,
        Mutations,
    );

    mount.mount("/graphql", graphql_endpoint);

    let chain = iron::Chain::new(mount);

    iron::Iron::new(chain).http("0.0.0.0:2777").unwrap();

    println!("---DONE---");
}

