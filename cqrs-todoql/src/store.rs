use cqrs::{EventNumber, Precondition, SequencedEvent, StateSnapshot};
use cqrs_data::Since;
use cqrs_data::{EventSink, EventSource, SnapshotSink, SnapshotSource};
use cqrs_data::trivial::NullStore;
use cqrs_data::memory::{EventStore,StateStore};
use cqrs::error::{AppendEventsError};

use cqrs_redis;
use cqrs_todo_core::{Event, TodoAggregate};

use r2d2;
use r2d2_redis::RedisConnectionManager;

use void::ResultVoidExt;

pub enum MemoryOrNullEventStore
{
    Memory(EventStore<TodoAggregate>),
    Null,
    Redis(cqrs_redis::Config, r2d2::Pool<RedisConnectionManager>)
}

impl MemoryOrNullEventStore
{
    pub fn new_memory_store() -> Self {
        MemoryOrNullEventStore::Memory(EventStore::default())
    }

    pub fn new_null_store() -> Self {
        MemoryOrNullEventStore::Null
    }

    pub fn new_redis_store(config: cqrs_redis::Config, pool: r2d2::Pool<RedisConnectionManager>) -> Self {
        MemoryOrNullEventStore::Redis(config, pool)
    }
}

impl EventSource<TodoAggregate> for MemoryOrNullEventStore
{
    type Events = Vec<Result<SequencedEvent<Event>, Self::Error>>;
    type Error = ::redis::RedisError;

    fn read_events<Id: AsRef<str> + Into<String>>(&self, id: Id, since: Since) -> Result<Option<Self::Events>, Self::Error> {
        match *self {
            MemoryOrNullEventStore::Memory(ref mem) => Ok(mem.read_events(id, since).void_unwrap().map(|es| es.into_iter().map(|r| Ok(r.void_unwrap())).collect())),
            MemoryOrNullEventStore::Null => Ok(EventSource::<TodoAggregate>::read_events(&NullStore, id, since).void_unwrap().map(|es| es.into_iter().map(|r| Ok(r.void_unwrap())).collect())),
            MemoryOrNullEventStore::Redis(ref config, ref pool) => {
                let conn = pool.get().unwrap();
                let c = config.with_connection(&*conn);
                let store = c.for_snapshot_with_serializer(SerdeSnapshotSerializer::default());
                let y = EventSource::<TodoAggregate>::read_events(&store, id, since)?;
                Ok(y.map(|x| x.collect()))
            }
        }
    }
}

impl EventSink<TodoAggregate> for MemoryOrNullEventStore
{
    type Error = AppendEventsError<::redis::RedisError>;

    fn append_events<Id: AsRef<str> + Into<String>>(&self, id: Id, events: &[Event], precondition: Option<Precondition>) -> Result<EventNumber, Self::Error> {
        match *self {
            MemoryOrNullEventStore::Memory(ref mem) => Ok(mem.append_events(id, events, precondition).map_err(|::cqrs_data::memory::PreconditionFailed(p)| AppendEventsError::PreconditionFailed(p))?),
            MemoryOrNullEventStore::Null => Ok(EventSink::<TodoAggregate>::append_events(&NullStore, id, events, precondition).void_unwrap()),
            MemoryOrNullEventStore::Redis(ref config, ref pool) => {
                let conn = pool.get().unwrap();
                let c = config.with_connection(&*conn);
                let store = c.for_snapshot_with_serializer(SerdeSnapshotSerializer::default());
                let e = EventSink::<TodoAggregate>::append_events(&store, id, events, precondition)?;
                Ok(e)
            },
        }
    }
}

pub enum MemoryOrNullSnapshotStore
{
    Memory(StateStore<TodoAggregate>),
    Null,
    Redis(cqrs_redis::Config, r2d2::Pool<RedisConnectionManager>)
}

impl MemoryOrNullSnapshotStore
{
    pub fn new_memory_store() -> Self {
        MemoryOrNullSnapshotStore::Memory(StateStore::default())
    }

    pub fn new_null_store() -> Self {
        MemoryOrNullSnapshotStore::Null
    }

    pub fn new_redis_store(config: cqrs_redis::Config, pool: r2d2::Pool<RedisConnectionManager>) -> Self {
        MemoryOrNullSnapshotStore::Redis(config, pool)
    }
}

impl SnapshotSource<TodoAggregate> for MemoryOrNullSnapshotStore
{
    type Error = ::redis::RedisError;

    fn get_snapshot<Id: AsRef<str> + Into<String>>(&self, id: Id) -> Result<Option<StateSnapshot<TodoAggregate>>, Self::Error> {
        match *self {
            MemoryOrNullSnapshotStore::Memory(ref mem) => Ok(mem.get_snapshot(id).void_unwrap()),
            MemoryOrNullSnapshotStore::Null => Ok(NullStore.get_snapshot(id).void_unwrap()),
            MemoryOrNullSnapshotStore::Redis(ref config, ref mgr) => {
                let x = config.with_connection(&*mgr.get().unwrap())
                    .for_snapshot_with_serializer(SerdeSnapshotSerializer::default())
                    .get_snapshot(id)?;
                Ok(x)
            },
        }
    }
}

impl SnapshotSink<TodoAggregate> for MemoryOrNullSnapshotStore
{
    type Error = ::redis::RedisError;

    fn persist_snapshot<Id: AsRef<str> + Into<String>>(&self, id: Id, snapshot: StateSnapshot<TodoAggregate>) -> Result<(), Self::Error> {
        match *self {
            MemoryOrNullSnapshotStore::Memory(ref mem) => Ok(mem.persist_snapshot(id, snapshot).void_unwrap()),
            MemoryOrNullSnapshotStore::Null => Ok(NullStore.persist_snapshot(id, snapshot).void_unwrap()),
            MemoryOrNullSnapshotStore::Redis(ref config, ref mgr) => {
                let data = config.with_connection(&*mgr.get().unwrap())
                    .for_snapshot_with_serializer(SerdeSnapshotSerializer::default())
                    .persist_snapshot(id, snapshot)?;
                Ok(data)
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SerdeSnapshotSerializer<S: ::serde::Serialize + ::serde::de::DeserializeOwned> {
    _phantom: ::std::marker::PhantomData<S>,
}

impl<S: ::serde::Serialize + ::serde::de::DeserializeOwned> Default for SerdeSnapshotSerializer<S> {
    fn default() -> Self {
        SerdeSnapshotSerializer {
            _phantom: ::std::marker::PhantomData,
        }
    }
}

impl<S: ::serde::Serialize + ::serde::de::DeserializeOwned> cqrs_redis::RedisSerializer for SerdeSnapshotSerializer<S> {
    type Value = S;
    type Output = Vec<u8>;
    type Input = Vec<u8>;
    type Error = ::serde_json::Error;

    fn serialize(&self, snapshot: Self::Value) -> Self::Output {
        ::serde_json::to_vec(&snapshot).unwrap()
    }

    fn deserialize(&self, snapshot: Self::Input) -> Result<Self::Value, Self::Error> {
        ::serde_json::from_slice(&snapshot)
    }
}