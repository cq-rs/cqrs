extern crate cqrs;
extern crate cqrs_memory;
extern crate cqrs_todo_core;
extern crate fnv;
#[macro_use] extern crate nickel;
extern crate clap;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;

use cqrs::trivial::{NullEventStore, NullSnapshotStore, NopEventDecorator};
use cqrs::{Precondition, Since, VersionedEvent, VersionedSnapshot, Version, EventSource, EventAppend, SnapshotPersist, SnapshotSource};
use cqrs::domain::command::{DecoratedAggregateCommand, PersistAndSnapshotAggregateCommander};
use cqrs::domain::query::{AggregateQuery, SnapshotPlusEventsAggregateView};
use cqrs::domain::HydratedAggregate;
use cqrs::error::{CommandAggregateError, LoadAggregateError, PersistAggregateError, AppendEventsError, Never};
use cqrs_memory::{MemoryEventStore, MemoryStateStore};

use std::borrow::Borrow;
use std::error::Error as StdError;
use std::time::{Instant, Duration};
use std::sync::Arc;

use cqrs_todo_core::{Event, TodoAggregate, TodoState, TodoStatus, Command};
use cqrs_todo_core::domain;
use cqrs_todo_core::error;

use nickel::{Nickel, HttpRouter};
use nickel::status::StatusCode;
use nickel::MediaType;
use clap::{App, Arg};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
struct TodoDto<'a> {
    description: &'a str,
    reminder: Option<&'a str>,
    is_complete: bool,
}

trait EventStore {
    type Event;
    type AggregateId;
    type ReadError: StdError;
    type AppendError: StdError;
}

impl<T, Event, AggId> EventStore for T
    where
        T: cqrs::EventSource<Event=Event, AggregateId=AggId> + cqrs::EventAppend<Event=Event, AggregateId=AggId> + Sized,
        <T as cqrs::EventSource>::Error: StdError,
        <T as cqrs::EventAppend>::Error: StdError,
{
    type Event = Event;
    type AggregateId = AggId;
    type ReadError = <T as cqrs::EventSource>::Error;
    type AppendError = <T as cqrs::EventAppend>::Error;
}

trait SnapshotStore {
    type Snapshot;
    type AggregateId;
    type ReadError: StdError;
    type PersistError: StdError;
}

impl<T, Snapshot, AggId> SnapshotStore for T
    where
        T: cqrs::SnapshotSource<Snapshot=Snapshot, AggregateId=AggId> + cqrs::SnapshotPersist<Snapshot=Snapshot, AggregateId=AggId>,
        <T as cqrs::SnapshotSource>::Error: StdError,
        <T as cqrs::SnapshotPersist>::Error: StdError,
{
    type Snapshot = Snapshot;
    type AggregateId = AggId;
    type ReadError = <T as cqrs::SnapshotSource>::Error;
    type PersistError = <T as cqrs::SnapshotPersist>::Error;
}

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

    fn append_events(&self, agg_id: &Self::AggregateId, events: &[Self::Event], precondition: Precondition) -> Result<(), Self::Error> {
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

    fn persist_snapshot(&self, agg_id: &Self::AggregateId, version: Version, snapshot: S) -> Result<(), Self::Error> {
        match *self {
            MemoryOrNullSnapshotStore::Memory(ref mem) => mem.persist_snapshot(agg_id, version, snapshot),
            MemoryOrNullSnapshotStore::Null(ref nil) => nil.persist_snapshot(agg_id, version, snapshot),
        }
    }
}

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

    es.append_events(&0, &events, Precondition::Always).unwrap();

    let view = SnapshotPlusEventsAggregateView::new(Arc::clone(&es), Arc::clone(&ss));
    let command_view = SnapshotPlusEventsAggregateView::new(Arc::clone(&es), Arc::clone(&ss));
    let command  : PersistAndSnapshotAggregateCommander<TodoAggregate, _, _, _> = PersistAndSnapshotAggregateCommander::new(command_view, Arc::clone(&es), Arc::clone(&ss));

    let view = Arc::new(view);
    let command = Arc::new(command);

    let get_view = Arc::clone(&view);

    let mut server = Nickel::new();

    server.utilize(router! {
        get "/todo/:id" => |req, mut res| {
            match req.param("id").unwrap().parse::<usize>() {
                Ok(id) => {
                    let x: HydratedAggregate<TodoAggregate> = get_view.rehydrate(&id).unwrap();
                    if let TodoState::Created(ref data) = *x.inspect_aggregate().inspect_state() {
                        let ret = TodoDto {
                            description: Borrow::<str>::borrow(&data.description),
                            reminder: data.reminder.map(|_| "yes"),
                            is_complete: data.status == TodoStatus::Completed,
                        };
                        res.set(MediaType::Json);
                        let h = nickel::hyper::header::Link::new(vec![nickel::hyper::header::LinkValue::new(x.get_version().to_string())]);
                        res.headers_mut().set(h);
                        serde_json::to_string(&ret).unwrap()
                    } else {
                        res.set(StatusCode::NotFound);
                        format!("Nice try with {}, but it wasn't there.", req.param("id").unwrap())
                    }
                }
                Err(_) => {
                    res.set(StatusCode::NotFound);
                    "That wasn't a valid identifier".to_owned()
                }
            }
        }
        get "/todo/:id/:description" => |req, mut res| {
            match req.param("id").unwrap().parse::<usize>() {
                Ok(id) => {
                    match domain::Description::new(req.param("description").unwrap()) {
                        Ok(desc) => {
                            let r = command.execute_with_decorator(&id, Command::UpdateText(desc), NopEventDecorator::<Event>::default());
                            match r {
                                Ok(event_count) => format!("Generated {} new events", event_count),
                                Err(e) => {
                                    res.set(StatusCode::BadRequest);
                                    format!("Error: {}", e)
                                }
                            }
                        }
                        Err(_) => {
                            res.set(StatusCode::BadRequest);
                            "Invalid description".to_owned()
                        }
                    }
                }
                Err(_) => {
                    res.set(StatusCode::NotFound);
                    "I don't know that one".to_owned()
                }
            }
        }
    });

    server.listen("127.0.0.1:2777");

    println!("---DONE---");
}

